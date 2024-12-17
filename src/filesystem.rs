use log::debug;
use std::collections::HashSet;
use std::path::PathBuf;
use walkdir::{DirEntry, WalkDir};

fn normalize_path(path: &PathBuf) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.clone())
}

pub fn collect_files(dir: &str, excludes: &[String]) -> Vec<PathBuf> {
    let exclude_set: HashSet<_> = excludes.iter().collect();
    let base_dir = PathBuf::from(dir)
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from(dir));

    WalkDir::new(&base_dir)
        .into_iter()
        .filter_entry(|e| should_include(e, &exclude_set))
        .filter_map(|e| e.ok())
        .filter(|entry| {
            if entry.file_type().is_file() {
                match entry.path().extension().and_then(|ext| ext.to_str()) {
                    Some("ts" | "tsx" | "js" | "jsx" | "cjs" | "mjs") => true,
                    _ => false,
                }
            } else {
                false
            }
        })
        .map(|e| {
            let absolute_path = e
                .path()
                .canonicalize()
                .unwrap_or_else(|_| e.path().to_path_buf());
            let normalized = normalize_path(&absolute_path);
            debug!("Collected file: {:?}", normalized); // Debug statement
            normalized
        })
        .collect()
}

fn should_include(entry: &DirEntry, exclude_set: &HashSet<&String>) -> bool {
    let path = entry.path();
    if entry.file_type().is_dir() {
        if let Some(name) = path.file_name().and_then(|os| os.to_str()) {
            if exclude_set.contains(&name.to_string()) {
                return false;
            }
        }
    }
    true
}
