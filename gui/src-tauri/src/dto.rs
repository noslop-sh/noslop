//! Data transfer objects for Tauri IPC
//!
//! These types serialize noslop domain models for the frontend.

use noslop::core::models::{Feedback, FeedbackSource, Review};
use serde::Serialize;

/// Review data for frontend
#[derive(Debug, Serialize)]
pub struct ReviewDto {
    pub id: String,
    pub base: String,
    pub head: String,
    pub status: String,
    pub feedbacks: Vec<FeedbackDto>,
    pub branch: Option<String>,
    pub viewed_files: Vec<String>,
    pub created_at: String,
    pub closed_at: Option<String>,
}

/// Feedback data for frontend
#[derive(Debug, Serialize)]
pub struct FeedbackDto {
    pub id: String,
    pub target: TargetDto,
    pub severity: String,
    pub message: String,
    pub source: SourceDto,
    pub status: String,
    pub suggestion: Option<SuggestionDto>,
    pub dismiss_reason: Option<String>,
    pub resolution_reason: Option<String>,
    pub confidence: Option<f32>,
    pub notes: Vec<FeedbackNoteDto>,
    pub created_at: String,
}

/// Structured target location for frontend
#[derive(Debug, Serialize)]
pub struct TargetDto {
    pub path: String,
    pub span: Option<SpanDto>,
    pub commit: Option<String>,
}

/// Line span for frontend
#[derive(Debug, Serialize)]
pub struct SpanDto {
    pub start: u32,
    pub end: u32,
}

/// Feedback source for frontend
#[derive(Debug, Serialize)]
pub struct SourceDto {
    pub kind: String,
    pub name: Option<String>,
}

/// Suggestion for frontend
#[derive(Debug, Clone, Serialize)]
pub struct SuggestionDto {
    pub replacement: String,
    pub original: Option<String>,
    pub edited: bool,
}

/// Feedback note for frontend
#[derive(Debug, Clone, Serialize)]
pub struct FeedbackNoteDto {
    pub id: String,
    pub content: String,
    pub created_at: String,
}

impl From<Review> for ReviewDto {
    fn from(r: Review) -> Self {
        Self {
            id: r.id,
            base: r.base,
            head: r.head,
            status: format!("{:?}", r.status).to_lowercase(),
            feedbacks: r.feedbacks.into_iter().map(FeedbackDto::from).collect(),
            branch: r.branch,
            viewed_files: r.viewed_files,
            created_at: r.created_at,
            closed_at: r.closed_at,
        }
    }
}

impl From<Feedback> for FeedbackDto {
    fn from(f: Feedback) -> Self {
        let target = TargetDto {
            path: f.target.path,
            span: f.target.span.map(|s| SpanDto {
                start: s.start,
                end: s.end,
            }),
            commit: f.target.commit,
        };

        let source = match &f.source {
            FeedbackSource::Check(name) => SourceDto {
                kind: "check".to_string(),
                name: Some(name.clone()),
            },
            FeedbackSource::Script(name) => SourceDto {
                kind: "script".to_string(),
                name: Some(name.clone()),
            },
            FeedbackSource::Agent(name) => SourceDto {
                kind: "agent".to_string(),
                name: Some(name.clone()),
            },
            FeedbackSource::Human => SourceDto {
                kind: "human".to_string(),
                name: None,
            },
        };

        // Suggestion is still a plain string in the domain model;
        // wrap it into SuggestionDto for the frontend
        let suggestion = f.suggestion.map(|s| SuggestionDto {
            replacement: s,
            original: None,
            edited: false,
        });

        let notes = f
            .notes
            .into_iter()
            .map(|n| FeedbackNoteDto {
                id: n.id,
                content: n.content,
                created_at: n.created_at,
            })
            .collect();

        Self {
            id: f.id,
            target,
            severity: f.severity.to_string(),
            message: f.message,
            source,
            status: format!("{:?}", f.status).to_lowercase(),
            suggestion,
            dismiss_reason: f.dismiss_reason.map(|r| {
                serde_json::to_value(r)
                    .ok()
                    .and_then(|v| v.as_str().map(String::from))
                    .unwrap_or_default()
            }),
            resolution_reason: f.resolution_reason.map(|r| {
                serde_json::to_value(r)
                    .ok()
                    .and_then(|v| v.as_str().map(String::from))
                    .unwrap_or_default()
            }),
            confidence: f.confidence,
            notes,
            created_at: f.created_at,
        }
    }
}
