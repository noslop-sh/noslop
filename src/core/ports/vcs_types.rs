//! VCS port types for diff and commit information
//!
//! These types are part of the `VersionControl` port interface.
//! They represent VCS-specific data structures used by the review
//! pipeline. They are NOT domain models -- domain types (Finding,
//! Review, Target, etc.) live in `core::models`.

use std::path::PathBuf;

use serde::Serialize;

/// A single file's diff between two revisions
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileDiff {
    /// Path to the file (new path if renamed)
    pub path: PathBuf,
    /// Original path if the file was renamed
    pub old_path: Option<PathBuf>,
    /// What happened to this file
    pub status: DiffStatus,
    /// Individual hunks of changes
    pub hunks: Vec<DiffHunk>,
}

/// What happened to a file in a diff
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum DiffStatus {
    /// File was added
    Added,
    /// File was modified
    Modified,
    /// File was deleted
    Deleted,
    /// File was renamed
    Renamed,
}

/// A contiguous region of changes within a file
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffHunk {
    /// Starting line in the old file
    pub old_start: u32,
    /// Number of lines from the old file
    pub old_lines: u32,
    /// Starting line in the new file
    pub new_start: u32,
    /// Number of lines in the new file
    pub new_lines: u32,
    /// Individual lines within this hunk
    pub lines: Vec<DiffLine>,
}

/// A single line within a diff hunk
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffLine {
    /// Whether this line was added, removed, or unchanged
    pub kind: DiffLineKind,
    /// The line content (without leading +/-/ )
    pub content: String,
}

/// Classification of a diff line
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffLineKind {
    /// Unchanged context line
    Context,
    /// Added line
    Addition,
    /// Removed line
    Deletion,
}

/// Metadata about a single commit
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitInfo {
    /// Short SHA (first 7 characters)
    pub short_sha: String,
    /// Full SHA
    pub sha: String,
    /// First line of the commit message
    pub summary: String,
    /// Full commit message
    pub message: String,
    /// Author name
    pub author: String,
    /// Unix timestamp of the commit
    pub timestamp: i64,
}

/// Summary statistics for a diff
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiffStat {
    /// Number of files changed
    pub files_changed: usize,
    /// Number of lines inserted
    pub insertions: usize,
    /// Number of lines deleted
    pub deletions: usize,
}

impl DiffStat {
    /// Create a new empty `DiffStat`
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            files_changed: 0,
            insertions: 0,
            deletions: 0,
        }
    }
}

impl std::fmt::Display for DiffStat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} file(s) changed, {} insertion(s), {} deletion(s)",
            self.files_changed, self.insertions, self.deletions
        )
    }
}

impl std::fmt::Display for DiffStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Added => write!(f, "added"),
            Self::Modified => write!(f, "modified"),
            Self::Deleted => write!(f, "deleted"),
            Self::Renamed => write!(f, "renamed"),
        }
    }
}
