//! Test fixtures and builders
//!
//! Provides convenient builders for creating test data.

use noslop::core::models::{Acknowledgment, Check, Severity};

/// Builder for creating test checks
pub struct CheckBuilder {
    id: String,
    target: String,
    message: String,
    severity: Severity,
}

impl CheckBuilder {
    pub fn new() -> Self {
        Self {
            id: "TEST-1".to_string(),
            target: "*.rs".to_string(),
            message: "Test check".to_string(),
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

    pub fn build(self) -> Check {
        Check::new(Some(self.id), self.target, self.message, self.severity)
    }
}

impl Default for CheckBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for creating test acknowledgments
pub struct AckBuilder {
    check_id: String,
    message: String,
    acknowledged_by: String,
}

impl AckBuilder {
    pub fn new() -> Self {
        Self {
            check_id: "TEST-1".to_string(),
            message: "Test acknowledgment".to_string(),
            acknowledged_by: "human".to_string(),
        }
    }

    pub fn check_id(mut self, id: &str) -> Self {
        self.check_id = id.to_string();
        self
    }

    pub fn message(mut self, message: &str) -> Self {
        self.message = message.to_string();
        self
    }

    pub fn acknowledged_by(mut self, by: &str) -> Self {
        self.acknowledged_by = by.to_string();
        self
    }

    pub fn build(self) -> Acknowledgment {
        Acknowledgment::new(self.check_id, self.message, self.acknowledged_by)
    }
}

impl Default for AckBuilder {
    fn default() -> Self {
        Self::new()
    }
}
