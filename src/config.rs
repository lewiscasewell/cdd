use log::debug;
use serde::Deserialize;
use std::path::{Path, PathBuf};

/// Configuration loaded from a config file (.cddrc.json or cdd.config.json).
#[derive(Debug, Default, Deserialize)]
pub struct CddConfig {
    /// Directories to exclude from analysis.
    pub exclude: Option<Vec<String>>,
    /// Whether to ignore type-only imports.
    pub ignore_type_imports: Option<bool>,
    /// Expected number of cycles (for CI assertions).
    pub expected_cycles: Option<usize>,
    /// Path to tsconfig.json for path alias resolution.
    pub tsconfig_path: Option<String>,
}

const CONFIG_FILE_NAMES: &[&str] = &[".cddrc.json", "cdd.config.json"];

/// Searches for a config file starting from the given directory and walking up.
///
/// Returns the parsed config if found, or None if no config file exists.
pub fn find_config(start_dir: &Path) -> Option<CddConfig> {
    let mut current = start_dir.to_path_buf();

    loop {
        for config_name in CONFIG_FILE_NAMES {
            let config_path = current.join(config_name);
            if config_path.is_file() {
                debug!("Found config file: {}", config_path.display());
                return load_config(&config_path);
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

/// Merged configuration from CLI arguments and config file.
/// CLI arguments take precedence over config file values.
#[derive(Debug)]
pub struct MergedConfig {
    pub exclude: Vec<String>,
    pub ignore_type_imports: bool,
    pub expected_cycles: usize,
    pub tsconfig_path: Option<String>,
}

impl MergedConfig {
    /// Creates a merged config from CLI arguments and an optional config file.
    /// CLI arguments always take precedence when specified.
    pub fn new(
        cli_exclude: Vec<String>,
        cli_ignore_type_imports: bool,
        cli_expected_cycles: Option<usize>,
        cli_tsconfig_path: Option<String>,
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

        MergedConfig {
            exclude,
            ignore_type_imports,
            expected_cycles,
            tsconfig_path,
        }
    }
}
