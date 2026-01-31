//! Output formatting for cycle detection results.
//!
//! This module provides functions for displaying cycles in various formats:
//! - Detailed text output with line numbers and import statements
//! - JSON output for tooling integration
//! - Hash computation for CI validation

use crate::config::AllowedCycle;
use crate::graph::CycleInfo;
use crate::utils::{hash_strings, relative_path_string};
use colored::*;
use serde::Serialize;
use std::collections::HashSet;
use std::path::Path;

/// Output format for cycle detection results
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    /// Default text output with line numbers and import statements
    Text,
    /// JSON output for tooling integration
    Json,
}

/// JSON output structure for cycle detection results
#[derive(Debug, Serialize)]
pub struct JsonOutput {
    /// Total number of files analyzed
    pub total_files: usize,
    /// Total number of cycles found
    pub total_cycles: usize,
    /// Hash of all cycles (for CI validation)
    pub cycles_hash: String,
    /// Detailed information about each cycle
    pub cycles: Vec<JsonCycle>,
}

/// JSON representation of a single cycle
#[derive(Debug, Serialize)]
pub struct JsonCycle {
    /// Unique hash for this cycle
    pub hash: String,
    /// The edges forming this cycle
    pub edges: Vec<JsonEdge>,
}

/// JSON representation of a cycle edge
#[derive(Debug, Serialize)]
pub struct JsonEdge {
    /// Source file (relative path)
    pub from_file: String,
    /// Target file (relative path)
    pub to_file: String,
    /// Line number of the import
    pub line: u32,
    /// The import statement text
    pub import_text: String,
}

/// JSON error output structure
#[derive(Debug, Serialize)]
pub struct JsonError {
    /// Error message
    pub error: String,
}

/// Compute a hash of all cycles for CI validation.
///
/// The hash is computed from the sorted individual cycle hashes,
/// ensuring consistent results regardless of detection order.
pub fn compute_cycles_hash(cycles: &[CycleInfo]) -> String {
    // Sort cycles by their individual hashes for consistency
    let mut sorted_hashes: Vec<_> = cycles.iter().map(|c| c.hash.clone()).collect();
    sorted_hashes.sort();

    hash_strings(&sorted_hashes, 12)
}

/// Print cycles in detailed text format with line numbers and import statements.
///
/// Output format:
/// ```text
/// 1) Circular dependency [hash]:
///    src/a.ts:3
///    | import { b } from './b';
///    v
///    src/b.ts:2
///    | import { a } from './a';
///    ^-- (cycle)
/// ```
pub fn print_cycles_detailed(cycles: &[CycleInfo], root: &Path) {
    if cycles.is_empty() {
        log::info!("{}", "no circular dependencies found.".green().bold());
        return;
    }

    log::info!(
        "{} Found {} circular dependencies!\n",
        "X".red().bold(),
        cycles.len().to_string().red()
    );

    for (i, cycle) in cycles.iter().enumerate() {
        log::info!(
            "{}) Circular dependency [{}]:",
            (i + 1).to_string().bright_blue().bold(),
            cycle.hash.dimmed()
        );

        for (j, edge) in cycle.edges.iter().enumerate() {
            let from_relative = relative_path_string(&edge.from_file, root);

            // Print the file and line number
            log::info!(
                "   {}:{}",
                from_relative.cyan(),
                edge.line.to_string().yellow()
            );

            // Print the import statement
            let import_text = edge.import_text.trim();
            log::info!("   {} {}", "|".dimmed(), import_text.dimmed());

            // Print arrow or cycle indicator
            if j < cycle.edges.len() - 1 {
                log::info!("   {}", "v".bright_blue());
            } else {
                log::info!("   {} (cycle)", "^--".bright_blue());
            }
        }
        log::info!("");
    }
}

/// Generate JSON output structure for cycles.
///
/// All file paths in the output are relative to the root directory.
pub fn generate_json_output(cycles: &[CycleInfo], root: &Path, total_files: usize) -> JsonOutput {
    let cycles_hash = compute_cycles_hash(cycles);

    let json_cycles: Vec<JsonCycle> = cycles
        .iter()
        .map(|cycle| {
            let edges: Vec<JsonEdge> = cycle
                .edges
                .iter()
                .map(|edge| JsonEdge {
                    from_file: relative_path_string(&edge.from_file, root),
                    to_file: relative_path_string(&edge.to_file, root),
                    line: edge.line,
                    import_text: edge.import_text.clone(),
                })
                .collect();

            JsonCycle {
                hash: cycle.hash.clone(),
                edges,
            }
        })
        .collect();

    JsonOutput {
        total_files,
        total_cycles: cycles.len(),
        cycles_hash,
        cycles: json_cycles,
    }
}

/// Print JSON output to stdout.
pub fn print_json_output(output: &JsonOutput) {
    match serde_json::to_string_pretty(output) {
        Ok(json) => println!("{}", json),
        Err(e) => {
            let error = JsonError {
                error: format!("Failed to serialize output: {}", e),
            };
            if let Ok(error_json) = serde_json::to_string_pretty(&error) {
                eprintln!("{}", error_json);
            }
        }
    }
}

/// Print a JSON error message to stderr.
pub fn print_json_error(message: &str) {
    let error = JsonError {
        error: message.to_string(),
    };
    if let Ok(json) = serde_json::to_string_pretty(&error) {
        eprintln!("{}", json);
    } else {
        // Fallback if serialization fails
        eprintln!("{{\"error\": \"{}\"}}", message.replace('"', "\\\""));
    }
}

/// Filter out cycles that match the allowlist.
///
/// Cycles are matched by comparing the set of files involved.
/// Order doesn't matter - a cycle A->B->A matches allowlist entry [A, B].
pub fn filter_allowed_cycles(
    cycles: Vec<CycleInfo>,
    allowed: &[AllowedCycle],
    root: &Path,
) -> Vec<CycleInfo> {
    cycles
        .into_iter()
        .filter(|cycle| !is_cycle_allowed(cycle, allowed, root))
        .collect()
}

/// Check if a cycle matches any entry in the allowlist.
fn is_cycle_allowed(cycle: &CycleInfo, allowed: &[AllowedCycle], root: &Path) -> bool {
    let cycle_files: HashSet<String> = cycle
        .files()
        .iter()
        .map(|p| relative_path_string(p, root))
        .collect();

    for allowed_cycle in allowed {
        let allowed_files: HashSet<String> = allowed_cycle.files.iter().cloned().collect();

        // Check if the cycle files match the allowed files exactly
        if cycle_files == allowed_files {
            if let Some(reason) = &allowed_cycle.reason {
                log::debug!(
                    "Cycle allowed by allowlist: {} (reason: {})",
                    cycle_files.iter().cloned().collect::<Vec<_>>().join(" > "),
                    reason
                );
            }
            return true;
        }
    }

    false
}

/// Parse an allowlist file in simple text format.
///
/// Format:
/// - One cycle per line
/// - Files separated by " > "
/// - Lines starting with # are comments
/// - Empty lines are ignored
///
/// Example:
/// ```text
/// # Known cycles
/// src/a.ts > src/b.ts
/// src/hooks/useAuth.ts > src/hooks/useUser.ts
/// ```
pub fn parse_allowlist_file(content: &str) -> Vec<AllowedCycle> {
    content
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.is_empty() && !trimmed.starts_with('#')
        })
        .map(|line| {
            let files: Vec<String> = line.split(" > ").map(|s| s.trim().to_string()).collect();
            AllowedCycle {
                files,
                reason: None,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn make_cycle(files: &[&str], root: &Path) -> CycleInfo {
        use crate::graph::CycleEdge;

        let edges: Vec<CycleEdge> = files
            .windows(2)
            .chain(std::iter::once(
                [files[files.len() - 1], files[0]].as_slice(),
            ))
            .map(|pair| CycleEdge {
                from_file: root.join(pair[0]),
                to_file: root.join(pair[1]),
                line: 1,
                import_text: format!("import from '{}'", pair[1]),
            })
            .take(files.len())
            .collect();

        CycleInfo {
            edges,
            hash: "testhash".to_string(),
        }
    }

    #[test]
    fn test_compute_cycles_hash_deterministic() {
        let root = PathBuf::from("/project");
        let cycles = vec![
            make_cycle(&["a.ts", "b.ts"], &root),
            make_cycle(&["c.ts", "d.ts"], &root),
        ];

        let hash1 = compute_cycles_hash(&cycles);
        let hash2 = compute_cycles_hash(&cycles);

        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 12);
    }

    #[test]
    fn test_filter_allowed_cycles() {
        let root = PathBuf::from("/project");
        let cycles = vec![
            make_cycle(&["a.ts", "b.ts"], &root),
            make_cycle(&["c.ts", "d.ts"], &root),
        ];

        let allowed = vec![AllowedCycle {
            files: vec!["a.ts".to_string(), "b.ts".to_string()],
            reason: Some("Known issue".to_string()),
        }];

        let filtered = filter_allowed_cycles(cycles, &allowed, &root);

        assert_eq!(filtered.len(), 1);
        assert!(filtered[0].files().iter().any(|f| f.ends_with("c.ts")));
    }

    #[test]
    fn test_parse_allowlist_file() {
        let content = r#"
# This is a comment
src/a.ts > src/b.ts

src/c.ts > src/d.ts > src/e.ts
"#;

        let allowed = parse_allowlist_file(content);

        assert_eq!(allowed.len(), 2);
        assert_eq!(allowed[0].files, vec!["src/a.ts", "src/b.ts"]);
        assert_eq!(allowed[1].files, vec!["src/c.ts", "src/d.ts", "src/e.ts"]);
    }

    #[test]
    fn test_generate_json_output() {
        let root = PathBuf::from("/project");
        let cycles = vec![make_cycle(&["a.ts", "b.ts"], &root)];

        let output = generate_json_output(&cycles, &root, 10);

        assert_eq!(output.total_files, 10);
        assert_eq!(output.total_cycles, 1);
        assert_eq!(output.cycles.len(), 1);
        assert_eq!(output.cycles[0].edges[0].from_file, "a.ts");
    }
}
