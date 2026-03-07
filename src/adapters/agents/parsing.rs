//! Shared output parsing for agent adapters
//!
//! Extracts structured [`Feedback`]s from raw agent text output. Both
//! Claude and Codex adapters delegate here via [`extract_feedbacks`].

use serde::Deserialize;

use crate::core::models::{Feedback, FeedbackSource, FeedbackStatus, Severity, Span, Target};
use crate::core::ports::agent::ReviewOutput;

/// Intermediate type mirroring what agents actually output.
///
/// Flat JSON schema requested in review prompts. Converted to domain
/// [`Feedback`] via [`RawAgentFeedback::into_feedback`].
#[derive(Debug, Deserialize)]
struct RawAgentFeedback {
    severity: String,
    file: String,
    line: Option<u32>,
    message: String,
    suggestion: Option<String>,
}

impl RawAgentFeedback {
    fn into_feedback(self, source: FeedbackSource) -> Feedback {
        Feedback {
            id: String::new(),
            target: Target {
                path: self.file,
                span: self.line.map(|l| Span { start: l, end: l }),
                commit: None,
            },
            severity: parse_severity(&self.severity),
            message: self.message,
            source,
            status: FeedbackStatus::Open,
            suggestion: self.suggestion,
            dismiss_reason: None,
            resolution_reason: None,
            confidence: None,
            notes: Vec::new(),
            created_at: String::new(),
        }
    }
}

/// Map agent severity strings to domain [`Severity`].
///
/// Accepts noslop's 3-level terms and common 4-level alternatives.
fn parse_severity(s: &str) -> Severity {
    match s.to_lowercase().as_str() {
        "info" | "informational" | "note" => Severity::Info,
        "block" | "error" | "critical" | "high" => Severity::Block,
        // "warn", "warning", "suggestion", and anything unrecognized default to Warn
        _ => Severity::Warn,
    }
}

/// Extract feedbacks from raw agent output.
///
/// Looks for a JSON array in the text, deserializes it into
/// [`RawAgentFeedback`]s, and converts them to domain [`Feedback`]s.
#[must_use]
pub fn extract_feedbacks(output: &str, source: &FeedbackSource) -> ReviewOutput {
    let feedbacks = extract_json_array(output)
        .and_then(|json| serde_json::from_str::<Vec<RawAgentFeedback>>(&json).ok())
        .map(|raw| raw.into_iter().map(|r| r.into_feedback(source.clone())).collect())
        .unwrap_or_default();

    ReviewOutput {
        feedbacks,
        raw_output: output.to_string(),
        errors: extract_error_lines(output),
    }
}

/// Find the outermost JSON array in text, handling nested arrays and strings.
fn extract_json_array(output: &str) -> Option<String> {
    let start = output.find('[')?;
    let mut depth = 0i32;
    let mut in_string = false;
    let mut escape_next = false;

    for (i, &b) in output.as_bytes()[start..].iter().enumerate() {
        if escape_next {
            escape_next = false;
            continue;
        }
        match b {
            b'\\' if in_string => escape_next = true,
            b'"' => in_string = !in_string,
            b'[' if !in_string => depth += 1,
            b']' if !in_string => {
                depth -= 1;
                if depth == 0 {
                    return Some(output[start..=start + i].to_string());
                }
            },
            _ => {},
        }
    }
    None
}

/// Extract error lines from agent output.
fn extract_error_lines(output: &str) -> Vec<String> {
    output
        .lines()
        .map(str::trim)
        .filter(|l| {
            l.starts_with("Error:")
                || l.starts_with("error:")
                || l.starts_with("FAILED")
                || l.contains("panicked at")
        })
        .map(String::from)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn src() -> FeedbackSource {
        FeedbackSource::Agent("security".into())
    }

    #[test]
    fn clean_json() {
        let out = r#"[{"severity":"warn","file":"src/auth.rs","line":42,"message":"Timing attack","suggestion":"Use constant-time comparison"}]"#;
        let r = extract_feedbacks(out, &src());
        assert_eq!(r.feedbacks.len(), 1);
        assert_eq!(r.feedbacks[0].severity, Severity::Warn);
        assert_eq!(r.feedbacks[0].target.path, "src/auth.rs");
        assert_eq!(r.feedbacks[0].target.span, Some(Span { start: 42, end: 42 }));
        assert_eq!(r.feedbacks[0].suggestion.as_deref(), Some("Use constant-time comparison"));
    }

    #[test]
    fn narrative_wrapper() {
        let out = "Here are feedbacks:\n\n[{\"severity\":\"block\",\"file\":\"src/db.rs\",\"line\":15,\"message\":\"SQL injection\",\"suggestion\":\"Parameterized queries\"}]\n\nClean otherwise.";
        let r = extract_feedbacks(out, &src());
        assert_eq!(r.feedbacks.len(), 1);
        assert_eq!(r.feedbacks[0].severity, Severity::Block);
        assert_eq!(r.feedbacks[0].target.path, "src/db.rs");
    }

    #[test]
    fn four_level_severity_mapping() {
        let cases = [
            ("error", Severity::Block),
            ("critical", Severity::Block),
            ("high", Severity::Block),
            ("warning", Severity::Warn),
            ("suggestion", Severity::Warn),
            ("informational", Severity::Info),
            ("note", Severity::Info),
        ];
        for (sev, expected) in cases {
            let out = format!(r#"[{{"severity":"{sev}","file":"f.rs","line":1,"message":"x"}}]"#);
            assert_eq!(
                extract_feedbacks(&out, &src()).feedbacks[0].severity,
                expected,
                "severity '{sev}' should map to {expected:?}"
            );
        }
    }

    #[test]
    fn unknown_severity_defaults_to_warn() {
        let out = r#"[{"severity":"unknown","file":"f.rs","line":1,"message":"x"}]"#;
        assert_eq!(extract_feedbacks(out, &src()).feedbacks[0].severity, Severity::Warn);
    }

    #[test]
    fn no_json_returns_empty() {
        let r = extract_feedbacks("No issues found.", &src());
        assert!(r.feedbacks.is_empty());
        assert_eq!(r.raw_output, "No issues found.");
    }

    #[test]
    fn malformed_json_returns_empty() {
        assert!(extract_feedbacks("[{bad json}]", &src()).feedbacks.is_empty());
    }

    #[test]
    fn empty_array_returns_empty() {
        let r = extract_feedbacks("[]", &src());
        assert!(r.feedbacks.is_empty());
    }

    #[test]
    fn error_line_extraction() {
        let r = extract_feedbacks("ok\nError: timeout\nFAILED to connect\ndone", &src());
        assert_eq!(r.errors.len(), 2);
        assert!(r.errors[0].contains("timeout"));
        assert!(r.errors[1].contains("FAILED"));
    }

    #[test]
    fn panicked_at_extraction() {
        let r = extract_feedbacks("thread 'main' panicked at 'oops'", &src());
        assert_eq!(r.errors.len(), 1);
        assert!(r.errors[0].contains("panicked at"));
    }

    #[test]
    fn multiple_feedbacks() {
        let out = r#"[
            {"severity":"warn","file":"a.rs","line":1,"message":"first"},
            {"severity":"block","file":"b.rs","line":2,"message":"second"},
            {"severity":"info","file":"c.rs","line":null,"message":"third"}
        ]"#;
        let r = extract_feedbacks(out, &src());
        assert_eq!(r.feedbacks.len(), 3);
        assert_eq!(r.feedbacks[0].severity, Severity::Warn);
        assert_eq!(r.feedbacks[1].severity, Severity::Block);
        assert_eq!(r.feedbacks[2].severity, Severity::Info);
        assert!(r.feedbacks[2].target.span.is_none());
    }

    #[test]
    fn null_suggestion_is_none() {
        let out = r#"[{"severity":"warn","file":"f.rs","line":1,"message":"x","suggestion":null}]"#;
        let r = extract_feedbacks(out, &src());
        assert!(r.feedbacks[0].suggestion.is_none());
    }

    #[test]
    fn feedback_source_is_preserved() {
        let out = r#"[{"severity":"warn","file":"f.rs","line":1,"message":"x"}]"#;
        let r = extract_feedbacks(out, &FeedbackSource::Agent("quality".into()));
        assert_eq!(r.feedbacks[0].source, FeedbackSource::Agent("quality".into()));
    }

    #[test]
    fn nested_json_arrays_in_strings() {
        // The outer array should be extracted correctly even with JSON-like content in strings
        let out =
            r#"[{"severity":"warn","file":"f.rs","line":1,"message":"found [1,2,3] in code"}]"#;
        let r = extract_feedbacks(out, &src());
        assert_eq!(r.feedbacks.len(), 1);
        assert_eq!(r.feedbacks[0].message, "found [1,2,3] in code");
    }

    #[test]
    fn extract_json_array_none_on_no_bracket() {
        assert!(extract_json_array("no brackets here").is_none());
    }

    #[test]
    fn extract_json_array_none_on_unclosed() {
        assert!(extract_json_array("[unclosed").is_none());
    }
}
