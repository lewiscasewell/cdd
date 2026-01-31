use crate::filesystem::normalize_path;
use crate::parser::{get_imports_from_file, ImportInfo, ParserOptions};
use crate::tsconfig::PathAliases;
use crate::utils::{hash_strings, relative_path_string, EXTENSIONS};
use crate::workspace::Workspace;

use log::{debug, warn};
use petgraph::algo::kosaraju_scc;
use petgraph::graph::NodeIndex;
use petgraph::visit::EdgeRef;
use petgraph::Graph;
use rayon::prelude::*;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// Information stored on each edge in the dependency graph
#[derive(Debug, Clone, Serialize)]
pub struct EdgeInfo {
    /// The import information that created this edge
    pub import: ImportInfo,
}

/// A single edge in a cycle, with file and import information
#[derive(Debug, Clone, Serialize)]
pub struct CycleEdge {
    /// Source file of the import
    pub from_file: PathBuf,
    /// Target file of the import
    pub to_file: PathBuf,
    /// Line number of the import statement (1-indexed)
    pub line: u32,
    /// The full import text
    pub import_text: String,
}

/// Information about a detected cycle
#[derive(Debug, Clone, Serialize)]
pub struct CycleInfo {
    /// The edges that form this cycle
    pub edges: Vec<CycleEdge>,
    /// A stable hash of this cycle (based on relative file paths)
    pub hash: String,
}

impl CycleInfo {
    /// Get the files involved in this cycle (in order)
    pub fn files(&self) -> Vec<&PathBuf> {
        self.edges.iter().map(|e| &e.from_file).collect()
    }

    /// Create a canonical key for deduplication (based on file paths)
    fn canonical_key(&self, root: &Path) -> String {
        if self.edges.is_empty() {
            return String::new();
        }

        let files: Vec<String> = self
            .files()
            .iter()
            .map(|p| relative_path_string(p, root))
            .collect();

        // Find the lexicographically smallest file
        let min_idx = files
            .iter()
            .enumerate()
            .min_by_key(|(_, p)| p.as_str())
            .map(|(i, _)| i)
            .unwrap_or(0);

        // Rotate to start with min file
        let rotated: Vec<_> = files[min_idx..]
            .iter()
            .chain(files[..min_idx].iter())
            .cloned()
            .collect();

        rotated.join(" > ")
    }
}

/// Builds the dependency graph from a list of files.
/// Handles relative imports, path aliases, and workspace packages.
/// Parses files in parallel for performance.
pub fn build_dependency_graph(
    files: &[PathBuf],
    options: &ParserOptions,
    path_aliases: Option<&PathAliases>,
    workspace: Option<&Workspace>,
) -> Graph<PathBuf, EdgeInfo> {
    let mut graph = Graph::new();
    let mut node_indices = HashMap::new();

    // Convert static extensions to owned strings for compatibility
    let extensions: Vec<String> = EXTENSIONS.iter().map(|s| s.to_string()).collect();

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
            if let Some(resolved) =
                resolve_import(file, &import.source, &extensions, path_aliases, workspace)
            {
                if let Some(&to_idx) = node_indices.get(&resolved) {
                    let from_idx = node_indices[file];
                    graph.add_edge(from_idx, to_idx, EdgeInfo { import });
                    debug!("Added edge: {:?} -> {:?}", file, resolved);
                } else {
                    warn!("Resolved import not found in node_indices: {:?}", resolved);
                }
            } else {
                debug!(
                    "Skipped external or unresolved import '{}' from {:?}",
                    import.source, file
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
        for idx_ext in EXTENSIONS {
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
/// Returns CycleInfo structs with edge metadata.
/// The `root` parameter is used to compute stable hashes with relative paths.
pub fn find_all_cycles(graph: &Graph<PathBuf, EdgeInfo>, root: &Path) -> Vec<CycleInfo> {
    let sccs = kosaraju_scc(graph);
    let mut cycles = Vec::new();

    for scc in sccs {
        if scc.len() > 1 {
            // A strongly connected component with more than one node indicates a cycle
            if let Some(cycle_info) = extract_cycle_info(graph, &scc, root) {
                cycles.push(cycle_info);
            }
        } else {
            // Check for self-loop
            let node = scc[0];
            if graph.contains_edge(node, node) {
                if let Some(cycle_info) = extract_self_loop_info(graph, node, root) {
                    cycles.push(cycle_info);
                }
            }
        }
    }

    cycles
}

/// Extract cycle information from an SCC (strongly connected component)
fn extract_cycle_info(
    graph: &Graph<PathBuf, EdgeInfo>,
    scc: &[NodeIndex],
    root: &Path,
) -> Option<CycleInfo> {
    let scc_set: HashSet<_> = scc.iter().copied().collect();
    let mut edges = Vec::new();

    // Find edges that form the cycle within this SCC
    for &from_node in scc {
        let from_file = graph.node_weight(from_node)?;

        for edge in graph.edges(from_node) {
            let to_node = edge.target();
            if scc_set.contains(&to_node) {
                let to_file = graph.node_weight(to_node)?;
                let edge_info = edge.weight();

                edges.push(CycleEdge {
                    from_file: from_file.clone(),
                    to_file: to_file.clone(),
                    line: edge_info.import.line,
                    import_text: edge_info.import.import_text.clone(),
                });
            }
        }
    }

    if edges.is_empty() {
        return None;
    }

    // Order edges to form a proper cycle path
    let ordered_edges = order_cycle_edges(edges, root);

    // Compute hash based on relative file paths (for stability across machines)
    let hash = compute_cycle_hash(&ordered_edges, root);

    Some(CycleInfo {
        edges: ordered_edges,
        hash,
    })
}

/// Order edges to form a coherent cycle path starting from the lexicographically smallest file.
fn order_cycle_edges(mut edges: Vec<CycleEdge>, root: &Path) -> Vec<CycleEdge> {
    if edges.is_empty() {
        return edges;
    }

    // Build adjacency map: from_file -> list of edges from that file
    let mut adjacency: HashMap<PathBuf, Vec<CycleEdge>> = HashMap::new();
    for edge in edges.drain(..) {
        adjacency
            .entry(edge.from_file.clone())
            .or_default()
            .push(edge);
    }

    // Find the lexicographically smallest starting file (using relative paths)
    let start_file = adjacency
        .keys()
        .min_by_key(|p| relative_path_string(p, root))
        .cloned();

    let start_file = match start_file {
        Some(f) => f,
        None => return vec![],
    };

    // Follow the cycle from the start, building a path
    let mut result = Vec::new();
    let mut current = start_file.clone();
    let mut visited = HashSet::new();

    while !visited.contains(&current) {
        visited.insert(current.clone());

        if let Some(edges_from_current) = adjacency.get(&current) {
            // Prefer the edge that continues the cycle (to an unvisited node, or back to start)
            let next_edge = edges_from_current
                .iter()
                .find(|e| !visited.contains(&e.to_file) || e.to_file == start_file);

            if let Some(edge) = next_edge {
                result.push(edge.clone());
                current = edge.to_file.clone();
            } else {
                break;
            }
        } else {
            break;
        }
    }

    result
}

/// Extract cycle info for a self-loop
fn extract_self_loop_info(
    graph: &Graph<PathBuf, EdgeInfo>,
    node: NodeIndex,
    root: &Path,
) -> Option<CycleInfo> {
    let file = graph.node_weight(node)?;

    // Find the self-loop edge
    for edge in graph.edges(node) {
        if edge.target() == node {
            let edge_info = edge.weight();
            let cycle_edge = CycleEdge {
                from_file: file.clone(),
                to_file: file.clone(),
                line: edge_info.import.line,
                import_text: edge_info.import.import_text.clone(),
            };

            let hash = compute_cycle_hash(&[cycle_edge.clone()], root);

            return Some(CycleInfo {
                edges: vec![cycle_edge],
                hash,
            });
        }
    }

    None
}

/// Compute a stable hash for a cycle based on relative file paths.
/// Using relative paths ensures the hash is consistent across machines and directories.
fn compute_cycle_hash(edges: &[CycleEdge], root: &Path) -> String {
    // Get relative paths and sort for consistent hashing
    let mut files: Vec<String> = edges
        .iter()
        .map(|e| relative_path_string(&e.from_file, root))
        .collect();
    files.sort();

    hash_strings(&files, 12)
}

/// Deduplicates cycles by creating a canonical representation for each cycle.
fn deduplicate_cycles(cycles: Vec<CycleInfo>, root: &Path) -> Vec<CycleInfo> {
    let mut seen = HashSet::new();
    let mut unique_cycles = Vec::new();

    for cycle in cycles {
        let key = cycle.canonical_key(root);

        if !seen.contains(&key) {
            seen.insert(key.clone());
            unique_cycles.push(cycle);
            debug!("Unique cycle added: {}", key);
        }
    }

    unique_cycles
}

/// Integrates cycle finding using Kosaraju's algorithm and deduplication.
/// The `root` parameter is used to compute stable hashes with relative paths.
pub fn get_unique_cycles(graph: &Graph<PathBuf, EdgeInfo>, root: &Path) -> Vec<CycleInfo> {
    let cycles = find_all_cycles(graph, root);
    deduplicate_cycles(cycles, root)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_cycle_hash_uses_relative_paths() {
        let root = PathBuf::from("/home/user/project");

        let edges = vec![
            CycleEdge {
                from_file: PathBuf::from("/home/user/project/src/a.ts"),
                to_file: PathBuf::from("/home/user/project/src/b.ts"),
                line: 1,
                import_text: "import { b } from './b'".to_string(),
            },
            CycleEdge {
                from_file: PathBuf::from("/home/user/project/src/b.ts"),
                to_file: PathBuf::from("/home/user/project/src/a.ts"),
                line: 1,
                import_text: "import { a } from './a'".to_string(),
            },
        ];

        let hash1 = compute_cycle_hash(&edges, &root);

        // Same relative paths from different absolute root should produce same hash
        let root2 = PathBuf::from("/different/path/project");
        let edges2 = vec![
            CycleEdge {
                from_file: PathBuf::from("/different/path/project/src/a.ts"),
                to_file: PathBuf::from("/different/path/project/src/b.ts"),
                line: 1,
                import_text: "import { b } from './b'".to_string(),
            },
            CycleEdge {
                from_file: PathBuf::from("/different/path/project/src/b.ts"),
                to_file: PathBuf::from("/different/path/project/src/a.ts"),
                line: 1,
                import_text: "import { a } from './a'".to_string(),
            },
        ];

        let hash2 = compute_cycle_hash(&edges2, &root2);

        assert_eq!(
            hash1, hash2,
            "Hashes should be equal for same relative paths"
        );
    }

    #[test]
    fn test_cycle_canonical_key() {
        let root = PathBuf::from("/project");

        let cycle = CycleInfo {
            edges: vec![
                CycleEdge {
                    from_file: PathBuf::from("/project/b.ts"),
                    to_file: PathBuf::from("/project/a.ts"),
                    line: 1,
                    import_text: String::new(),
                },
                CycleEdge {
                    from_file: PathBuf::from("/project/a.ts"),
                    to_file: PathBuf::from("/project/b.ts"),
                    line: 1,
                    import_text: String::new(),
                },
            ],
            hash: String::new(),
        };

        let key = cycle.canonical_key(&root);
        // Should start with lexicographically smallest file (a.ts)
        assert!(
            key.starts_with("a.ts"),
            "Key should start with a.ts: {}",
            key
        );
    }
}
