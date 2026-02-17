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

#[cfg(test)]
mod tests {
    use super::*;

    // --- DiffStat tests ---

    #[test]
    fn diff_stat_empty() {
        let stat = DiffStat::empty();
        assert_eq!(stat.files_changed, 0);
        assert_eq!(stat.insertions, 0);
        assert_eq!(stat.deletions, 0);
    }

    #[test]
    fn diff_stat_display() {
        let stat = DiffStat {
            files_changed: 3,
            insertions: 42,
            deletions: 7,
        };
        assert_eq!(stat.to_string(), "3 file(s) changed, 42 insertion(s), 7 deletion(s)");
    }

    #[test]
    fn diff_stat_display_singular() {
        let stat = DiffStat {
            files_changed: 1,
            insertions: 1,
            deletions: 1,
        };
        assert_eq!(stat.to_string(), "1 file(s) changed, 1 insertion(s), 1 deletion(s)");
    }

    #[test]
    fn diff_stat_display_zero() {
        let stat = DiffStat::empty();
        assert_eq!(stat.to_string(), "0 file(s) changed, 0 insertion(s), 0 deletion(s)");
    }

    // --- DiffStatus tests ---

    #[test]
    fn diff_status_display() {
        assert_eq!(DiffStatus::Added.to_string(), "added");
        assert_eq!(DiffStatus::Modified.to_string(), "modified");
        assert_eq!(DiffStatus::Deleted.to_string(), "deleted");
        assert_eq!(DiffStatus::Renamed.to_string(), "renamed");
    }

    // --- Structural equality tests ---

    #[test]
    fn diff_line_equality() {
        let a = DiffLine {
            kind: DiffLineKind::Addition,
            content: "hello".to_string(),
        };
        let b = DiffLine {
            kind: DiffLineKind::Addition,
            content: "hello".to_string(),
        };
        assert_eq!(a, b);
    }

    #[test]
    fn diff_line_kind_equality() {
        assert_eq!(DiffLineKind::Context, DiffLineKind::Context);
        assert_ne!(DiffLineKind::Addition, DiffLineKind::Deletion);
    }

    #[test]
    fn file_diff_equality() {
        let a = FileDiff {
            path: PathBuf::from("src/main.rs"),
            old_path: None,
            status: DiffStatus::Added,
            hunks: vec![],
        };
        let b = FileDiff {
            path: PathBuf::from("src/main.rs"),
            old_path: None,
            status: DiffStatus::Added,
            hunks: vec![],
        };
        assert_eq!(a, b);
    }

    #[test]
    fn commit_info_equality() {
        let a = CommitInfo {
            short_sha: "abc1234".to_string(),
            sha: "abc1234def5678901234567890123456789abcde".to_string(),
            summary: "test commit".to_string(),
            message: "test commit".to_string(),
            author: "test".to_string(),
            timestamp: 1000,
        };
        let b = a.clone();
        assert_eq!(a, b);
    }
}
