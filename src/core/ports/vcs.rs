//! Version control system port
//!
//! Defines the interface for interacting with version control.

use std::path::Path;

/// Version control system abstraction
///
/// Implementations handle interactions with git or other VCS systems.
#[cfg_attr(test, mockall::automock)]
pub trait VersionControl: Send + Sync {
    /// Get list of staged files (ready to be committed)
    fn staged_files(&self) -> anyhow::Result<Vec<String>>;

    /// Get the repository name
    fn repo_name(&self) -> String;

    /// Get the repository root path
    fn repo_root(&self) -> anyhow::Result<std::path::PathBuf>;

    /// Install git hooks for noslop integration
    fn install_hooks(&self, force: bool) -> anyhow::Result<()>;

    /// Check if a path is inside the repository
    fn is_inside_repo(&self, path: &Path) -> bool;

    /// Get the current branch name
    fn current_branch(&self) -> anyhow::Result<Option<String>>;
}
