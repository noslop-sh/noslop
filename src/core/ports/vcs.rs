//! Version control system port
//!
//! Defines the interface for interacting with version control.

use std::path::Path;

use super::vcs_types::{CommitInfo, DiffStat, FileDiff};

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

    // --- checkpoint methods ---

    /// Returns true if no staged, unstaged, or untracked changes exist.
    fn is_clean(&self) -> anyhow::Result<bool>;

    /// Stage all changes and commit with the given message, bypassing hooks.
    /// Returns the short SHA of the new commit, or None if the tree was clean.
    fn commit_all(&self, message: &str) -> anyhow::Result<Option<String>>;

    // --- review pipeline methods ---

    /// Diff between two refs. Primary input to the review pipeline.
    fn diff_between(&self, base: &str, head: &str) -> anyhow::Result<Vec<FileDiff>>;

    /// List commits between two refs (for commit message analysis).
    fn commits_between(&self, base: &str, head: &str) -> anyhow::Result<Vec<CommitInfo>>;

    /// Read file content at a specific revision (for full-file context).
    fn file_at_revision(&self, path: &Path, rev: &str) -> anyhow::Result<String>;

    /// Recent commits from HEAD (for context).
    fn recent_commits(&self, count: usize) -> anyhow::Result<Vec<CommitInfo>>;

    /// Summary statistics for changes since a ref.
    fn diff_stat_since(&self, since: &str) -> anyhow::Result<DiffStat>;
}
