//! Target matcher service - matches check targets to files
//!
//! This module contains pure matching logic with no I/O dependencies.

use std::path::Path;

/// Check if a target pattern matches a file path
///
/// Supports multiple pattern types:
/// - `*` - matches all files
/// - `*.ext` - matches files with extension
/// - `dir/*.ext` - matches files in directory with extension
/// - `dir/**/*.ext` - matches files recursively with extension
/// - Exact paths and prefix matches
///
/// # Arguments
///
/// * `target` - The target pattern to match
/// * `file` - The file path to check
/// * `base_dir` - Base directory for relative path resolution
/// * `cwd` - Current working directory
///
/// # Returns
///
/// `true` if the target pattern matches the file
#[must_use]
pub fn matches_target(target: &str, file: &str, base_dir: &Path, cwd: &Path) -> bool {
    // Get relative path from base_dir
    let file_abs = cwd.join(file);
    let file_rel = file_abs
        .strip_prefix(base_dir)
        .map_or_else(|_| file.to_string(), |p| p.to_string_lossy().to_string());

    // Wildcard: matches all files
    if target == "*" {
        return true;
    }

    // Glob-style: *.rs matches any .rs file
    if target.starts_with("*.") {
        let ext = &target[1..]; // ".rs"
        return file_rel.ends_with(ext);
    }

    // Glob-style: dir/*.ext matches files in dir with extension
    if let Some(star_pos) = target.find("/*") {
        // Check for double-star first
        if target[star_pos..].starts_with("/**") {
            return matches_recursive_glob(target, &file_rel, file);
        }

        let prefix = &target[..=star_pos]; // "src/"
        let suffix = &target[star_pos + 2..]; // ".rs" or ""

        // File must start with prefix
        if !file_rel.starts_with(prefix) && !file.starts_with(prefix) {
            return false;
        }

        // Get the part after prefix
        let remainder = file_rel
            .strip_prefix(prefix)
            .or_else(|| file.strip_prefix(prefix))
            .unwrap_or("");

        // For /*.ext, remainder must not contain / (direct children only) and must end with ext
        if !remainder.contains('/') {
            if suffix.is_empty() || suffix == "*" {
                return true;
            }
            if suffix.starts_with('.') {
                return remainder.ends_with(suffix);
            }
        }
        return false;
    }

    // Glob-style: dir/**/*.ext matches all files recursively
    if target.contains("/**") {
        return matches_recursive_glob(target, &file_rel, file);
    }

    // Exact or prefix match
    file_rel == target || file_rel.starts_with(target) || file.contains(target)
}

/// Match a recursive glob pattern (dir/**/*.ext)
fn matches_recursive_glob(target: &str, file_rel: &str, file: &str) -> bool {
    if let Some(doublestar_pos) = target.find("/**") {
        let prefix = &target[..=doublestar_pos]; // "src/"
        let suffix = target.strip_suffix(".rs").map_or("", |_| ".rs");

        if (file_rel.starts_with(prefix) || file.starts_with(prefix)) && file_rel.ends_with(suffix)
        {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn test_match(target: &str, file: &str) -> bool {
        let base = PathBuf::from("/repo");
        let cwd = PathBuf::from("/repo");
        matches_target(target, file, &base, &cwd)
    }

    #[test]
    fn test_wildcard_matches_all() {
        assert!(test_match("*", "src/main.rs"));
        assert!(test_match("*", "README.md"));
    }

    #[test]
    fn test_extension_glob() {
        assert!(test_match("*.rs", "main.rs"));
        assert!(test_match("*.rs", "src/lib.rs"));
        assert!(!test_match("*.rs", "main.py"));
    }

    #[test]
    fn test_dir_glob() {
        assert!(test_match("src/*.rs", "src/main.rs"));
        assert!(!test_match("src/*.rs", "src/sub/main.rs")); // Not recursive
        assert!(!test_match("src/*.rs", "tests/main.rs"));
    }

    #[test]
    fn test_recursive_glob() {
        assert!(test_match("src/**/*.rs", "src/main.rs"));
        assert!(test_match("src/**/*.rs", "src/sub/main.rs"));
        assert!(test_match("src/**/*.rs", "src/a/b/c/main.rs"));
        assert!(!test_match("src/**/*.rs", "tests/main.rs"));
    }

    #[test]
    fn test_exact_match() {
        assert!(test_match("src/main.rs", "src/main.rs"));
        assert!(!test_match("src/main.rs", "src/lib.rs"));
    }

    #[test]
    fn test_prefix_match() {
        assert!(test_match("src/", "src/main.rs"));
        assert!(test_match("src/", "src/sub/lib.rs"));
    }
}
