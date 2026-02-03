//! Git integration adapter
//!
//! Implements `VersionControl` trait using git commands.
//!
//! - [`hooks`] - Git hooks installation
//! - [`staging`] - Staged file detection

pub mod hooks;
pub mod staging;

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::core::ports::VersionControl;

pub use hooks::{install_commit_msg, install_post_commit, install_pre_commit, remove_noslop_hooks};
pub use staging::get_staged_files;

/// Git-based version control implementation
#[derive(Debug, Clone)]
pub struct GitVersionControl {
    /// Working directory
    workdir: PathBuf,
}

impl GitVersionControl {
    /// Create a new git version control adapter
    #[must_use]
    pub const fn new(workdir: PathBuf) -> Self {
        Self { workdir }
    }

    /// Create a git adapter for the current directory
    ///
    /// # Errors
    ///
    /// Returns an error if the current directory cannot be determined.
    pub fn current_dir() -> anyhow::Result<Self> {
        Ok(Self::new(std::env::current_dir()?))
    }
}

impl Default for GitVersionControl {
    fn default() -> Self {
        Self::new(std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
    }
}

impl VersionControl for GitVersionControl {
    fn staged_files(&self) -> anyhow::Result<Vec<String>> {
        get_staged_files()
    }

    fn repo_name(&self) -> String {
        get_repo_name()
    }

    fn repo_root(&self) -> anyhow::Result<PathBuf> {
        let output = Command::new("git")
            .current_dir(&self.workdir)
            .args(["rev-parse", "--show-toplevel"])
            .output()?;

        if !output.status.success() {
            anyhow::bail!("Not a git repository");
        }

        let root = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(PathBuf::from(root))
    }

    fn install_hooks(&self, force: bool) -> anyhow::Result<()> {
        if force {
            remove_noslop_hooks()?;
        }
        install_pre_commit()?;
        install_commit_msg()?;
        install_post_commit()?;
        Ok(())
    }

    fn is_inside_repo(&self, path: &Path) -> bool {
        let check_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.workdir.join(path)
        };

        Command::new("git")
            .current_dir(&check_path)
            .args(["rev-parse", "--is-inside-work-tree"])
            .output()
            .is_ok_and(|o| o.status.success())
    }

    fn current_branch(&self) -> anyhow::Result<Option<String>> {
        let output = Command::new("git")
            .current_dir(&self.workdir)
            .args(["branch", "--show-current"])
            .output()?;

        if !output.status.success() {
            return Ok(None);
        }

        let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if branch.is_empty() {
            Ok(None) // Detached HEAD
        } else {
            Ok(Some(branch))
        }
    }
}

/// Get the repository name from git remote or directory name
#[must_use]
pub fn get_repo_name() -> String {
    // Try to get repo name from git remote
    Command::new("git")
        .args(["remote", "get-url", "origin"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| {
            // Extract repo name from URL: https://github.com/user/repo.git -> repo
            s.trim().rsplit('/').next().map(|n| n.trim_end_matches(".git").to_string())
        })
        .or_else(|| {
            // Fallback: use directory name
            std::env::current_dir()
                .ok()
                .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
        })
        .unwrap_or_else(|| "project".to_string())
}
