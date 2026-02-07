//! Data transfer objects for Tauri IPC
//!
//! These types serialize noslop domain models for the frontend.

use noslop::core::models::{Review, ReviewComment};
use serde::Serialize;

/// Review data for frontend
#[derive(Debug, Serialize)]
pub struct ReviewDto {
    pub id: String,
    pub base_sha: String,
    pub head_sha: String,
    pub status: String,
    pub comments: Vec<CommentDto>,
    pub created_at: String,
}

/// Comment data for frontend
#[derive(Debug, Serialize)]
pub struct CommentDto {
    pub id: String,
    pub target: String,
    pub message: String,
    pub line: Option<u32>,
    pub status: String,
    pub resolution_message: Option<String>,
}

impl From<Review> for ReviewDto {
    fn from(r: Review) -> Self {
        Self {
            id: r.id,
            base_sha: r.base_sha,
            head_sha: r.head_sha,
            status: format!("{:?}", r.status).to_lowercase(),
            comments: r.comments.into_iter().map(CommentDto::from).collect(),
            created_at: r.created_at,
        }
    }
}

impl From<ReviewComment> for CommentDto {
    fn from(c: ReviewComment) -> Self {
        Self {
            id: c.check.id,
            target: c.check.target,
            message: c.check.message,
            line: c.position.new_line,
            status: format!("{:?}", c.status).to_lowercase(),
            resolution_message: c.resolution_message,
        }
    }
}
