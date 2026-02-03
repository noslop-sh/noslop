//! Acknowledgment model
//!
//! An acknowledgment proves that a check was considered.
//! It's attached to the commit that addresses the check.

use serde::{Deserialize, Serialize};

/// An acknowledgment - proof that a check was considered
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Acknowledgment {
    /// ID of the check being acknowledged
    pub check_id: String,

    /// The acknowledgment message - how/why it was addressed
    pub message: String,

    /// Who acknowledged: "human", "claude", "gpt-4", etc.
    pub acknowledged_by: String,

    /// When this acknowledgment was made
    pub created_at: String,
}

impl Acknowledgment {
    /// Create a new acknowledgment
    #[must_use]
    pub fn new(check_id: String, message: String, acknowledged_by: String) -> Self {
        Self {
            check_id,
            message,
            acknowledged_by,
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// Create an acknowledgment by a human
    #[must_use]
    pub fn by_human(check_id: String, message: String) -> Self {
        Self::new(check_id, message, "human".to_string())
    }

    /// Create an acknowledgment by an LLM
    #[must_use]
    #[allow(dead_code)]
    pub fn by_llm(check_id: String, message: String, model: &str) -> Self {
        Self::new(check_id, message, model.to_string())
    }
}
