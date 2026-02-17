//! Review model
//!
//! A review session on a branch diff. Contains all findings from all
//! analyzers across all pipeline runs.

use chrono::Utc;
use serde::{Deserialize, Serialize};

use super::finding::Finding;

/// A code review session on a branch diff.
///
/// Created by `noslop review`. Contains findings from every analyzer
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
    /// All findings from all sources.
    pub findings: Vec<Finding>,
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
            findings: Vec::new(),
            created_at: Utc::now().to_rfc3339(),
            closed_at: None,
        }
    }

    /// Add a finding to this review.
    pub fn add_finding(&mut self, finding: Finding) {
        self.findings.push(finding);
    }

    /// Add multiple findings from an analyzer pass.
    pub fn add_findings(&mut self, findings: impl IntoIterator<Item = Finding>) {
        self.findings.extend(findings);
    }

    /// All findings that currently prevent pushing
    /// (severity == Block and status == Open).
    #[must_use]
    pub fn blocking_findings(&self) -> Vec<&Finding> {
        self.findings.iter().filter(|f| f.is_blocking()).collect()
    }

    /// Whether this review has any blocking findings.
    /// Used by pre-push hook: `noslop review --check` exits 1 when true.
    #[must_use]
    pub fn is_blocked(&self) -> bool {
        self.findings.iter().any(Finding::is_blocking)
    }

    /// All findings that target a specific file path.
    #[must_use]
    pub fn findings_for_file(&self, path: &str) -> Vec<&Finding> {
        self.findings.iter().filter(|f| f.target.path == path).collect()
    }

    /// All findings that are still open (any severity).
    #[must_use]
    pub fn open_findings(&self) -> Vec<&Finding> {
        self.findings.iter().filter(|f| f.is_open()).collect()
    }

    /// Close this review.
    pub fn close(&mut self) {
        self.status = ReviewStatus::Closed;
        self.closed_at = Some(Utc::now().to_rfc3339());
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
    use crate::core::models::primitives::{FindingSource, Severity, Span, Target};

    #[test]
    fn new_review() {
        let review = Review::new("abc123", "def456");
        assert!(review.id.starts_with("REV-"));
        assert_eq!(review.base, "abc123");
        assert_eq!(review.head, "def456");
        assert!(review.is_open());
        assert!(review.findings.is_empty());
    }

    #[test]
    fn add_finding() {
        let mut review = Review::new("base", "head");
        let finding = Finding::new(
            Target::file("src/auth.rs").with_span(Span::line(42)),
            Severity::Block,
            "Session handling changed",
            FindingSource::Check("NOS-1".into()),
        );
        review.add_finding(finding);
        assert_eq!(review.findings.len(), 1);
        assert!(review.is_blocked());
    }

    #[test]
    fn blocking_findings() {
        let mut review = Review::new("base", "head");
        review.add_finding(Finding::new(
            Target::file("src/auth.rs"),
            Severity::Block,
            "Blocker",
            FindingSource::Check("NOS-1".into()),
        ));
        review.add_finding(Finding::new(
            Target::file("src/auth.rs"),
            Severity::Warn,
            "Warning",
            FindingSource::Agent("security".into()),
        ));
        assert_eq!(review.blocking_findings().len(), 1);
        assert!(review.is_blocked());
    }

    #[test]
    fn resolve_removes_block() {
        let mut review = Review::new("base", "head");
        review.add_finding(Finding::new(
            Target::file("src/auth.rs"),
            Severity::Block,
            "Blocker",
            FindingSource::Check("NOS-1".into()),
        ));
        assert!(review.is_blocked());

        review.findings[0].resolve();
        assert!(!review.is_blocked());
    }

    #[test]
    fn findings_for_file() {
        let mut review = Review::new("base", "head");
        review.add_finding(Finding::new(
            Target::file("src/auth.rs"),
            Severity::Block,
            "Auth finding",
            FindingSource::Human,
        ));
        review.add_finding(Finding::new(
            Target::file("src/main.rs"),
            Severity::Warn,
            "Main finding",
            FindingSource::Human,
        ));
        assert_eq!(review.findings_for_file("src/auth.rs").len(), 1);
        assert_eq!(review.findings_for_file("src/main.rs").len(), 1);
        assert_eq!(review.findings_for_file("src/other.rs").len(), 0);
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
    fn open_findings() {
        let mut review = Review::new("base", "head");
        review.add_finding(Finding::new(
            Target::file("a.rs"),
            Severity::Block,
            "Open",
            FindingSource::Human,
        ));
        review.add_finding(Finding::new(
            Target::file("b.rs"),
            Severity::Block,
            "Also open",
            FindingSource::Human,
        ));
        review.findings[0].resolve();
        assert_eq!(review.open_findings().len(), 1);
    }

    #[test]
    fn review_serde_roundtrip() {
        let mut review = Review::new("abc", "def");
        review.add_finding(Finding::new(
            Target::file("src/auth.rs"),
            Severity::Block,
            "Test",
            FindingSource::Human,
        ));
        let json = serde_json::to_string(&review).unwrap();
        let parsed: Review = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, review.id);
        assert_eq!(parsed.findings.len(), 1);
    }
}
