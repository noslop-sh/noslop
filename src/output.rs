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
    /// Checks that are blocking (need acknowledgment)
    pub blocking: Vec<CheckMatch>,
    /// Checks that are warnings
    pub warnings: Vec<CheckMatch>,
    /// Checks that were acknowledged
    pub acknowledged: Vec<CheckMatch>,
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
    /// Whether this check was acknowledged
    pub acknowledged: bool,
}

/// Result of a check list operation
#[derive(Debug, Serialize)]
pub struct CheckListResult {
    /// List of checks
    pub checks: Vec<CheckInfo>,
}

/// Information about a check
#[derive(Debug, Serialize)]
pub struct CheckInfo {
    /// Check ID (<file:index> format)
    pub id: String,
    /// Target pattern
    pub target: String,
    /// Check message
    pub message: String,
    /// Severity level
    pub severity: String,
    /// Source file containing this check
    pub source_file: String,
}

/// Result of an ack operation
#[derive(Debug, Serialize)]
pub struct AckResult {
    /// Whether the acknowledgment was successful
    pub success: bool,
    /// The check ID that was acknowledged
    pub check_id: String,
    /// The acknowledgment message
    pub message: String,
}

/// Generic operation result for simple commands
#[derive(Debug, Serialize)]
pub struct OperationResult {
    /// Whether the operation succeeded
    pub success: bool,
    /// Human-readable message
    pub message: String,
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

        if self.blocking.is_empty() && self.warnings.is_empty() && self.acknowledged.is_empty() {
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
            println!("All checks acknowledged. Commit may proceed.");
        } else {
            println!("Blocking:");
            for m in &self.blocking {
                println!("  [{}] {}", m.id, m.file);
                println!("          {}\n", m.message);
            }
            println!("BLOCKED: {} unacknowledged check(s)\n", self.blocking.len());
            println!("To acknowledge: noslop ack <check-id> -m \"your acknowledgment\"");
            println!(
                "Example:        noslop ack {} -m \"reviewed and verified\"",
                self.blocking.first().map_or("CHK-1", |b| b.id.as_str())
            );
        }
    }

    fn render_json(&self) {
        println!("{}", serde_json::to_string_pretty(self).unwrap_or_default());
    }
}

impl CheckListResult {
    /// Render the result based on output mode
    pub fn render(&self, mode: OutputMode) {
        match mode {
            OutputMode::Human => self.render_human(),
            OutputMode::Json => self.render_json(),
        }
    }

    fn render_human(&self) {
        if self.checks.is_empty() {
            println!("No checks found.");
            return;
        }

        println!("Checks:\n");
        for c in &self.checks {
            println!("  [{}] {}", c.severity.to_uppercase(), c.target);
            println!("  ID: {}", c.id);
            println!("  {}\n", c.message);
        }
    }

    fn render_json(&self) {
        println!("{}", serde_json::to_string_pretty(self).unwrap_or_default());
    }
}

impl AckResult {
    /// Render the result based on output mode
    pub fn render(&self, mode: OutputMode) {
        match mode {
            OutputMode::Human => self.render_human(),
            OutputMode::Json => self.render_json(),
        }
    }

    fn render_human(&self) {
        if self.success {
            println!("Acknowledged: {}", self.check_id);
            println!("Message: {}", self.message);
        } else {
            println!("Failed to acknowledge: {}", self.message);
        }
    }

    fn render_json(&self) {
        println!("{}", serde_json::to_string_pretty(self).unwrap_or_default());
    }
}

impl OperationResult {
    /// Render the result based on output mode
    pub fn render(&self, mode: OutputMode) {
        match mode {
            OutputMode::Human => println!("{}", self.message),
            OutputMode::Json => {
                println!("{}", serde_json::to_string_pretty(self).unwrap_or_default());
            },
        }
    }
}
