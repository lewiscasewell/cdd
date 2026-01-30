use crate::filesystem::normalize_path;
use crate::parser::{get_imports_from_file, ParserOptions};
use crate::tsconfig::PathAliases;
use crate::workspace::Workspace;

use colored::*;
use log::{debug, info, warn};
use petgraph::algo::kosaraju_scc;
use petgraph::Graph;
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

fn get_static_extension_list() -> Vec<String> {
    vec![
        ".tsx".to_string(),
        ".ts".to_string(),
        ".jsx".to_string(),
        ".js".to_string(),
        ".cjs".to_string(),
        ".mjs".to_string(),
    ]
}

/// Builds the dependency graph from a list of files.
/// Handles relative imports, path aliases, and workspace packages. Parses files in parallel for performance.
pub fn build_dependency_graph(
    files: &[PathBuf],
    options: &ParserOptions,
    path_aliases: Option<&PathAliases>,
    workspace: Option<&Workspace>,
) -> Graph<PathBuf, ()> {
    let mut graph = Graph::new();
    let mut node_indices = HashMap::new();

    // Dynamically generate the extensions list
    let extensions = get_static_extension_list();

    // Insert all files as nodes
    for file in files {
        let idx = graph.add_node(file.clone());
        node_indices.insert(file.clone(), idx);
        debug!("Added node: {:?}", file);
    }

    // Parse files in parallel and collect imports
    let file_imports: Vec<_> = files
        .par_iter()
        .map(|file| {
            let imports = get_imports_from_file(file, options);
            (file, imports)
        })
        .collect();

    // Build edges from the collected imports (must be sequential for graph mutation)
    for (file, imports) in file_imports {
        debug!("Processing file: {:?}", file);
        for import in imports {
            if let Some(resolved) = resolve_import(file, &import, &extensions, path_aliases, workspace) {
                if let Some(&to_idx) = node_indices.get(&resolved) {
                    let from_idx = node_indices[file];
                    graph.add_edge(from_idx, to_idx, ());
                    debug!("Added edge: {:?} -> {:?}", file, resolved);
                } else {
                    warn!("Resolved import not found in node_indices: {:?}", resolved);
                }
            } else {
                debug!(
                    "Skipped external or unresolved import '{}' from {:?}",
                    import, file
                );
            }
        }
    }

    graph
}

/// Resolves an import to an absolute, normalized PathBuf.
/// Handles relative imports, path aliases, and workspace packages.
/// Returns `None` if the import cannot be resolved.
fn resolve_import(
    base: &Path,
    import: &str,
    extensions: &[String],
    path_aliases: Option<&PathAliases>,
    workspace: Option<&Workspace>,
) -> Option<PathBuf> {
    debug!("Attempting to resolve import: '{}' from {:?}", import, base);

    // Try relative imports first
    if import.starts_with('.') {
        let candidate = base.parent()?.join(import);
        if let Some(resolved) = check_candidates(candidate, extensions) {
            return Some(resolved);
        }
    }

    // Try path aliases if configured
    if let Some(aliases) = path_aliases {
        if let Some(candidate) = aliases.resolve(import) {
            if let Some(resolved) = check_candidates(candidate, extensions) {
                return Some(resolved);
            }
        }
    }

    // Try workspace package resolution
    if let Some(ws) = workspace {
        if let Some(resolved) = ws.resolve(import) {
            let normalized = normalize_path(&resolved);
            debug!("Resolved workspace import '{}' to {:?}", import, normalized);
            return Some(normalized);
        }
    }

    None
}

fn handle_if_file(path: &Path) -> Option<PathBuf> {
    if path.is_file() {
        let canonical = normalize_path(path);
        debug!("check_candidates: Found file directly {:?}", canonical);
        return Some(canonical);
    }
    None
}

/// Checks various possibilities for the import path.
/// Returns the resolved, canonicalized PathBuf if found.
fn check_candidates(candidate: PathBuf, extensions: &[String]) -> Option<PathBuf> {
    if let Some(canonical) = handle_if_file(&candidate) {
        return Some(canonical);
    }

    for ext in extensions {
        let file_name = candidate
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();

        let new_file_name = format!("{}{}", file_name, ext);
        let new_path = candidate.with_file_name(&new_file_name);

        if let Some(canonical) = handle_if_file(&new_path) {
            return Some(canonical);
        }
    }

    // If candidate is a directory, try index files
    if candidate.is_dir() {
        let index_extensions = get_static_extension_list();

        for idx_ext in index_extensions {
            let idx_file = candidate.join(format!("index{}", idx_ext));
            if let Some(canonical) = handle_if_file(&idx_file) {
                return Some(canonical);
            }
        }
    }

    debug!(
        "check_candidates: Could not resolve candidate {:?}",
        candidate
    );
    None
}

/// Finds all strongly connected components (cycles) in the dependency graph.
/// Each SCC with more than one node or a self-loop is considered a cycle.
pub fn find_all_cycles(graph: &Graph<PathBuf, ()>) -> Vec<Vec<&PathBuf>> {
    let sccs = kosaraju_scc(graph);
    let mut cycles = Vec::new();

    for scc in sccs {
        if scc.len() > 1 {
            // A strongly connected component with more than one node indicates a cycle
            let cycle = scc
                .iter()
                .map(|node_index| {
                    graph
                        .node_weight(*node_index)
                        .expect("node index from SCC must exist in graph")
                })
                .collect::<Vec<_>>();
            cycles.push(cycle);
        } else {
            // Check for self-loop
            let node = scc[0];
            if graph.contains_edge(node, node) {
                cycles.push(vec![graph
                    .node_weight(node)
                    .expect("node index from SCC must exist in graph")]);
                debug!(
                    "Detected self-loop cycle for node: {:?}",
                    graph
                        .node_weight(node)
                        .expect("node index from SCC must exist in graph")
                );
            }
        }
    }

    cycles
}

/// Deduplicates cycles by creating a canonical representation for each cycle.
/// Each cycle is rotated so that the lexicographically smallest PathBuf is first.
fn deduplicate_cycles<'a>(cycles: &[Vec<&'a PathBuf>]) -> Vec<Vec<&'a PathBuf>> {
    let mut seen = HashSet::new();
    let mut unique_cycles = Vec::new();

    for cycle in cycles {
        if cycle.is_empty() {
            continue;
        }

        // Find the lex smallest path in the cycle
        let min_path = cycle
            .iter()
            .min_by_key(|path| path.to_string_lossy())
            .expect("cycle is non-empty, checked above");
        let min_index = cycle
            .iter()
            .position(|&path| path == *min_path)
            .expect("min_path came from cycle, so it must exist");

        // Rotate the cycle so that min_path is first
        let rotated_cycle: Vec<&PathBuf> = cycle[min_index..]
            .iter()
            .chain(cycle[..min_index].iter())
            .cloned()
            .collect();

        // Create a unique key for the cycle
        let key = rotated_cycle
            .iter()
            .map(|path| path.to_string_lossy())
            .collect::<Vec<_>>()
            .join(" > ");

        // Check if this cycle has already been seen
        if !seen.contains(&key) {
            seen.insert(key.clone());
            unique_cycles.push(rotated_cycle);
            debug!("Unique cycle added: {}", key);
        }
    }

    unique_cycles
}

/// Integrates cycle finding using Kosaraju's algorithm and deduplication.
pub fn get_unique_cycles(graph: &Graph<PathBuf, ()>) -> Vec<Vec<&PathBuf>> {
    let cycles = find_all_cycles(graph);
    deduplicate_cycles(&cycles)
}

/// Prints the detected cycles in a Madge-like format with relative paths.
pub fn print_cycles(cycles: &[Vec<&PathBuf>], root: &Path) {
    if cycles.is_empty() {
        log::info!("{}", "no circular dependencies found.".green().bold());
        return;
    }

    info!(
        "✖ Found {} circular dependencies!\n",
        cycles.len().to_string().red()
    );
    for (i, cycle) in cycles.iter().enumerate() {
        let relative_paths: Vec<String> = cycle
            .iter()
            .map(|p| p.strip_prefix(root).unwrap_or(p).display().to_string())
            .collect();
        info!(
            "{}) {}",
            (i + 1).to_string().bright_blue().bold(),
            relative_paths.join(&" > ".bright_blue().bold().to_string())
        );
    }
}
