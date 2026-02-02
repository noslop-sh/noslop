//! Test fixtures and builders
//!
//! Provides convenient builders for creating test data.

use noslop::core::models::{Assertion, Attestation, Severity};

/// Builder for creating test assertions
pub struct AssertionBuilder {
    id: String,
    target: String,
    message: String,
    severity: Severity,
}

impl AssertionBuilder {
    pub fn new() -> Self {
        Self {
            id: "TEST-1".to_string(),
            target: "*.rs".to_string(),
            message: "Test assertion".to_string(),
            severity: Severity::Block,
        }
    }

    pub fn id(mut self, id: &str) -> Self {
        self.id = id.to_string();
        self
    }

    pub fn target(mut self, target: &str) -> Self {
        self.target = target.to_string();
        self
    }

    pub fn message(mut self, message: &str) -> Self {
        self.message = message.to_string();
        self
    }

    pub fn severity(mut self, severity: Severity) -> Self {
        self.severity = severity;
        self
    }

    pub fn build(self) -> Assertion {
        Assertion::new(Some(self.id), self.target, self.message, self.severity)
    }
}

impl Default for AssertionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for creating test attestations
pub struct AttestationBuilder {
    assertion_id: String,
    message: String,
    attested_by: String,
}

impl AttestationBuilder {
    pub fn new() -> Self {
        Self {
            assertion_id: "TEST-1".to_string(),
            message: "Test attestation".to_string(),
            attested_by: "human".to_string(),
        }
    }

    pub fn assertion_id(mut self, id: &str) -> Self {
        self.assertion_id = id.to_string();
        self
    }

    pub fn message(mut self, message: &str) -> Self {
        self.message = message.to_string();
        self
    }

    pub fn attested_by(mut self, by: &str) -> Self {
        self.attested_by = by.to_string();
        self
    }

    pub fn build(self) -> Attestation {
        Attestation::new(self.assertion_id, self.message, self.attested_by)
    }
}

impl Default for AttestationBuilder {
    fn default() -> Self {
        Self::new()
    }
}
