//! Shared test fixtures and helpers
//!
//! This module provides common utilities for testing noslop components.

use std::fs;
use std::path::Path;
use tempfile::TempDir;

/// A test repository with a standard directory structure
pub struct TestRepo {
    dir: TempDir,
}

impl TestRepo {
    /// Create a new test repository with standard structure:
    /// ```text
    /// /
    /// ├── src/
    /// │   ├── auth/
    /// │   │   ├── login.rs
    /// │   │   └── session.rs
    /// │   └── api/
    /// │       └── routes.rs
    /// └── README.md
    /// ```
    pub fn new() -> Self {
        let dir = TempDir::new().expect("failed to create temp dir");

        // Create directory structure
        fs::create_dir_all(dir.path().join("src/auth")).unwrap();
        fs::create_dir_all(dir.path().join("src/api")).unwrap();

        // Create test files with predictable content
        fs::write(
            dir.path().join("src/auth/login.rs"),
            "fn login() {\n    // line 2\n    // line 3\n}\n",
        )
        .unwrap();
        fs::write(dir.path().join("src/auth/session.rs"), "struct Session;\n").unwrap();
        fs::write(dir.path().join("src/api/routes.rs"), "fn routes() {}\n").unwrap();
        fs::write(dir.path().join("README.md"), "# Test\n").unwrap();

        Self { dir }
    }

    /// Get the root path of the test repository
    pub fn path(&self) -> &Path {
        self.dir.path()
    }

    /// Add a file to the test repository
    pub fn add_file(&self, path: &str, content: &str) {
        let full_path = self.dir.path().join(path);
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(full_path, content).unwrap();
    }

    /// Add a hidden file (starts with .)
    pub fn add_hidden_file(&self, name: &str, content: &str) {
        fs::write(self.dir.path().join(name), content).unwrap();
    }

    /// Add a hidden directory with a file
    pub fn add_hidden_dir(&self, dir_name: &str, file_name: &str, content: &str) {
        let dir_path = self.dir.path().join(dir_name);
        fs::create_dir_all(&dir_path).unwrap();
        fs::write(dir_path.join(file_name), content).unwrap();
    }
}

impl Default for TestRepo {
    fn default() -> Self {
        Self::new()
    }
}
