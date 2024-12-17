use petgraph::algo::kosaraju_scc;
use petgraph::Graph;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::parser::get_imports_from_file;

use log::{debug, warn};

/// Normalizes a path by resolving it to an absolute path
/// and removing redundant components.
fn normalize_path(path: &PathBuf) -> PathBuf {
    match path.canonicalize() {
        Ok(canonical) => canonical,
        Err(_) => path.clone(),
    }
}

/// Generates a list of unique extensions sorted by descending length.
fn generate_extension_list(dir: &str) -> Vec<String> {
    let mut extensions = HashSet::new();

    for entry in WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        if let Some(ext) = entry.path().extension().and_then(|e| e.to_str()) {
            extensions.insert(ext.to_string());
        }

        // Handle multiple-dot extensions
        let file_name = entry.file_name().to_string_lossy();
        if let Some(pos) = file_name.find('.') {
            let sub_ext = &file_name[pos..];
            extensions.insert(sub_ext.to_string());
        }
    }

    let mut ext_vec: Vec<String> = extensions.into_iter().collect();
    // Sort by descending length
    ext_vec.sort_by(|a, b| b.len().cmp(&a.len()));
    ext_vec
}

/// Builds the dependency graph from a list of files.
/// Handles only relative imports.
pub fn build_dependency_graph(files: &[PathBuf]) -> Graph<PathBuf, ()> {
    let mut graph = Graph::new();
    let mut node_indices = HashMap::new();

    // Dynamically generate the extensions list
    let extensions = generate_extension_list("."); // Pass the root directory

    // Insert all files as nodes
    for file in files {
        let idx = graph.add_node(file.clone());
        node_indices.insert(file.clone(), idx);
        debug!("Added node: {:?}", file);
    }

    for file in files {
        let imports = get_imports_from_file(file);
        debug!("Processing file: {:?}", file);
        for import in imports {
            if let Some(resolved) = resolve_import(file, &import, &extensions) {
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

/// Resolves a relative import to an absolute, normalized PathBuf.
/// Returns `None` if the import cannot be resolved.
fn resolve_import(base: &Path, import: &str, extensions: &Vec<String>) -> Option<PathBuf> {
    if !import.starts_with('.') {
        return None; // Only handle relative imports
    }

    let candidate = base.parent()?.join(import);
    debug!("Attempting to resolve import: '{}' from {:?}", import, base);

    check_candidates(candidate, extensions)
}

/// Checks various possibilities for the import path.
/// Returns the resolved, canonicalized PathBuf if found.
fn check_candidates(candidate: PathBuf, extensions: &Vec<String>) -> Option<PathBuf> {
    for ext in extensions {
        let mut p = candidate.clone();
        if !ext.is_empty() {
            // Remove the leading dot before setting the extension
            p.set_extension(&ext[1..]);
        }
        if p.is_file() {
            let canonical = normalize_path(&p);
            debug!("check_candidates: Found file {:?}", canonical);
            return Some(canonical);
        }
    }

    // If candidate is a directory, try index files
    if candidate.is_dir() {
        let index_extensions = vec![".ts", ".tsx", ".js", ".jsx", ".cjs", ".mjs"];

        for idx_ext in index_extensions {
            let idx_file = candidate.join(format!("index{}", idx_ext));
            if idx_file.is_file() {
                let canonical = normalize_path(&idx_file);
                debug!("check_candidates: Found index file {:?}", canonical);
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
                .map(|node_index| graph.node_weight(*node_index).unwrap())
                .collect::<Vec<_>>();
            cycles.push(cycle);
        } else {
            // Check for self-loop
            let node = scc[0];
            if graph.contains_edge(node, node) {
                cycles.push(vec![graph.node_weight(node).unwrap()]);
                debug!(
                    "Detected self-loop cycle for node: {:?}",
                    graph.node_weight(node).unwrap()
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
            .unwrap();
        let min_index = cycle.iter().position(|&path| path == *min_path).unwrap();

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
pub fn get_unique_cycles<'a>(graph: &'a Graph<PathBuf, ()>) -> Vec<Vec<&'a PathBuf>> {
    let cycles = find_all_cycles(graph);
    deduplicate_cycles(&cycles)
}

/// Prints the detected cycles in a Madge-like format with relative paths.
pub fn print_cycles(cycles: &[Vec<&PathBuf>], root: &Path) {
    if cycles.is_empty() {
        println!("No circular dependencies found.");
        return;
    }

    eprintln!("✖ Found {} circular dependencies!\n", cycles.len());
    for (i, cycle) in cycles.iter().enumerate() {
        let relative_paths: Vec<String> = cycle
            .iter()
            .map(|p| p.strip_prefix(root).unwrap_or(p).display().to_string())
            .collect();
        eprintln!("{}) {}", i + 1, relative_paths.join(" > "));
    }
}
