//! Review models
//!
//! A Review is a code review session containing comments on a diff.
//! `ReviewComment` extends Check with diff position and resolution status.

use serde::{Deserialize, Serialize};

use super::{Check, Severity};

/// A code review session on a commit range
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Review {
    /// Unique identifier (e.g., "REV-abc123")
    pub id: String,

    /// Base commit SHA (start of diff range)
    pub base_sha: String,

    /// Head commit SHA (end of diff range)
    pub head_sha: String,

    /// Review status
    pub status: ReviewStatus,

    /// Comments in this review
    pub comments: Vec<ReviewComment>,

    /// When this review was created
    pub created_at: String,

    /// When this review was closed (if closed)
    pub closed_at: Option<String>,

    /// Optional description
    pub description: Option<String>,
}

/// Review lifecycle status
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ReviewStatus {
    /// Review is active, comments block commits
    #[default]
    Open,
    /// Review is closed (merged or abandoned)
    Closed,
}

/// A review comment - extends Check with diff position and resolution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewComment {
    /// The underlying check (id, target, message, severity)
    pub check: Check,

    /// Position in the diff
    pub position: DiffPosition,

    /// Resolution status
    pub status: CommentStatus,

    /// When resolved (if resolved)
    pub resolved_at: Option<String>,

    /// Resolution message (if explicitly resolved)
    pub resolution_message: Option<String>,
}

/// Position within a diff
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct DiffPosition {
    /// Line number in the old file (base) - for deletions/context
    pub old_line: Option<u32>,

    /// Line number in the new file (head) - for additions/context
    pub new_line: Option<u32>,

    /// Which side of the diff this comment is on
    pub side: DiffSide,

    /// End line for multi-line comments
    pub end_line: Option<u32>,
}

/// Which side of a diff
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DiffSide {
    /// Left side (old/deleted code)
    Left,
    /// Right side (new/added code) - default
    #[default]
    Right,
}

/// Status of a review comment
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CommentStatus {
    /// Comment is open and blocks commits
    #[default]
    Open,
    /// Explicitly resolved
    Resolved,
}

// --- Review impl ---

impl Review {
    /// Create a new review for a commit range
    #[must_use]
    pub fn new(base_sha: String, head_sha: String) -> Self {
        Self {
            id: generate_review_id(),
            base_sha,
            head_sha,
            status: ReviewStatus::Open,
            comments: Vec::new(),
            created_at: chrono::Utc::now().to_rfc3339(),
            closed_at: None,
            description: None,
        }
    }

    /// Add a comment to this review
    pub fn add_comment(&mut self, target: String, message: String, position: DiffPosition) {
        let index = self.comments.len() + 1;
        let check =
            Check::new(Some(format!("{}:{index}", self.id)), target, message, Severity::Block);
        self.comments.push(ReviewComment {
            check,
            position,
            status: CommentStatus::Open,
            resolved_at: None,
            resolution_message: None,
        });
    }

    /// Get all open comments
    #[must_use]
    pub fn open_comments(&self) -> Vec<&ReviewComment> {
        self.comments.iter().filter(|c| c.status == CommentStatus::Open).collect()
    }

    /// Get open comments for a specific file
    #[must_use]
    pub fn open_comments_for_file(&self, file: &str) -> Vec<&ReviewComment> {
        self.comments
            .iter()
            .filter(|c| c.status == CommentStatus::Open && c.check.applies_to(file))
            .collect()
    }

    /// Extract all open checks (for commit validation)
    #[must_use]
    pub fn open_checks(&self) -> Vec<&Check> {
        self.comments
            .iter()
            .filter(|c| c.status == CommentStatus::Open)
            .map(|c| &c.check)
            .collect()
    }

    /// Close this review
    pub fn close(&mut self) {
        self.status = ReviewStatus::Closed;
        self.closed_at = Some(chrono::Utc::now().to_rfc3339());
    }

    /// Reopen this review
    pub fn reopen(&mut self) {
        self.status = ReviewStatus::Open;
        self.closed_at = None;
    }

    /// Check if this review is open
    #[must_use]
    pub fn is_open(&self) -> bool {
        self.status == ReviewStatus::Open
    }
}

// --- ReviewComment impl ---

impl ReviewComment {
    /// Resolve this comment
    pub fn resolve(&mut self, message: Option<String>) {
        self.status = CommentStatus::Resolved;
        self.resolved_at = Some(chrono::Utc::now().to_rfc3339());
        self.resolution_message = message;
    }

    /// Check if this comment is open
    #[must_use]
    pub fn is_open(&self) -> bool {
        self.status == CommentStatus::Open
    }

    /// Get display string for line position
    #[must_use]
    pub fn line_display(&self) -> String {
        match (self.position.new_line, self.position.end_line) {
            (Some(start), Some(end)) if start != end => format!("L{start}-L{end}"),
            (Some(line), _) => format!("L{line}"),
            (None, _) => self
                .position
                .old_line
                .map_or_else(|| "file".to_string(), |line| format!("L{line} (old)")),
        }
    }
}

// --- DiffPosition impl ---

impl DiffPosition {
    /// Position for a new/added line
    #[must_use]
    pub const fn new_line(line: u32) -> Self {
        Self {
            old_line: None,
            new_line: Some(line),
            side: DiffSide::Right,
            end_line: None,
        }
    }

    /// Position for an old/deleted line
    #[must_use]
    pub const fn old_line(line: u32) -> Self {
        Self {
            old_line: Some(line),
            new_line: None,
            side: DiffSide::Left,
            end_line: None,
        }
    }

    /// Position for a range of new lines
    #[must_use]
    pub const fn line_range(start: u32, end: u32) -> Self {
        Self {
            old_line: None,
            new_line: Some(start),
            side: DiffSide::Right,
            end_line: Some(end),
        }
    }
}

// --- ID generation ---

#[allow(clippy::cast_possible_truncation)]
fn generate_review_id() -> String {
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static COUNTER: AtomicU32 = AtomicU32::new(0);

    let ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
    let counter = COUNTER.fetch_add(1, Ordering::Relaxed);

    format!("REV-{:04x}{:04x}", (ts & 0xFFFF) as u16, counter as u16)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_review() {
        let review = Review::new("abc123".to_string(), "def456".to_string());

        assert!(review.id.starts_with("REV-"));
        assert_eq!(review.base_sha, "abc123");
        assert_eq!(review.head_sha, "def456");
        assert!(review.is_open());
        assert!(review.comments.is_empty());
    }

    #[test]
    fn test_add_comment() {
        let mut review = Review::new("base".to_string(), "head".to_string());
        let review_id = review.id.clone();

        review.add_comment(
            "src/main.rs".to_string(),
            "Add error handling".to_string(),
            DiffPosition::new_line(42),
        );

        assert_eq!(review.comments.len(), 1);
        assert_eq!(review.comments[0].check.id, format!("{review_id}:1"));
        assert_eq!(review.comments[0].check.target, "src/main.rs");
        assert!(review.comments[0].is_open());
    }

    #[test]
    fn test_open_checks() {
        let mut review = Review::new("base".to_string(), "head".to_string());

        review.add_comment(
            "src/a.rs".to_string(),
            "Comment 1".to_string(),
            DiffPosition::new_line(10),
        );
        review.add_comment(
            "src/b.rs".to_string(),
            "Comment 2".to_string(),
            DiffPosition::new_line(20),
        );

        // Resolve first comment
        review.comments[0].resolve(Some("Fixed".to_string()));

        let open = review.open_checks();
        assert_eq!(open.len(), 1);
        assert_eq!(open[0].target, "src/b.rs");
    }

    #[test]
    fn test_resolve_comment() {
        let mut review = Review::new("base".to_string(), "head".to_string());
        review.add_comment(
            "src/main.rs".to_string(),
            "Fix this".to_string(),
            DiffPosition::new_line(10),
        );

        assert!(review.comments[0].is_open());

        review.comments[0].resolve(Some("Done".to_string()));

        assert!(!review.comments[0].is_open());
        assert_eq!(review.comments[0].status, CommentStatus::Resolved);
        assert!(review.comments[0].resolved_at.is_some());
    }

    #[test]
    fn test_close_reopen() {
        let mut review = Review::new("base".to_string(), "head".to_string());

        assert!(review.is_open());

        review.close();
        assert!(!review.is_open());
        assert!(review.closed_at.is_some());

        review.reopen();
        assert!(review.is_open());
        assert!(review.closed_at.is_none());
    }

    #[test]
    fn test_line_display() {
        let comment = ReviewComment {
            check: Check::new(None, "f".to_string(), "m".to_string(), Severity::Block),
            position: DiffPosition::new_line(42),
            status: CommentStatus::Open,
            resolved_at: None,
            resolution_message: None,
        };
        assert_eq!(comment.line_display(), "L42");

        let range_comment = ReviewComment {
            check: Check::new(None, "f".to_string(), "m".to_string(), Severity::Block),
            position: DiffPosition::line_range(10, 20),
            status: CommentStatus::Open,
            resolved_at: None,
            resolution_message: None,
        };
        assert_eq!(range_comment.line_display(), "L10-L20");
    }
}
