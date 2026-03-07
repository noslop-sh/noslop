//! Output formatting for human and JSON modes
//!
//! This module provides structured output that can be rendered either as
//! human-readable text or machine-parseable JSON.

use serde::Serialize;

/// Output mode for the CLI
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputMode {
    /// Human-readable output (default)
    #[default]
    Human,
    /// JSON output (machine-readable)
    Json,
}

/// Result of a check operation
#[derive(Debug, Serialize)]
pub struct CheckResult {
    /// Whether the check passed
    pub passed: bool,
    /// Number of files checked
    pub files_checked: usize,
    /// Checks that are blocking
    pub blocking: Vec<CheckMatch>,
    /// Checks that are warnings
    pub warnings: Vec<CheckMatch>,
}

/// A check matched to a file
#[derive(Debug, Serialize)]
pub struct CheckMatch {
    /// The check ID (e.g., "NSL-1")
    pub id: String,
    /// The file that matched
    pub file: String,
    /// The check target pattern
    pub target: String,
    /// The check message
    pub message: String,
    /// Severity level
    pub severity: String,
}

impl CheckResult {
    /// Render the result based on output mode
    pub fn render(&self, mode: OutputMode) {
        match mode {
            OutputMode::Human => self.render_human(),
            OutputMode::Json => self.render_json(),
        }
    }

    fn render_human(&self) {
        if self.files_checked == 0 {
            println!("No staged changes.");
            return;
        }

        println!("Checking {} staged file(s)...\n", self.files_checked);

        if self.blocking.is_empty() && self.warnings.is_empty() {
            println!("No checks apply. Commit may proceed.");
            return;
        }

        if !self.warnings.is_empty() {
            println!("Warnings:");
            for m in &self.warnings {
                println!("  [{}] {}", m.id, m.file);
                println!("          {}\n", m.message);
            }
        }

        if self.blocking.is_empty() {
            println!("All checks passed. Commit may proceed.");
        } else {
            println!("Blocking:");
            for m in &self.blocking {
                println!("  [{}] {}", m.id, m.file);
                println!("          {}\n", m.message);
            }
            println!("BLOCKED: {} unresolved check(s)\n", self.blocking.len());
        }
    }

    fn render_json(&self) {
        println!("{}", serde_json::to_string_pretty(self).unwrap_or_default());
    }
}
