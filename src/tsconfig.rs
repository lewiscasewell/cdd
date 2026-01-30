use log::debug;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Parsed tsconfig.json structure (only the parts we need).
#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct TsConfigJson {
    extends: Option<String>,
    compiler_options: Option<CompilerOptions>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct CompilerOptions {
    base_url: Option<String>,
    paths: Option<HashMap<String, Vec<String>>>,
}

/// Resolved path alias configuration.
#[derive(Debug, Clone)]
pub struct PathAliases {
    /// Base URL for non-relative imports (relative to tsconfig location).
    pub base_url: Option<PathBuf>,
    /// Path mappings: pattern -> list of replacement paths.
    /// Pattern may end with `*` for wildcard matching.
    pub paths: HashMap<String, Vec<PathBuf>>,
    /// Directory containing the tsconfig.json.
    pub config_dir: PathBuf,
}

impl PathAliases {
    /// Attempts to resolve an import path using the configured aliases.
    /// Returns the resolved path if successful, None otherwise.
    pub fn resolve(&self, import: &str) -> Option<PathBuf> {
        // Try exact match first
        if let Some(replacements) = self.paths.get(import) {
            for replacement in replacements {
                let resolved = self.config_dir.join(replacement);
                if resolved.exists() || resolved.with_extension("ts").exists() {
                    debug!("Resolved '{}' via exact match to {:?}", import, resolved);
                    return Some(resolved);
                }
            }
        }

        // Try wildcard patterns (e.g., "@/*" -> "src/*")
        for (pattern, replacements) in &self.paths {
            if let Some(prefix) = pattern.strip_suffix('*') {
                if let Some(suffix) = import.strip_prefix(prefix) {
                    for replacement in replacements {
                        let replacement_str = replacement.to_string_lossy();
                        if let Some(base) = replacement_str.strip_suffix('*') {
                            let resolved = self.config_dir.join(base).join(suffix);
                            debug!("Trying wildcard resolution: '{}' -> {:?}", import, resolved);
                            // Return the path even if it doesn't exist yet -
                            // the caller will handle extension resolution
                            return Some(resolved);
                        }
                    }
                }
            }
        }

        // Try baseUrl resolution
        if let Some(base_url) = &self.base_url {
            let resolved = base_url.join(import);
            debug!("Trying baseUrl resolution: '{}' -> {:?}", import, resolved);
            return Some(resolved);
        }

        None
    }
}

/// Loads and parses a tsconfig.json file, following `extends` chains.
pub fn load_tsconfig(path: &Path) -> Option<PathAliases> {
    let config_path = if path.is_file() {
        path.to_path_buf()
    } else {
        path.join("tsconfig.json")
    };

    if !config_path.exists() {
        debug!("tsconfig not found at {:?}", config_path);
        return None;
    }

    let config_dir = config_path.parent().unwrap_or(Path::new(".")).to_path_buf();

    load_tsconfig_with_extends(&config_path, &config_dir)
}

fn load_tsconfig_with_extends(path: &Path, config_dir: &Path) -> Option<PathAliases> {
    let content = std::fs::read_to_string(path).ok()?;
    let config: TsConfigJson = serde_json::from_str(&content)
        .map_err(|e| {
            log::warn!("Failed to parse tsconfig '{}': {}", path.display(), e);
            e
        })
        .ok()?;

    // Start with parent config if extends is specified
    let mut aliases = if let Some(extends) = &config.extends {
        let parent_path = resolve_extends(extends, config_dir)?;
        let parent_dir = parent_path.parent().unwrap_or(Path::new(".")).to_path_buf();
        load_tsconfig_with_extends(&parent_path, &parent_dir).unwrap_or_else(|| PathAliases {
            base_url: None,
            paths: HashMap::new(),
            config_dir: config_dir.to_path_buf(),
        })
    } else {
        PathAliases {
            base_url: None,
            paths: HashMap::new(),
            config_dir: config_dir.to_path_buf(),
        }
    };

    // Override with current config's values
    aliases.config_dir = config_dir.to_path_buf();

    if let Some(compiler_options) = config.compiler_options {
        if let Some(base_url) = compiler_options.base_url {
            aliases.base_url = Some(config_dir.join(&base_url));
        }

        if let Some(paths) = compiler_options.paths {
            for (pattern, replacements) in paths {
                let resolved_replacements: Vec<PathBuf> =
                    replacements.into_iter().map(PathBuf::from).collect();
                aliases.paths.insert(pattern, resolved_replacements);
            }
        }
    }

    debug!("Loaded tsconfig from {:?}: {:?}", path, aliases);
    Some(aliases)
}

fn resolve_extends(extends: &str, config_dir: &Path) -> Option<PathBuf> {
    if extends.starts_with('.') {
        // Relative path
        let mut path = config_dir.join(extends);
        if !path.extension().map_or(false, |e| e == "json") {
            path = path.with_extension("json");
        }
        if path.exists() {
            return Some(path);
        }
        // Try without adding extension (it might be a directory with tsconfig.json)
        let dir_path = config_dir.join(extends).join("tsconfig.json");
        if dir_path.exists() {
            return Some(dir_path);
        }
    } else {
        // Node module - look in node_modules
        let node_modules_path = config_dir.join("node_modules").join(extends);
        if node_modules_path.exists() {
            return Some(node_modules_path);
        }
        // Try with tsconfig.json
        let with_tsconfig = node_modules_path.join("tsconfig.json");
        if with_tsconfig.exists() {
            return Some(with_tsconfig);
        }
    }

    debug!("Could not resolve tsconfig extends: {}", extends);
    None
}
