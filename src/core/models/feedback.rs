//! Feedback model
//!
//! A Feedback is the universal note type. Anything that says "there is
//! something at this location in the code" is a Feedback, regardless of
//! whether a check rule, a script, an LLM, or a human produced it.

use std::fmt;
use std::str::FromStr;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::primitives::{FeedbackSource, Severity, Target};

/// Reason why a feedback was dismissed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DismissReason {
    /// Analyzer was wrong; this is not actually an issue
    FalsePositive,
    /// Known issue, intentionally not fixing
    WontFix,
    /// Feedback does not apply to this context
    NotApplicable,
    /// Will look into it later
    InvestigateLater,
}

impl FromStr for DismissReason {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "false_positive" => Ok(Self::FalsePositive),
            "wont_fix" => Ok(Self::WontFix),
            "not_applicable" => Ok(Self::NotApplicable),
            "investigate_later" => Ok(Self::InvestigateLater),
            other => Err(format!(
                "Invalid dismiss reason: {other}. Expected: false_positive, wont_fix, not_applicable, investigate_later"
            )),
        }
    }
}

/// Reason why a feedback was resolved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResolutionReason {
    /// Developer manually fixed the issue
    Manual,
    /// The suggested fix was applied
    SuggestionApplied,
    /// The relevant code was changed (auto-detected)
    CodeChanged,
    /// The file containing the feedback was removed
    FileRemoved,
}

/// A timestamped note on a feedback.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackNote {
    /// Unique identifier
    pub id: String,
    /// Note content
    pub content: String,
    /// ISO 8601 timestamp
    pub created_at: String,
}

/// A note on code from any source.
///
/// Feedbacks are the primary output of the review pipeline. Every analyzer
/// produces `Vec<Feedback>`. The user triages each feedback: resolve it,
/// dismiss it, or leave it open. Open feedbacks with `severity = Block`
/// prevent push.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Feedback {
    /// Unique identifier (e.g., "F-a1b2c3d4").
    pub id: String,
    /// Where in the codebase this feedback points to.
    pub target: Target,
    /// How strictly this feedback is enforced.
    pub severity: Severity,
    /// Human-readable description of the issue.
    pub message: String,
    /// What produced this feedback.
    pub source: FeedbackSource,
    /// Lifecycle state: open, resolved, or dismissed.
    pub status: FeedbackStatus,
    /// Optional suggested fix or action.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
    /// Why the feedback was dismissed (set when status is Dismissed).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dismiss_reason: Option<DismissReason>,
    /// Why the feedback was resolved (set when status is Resolved).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolution_reason: Option<ResolutionReason>,
    /// Confidence score from the analyzer (0.0 - 1.0).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f32>,
    /// Timestamped notes on this feedback.
    #[serde(default)]
    pub notes: Vec<FeedbackNote>,
    /// ISO 8601 timestamp of creation.
    pub created_at: String,
}

/// Lifecycle state of a feedback.
///
/// - `Open`: not yet addressed, may block push
/// - `Resolved`: addressed (code was fixed)
/// - `Dismissed`: intentionally ignored (with reason tracked elsewhere)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FeedbackStatus {
    /// Not yet addressed
    #[default]
    Open,
    /// Addressed (code was fixed)
    Resolved,
    /// Intentionally ignored
    Dismissed,
}

impl Feedback {
    /// Create a new open feedback.
    #[must_use]
    pub fn new(
        target: Target,
        severity: Severity,
        message: impl Into<String>,
        source: FeedbackSource,
    ) -> Self {
        Self {
            id: format!("F-{}", &Uuid::new_v4().to_string()[..8]),
            target,
            severity,
            message: message.into(),
            source,
            status: FeedbackStatus::Open,
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

    /// Whether this feedback currently prevents pushing.
    #[must_use]
    pub fn is_blocking(&self) -> bool {
        self.severity == Severity::Block && self.status == FeedbackStatus::Open
    }

    /// Mark as resolved, optionally with a reason.
    pub const fn resolve(&mut self, reason: Option<ResolutionReason>) {
        self.status = FeedbackStatus::Resolved;
        self.resolution_reason = reason;
    }

    /// Mark as dismissed with a reason.
    pub const fn dismiss(&mut self, reason: DismissReason) {
        self.status = FeedbackStatus::Dismissed;
        self.dismiss_reason = Some(reason);
    }

    /// Add a timestamped note to this feedback.
    ///
    /// # Panics
    /// Cannot panic — the note is pushed immediately before access.
    pub fn add_note(&mut self, content: String) -> &FeedbackNote {
        let note = FeedbackNote {
            id: format!("N-{}", &Uuid::new_v4().to_string()[..8]),
            content,
            created_at: Utc::now().to_rfc3339(),
        };
        self.notes.push(note);
        self.notes.last().expect("just pushed")
    }

    /// Whether this feedback is still open.
    #[must_use]
    pub fn is_open(&self) -> bool {
        self.status == FeedbackStatus::Open
    }
}

impl fmt::Display for Feedback {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {} {} ({})", self.severity, self.id, self.message, self.source)
    }
}

impl fmt::Display for FeedbackStatus {
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
    fn feedback_new_is_open() {
        let f = Feedback::new(
            Target::file("src/auth.rs"),
            Severity::Block,
            "Test feedback",
            FeedbackSource::Human,
        );
        assert!(f.is_open());
        assert!(f.is_blocking());
        assert!(f.id.starts_with("F-"));
    }

    #[test]
    fn feedback_resolve() {
        let mut f = Feedback::new(
            Target::file("src/auth.rs"),
            Severity::Block,
            "Test",
            FeedbackSource::Human,
        );
        f.resolve(None);
        assert!(!f.is_open());
        assert!(!f.is_blocking());
        assert_eq!(f.status, FeedbackStatus::Resolved);
    }

    #[test]
    fn feedback_resolve_with_reason() {
        let mut f = Feedback::new(
            Target::file("src/auth.rs"),
            Severity::Block,
            "Test",
            FeedbackSource::Human,
        );
        f.resolve(Some(ResolutionReason::Manual));
        assert_eq!(f.status, FeedbackStatus::Resolved);
        assert_eq!(f.resolution_reason, Some(ResolutionReason::Manual));
    }

    #[test]
    fn feedback_dismiss() {
        let mut f = Feedback::new(
            Target::file("src/auth.rs"),
            Severity::Block,
            "Test",
            FeedbackSource::Human,
        );
        f.dismiss(DismissReason::FalsePositive);
        assert!(!f.is_open());
        assert!(!f.is_blocking());
        assert_eq!(f.status, FeedbackStatus::Dismissed);
        assert_eq!(f.dismiss_reason, Some(DismissReason::FalsePositive));
    }

    #[test]
    fn feedback_info_never_blocks() {
        let f = Feedback::new(
            Target::file("src/auth.rs"),
            Severity::Info,
            "Info feedback",
            FeedbackSource::Human,
        );
        assert!(!f.is_blocking());
        assert!(f.is_open());
    }

    #[test]
    fn feedback_with_suggestion() {
        let f = Feedback::new(
            Target::file("src/auth.rs"),
            Severity::Block,
            "Test",
            FeedbackSource::Human,
        )
        .with_suggestion("Fix this");
        assert_eq!(f.suggestion.as_deref(), Some("Fix this"));
    }

    #[test]
    fn feedback_display() {
        let f = Feedback::new(
            Target::file("src/auth.rs").with_span(Span::line(42)),
            Severity::Block,
            "Session handling changed",
            FeedbackSource::Check("NOS-1".into()),
        );
        let display = f.to_string();
        assert!(display.contains("[block]"));
        assert!(display.contains("Session handling changed"));
        assert!(display.contains("check:NOS-1"));
    }

    #[test]
    fn feedback_serde_roundtrip() {
        let f = Feedback::new(
            Target::file("src/auth.rs").with_span(Span::line(42)),
            Severity::Block,
            "Test feedback",
            FeedbackSource::Check("NOS-1".into()),
        );
        let json = serde_json::to_string(&f).unwrap();
        let parsed: Feedback = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, f.id);
        assert_eq!(parsed.severity, f.severity);
        assert_eq!(parsed.message, f.message);
    }

    #[test]
    fn feedback_status_display() {
        assert_eq!(FeedbackStatus::Open.to_string(), "open");
        assert_eq!(FeedbackStatus::Resolved.to_string(), "resolved");
        assert_eq!(FeedbackStatus::Dismissed.to_string(), "dismissed");
    }

    #[test]
    fn feedback_status_default_is_open() {
        assert_eq!(FeedbackStatus::default(), FeedbackStatus::Open);
    }

    #[test]
    fn feedback_warn_never_blocks() {
        let f = Feedback::new(
            Target::file("src/auth.rs"),
            Severity::Warn,
            "Warning feedback",
            FeedbackSource::Human,
        );
        assert!(!f.is_blocking());
        assert!(f.is_open());
    }

    #[test]
    fn feedback_resolved_block_not_blocking() {
        let mut f = Feedback::new(
            Target::file("src/auth.rs"),
            Severity::Block,
            "Resolved blocker",
            FeedbackSource::Human,
        );
        assert!(f.is_blocking());
        f.resolve(None);
        assert!(!f.is_blocking());
    }

    #[test]
    fn feedback_dismissed_block_not_blocking() {
        let mut f = Feedback::new(
            Target::file("src/auth.rs"),
            Severity::Block,
            "Dismissed blocker",
            FeedbackSource::Human,
        );
        f.dismiss(DismissReason::WontFix);
        assert!(!f.is_blocking());
    }

    #[test]
    fn feedback_add_note() {
        let mut f = Feedback::new(
            Target::file("src/auth.rs"),
            Severity::Block,
            "Test",
            FeedbackSource::Human,
        );
        assert!(f.notes.is_empty());
        let note = f.add_note("This needs attention".into());
        assert!(note.id.starts_with("N-"));
        assert_eq!(note.content, "This needs attention");
        assert_eq!(f.notes.len(), 1);
    }

    #[test]
    fn feedback_notes_default_empty() {
        let f =
            Feedback::new(Target::file("test.rs"), Severity::Info, "Test", FeedbackSource::Human);
        assert!(f.notes.is_empty());
    }

    #[test]
    fn feedback_new_fields_default_none() {
        let f =
            Feedback::new(Target::file("test.rs"), Severity::Info, "Test", FeedbackSource::Human);
        assert!(f.dismiss_reason.is_none());
        assert!(f.resolution_reason.is_none());
        assert!(f.confidence.is_none());
    }

    #[test]
    fn feedback_backwards_compat_serde() {
        // JSON without new fields should deserialize successfully
        let json = r#"{
            "id": "F-test1234",
            "target": {"path": "test.rs"},
            "severity": "block",
            "message": "Old feedback",
            "source": {"kind": "Human"},
            "status": "open",
            "created_at": "2025-01-01T00:00:00+00:00"
        }"#;
        let f: Feedback = serde_json::from_str(json).unwrap();
        assert_eq!(f.id, "F-test1234");
        assert!(f.dismiss_reason.is_none());
        assert!(f.resolution_reason.is_none());
        assert!(f.confidence.is_none());
        assert!(f.notes.is_empty());
    }

    #[test]
    fn feedback_id_format() {
        let f =
            Feedback::new(Target::file("test.rs"), Severity::Info, "Test", FeedbackSource::Human);
        assert!(f.id.starts_with("F-"));
        assert_eq!(f.id.len(), 10); // "F-" + 8 hex chars
    }

    #[test]
    fn feedback_created_at_is_rfc3339() {
        let f =
            Feedback::new(Target::file("test.rs"), Severity::Info, "Test", FeedbackSource::Human);
        // Should parse as RFC 3339
        chrono::DateTime::parse_from_rfc3339(&f.created_at)
            .expect("created_at should be valid RFC 3339");
    }

    #[test]
    fn feedback_display_all_sources() {
        let sources = vec![
            FeedbackSource::Check("NOS-1".into()),
            FeedbackSource::Script("lint".into()),
            FeedbackSource::Agent("security".into()),
            FeedbackSource::Human,
        ];
        for src in sources {
            let f = Feedback::new(Target::file("test.rs"), Severity::Warn, "msg", src);
            let display = f.to_string();
            assert!(display.contains("[warn]"));
            assert!(display.contains("msg"));
        }
    }
}
