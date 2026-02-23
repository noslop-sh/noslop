//! Review model
//!
//! A review session on a branch diff. Contains all feedbacks from all
//! analyzers across all pipeline runs.

use chrono::Utc;
use serde::{Deserialize, Serialize};

use super::feedback::Feedback;

/// A code review session on a branch diff.
///
/// Created by `noslop review`. Contains feedbacks from every analyzer
/// that ran. The pre-push hook checks `is_blocked()` to gate push.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Review {
    /// Unique identifier (e.g., "REV-a1b2c3d4").
    pub id: String,
    /// Base commit SHA (typically merge-base with main).
    pub base: String,
    /// Head commit SHA (tip of the feature branch).
    pub head: String,
    /// Lifecycle state.
    pub status: ReviewStatus,
    /// All feedbacks from all sources.
    #[serde(alias = "findings")]
    pub feedbacks: Vec<Feedback>,
    /// Branch name this review was created for.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    /// File paths that have been viewed/reviewed.
    #[serde(default)]
    pub viewed_files: Vec<String>,
    /// ISO 8601 timestamp of creation.
    pub created_at: String,
    /// ISO 8601 timestamp when closed (if closed).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub closed_at: Option<String>,
}

/// Lifecycle state of a review.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReviewStatus {
    /// Review is active
    #[default]
    Open,
    /// Review is closed (merged or abandoned)
    Closed,
}

impl Review {
    /// Create a new open review for a commit range.
    #[must_use]
    pub fn new(base: impl Into<String>, head: impl Into<String>) -> Self {
        Self {
            id: generate_review_id(),
            base: base.into(),
            head: head.into(),
            status: ReviewStatus::Open,
            feedbacks: Vec::new(),
            branch: None,
            viewed_files: Vec::new(),
            created_at: Utc::now().to_rfc3339(),
            closed_at: None,
        }
    }

    /// Add a feedback to this review.
    pub fn add_feedback(&mut self, feedback: Feedback) {
        self.feedbacks.push(feedback);
    }

    /// Add multiple feedbacks from an analyzer pass.
    pub fn add_feedbacks(&mut self, feedbacks: impl IntoIterator<Item = Feedback>) {
        self.feedbacks.extend(feedbacks);
    }

    /// All feedbacks that currently prevent pushing
    /// (severity == Block and status == Open).
    #[must_use]
    pub fn blocking_feedbacks(&self) -> Vec<&Feedback> {
        self.feedbacks.iter().filter(|f| f.is_blocking()).collect()
    }

    /// Whether this review has any blocking feedbacks.
    /// Used by pre-push hook: `noslop review --check` exits 1 when true.
    #[must_use]
    pub fn is_blocked(&self) -> bool {
        self.feedbacks.iter().any(Feedback::is_blocking)
    }

    /// All feedbacks that target a specific file path.
    #[must_use]
    pub fn feedbacks_for_file(&self, path: &str) -> Vec<&Feedback> {
        self.feedbacks.iter().filter(|f| f.target.path == path).collect()
    }

    /// All feedbacks that are still open (any severity).
    #[must_use]
    pub fn open_feedbacks(&self) -> Vec<&Feedback> {
        self.feedbacks.iter().filter(|f| f.is_open()).collect()
    }

    /// Close this review.
    pub fn close(&mut self) {
        self.status = ReviewStatus::Closed;
        self.closed_at = Some(Utc::now().to_rfc3339());
    }

    /// Reopen a closed review.
    pub fn reopen(&mut self) {
        self.status = ReviewStatus::Open;
        self.closed_at = None;
    }

    /// Toggle a file path in the viewed files list.
    /// If already present, removes it; otherwise adds it.
    pub fn mark_file_viewed(&mut self, path: String) {
        if let Some(pos) = self.viewed_files.iter().position(|p| *p == path) {
            self.viewed_files.remove(pos);
        } else {
            self.viewed_files.push(path);
        }
    }

    /// Whether this review is open.
    #[must_use]
    pub fn is_open(&self) -> bool {
        self.status == ReviewStatus::Open
    }
}

fn generate_review_id() -> String {
    use uuid::Uuid;
    format!("REV-{}", &Uuid::new_v4().to_string()[..8])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::models::primitives::{FeedbackSource, Severity, Span, Target};

    #[test]
    fn new_review() {
        let review = Review::new("abc123", "def456");
        assert!(review.id.starts_with("REV-"));
        assert_eq!(review.base, "abc123");
        assert_eq!(review.head, "def456");
        assert!(review.is_open());
        assert!(review.feedbacks.is_empty());
    }

    #[test]
    fn add_feedback() {
        let mut review = Review::new("base", "head");
        let feedback = Feedback::new(
            Target::file("src/auth.rs").with_span(Span::line(42)),
            Severity::Block,
            "Session handling changed",
            FeedbackSource::Check("NOS-1".into()),
        );
        review.add_feedback(feedback);
        assert_eq!(review.feedbacks.len(), 1);
        assert!(review.is_blocked());
    }

    #[test]
    fn blocking_feedbacks() {
        let mut review = Review::new("base", "head");
        review.add_feedback(Feedback::new(
            Target::file("src/auth.rs"),
            Severity::Block,
            "Blocker",
            FeedbackSource::Check("NOS-1".into()),
        ));
        review.add_feedback(Feedback::new(
            Target::file("src/auth.rs"),
            Severity::Warn,
            "Warning",
            FeedbackSource::Agent("security".into()),
        ));
        assert_eq!(review.blocking_feedbacks().len(), 1);
        assert!(review.is_blocked());
    }

    #[test]
    fn resolve_removes_block() {
        let mut review = Review::new("base", "head");
        review.add_feedback(Feedback::new(
            Target::file("src/auth.rs"),
            Severity::Block,
            "Blocker",
            FeedbackSource::Check("NOS-1".into()),
        ));
        assert!(review.is_blocked());

        review.feedbacks[0].resolve(None);
        assert!(!review.is_blocked());
    }

    #[test]
    fn feedbacks_for_file() {
        let mut review = Review::new("base", "head");
        review.add_feedback(Feedback::new(
            Target::file("src/auth.rs"),
            Severity::Block,
            "Auth feedback",
            FeedbackSource::Human,
        ));
        review.add_feedback(Feedback::new(
            Target::file("src/main.rs"),
            Severity::Warn,
            "Main feedback",
            FeedbackSource::Human,
        ));
        assert_eq!(review.feedbacks_for_file("src/auth.rs").len(), 1);
        assert_eq!(review.feedbacks_for_file("src/main.rs").len(), 1);
        assert_eq!(review.feedbacks_for_file("src/other.rs").len(), 0);
    }

    #[test]
    fn close_review() {
        let mut review = Review::new("base", "head");
        assert!(review.is_open());

        review.close();
        assert!(!review.is_open());
        assert_eq!(review.status, ReviewStatus::Closed);
        assert!(review.closed_at.is_some());
    }

    #[test]
    fn open_feedbacks() {
        let mut review = Review::new("base", "head");
        review.add_feedback(Feedback::new(
            Target::file("a.rs"),
            Severity::Block,
            "Open",
            FeedbackSource::Human,
        ));
        review.add_feedback(Feedback::new(
            Target::file("b.rs"),
            Severity::Block,
            "Also open",
            FeedbackSource::Human,
        ));
        review.feedbacks[0].resolve(None);
        assert_eq!(review.open_feedbacks().len(), 1);
    }

    #[test]
    fn review_serde_roundtrip() {
        let mut review = Review::new("abc", "def");
        review.add_feedback(Feedback::new(
            Target::file("src/auth.rs"),
            Severity::Block,
            "Test",
            FeedbackSource::Human,
        ));
        let json = serde_json::to_string(&review).unwrap();
        let parsed: Review = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, review.id);
        assert_eq!(parsed.feedbacks.len(), 1);
    }

    // --- Additional edge case tests ---

    #[test]
    fn review_id_format() {
        let review = Review::new("base", "head");
        assert!(review.id.starts_with("REV-"));
        assert_eq!(review.id.len(), 12); // "REV-" + 8 hex chars
    }

    #[test]
    fn review_add_feedbacks_batch() {
        let mut review = Review::new("base", "head");
        let feedbacks = vec![
            Feedback::new(Target::file("a.rs"), Severity::Block, "A", FeedbackSource::Human),
            Feedback::new(Target::file("b.rs"), Severity::Warn, "B", FeedbackSource::Human),
            Feedback::new(Target::file("c.rs"), Severity::Info, "C", FeedbackSource::Human),
        ];
        review.add_feedbacks(feedbacks);
        assert_eq!(review.feedbacks.len(), 3);
    }

    #[test]
    fn review_not_blocked_by_warnings_only() {
        let mut review = Review::new("base", "head");
        review.add_feedback(Feedback::new(
            Target::file("src/auth.rs"),
            Severity::Warn,
            "Warning only",
            FeedbackSource::Human,
        ));
        review.add_feedback(Feedback::new(
            Target::file("src/main.rs"),
            Severity::Info,
            "Info only",
            FeedbackSource::Human,
        ));
        assert!(!review.is_blocked());
        assert!(review.blocking_feedbacks().is_empty());
        assert_eq!(review.open_feedbacks().len(), 2);
    }

    #[test]
    fn review_close_sets_timestamp() {
        let mut review = Review::new("base", "head");
        assert!(review.closed_at.is_none());
        review.close();
        let ts = review.closed_at.as_ref().unwrap();
        chrono::DateTime::parse_from_rfc3339(ts).expect("closed_at should be valid RFC 3339");
    }

    #[test]
    fn review_created_at_is_rfc3339() {
        let review = Review::new("base", "head");
        chrono::DateTime::parse_from_rfc3339(&review.created_at)
            .expect("created_at should be valid RFC 3339");
    }

    #[test]
    fn review_feedbacks_for_file_empty_review() {
        let review = Review::new("base", "head");
        assert!(review.feedbacks_for_file("any.rs").is_empty());
    }

    #[test]
    fn review_open_feedbacks_all_resolved() {
        let mut review = Review::new("base", "head");
        review.add_feedback(Feedback::new(
            Target::file("a.rs"),
            Severity::Block,
            "Resolved",
            FeedbackSource::Human,
        ));
        review.feedbacks[0].resolve(None);
        assert!(review.open_feedbacks().is_empty());
        assert!(!review.is_blocked());
    }

    #[test]
    fn review_serde_closed_at_skipped_when_none() {
        let review = Review::new("base", "head");
        let json = serde_json::to_string(&review).unwrap();
        assert!(!json.contains("closed_at"));
    }

    #[test]
    fn review_serde_closed_at_present_when_closed() {
        let mut review = Review::new("base", "head");
        review.close();
        let json = serde_json::to_string(&review).unwrap();
        assert!(json.contains("closed_at"));
    }

    #[test]
    fn review_status_default_is_open() {
        assert_eq!(ReviewStatus::default(), ReviewStatus::Open);
    }
}
