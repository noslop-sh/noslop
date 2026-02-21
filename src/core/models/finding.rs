//! Finding model
//!
//! A Finding is the universal note type. Anything that says "there is
//! something at this location in the code" is a Finding, regardless of
//! whether a check rule, a script, an LLM, or a human produced it.

use std::fmt;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::primitives::{FindingSource, Severity, Target};

/// Reason why a finding was dismissed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DismissReason {
    /// Analyzer was wrong; this is not actually an issue
    FalsePositive,
    /// Known issue, intentionally not fixing
    WontFix,
    /// Finding does not apply to this context
    NotApplicable,
    /// Will look into it later
    InvestigateLater,
}

/// Reason why a finding was resolved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResolutionReason {
    /// Developer manually fixed the issue
    Manual,
    /// The suggested fix was applied
    SuggestionApplied,
    /// The relevant code was changed (auto-detected)
    CodeChanged,
    /// The file containing the finding was removed
    FileRemoved,
}

/// A timestamped note on a finding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindingNote {
    /// Unique identifier
    pub id: String,
    /// Note content
    pub content: String,
    /// ISO 8601 timestamp
    pub created_at: String,
}

/// A note on code from any source.
///
/// Findings are the primary output of the review pipeline. Every analyzer
/// produces `Vec<Finding>`. The user triages each finding: resolve it,
/// dismiss it, or leave it open. Open findings with `severity = Block`
/// prevent push.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    /// Unique identifier (e.g., "F-a1b2c3d4").
    pub id: String,
    /// Where in the codebase this finding points to.
    pub target: Target,
    /// How strictly this finding is enforced.
    pub severity: Severity,
    /// Human-readable description of the issue.
    pub message: String,
    /// What produced this finding.
    pub source: FindingSource,
    /// Lifecycle state: open, resolved, or dismissed.
    pub status: FindingStatus,
    /// Optional suggested fix or action.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
    /// Why the finding was dismissed (set when status is Dismissed).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dismiss_reason: Option<DismissReason>,
    /// Why the finding was resolved (set when status is Resolved).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolution_reason: Option<ResolutionReason>,
    /// Confidence score from the analyzer (0.0 - 1.0).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f32>,
    /// Timestamped notes on this finding.
    #[serde(default)]
    pub notes: Vec<FindingNote>,
    /// ISO 8601 timestamp of creation.
    pub created_at: String,
}

/// Lifecycle state of a finding.
///
/// - `Open`: not yet addressed, may block push
/// - `Resolved`: addressed (code was fixed)
/// - `Dismissed`: intentionally ignored (with reason tracked elsewhere)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FindingStatus {
    /// Not yet addressed
    #[default]
    Open,
    /// Addressed (code was fixed)
    Resolved,
    /// Intentionally ignored
    Dismissed,
}

impl Finding {
    /// Create a new open finding.
    #[must_use]
    pub fn new(
        target: Target,
        severity: Severity,
        message: impl Into<String>,
        source: FindingSource,
    ) -> Self {
        Self {
            id: format!("F-{}", &Uuid::new_v4().to_string()[..8]),
            target,
            severity,
            message: message.into(),
            source,
            status: FindingStatus::Open,
            suggestion: None,
            dismiss_reason: None,
            resolution_reason: None,
            confidence: None,
            notes: Vec::new(),
            created_at: Utc::now().to_rfc3339(),
        }
    }

    /// Attach a suggested fix.
    #[must_use]
    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }

    /// Whether this finding currently prevents pushing.
    #[must_use]
    pub fn is_blocking(&self) -> bool {
        self.severity == Severity::Block && self.status == FindingStatus::Open
    }

    /// Mark as resolved, optionally with a reason.
    pub const fn resolve(&mut self, reason: Option<ResolutionReason>) {
        self.status = FindingStatus::Resolved;
        self.resolution_reason = reason;
    }

    /// Mark as dismissed with a reason.
    pub const fn dismiss(&mut self, reason: DismissReason) {
        self.status = FindingStatus::Dismissed;
        self.dismiss_reason = Some(reason);
    }

    /// Add a timestamped note to this finding.
    ///
    /// # Panics
    /// Cannot panic — the note is pushed immediately before access.
    pub fn add_note(&mut self, content: String) -> &FindingNote {
        let note = FindingNote {
            id: format!("N-{}", &Uuid::new_v4().to_string()[..8]),
            content,
            created_at: Utc::now().to_rfc3339(),
        };
        self.notes.push(note);
        self.notes.last().expect("just pushed")
    }

    /// Whether this finding is still open.
    #[must_use]
    pub fn is_open(&self) -> bool {
        self.status == FindingStatus::Open
    }
}

impl fmt::Display for Finding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {} {} ({})", self.severity, self.id, self.message, self.source)
    }
}

impl fmt::Display for FindingStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Open => f.write_str("open"),
            Self::Resolved => f.write_str("resolved"),
            Self::Dismissed => f.write_str("dismissed"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::models::primitives::Span;

    #[test]
    fn finding_new_is_open() {
        let f = Finding::new(
            Target::file("src/auth.rs"),
            Severity::Block,
            "Test finding",
            FindingSource::Human,
        );
        assert!(f.is_open());
        assert!(f.is_blocking());
        assert!(f.id.starts_with("F-"));
    }

    #[test]
    fn finding_resolve() {
        let mut f = Finding::new(
            Target::file("src/auth.rs"),
            Severity::Block,
            "Test",
            FindingSource::Human,
        );
        f.resolve(None);
        assert!(!f.is_open());
        assert!(!f.is_blocking());
        assert_eq!(f.status, FindingStatus::Resolved);
    }

    #[test]
    fn finding_resolve_with_reason() {
        let mut f = Finding::new(
            Target::file("src/auth.rs"),
            Severity::Block,
            "Test",
            FindingSource::Human,
        );
        f.resolve(Some(ResolutionReason::Manual));
        assert_eq!(f.status, FindingStatus::Resolved);
        assert_eq!(f.resolution_reason, Some(ResolutionReason::Manual));
    }

    #[test]
    fn finding_dismiss() {
        let mut f = Finding::new(
            Target::file("src/auth.rs"),
            Severity::Block,
            "Test",
            FindingSource::Human,
        );
        f.dismiss(DismissReason::FalsePositive);
        assert!(!f.is_open());
        assert!(!f.is_blocking());
        assert_eq!(f.status, FindingStatus::Dismissed);
        assert_eq!(f.dismiss_reason, Some(DismissReason::FalsePositive));
    }

    #[test]
    fn finding_info_never_blocks() {
        let f = Finding::new(
            Target::file("src/auth.rs"),
            Severity::Info,
            "Info finding",
            FindingSource::Human,
        );
        assert!(!f.is_blocking());
        assert!(f.is_open());
    }

    #[test]
    fn finding_with_suggestion() {
        let f = Finding::new(
            Target::file("src/auth.rs"),
            Severity::Block,
            "Test",
            FindingSource::Human,
        )
        .with_suggestion("Fix this");
        assert_eq!(f.suggestion.as_deref(), Some("Fix this"));
    }

    #[test]
    fn finding_display() {
        let f = Finding::new(
            Target::file("src/auth.rs").with_span(Span::line(42)),
            Severity::Block,
            "Session handling changed",
            FindingSource::Check("NOS-1".into()),
        );
        let display = f.to_string();
        assert!(display.contains("[block]"));
        assert!(display.contains("Session handling changed"));
        assert!(display.contains("check:NOS-1"));
    }

    #[test]
    fn finding_serde_roundtrip() {
        let f = Finding::new(
            Target::file("src/auth.rs").with_span(Span::line(42)),
            Severity::Block,
            "Test finding",
            FindingSource::Check("NOS-1".into()),
        );
        let json = serde_json::to_string(&f).unwrap();
        let parsed: Finding = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, f.id);
        assert_eq!(parsed.severity, f.severity);
        assert_eq!(parsed.message, f.message);
    }

    #[test]
    fn finding_status_display() {
        assert_eq!(FindingStatus::Open.to_string(), "open");
        assert_eq!(FindingStatus::Resolved.to_string(), "resolved");
        assert_eq!(FindingStatus::Dismissed.to_string(), "dismissed");
    }

    #[test]
    fn finding_status_default_is_open() {
        assert_eq!(FindingStatus::default(), FindingStatus::Open);
    }

    #[test]
    fn finding_warn_never_blocks() {
        let f = Finding::new(
            Target::file("src/auth.rs"),
            Severity::Warn,
            "Warning finding",
            FindingSource::Human,
        );
        assert!(!f.is_blocking());
        assert!(f.is_open());
    }

    #[test]
    fn finding_resolved_block_not_blocking() {
        let mut f = Finding::new(
            Target::file("src/auth.rs"),
            Severity::Block,
            "Resolved blocker",
            FindingSource::Human,
        );
        assert!(f.is_blocking());
        f.resolve(None);
        assert!(!f.is_blocking());
    }

    #[test]
    fn finding_dismissed_block_not_blocking() {
        let mut f = Finding::new(
            Target::file("src/auth.rs"),
            Severity::Block,
            "Dismissed blocker",
            FindingSource::Human,
        );
        f.dismiss(DismissReason::WontFix);
        assert!(!f.is_blocking());
    }

    #[test]
    fn finding_add_note() {
        let mut f = Finding::new(
            Target::file("src/auth.rs"),
            Severity::Block,
            "Test",
            FindingSource::Human,
        );
        assert!(f.notes.is_empty());
        let note = f.add_note("This needs attention".into());
        assert!(note.id.starts_with("N-"));
        assert_eq!(note.content, "This needs attention");
        assert_eq!(f.notes.len(), 1);
    }

    #[test]
    fn finding_notes_default_empty() {
        let f = Finding::new(Target::file("test.rs"), Severity::Info, "Test", FindingSource::Human);
        assert!(f.notes.is_empty());
    }

    #[test]
    fn finding_new_fields_default_none() {
        let f = Finding::new(Target::file("test.rs"), Severity::Info, "Test", FindingSource::Human);
        assert!(f.dismiss_reason.is_none());
        assert!(f.resolution_reason.is_none());
        assert!(f.confidence.is_none());
    }

    #[test]
    fn finding_backwards_compat_serde() {
        // JSON without new fields should deserialize successfully
        let json = r#"{
            "id": "F-test1234",
            "target": {"path": "test.rs"},
            "severity": "block",
            "message": "Old finding",
            "source": {"kind": "Human"},
            "status": "open",
            "created_at": "2025-01-01T00:00:00+00:00"
        }"#;
        let f: Finding = serde_json::from_str(json).unwrap();
        assert_eq!(f.id, "F-test1234");
        assert!(f.dismiss_reason.is_none());
        assert!(f.resolution_reason.is_none());
        assert!(f.confidence.is_none());
        assert!(f.notes.is_empty());
    }

    #[test]
    fn finding_id_format() {
        let f = Finding::new(Target::file("test.rs"), Severity::Info, "Test", FindingSource::Human);
        assert!(f.id.starts_with("F-"));
        assert_eq!(f.id.len(), 10); // "F-" + 8 hex chars
    }

    #[test]
    fn finding_created_at_is_rfc3339() {
        let f = Finding::new(Target::file("test.rs"), Severity::Info, "Test", FindingSource::Human);
        // Should parse as RFC 3339
        chrono::DateTime::parse_from_rfc3339(&f.created_at)
            .expect("created_at should be valid RFC 3339");
    }

    #[test]
    fn finding_display_all_sources() {
        let sources = vec![
            FindingSource::Check("NOS-1".into()),
            FindingSource::Script("lint".into()),
            FindingSource::Agent("security".into()),
            FindingSource::Human,
        ];
        for src in sources {
            let f = Finding::new(Target::file("test.rs"), Severity::Warn, "msg", src);
            let display = f.to_string();
            assert!(display.contains("[warn]"));
            assert!(display.contains("msg"));
        }
    }
}
