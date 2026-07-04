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
    /// Who is committing (e.g. "human", "claude-code")
    pub actor: String,
    /// Whether blocking checks gate this actor (agents/CI yes, humans no)
    pub enforced: bool,
    /// Gate-time staged-tree object id (`git write-tree`). Additive within
    /// schema 1: joined against ledger record tree oids to distinguish
    /// action rate from answers that change nothing. Absent when git state is
    /// unavailable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tree_oid: Option<String>,
    /// Version of the cloud check set in force for this run (additive
    /// within schema 1; absent when the repo has no remote binding)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub check_set_version: Option<String>,
    /// Seconds since the cloud check set was fetched (0 = fetched this
    /// run). The gate is fail-open, so runs may proceed on cached rules;
    /// this lets the cloud flag runs that gated on a stale set. Additive
    /// within schema 1; absent when the repo has no remote binding.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub check_set_age_seconds: Option<u64>,
    /// Checks that are blocking (need acknowledgment)
    pub blocking: Vec<CheckMatch>,
    /// Checks that are warnings
    pub warnings: Vec<CheckMatch>,
    /// Checks that were acknowledged
    pub acknowledged: Vec<CheckMatch>,
    /// Monitor-state cloud checks that surfaced: recorded for promotion
    /// decisions, never agent-visible, never gating (additive within
    /// schema 1; omitted when empty)
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub monitor: Vec<CheckMatch>,
}

/// Version of the upload envelope format (see `docs/SCHEMA.md`)
pub const ENVELOPE_SCHEMA: u32 = 1;

/// Upload envelope: the hosted-ingestion contract.
///
/// Wraps the verbatim `noslop check --json` payload with run coordinates
/// and the run's ledger records.
#[derive(Debug, Serialize)]
pub struct UploadEnvelope {
    /// Envelope format version
    pub schema: u32,
    /// Repository slug (e.g. "noslop-sh/noslop")
    pub repo: String,
    /// Commit under test
    pub sha: String,
    /// Pull request number, empty for non-PR runs
    pub pr: String,
    /// Base ref the diff was computed against
    pub base: String,
    /// Head branch name, empty when unknown (additive, schema 1)
    #[serde(skip_serializing_if = "String::is_empty")]
    pub branch: String,
    /// Pull request title, empty for non-PR runs (additive, schema 1)
    #[serde(skip_serializing_if = "String::is_empty")]
    pub pr_title: String,
    /// The `noslop check --json` payload, verbatim
    pub check: serde_json::Value,
    /// Ledger records for the checks this run touched: the acknowledgment
    /// justification, actor, timestamp, and ack-time tree oid. Additive
    /// within schema 1 — consumers must treat it as optional.
    pub ledger: Vec<crate::adapters::ledger::LedgerRecord>,
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
        } else if self.enforced {
            println!("Needs answers:");
            for m in &self.blocking {
                println!("  [{}] {}", m.id, m.file);
                println!("          {}\n", m.message);
            }
            println!("NEEDS ANSWERS: {} check(s) on this commit\n", self.blocking.len());
            println!("To answer:      noslop ack <check-id> -m \"your acknowledgment\"");
            println!(
                "Example:        noslop ack {} -m \"reviewed and verified\"",
                self.blocking.first().map_or("CHK-1", |b| b.id.as_str())
            );
        } else {
            println!("Guidance (an agent would pause here):");
            for m in &self.blocking {
                println!("  [{}] {}", m.id, m.file);
                println!("          {}\n", m.message);
            }
            println!("Human committer - proceeding without acknowledgment.");
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
