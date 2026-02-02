//! Temporary git repository helper for integration tests

use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

/// A temporary git repository for testing
pub struct TempGitRepo {
    _temp_dir: TempDir,
    path: PathBuf,
}

impl TempGitRepo {
    /// Create a new temporary git repository
    pub fn new() -> Self {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let path = temp_dir.path().to_path_buf();

        // Initialize git repo
        Command::new("git")
            .args(["init"])
            .current_dir(&path)
            .output()
            .expect("Failed to init git repo");

        // Configure git user
        Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(&path)
            .output()
            .expect("Failed to set git user.name");

        Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(&path)
            .output()
            .expect("Failed to set git user.email");

        Self {
            _temp_dir: temp_dir,
            path,
        }
    }

    /// Get the path to the repository
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Write a file to the repository
    pub fn write_file(&self, name: &str, content: &str) {
        let file_path = self.path.join(name);
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent).expect("Failed to create parent directories");
        }
        std::fs::write(file_path, content).expect("Failed to write file");
    }

    /// Stage a file
    pub fn stage(&self, name: &str) {
        Command::new("git")
            .args(["add", name])
            .current_dir(&self.path)
            .output()
            .expect("Failed to stage file");
    }

    /// Commit staged changes
    pub fn commit(&self, message: &str) {
        Command::new("git")
            .args(["commit", "-m", message])
            .current_dir(&self.path)
            .output()
            .expect("Failed to commit");
    }

    /// Run a git command and return output
    pub fn git(&self, args: &[&str]) -> std::process::Output {
        Command::new("git")
            .args(args)
            .current_dir(&self.path)
            .output()
            .expect("Failed to run git command")
    }
}

impl Default for TempGitRepo {
    fn default() -> Self {
        Self::new()
    }
}
