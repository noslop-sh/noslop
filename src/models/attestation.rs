//! Attestation model
//!
//! An attestation proves that an assertion was considered.
//! It's attached to the commit that addresses the assertion.

use serde::{Deserialize, Serialize};

/// An attestation - proof that an assertion was considered
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attestation {
    /// ID of the assertion being attested to
    pub assertion_id: String,

    /// The attestation message - how/why it was addressed
    pub message: String,

    /// Who attested: "human", "claude", "gpt-4", etc.
    pub attested_by: String,

    /// When this attestation was made
    pub created_at: String,
}

impl Attestation {
    /// Create a new attestation
    pub fn new(assertion_id: String, message: String, attested_by: String) -> Self {
        Self {
            assertion_id,
            message,
            attested_by,
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// Create an attestation by a human
    pub fn by_human(assertion_id: String, message: String) -> Self {
        Self::new(assertion_id, message, "human".to_string())
    }

    /// Create an attestation by an LLM
    #[allow(dead_code)]
    pub fn by_llm(assertion_id: String, message: String, model: &str) -> Self {
        Self::new(assertion_id, message, model.to_string())
    }
}
