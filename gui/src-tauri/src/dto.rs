//! Data transfer objects for Tauri IPC
//!
//! These types serialize noslop domain models for the frontend.

use noslop::core::models::{Finding, Review};
use serde::Serialize;

/// Review data for frontend
#[derive(Debug, Serialize)]
pub struct ReviewDto {
    pub id: String,
    pub base: String,
    pub head: String,
    pub status: String,
    pub findings: Vec<FindingDto>,
    pub created_at: String,
}

/// Finding data for frontend
#[derive(Debug, Serialize)]
pub struct FindingDto {
    pub id: String,
    pub target: String,
    pub severity: String,
    pub message: String,
    pub source: String,
    pub status: String,
    pub suggestion: Option<String>,
}

impl From<Review> for ReviewDto {
    fn from(r: Review) -> Self {
        Self {
            id: r.id,
            base: r.base,
            head: r.head,
            status: format!("{:?}", r.status).to_lowercase(),
            findings: r.findings.into_iter().map(FindingDto::from).collect(),
            created_at: r.created_at,
        }
    }
}

impl From<Finding> for FindingDto {
    fn from(f: Finding) -> Self {
        Self {
            id: f.id,
            target: f.target.to_string(),
            severity: f.severity.to_string(),
            message: f.message,
            source: f.source.to_string(),
            status: format!("{:?}", f.status).to_lowercase(),
            suggestion: f.suggestion,
        }
    }
}
