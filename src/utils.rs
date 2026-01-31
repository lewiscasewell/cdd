//! Common utility functions used across the crate.

use sha2::{Digest, Sha256};
use std::path::Path;

/// Encode bytes as a hexadecimal string.
pub fn hex_encode(bytes: &[u8]) -> String {
    use std::fmt::Write;
    bytes.iter().fold(String::new(), |mut acc, b| {
        let _ = write!(acc, "{:02x}", b);
        acc
    })
}

/// Compute a SHA256 hash of the input strings and return the first N hex characters.
pub fn hash_strings(strings: &[String], hex_len: usize) -> String {
    let mut hasher = Sha256::new();

    for s in strings {
        hasher.update(s.as_bytes());
        hasher.update(b"\n");
    }

    let result = hasher.finalize();
    let bytes_needed = (hex_len + 1) / 2;
    hex_encode(&result[..bytes_needed.min(result.len())])[..hex_len].to_string()
}

/// Convert an absolute path to a relative path string for display/hashing.
/// Returns the path relative to root, or the original path if stripping fails.
pub fn relative_path_string(path: &Path, root: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}

/// List of supported file extensions for TypeScript/JavaScript files.
pub const EXTENSIONS: &[&str] = &[".tsx", ".ts", ".jsx", ".js", ".cjs", ".mjs"];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hex_encode() {
        assert_eq!(hex_encode(&[0x00, 0xff, 0xab]), "00ffab");
        assert_eq!(hex_encode(&[]), "");
    }

    #[test]
    fn test_hash_strings_deterministic() {
        let strings = vec!["foo".to_string(), "bar".to_string()];
        let hash1 = hash_strings(&strings, 12);
        let hash2 = hash_strings(&strings, 12);
        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 12);
    }

    #[test]
    fn test_hash_strings_order_matters() {
        let strings1 = vec!["foo".to_string(), "bar".to_string()];
        let strings2 = vec!["bar".to_string(), "foo".to_string()];
        let hash1 = hash_strings(&strings1, 12);
        let hash2 = hash_strings(&strings2, 12);
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_relative_path_string() {
        use std::path::PathBuf;

        let root = PathBuf::from("/home/user/project");
        let path = PathBuf::from("/home/user/project/src/index.ts");
        assert_eq!(relative_path_string(&path, &root), "src/index.ts");

        // When path is not under root, returns the full path
        let other_path = PathBuf::from("/other/path/file.ts");
        assert_eq!(
            relative_path_string(&other_path, &root),
            "/other/path/file.ts"
        );
    }
}
