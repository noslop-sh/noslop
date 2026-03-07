//! Git integration adapter
//!
//! Implements `VersionControl` trait using git commands and `git2`.
//!
//! - [`hooks`] - Git hooks installation
//! - [`staging`] - Staged file detection
//! - [`checkpoint`] - Clean-check and commit-all operations
//! - [`diff_types`] - VCS-specific diff/commit data types

pub mod checkpoint;
pub mod diff_types;
pub mod hooks;
pub mod staging;

use std::path::{Path, PathBuf};
use std::process::Command;

use git2::Repository;

use crate::core::ports::VersionControl;
use crate::core::ports::vcs_types::{
    CommitInfo, DiffHunk, DiffLine, DiffLineKind, DiffStat, DiffStatus, FileDiff,
};

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

    /// Open the git2 repository for the working directory
    fn open_repo(&self) -> anyhow::Result<Repository> {
        Repository::discover(&self.workdir)
            .map_err(|e| anyhow::anyhow!("Not a git repository: {e}"))
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

    // --- checkpoint methods ---

    fn is_clean(&self) -> anyhow::Result<bool> {
        let repo = self.open_repo()?;
        checkpoint::is_clean(&repo)
    }

    fn commit_all(&self, message: &str) -> anyhow::Result<Option<String>> {
        let repo = self.open_repo()?;
        checkpoint::commit_all(&repo, message)
    }

    // --- review pipeline methods ---

    fn diff_between(&self, base: &str, head: &str) -> anyhow::Result<Vec<FileDiff>> {
        let repo = self.open_repo()?;
        let base_obj = repo
            .revparse_single(base)
            .map_err(|e| anyhow::anyhow!("Cannot resolve ref '{base}': {e}"))?;
        let head_obj = repo
            .revparse_single(head)
            .map_err(|e| anyhow::anyhow!("Cannot resolve ref '{head}': {e}"))?;

        let base_tree = base_obj
            .peel_to_tree()
            .map_err(|e| anyhow::anyhow!("Cannot peel '{base}' to tree: {e}"))?;
        let head_tree = head_obj
            .peel_to_tree()
            .map_err(|e| anyhow::anyhow!("Cannot peel '{head}' to tree: {e}"))?;

        let diff = repo.diff_tree_to_tree(Some(&base_tree), Some(&head_tree), None)?;
        parse_diff(&diff)
    }

    fn commits_between(&self, base: &str, head: &str) -> anyhow::Result<Vec<CommitInfo>> {
        let repo = self.open_repo()?;
        let head_obj = repo
            .revparse_single(head)
            .map_err(|e| anyhow::anyhow!("Cannot resolve ref '{head}': {e}"))?;
        let base_obj = repo
            .revparse_single(base)
            .map_err(|e| anyhow::anyhow!("Cannot resolve ref '{base}': {e}"))?;

        let mut revwalk = repo.revwalk()?;
        revwalk.push(head_obj.id())?;
        revwalk.hide(base_obj.id())?;
        revwalk.set_sorting(git2::Sort::TOPOLOGICAL | git2::Sort::TIME)?;

        let mut commits = Vec::new();
        for oid in revwalk {
            let oid = oid?;
            let commit = repo.find_commit(oid)?;
            commits.push(commit_to_info(&commit));
        }
        Ok(commits)
    }

    fn file_at_revision(&self, path: &Path, rev: &str) -> anyhow::Result<String> {
        let repo = self.open_repo()?;
        let obj = repo
            .revparse_single(rev)
            .map_err(|e| anyhow::anyhow!("Cannot resolve ref '{rev}': {e}"))?;
        let tree = obj
            .peel_to_tree()
            .map_err(|e| anyhow::anyhow!("Cannot peel '{rev}' to tree: {e}"))?;

        let entry = tree.get_path(path).map_err(|e| {
            anyhow::anyhow!("File '{}' not found at revision '{rev}': {e}", path.display())
        })?;
        let blob = repo
            .find_blob(entry.id())
            .map_err(|e| anyhow::anyhow!("Cannot read blob for '{}': {e}", path.display()))?;

        let content = std::str::from_utf8(blob.content())
            .map_err(|e| anyhow::anyhow!("File '{}' is not valid UTF-8: {e}", path.display()))?;
        Ok(content.to_string())
    }

    fn recent_commits(&self, count: usize) -> anyhow::Result<Vec<CommitInfo>> {
        let repo = self.open_repo()?;
        let head = repo.head()?.peel_to_commit()?;

        let mut revwalk = repo.revwalk()?;
        revwalk.push(head.id())?;
        revwalk.set_sorting(git2::Sort::TOPOLOGICAL | git2::Sort::TIME)?;

        let mut commits = Vec::new();
        for oid in revwalk.take(count) {
            let oid = oid?;
            let commit = repo.find_commit(oid)?;
            commits.push(commit_to_info(&commit));
        }
        Ok(commits)
    }

    fn diff_stat_since(&self, since: &str) -> anyhow::Result<DiffStat> {
        let repo = self.open_repo()?;
        let since_obj = repo
            .revparse_single(since)
            .map_err(|e| anyhow::anyhow!("Cannot resolve ref '{since}': {e}"))?;
        let since_tree = since_obj
            .peel_to_tree()
            .map_err(|e| anyhow::anyhow!("Cannot peel '{since}' to tree: {e}"))?;

        let head = repo.head()?.peel_to_commit()?;
        let head_tree = head.tree()?;

        let diff = repo.diff_tree_to_tree(Some(&since_tree), Some(&head_tree), None)?;
        let stats = diff.stats()?;

        Ok(DiffStat {
            files_changed: stats.files_changed(),
            insertions: stats.insertions(),
            deletions: stats.deletions(),
        })
    }
}

/// Convert a `git2::Commit` to our `CommitInfo` type
fn commit_to_info(commit: &git2::Commit<'_>) -> CommitInfo {
    let sha = commit.id().to_string();
    let short_sha = if sha.len() >= 7 {
        sha[..7].to_string()
    } else {
        sha.clone()
    };
    let summary = commit.summary().unwrap_or("").to_string();
    let message = commit.message().unwrap_or("").to_string();
    let author = commit.author().name().unwrap_or("unknown").to_string();
    let timestamp = commit.time().seconds();

    CommitInfo {
        short_sha,
        sha,
        summary,
        message,
        author,
        timestamp,
    }
}

/// Parse a `git2::Diff` into our `Vec<FileDiff>` type
fn parse_diff(diff: &git2::Diff<'_>) -> anyhow::Result<Vec<FileDiff>> {
    let mut file_diffs = Vec::new();

    for (delta_idx, delta) in diff.deltas().enumerate() {
        let status = match delta.status() {
            git2::Delta::Added => DiffStatus::Added,
            git2::Delta::Deleted => DiffStatus::Deleted,
            git2::Delta::Renamed => DiffStatus::Renamed,
            // Modified, copies, and other statuses all map to Modified
            _ => DiffStatus::Modified,
        };

        let file_path = delta.new_file().path().unwrap_or_else(|| Path::new("")).to_path_buf();

        let old_path = if status == DiffStatus::Renamed {
            delta.old_file().path().map(Path::to_path_buf)
        } else {
            None
        };

        // Parse hunks for this delta
        let mut hunks = Vec::new();
        let maybe_patch = git2::Patch::from_diff(diff, delta_idx)?;
        if let Some(patch) = maybe_patch {
            for hunk_idx in 0..patch.num_hunks() {
                let (hunk, _) = patch.hunk(hunk_idx)?;
                let mut lines = Vec::new();

                for line_idx in 0..patch.num_lines_in_hunk(hunk_idx)? {
                    let line = patch.line_in_hunk(hunk_idx, line_idx)?;
                    let kind = match line.origin() {
                        '+' => DiffLineKind::Addition,
                        '-' => DiffLineKind::Deletion,
                        ' ' => DiffLineKind::Context,
                        // Header lines, etc. -- skip
                        _ => continue,
                    };
                    let content = std::str::from_utf8(line.content()).unwrap_or("").to_string();
                    lines.push(DiffLine { kind, content });
                }

                hunks.push(DiffHunk {
                    old_start: hunk.old_start(),
                    old_lines: hunk.old_lines(),
                    new_start: hunk.new_start(),
                    new_lines: hunk.new_lines(),
                    lines,
                });
            }
        }

        file_diffs.push(FileDiff {
            path: file_path,
            old_path,
            status,
            hunks,
        });
    }

    Ok(file_diffs)
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
