//! Verification model
//!
//! A verification proves that a check was addressed.
//! It's attached to the commit that addresses the check.

use serde::{Deserialize, Serialize};

/// A verification - proof that a check was addressed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Verification {
    /// ID of the check being verified
    pub check_id: String,

    /// The verification message - how/why it was addressed
    pub message: String,

    /// Who verified: "human", "claude", "gpt-4", etc.
    pub verified_by: String,

    /// When this verification was made
    pub created_at: String,
}

impl Verification {
    /// Create a new verification
    #[must_use]
    pub fn new(check_id: String, message: String, verified_by: String) -> Self {
        Self {
            check_id,
            message,
            verified_by,
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// Create a verification by a human
    #[must_use]
    pub fn by_human(check_id: String, message: String) -> Self {
        Self::new(check_id, message, "human".to_string())
    }

    /// Create a verification by an LLM
    #[must_use]
    #[allow(dead_code)]
    pub fn by_llm(check_id: String, message: String, model: &str) -> Self {
        Self::new(check_id, message, model.to_string())
    }
}
