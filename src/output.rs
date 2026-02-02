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
    /// Assertions that are blocking (need attestation)
    pub blocking: Vec<AssertionMatch>,
    /// Assertions that are warnings
    pub warnings: Vec<AssertionMatch>,
    /// Assertions that were attested
    pub attested: Vec<AssertionMatch>,
}

/// An assertion matched to a file
#[derive(Debug, Serialize)]
pub struct AssertionMatch {
    /// The assertion ID (e.g., "NSL-1")
    pub id: String,
    /// The file that matched
    pub file: String,
    /// The assertion target pattern
    pub target: String,
    /// The assertion message
    pub message: String,
    /// Severity level
    pub severity: String,
    /// Whether this assertion was attested
    pub attested: bool,
}

/// Result of an assert list operation
#[derive(Debug, Serialize)]
pub struct AssertListResult {
    /// List of assertions
    pub assertions: Vec<AssertionInfo>,
}

/// Information about an assertion
#[derive(Debug, Serialize)]
pub struct AssertionInfo {
    /// Assertion ID (<file:index> format)
    pub id: String,
    /// Target pattern
    pub target: String,
    /// Assertion message
    pub message: String,
    /// Severity level
    pub severity: String,
    /// Source file containing this assertion
    pub source_file: String,
}

/// Result of an attest operation
#[derive(Debug, Serialize)]
pub struct AttestResult {
    /// Whether the attestation was successful
    pub success: bool,
    /// The assertion ID that was attested
    pub assertion_id: String,
    /// The attestation message
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

        if self.blocking.is_empty() && self.warnings.is_empty() && self.attested.is_empty() {
            println!("No assertions apply. Commit may proceed.");
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
            println!("All assertions attested. Commit may proceed.");
        } else {
            println!("Blocking:");
            for m in &self.blocking {
                println!("  [{}] {}", m.id, m.file);
                println!("          {}\n", m.message);
            }
            println!("BLOCKED: {} unattested assertion(s)\n", self.blocking.len());
            println!("To attest: noslop attest <assertion-id> -m \"your attestation\"");
            println!(
                "Example:   noslop attest {} -m \"reviewed and verified\"",
                self.blocking.first().map_or("NSL-1", |b| b.id.as_str())
            );
        }
    }

    fn render_json(&self) {
        println!("{}", serde_json::to_string_pretty(self).unwrap_or_default());
    }
}

impl AssertListResult {
    /// Render the result based on output mode
    pub fn render(&self, mode: OutputMode) {
        match mode {
            OutputMode::Human => self.render_human(),
            OutputMode::Json => self.render_json(),
        }
    }

    fn render_human(&self) {
        if self.assertions.is_empty() {
            println!("No assertions found.");
            return;
        }

        println!("Assertions:\n");
        for a in &self.assertions {
            println!("  [{}] {}", a.severity.to_uppercase(), a.target);
            println!("  ID: {}", a.id);
            println!("  {}\n", a.message);
        }
    }

    fn render_json(&self) {
        println!("{}", serde_json::to_string_pretty(self).unwrap_or_default());
    }
}

impl AttestResult {
    /// Render the result based on output mode
    pub fn render(&self, mode: OutputMode) {
        match mode {
            OutputMode::Human => self.render_human(),
            OutputMode::Json => self.render_json(),
        }
    }

    fn render_human(&self) {
        if self.success {
            println!("Attested: {}", self.assertion_id);
            println!("Message: {}", self.message);
        } else {
            println!("Failed to attest: {}", self.message);
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
