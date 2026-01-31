use crate::graph::CycleInfo;
use crate::utils::relative_path_string;
use log::debug;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// An allowed cycle that won't cause CI failure
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AllowedCycle {
    /// Files that form the cycle (relative paths)
    pub files: Vec<String>,
    /// Optional reason for allowing this cycle
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// Configuration loaded from a config file (.cddrc.json or cdd.config.json).
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct CddConfig {
    /// Directories to exclude from analysis.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exclude: Option<Vec<String>>,
    /// Whether to ignore type-only imports.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ignore_type_imports: Option<bool>,
    /// Expected number of cycles (for CI assertions).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_cycles: Option<usize>,
    /// Path to tsconfig.json for path alias resolution.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tsconfig_path: Option<String>,
    /// Expected hash of all cycles for CI validation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_hash: Option<String>,
    /// Cycles that are allowed (won't cause CI failure).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_cycles: Option<Vec<AllowedCycle>>,
}

const CONFIG_FILE_NAMES: &[&str] = &[".cddrc.json", "cdd.config.json"];

/// Searches for a config file starting from the given directory and walking up.
///
/// Returns the path and parsed config if found, or None if no config file exists.
pub fn find_config(start_dir: &Path) -> Option<(PathBuf, CddConfig)> {
    let mut current = start_dir.to_path_buf();

    loop {
        for config_name in CONFIG_FILE_NAMES {
            let config_path = current.join(config_name);
            if config_path.is_file() {
                debug!("Found config file: {}", config_path.display());
                if let Some(config) = load_config(&config_path) {
                    return Some((config_path, config));
                }
            }
        }

        if !current.pop() {
            break;
        }
    }

    debug!("No config file found");
    None
}

/// Loads and parses a config file.
fn load_config(path: &PathBuf) -> Option<CddConfig> {
    let content = std::fs::read_to_string(path).ok()?;
    match serde_json::from_str(&content) {
        Ok(config) => Some(config),
        Err(e) => {
            log::warn!("Failed to parse config file '{}': {}", path.display(), e);
            None
        }
    }
}

/// Updates the expected_hash in the config file.
/// Creates the config file if it doesn't exist.
pub fn update_config_hash(dir: &Path, new_hash: &str) -> Result<PathBuf, String> {
    // Try to find existing config
    let (config_path, mut config) = find_config(dir).unwrap_or_else(|| {
        // Create new config file
        let new_path = dir.join(".cddrc.json");
        (new_path, CddConfig::default())
    });

    // Update the hash
    config.expected_hash = Some(new_hash.to_string());

    // Write back to file
    let content = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;

    std::fs::write(&config_path, content).map_err(|e| {
        format!(
            "Failed to write config file '{}': {}",
            config_path.display(),
            e
        )
    })?;

    Ok(config_path)
}

/// Load an allowlist from a file (simple text format)
/// Format: one cycle per line, files separated by " > "
/// Lines starting with # are comments
pub fn load_allowlist(path: &Path) -> Option<Vec<AllowedCycle>> {
    let content = std::fs::read_to_string(path).ok()?;
    Some(crate::output::parse_allowlist_file(&content))
}

/// Initialize a config file with current cycles as the allowed baseline.
/// This creates a .cddrc.json with all detected cycles in the allowlist,
/// so new cycles will cause failures but existing ones are accepted.
pub fn init_config(dir: &Path, cycles: &[CycleInfo]) -> Result<PathBuf, String> {
    let config_path = dir.join(".cddrc.json");

    // Check if config already exists
    if config_path.exists() {
        return Err(format!(
            "Config file already exists: {}. Delete it first or use --update-hash to update.",
            config_path.display()
        ));
    }

    // Convert cycles to allowed cycles
    let allowed_cycles: Vec<AllowedCycle> = cycles
        .iter()
        .map(|cycle| {
            let files: Vec<String> = cycle
                .files()
                .iter()
                .map(|p| relative_path_string(p, dir))
                .collect();
            AllowedCycle {
                files,
                reason: Some("Existing cycle from --init".to_string()),
            }
        })
        .collect();

    let config = CddConfig {
        expected_cycles: Some(0), // All cycles are allowed, so we expect 0 "new" cycles
        allowed_cycles: if allowed_cycles.is_empty() {
            None
        } else {
            Some(allowed_cycles)
        },
        ..Default::default()
    };

    let content = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;

    std::fs::write(&config_path, content).map_err(|e| {
        format!(
            "Failed to write config file '{}': {}",
            config_path.display(),
            e
        )
    })?;

    Ok(config_path)
}

/// Merged configuration from CLI arguments and config file.
/// CLI arguments take precedence over config file values.
#[derive(Debug)]
pub struct MergedConfig {
    pub exclude: Vec<String>,
    pub ignore_type_imports: bool,
    pub expected_cycles: usize,
    pub tsconfig_path: Option<String>,
    pub expected_hash: Option<String>,
    pub allowed_cycles: Vec<AllowedCycle>,
}

impl MergedConfig {
    /// Creates a merged config from CLI arguments and an optional config file.
    /// CLI arguments always take precedence when specified.
    pub fn new(
        cli_exclude: Vec<String>,
        cli_ignore_type_imports: bool,
        cli_expected_cycles: Option<usize>,
        cli_tsconfig_path: Option<String>,
        cli_expected_hash: Option<String>,
        cli_allowlist_path: Option<String>,
        file_config: Option<CddConfig>,
    ) -> Self {
        let file_config = file_config.unwrap_or_default();

        // For exclude: merge CLI and config file (CLI additions are added to config file list)
        let mut exclude = file_config.exclude.unwrap_or_default();
        for item in cli_exclude {
            if !exclude.contains(&item) {
                exclude.push(item);
            }
        }

        // For booleans: CLI takes precedence if true, otherwise use config file
        let ignore_type_imports = if cli_ignore_type_imports {
            true
        } else {
            file_config.ignore_type_imports.unwrap_or(false)
        };

        // For expected_cycles: CLI takes precedence if specified, otherwise use config file
        let expected_cycles = cli_expected_cycles
            .or(file_config.expected_cycles)
            .unwrap_or(0);

        let tsconfig_path = cli_tsconfig_path.or(file_config.tsconfig_path);

        // For expected_hash: CLI takes precedence if specified
        let expected_hash = cli_expected_hash.or(file_config.expected_hash);

        // For allowed_cycles: merge CLI allowlist file with config file
        let mut allowed_cycles = file_config.allowed_cycles.unwrap_or_default();

        // Load additional allowlist from file if specified via CLI
        if let Some(allowlist_path) = cli_allowlist_path {
            let path = Path::new(&allowlist_path);
            if let Some(file_allowlist) = load_allowlist(path) {
                allowed_cycles.extend(file_allowlist);
            } else {
                log::warn!("Could not load allowlist from '{}'", allowlist_path);
            }
        }

        MergedConfig {
            exclude,
            ignore_type_imports,
            expected_cycles,
            tsconfig_path,
            expected_hash,
            allowed_cycles,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_find_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join(".cddrc.json");
        let mut file = std::fs::File::create(&config_path).unwrap();
        writeln!(file, r#"{{"expected_cycles": 5}}"#).unwrap();

        let result = find_config(temp_dir.path());
        assert!(result.is_some());
        let (path, config) = result.unwrap();
        assert_eq!(path, config_path);
        assert_eq!(config.expected_cycles, Some(5));
    }

    #[test]
    fn test_update_config_hash_creates_file() {
        let temp_dir = TempDir::new().unwrap();

        let result = update_config_hash(temp_dir.path(), "abc123");
        assert!(result.is_ok());

        let config_path = result.unwrap();
        assert!(config_path.exists());

        let content = std::fs::read_to_string(&config_path).unwrap();
        assert!(content.contains("abc123"));
    }

    #[test]
    fn test_update_config_hash_preserves_existing() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join(".cddrc.json");
        std::fs::write(&config_path, r#"{"expected_cycles": 5}"#).unwrap();

        let result = update_config_hash(temp_dir.path(), "newhash");
        assert!(result.is_ok());

        let content = std::fs::read_to_string(&config_path).unwrap();
        assert!(content.contains("newhash"));
        assert!(content.contains("expected_cycles"));
    }

    #[test]
    fn test_merged_config_cli_precedence() {
        let file_config = CddConfig {
            expected_cycles: Some(5),
            expected_hash: Some("oldhash".to_string()),
            ..Default::default()
        };

        let merged = MergedConfig::new(
            vec![],
            false,
            Some(10),
            None,
            Some("newhash".to_string()),
            None,
            Some(file_config),
        );

        assert_eq!(merged.expected_cycles, 10);
        assert_eq!(merged.expected_hash, Some("newhash".to_string()));
    }
}
