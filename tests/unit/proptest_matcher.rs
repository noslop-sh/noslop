//! Property-based tests for the matcher module
//!
//! Uses proptest to verify properties that should hold for all inputs.

use noslop::core::services::matches_target;
use proptest::prelude::*;
use std::path::PathBuf;

proptest! {
    /// Wildcard "*" should match any file
    #[test]
    fn wildcard_matches_any_file(file in "[a-z]+\\.[a-z]+") {
        let base = PathBuf::from("/repo");
        let cwd = PathBuf::from("/repo");
        prop_assert!(matches_target("*", &file, &base, &cwd));
    }

    /// Extension glob "*.ext" should match files with that extension
    #[test]
    fn extension_glob_matches_extension(
        name in "[a-z]{1,10}",
        ext in "[a-z]{1,5}"
    ) {
        let file = format!("{name}.{ext}");
        let pattern = format!("*.{ext}");
        let base = PathBuf::from("/repo");
        let cwd = PathBuf::from("/repo");
        prop_assert!(matches_target(&pattern, &file, &base, &cwd));
    }

    /// Extension glob should NOT match different extensions
    #[test]
    fn extension_glob_rejects_different_extension(
        name in "[a-z]{1,10}",
        ext1 in "[a-z]{2,5}",
        ext2 in "[a-z]{2,5}"
    ) {
        prop_assume!(ext1 != ext2);
        let file = format!("{name}.{ext1}");
        let pattern = format!("*.{ext2}");
        let base = PathBuf::from("/repo");
        let cwd = PathBuf::from("/repo");
        prop_assert!(!matches_target(&pattern, &file, &base, &cwd));
    }

    /// Exact path should match itself
    #[test]
    fn exact_path_matches_self(
        dir in "[a-z]{1,5}",
        file in "[a-z]{1,10}\\.[a-z]{1,5}"
    ) {
        let path = format!("{dir}/{file}");
        let base = PathBuf::from("/repo");
        let cwd = PathBuf::from("/repo");
        prop_assert!(matches_target(&path, &path, &base, &cwd));
    }

    /// Directory prefix should match files in that directory
    #[test]
    fn dir_prefix_matches_subfiles(
        dir in "[a-z]{1,5}",
        file in "[a-z]{1,10}\\.[a-z]{1,5}"
    ) {
        let path = format!("{dir}/{file}");
        let pattern = format!("{dir}/");
        let base = PathBuf::from("/repo");
        let cwd = PathBuf::from("/repo");
        prop_assert!(matches_target(&pattern, &path, &base, &cwd));
    }
}

#[cfg(test)]
mod deterministic_tests {
    use super::*;

    #[test]
    fn recursive_glob_matches_nested() {
        let base = PathBuf::from("/repo");
        let cwd = PathBuf::from("/repo");

        assert!(matches_target("src/**/*.rs", "src/main.rs", &base, &cwd));
        assert!(matches_target("src/**/*.rs", "src/lib/mod.rs", &base, &cwd));
        assert!(matches_target("src/**/*.rs", "src/a/b/c.rs", &base, &cwd));
    }

    #[test]
    fn dir_glob_does_not_match_nested() {
        let base = PathBuf::from("/repo");
        let cwd = PathBuf::from("/repo");

        assert!(matches_target("src/*.rs", "src/main.rs", &base, &cwd));
        assert!(!matches_target("src/*.rs", "src/lib/mod.rs", &base, &cwd));
    }
}
