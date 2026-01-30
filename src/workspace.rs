use glob::glob;
use log::debug;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Information about a single workspace package.
#[derive(Debug, Clone)]
pub struct PackageInfo {
    /// Package name (e.g., "@acme/ui")
    pub name: String,
    /// Path to the package directory
    pub path: PathBuf,
    /// Main entry point (from "main" field)
    pub main: Option<String>,
    /// ES module entry point (from "module" field)
    pub module: Option<String>,
    /// Package exports configuration
    pub exports: Option<Exports>,
}

/// Represents the package.json "exports" field.
#[derive(Debug, Clone)]
pub enum Exports {
    /// Simple string export: `"exports": "./dist/index.js"`
    String(String),
    /// Object export with subpaths or conditions
    Object(HashMap<String, ExportValue>),
}

/// A single export value, which can be a string or conditional object.
#[derive(Debug, Clone)]
pub enum ExportValue {
    String(String),
    Conditional(HashMap<String, String>),
}

/// Workspace configuration containing all discovered packages.
#[derive(Debug)]
pub struct Workspace {
    /// Root directory of the workspace
    pub root: PathBuf,
    /// Map of package names to their info
    pub packages: HashMap<String, PackageInfo>,
}

/// Package.json structure (fields we need).
#[derive(Debug, Deserialize)]
struct PackageJson {
    name: Option<String>,
    main: Option<String>,
    module: Option<String>,
    exports: Option<serde_json::Value>,
    workspaces: Option<WorkspacesField>,
}

/// Workspaces field can be an array or object with "packages" key.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum WorkspacesField {
    Array(Vec<String>),
    Object { packages: Vec<String> },
}

impl WorkspacesField {
    fn patterns(&self) -> &[String] {
        match self {
            WorkspacesField::Array(patterns) => patterns,
            WorkspacesField::Object { packages } => packages,
        }
    }
}

/// pnpm-workspace.yaml structure.
#[derive(Debug, Deserialize)]
struct PnpmWorkspace {
    packages: Option<Vec<String>>,
}

impl Workspace {
    /// Detects and loads workspace configuration from the given root directory.
    /// Tries package.json workspaces first, then pnpm-workspace.yaml.
    pub fn detect(root: &Path) -> Option<Self> {
        let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());

        // Try package.json workspaces field (npm/yarn)
        let package_json_path = root.join("package.json");
        if package_json_path.exists() {
            if let Some(workspace) = Self::from_package_json(&root, &package_json_path) {
                return Some(workspace);
            }
        }

        // Try pnpm-workspace.yaml
        let pnpm_workspace_path = root.join("pnpm-workspace.yaml");
        if pnpm_workspace_path.exists() {
            if let Some(workspace) = Self::from_pnpm_workspace(&root, &pnpm_workspace_path) {
                return Some(workspace);
            }
        }

        debug!("No workspace configuration found in {:?}", root);
        None
    }

    fn from_package_json(root: &Path, path: &Path) -> Option<Self> {
        let content = std::fs::read_to_string(path).ok()?;
        let package_json: PackageJson = serde_json::from_str(&content).ok()?;

        let patterns = package_json.workspaces?.patterns().to_vec();
        if patterns.is_empty() {
            return None;
        }

        debug!("Found npm/yarn workspaces: {:?}", patterns);
        Some(Self::from_patterns(root, &patterns))
    }

    fn from_pnpm_workspace(root: &Path, path: &Path) -> Option<Self> {
        let content = std::fs::read_to_string(path).ok()?;
        let pnpm_workspace: PnpmWorkspace = serde_yaml::from_str(&content).ok()?;

        let patterns = pnpm_workspace.packages?;
        if patterns.is_empty() {
            return None;
        }

        debug!("Found pnpm workspaces: {:?}", patterns);
        Some(Self::from_patterns(root, &patterns))
    }

    fn from_patterns(root: &Path, patterns: &[String]) -> Self {
        let mut packages = HashMap::new();

        for pattern in patterns {
            // Convert workspace pattern to glob pattern
            let glob_pattern = root.join(pattern).join("package.json");
            let glob_str = glob_pattern.to_string_lossy();

            debug!("Searching for packages with pattern: {}", glob_str);

            if let Ok(entries) = glob(&glob_str) {
                for entry in entries.flatten() {
                    if let Some(info) = Self::load_package_info(&entry) {
                        debug!("Found package: {} at {:?}", info.name, info.path);
                        packages.insert(info.name.clone(), info);
                    }
                }
            }
        }

        Workspace {
            root: root.to_path_buf(),
            packages,
        }
    }

    fn load_package_info(package_json_path: &Path) -> Option<PackageInfo> {
        let content = std::fs::read_to_string(package_json_path).ok()?;
        let package_json: PackageJson = serde_json::from_str(&content).ok()?;

        let name = package_json.name?;
        let path = package_json_path
            .parent()?
            .canonicalize()
            .unwrap_or_else(|_| package_json_path.parent().unwrap().to_path_buf());

        let exports = package_json.exports.and_then(Self::parse_exports);

        Some(PackageInfo {
            name,
            path,
            main: package_json.main,
            module: package_json.module,
            exports,
        })
    }

    fn parse_exports(value: serde_json::Value) -> Option<Exports> {
        match value {
            serde_json::Value::String(s) => Some(Exports::String(s)),
            serde_json::Value::Object(obj) => {
                let mut map = HashMap::new();
                for (key, val) in obj {
                    let export_value = match val {
                        serde_json::Value::String(s) => ExportValue::String(s),
                        serde_json::Value::Object(cond_obj) => {
                            let mut conditions = HashMap::new();
                            for (cond_key, cond_val) in cond_obj {
                                if let serde_json::Value::String(s) = cond_val {
                                    conditions.insert(cond_key, s);
                                }
                            }
                            ExportValue::Conditional(conditions)
                        }
                        _ => continue,
                    };
                    map.insert(key, export_value);
                }
                Some(Exports::Object(map))
            }
            _ => None,
        }
    }

    /// Resolves a bare package import to a file path.
    /// Returns the resolved path if the import matches a workspace package.
    pub fn resolve(&self, import: &str) -> Option<PathBuf> {
        // Check for exact package match first
        if let Some(info) = self.packages.get(import) {
            return self.resolve_package_entry(info, ".");
        }

        // Check for subpath import (e.g., "@acme/ui/button")
        for (name, info) in &self.packages {
            if let Some(subpath) = import.strip_prefix(name) {
                if subpath.is_empty() {
                    return self.resolve_package_entry(info, ".");
                }
                if let Some(subpath) = subpath.strip_prefix('/') {
                    return self.resolve_subpath(info, subpath);
                }
            }
        }

        None
    }

    fn resolve_package_entry(&self, info: &PackageInfo, subpath: &str) -> Option<PathBuf> {
        // Try exports field first
        if let Some(exports) = &info.exports {
            if let Some(resolved) = self.resolve_from_exports(info, exports, subpath) {
                return Some(resolved);
            }
        }

        // For root entry ("."), fall back to module/main/index
        if subpath == "." {
            // Try module field (ESM)
            if let Some(module) = &info.module {
                let path = info.path.join(module);
                if path.exists() {
                    debug!("Resolved via module field: {:?}", path);
                    return Some(path);
                }
            }

            // Try main field
            if let Some(main) = &info.main {
                let path = info.path.join(main);
                if path.exists() {
                    debug!("Resolved via main field: {:?}", path);
                    return Some(path);
                }
            }

            // Try common entry points
            for entry in &["src/index.ts", "src/index.tsx", "index.ts", "index.js"] {
                let path = info.path.join(entry);
                if path.exists() {
                    debug!("Resolved via default entry: {:?}", path);
                    return Some(path);
                }
            }
        }

        None
    }

    fn resolve_subpath(&self, info: &PackageInfo, subpath: &str) -> Option<PathBuf> {
        // Try exports field first
        if let Some(exports) = &info.exports {
            let export_key = format!("./{}", subpath);
            if let Some(resolved) = self.resolve_from_exports(info, exports, &export_key) {
                return Some(resolved);
            }

            // Try wildcard patterns in exports
            if let Some(resolved) = self.resolve_wildcard_export(info, exports, subpath) {
                return Some(resolved);
            }
        }

        // Fall back to direct file resolution in src/
        let extensions = ["", ".ts", ".tsx", ".js", ".jsx"];
        let prefixes = ["src/", ""];

        for prefix in &prefixes {
            for ext in &extensions {
                let path = info.path.join(format!("{}{}{}", prefix, subpath, ext));
                if path.exists() {
                    debug!("Resolved subpath via direct file: {:?}", path);
                    return Some(path);
                }

                // Try as directory with index file
                let index_path = info.path.join(format!("{}{}/index{}", prefix, subpath, ext));
                if index_path.exists() {
                    debug!("Resolved subpath via index file: {:?}", index_path);
                    return Some(index_path);
                }
            }
        }

        None
    }

    fn resolve_from_exports(
        &self,
        info: &PackageInfo,
        exports: &Exports,
        subpath: &str,
    ) -> Option<PathBuf> {
        match exports {
            Exports::String(s) if subpath == "." => {
                let path = info.path.join(s.trim_start_matches("./"));
                if path.exists() {
                    return Some(path);
                }
            }
            Exports::Object(map) => {
                if let Some(export_value) = map.get(subpath) {
                    return self.resolve_export_value(info, export_value);
                }
            }
            _ => {}
        }
        None
    }

    fn resolve_wildcard_export(
        &self,
        info: &PackageInfo,
        exports: &Exports,
        subpath: &str,
    ) -> Option<PathBuf> {
        if let Exports::Object(map) = exports {
            for (pattern, value) in map {
                // Handle patterns like "./*" or "./components/*"
                if let Some(prefix) = pattern.strip_suffix('*') {
                    let prefix = prefix.trim_start_matches("./");
                    let full_subpath = format!("./{}", subpath);

                    if let Some(suffix) = full_subpath
                        .strip_prefix("./")
                        .and_then(|s| s.strip_prefix(prefix))
                    {
                        // Found a matching wildcard pattern
                        let resolved_path = match value {
                            ExportValue::String(s) => {
                                let target = s.replace('*', suffix);
                                info.path.join(target.trim_start_matches("./"))
                            }
                            ExportValue::Conditional(conditions) => {
                                // Prefer import > require > default
                                let target = conditions
                                    .get("import")
                                    .or_else(|| conditions.get("require"))
                                    .or_else(|| conditions.get("default"))?;
                                let target = target.replace('*', suffix);
                                info.path.join(target.trim_start_matches("./"))
                            }
                        };

                        if resolved_path.exists() {
                            debug!("Resolved via wildcard export: {:?}", resolved_path);
                            return Some(resolved_path);
                        }

                        // Try adding extensions
                        for ext in &[".ts", ".tsx", ".js", ".jsx"] {
                            let with_ext = resolved_path.with_extension(ext.trim_start_matches('.'));
                            if with_ext.exists() {
                                debug!("Resolved via wildcard export with extension: {:?}", with_ext);
                                return Some(with_ext);
                            }
                        }
                    }
                }
            }
        }
        None
    }

    fn resolve_export_value(&self, info: &PackageInfo, value: &ExportValue) -> Option<PathBuf> {
        let target = match value {
            ExportValue::String(s) => s.clone(),
            ExportValue::Conditional(conditions) => {
                // Prefer import > require > default
                conditions
                    .get("import")
                    .or_else(|| conditions.get("require"))
                    .or_else(|| conditions.get("default"))?
                    .clone()
            }
        };

        let path = info.path.join(target.trim_start_matches("./"));
        if path.exists() {
            debug!("Resolved export value: {:?}", path);
            return Some(path);
        }

        // Try adding extensions
        for ext in &[".ts", ".tsx", ".js", ".jsx"] {
            let with_ext = path.with_extension(ext.trim_start_matches('.'));
            if with_ext.exists() {
                debug!("Resolved export value with extension: {:?}", with_ext);
                return Some(with_ext);
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_workspace() -> TempDir {
        let temp = TempDir::new().unwrap();
        let root = temp.path();

        // Create root package.json with workspaces
        fs::write(
            root.join("package.json"),
            r#"{
                "name": "test-monorepo",
                "workspaces": ["packages/*"]
            }"#,
        )
        .unwrap();

        // Create packages/ui
        let ui_path = root.join("packages/ui");
        fs::create_dir_all(&ui_path).unwrap();
        fs::write(
            ui_path.join("package.json"),
            r#"{
                "name": "@test/ui",
                "main": "dist/index.js",
                "module": "dist/index.mjs"
            }"#,
        )
        .unwrap();
        fs::create_dir_all(ui_path.join("src")).unwrap();
        fs::write(ui_path.join("src/index.ts"), "export const ui = true;").unwrap();

        // Create packages/utils
        let utils_path = root.join("packages/utils");
        fs::create_dir_all(&utils_path).unwrap();
        fs::write(
            utils_path.join("package.json"),
            r#"{
                "name": "@test/utils",
                "exports": {
                    ".": {
                        "import": "./src/index.ts",
                        "require": "./dist/index.cjs"
                    },
                    "./helpers": "./src/helpers.ts"
                }
            }"#,
        )
        .unwrap();
        fs::create_dir_all(utils_path.join("src")).unwrap();
        fs::write(utils_path.join("src/index.ts"), "export const utils = true;").unwrap();
        fs::write(
            utils_path.join("src/helpers.ts"),
            "export const helpers = true;",
        )
        .unwrap();

        temp
    }

    #[test]
    fn test_workspace_detection() {
        let temp = create_test_workspace();
        let workspace = Workspace::detect(temp.path()).expect("Should detect workspace");

        assert_eq!(workspace.packages.len(), 2);
        assert!(workspace.packages.contains_key("@test/ui"));
        assert!(workspace.packages.contains_key("@test/utils"));
    }

    #[test]
    fn test_resolve_package_root() {
        let temp = create_test_workspace();
        let workspace = Workspace::detect(temp.path()).unwrap();

        // @test/ui should resolve to src/index.ts (fallback)
        let resolved = workspace.resolve("@test/ui");
        assert!(resolved.is_some());
        let path = resolved.unwrap();
        assert!(path.to_string_lossy().contains("index.ts"));
    }

    #[test]
    fn test_resolve_with_exports() {
        let temp = create_test_workspace();
        let workspace = Workspace::detect(temp.path()).unwrap();

        // @test/utils should resolve via exports field
        let resolved = workspace.resolve("@test/utils");
        assert!(resolved.is_some());
        let path = resolved.unwrap();
        assert!(path.to_string_lossy().contains("src/index.ts"));
    }

    #[test]
    fn test_resolve_subpath_export() {
        let temp = create_test_workspace();
        let workspace = Workspace::detect(temp.path()).unwrap();

        // @test/utils/helpers should resolve via exports field
        let resolved = workspace.resolve("@test/utils/helpers");
        assert!(resolved.is_some());
        let path = resolved.unwrap();
        assert!(path.to_string_lossy().contains("helpers.ts"));
    }

    #[test]
    fn test_no_workspace() {
        let temp = TempDir::new().unwrap();
        let workspace = Workspace::detect(temp.path());
        assert!(workspace.is_none());
    }
}
