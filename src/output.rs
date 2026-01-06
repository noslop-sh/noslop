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
    /// Checks that are blocking (need verification)
    pub blocking: Vec<CheckMatch>,
    /// Checks that are warnings
    pub warnings: Vec<CheckMatch>,
    /// Checks that were verified
    pub verified: Vec<CheckMatch>,
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
    /// Whether this check was verified
    pub verified: bool,
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

/// Result of a verify operation
#[derive(Debug, Serialize)]
pub struct VerifyResult {
    /// Whether the verification was successful
    pub success: bool,
    /// The check ID that was verified
    pub check_id: String,
    /// The verification message
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

        if self.blocking.is_empty() && self.warnings.is_empty() && self.verified.is_empty() {
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
            println!("All checks verified. Commit may proceed.");
        } else {
            println!("Blocking:");
            for m in &self.blocking {
                println!("  [{}] {}", m.id, m.file);
                println!("          {}\n", m.message);
            }
            println!("BLOCKED: {} unverified check(s)\n", self.blocking.len());
            println!("To verify: noslop verify <check-id> -m \"your verification\"");
            println!(
                "Example:   noslop verify {} -m \"reviewed and verified\"",
                self.blocking.first().map_or("NSL-1", |b| b.id.as_str())
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

impl VerifyResult {
    /// Render the result based on output mode
    pub fn render(&self, mode: OutputMode) {
        match mode {
            OutputMode::Human => self.render_human(),
            OutputMode::Json => self.render_json(),
        }
    }

    fn render_human(&self) {
        if self.success {
            println!("Verified: {}", self.check_id);
            println!("Message: {}", self.message);
        } else {
            println!("Failed to verify: {}", self.message);
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

// =============================================================================
// TASK OUTPUT TYPES
// =============================================================================

/// Result of a task list operation
#[derive(Debug, Serialize)]
pub struct TaskListResult {
    /// List of tasks
    pub tasks: Vec<TaskInfo>,
    /// Total count
    pub total: usize,
}

/// Information about a task
#[derive(Debug, Serialize)]
pub struct TaskInfo {
    /// Task ID
    pub id: String,
    /// Task title
    pub title: String,
    /// Current status
    pub status: String,
    /// Priority level
    pub priority: String,
    /// Tasks blocking this one
    pub blocked_by: Vec<String>,
    /// Whether this task is ready to work on
    pub ready: bool,
    /// Optional notes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    /// When created
    pub created_at: String,
}

/// Result of a task show operation
#[derive(Debug, Serialize)]
pub struct TaskShowResult {
    /// Whether the task was found
    pub found: bool,
    /// Task info (if found)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task: Option<TaskInfo>,
}

impl TaskListResult {
    /// Render the result based on output mode
    pub fn render(&self, mode: OutputMode) {
        match mode {
            OutputMode::Human => self.render_human(),
            OutputMode::Json => self.render_json(),
        }
    }

    fn render_human(&self) {
        if self.tasks.is_empty() {
            println!("No tasks found.");
            println!("Create one with: noslop task add \"task title\"");
            return;
        }

        println!("Tasks ({}):\n", self.total);
        for t in &self.tasks {
            let status_icon = match t.status.as_str() {
                "pending" => "○",
                "in_progress" => "◐",
                "done" => "●",
                _ => "?",
            };
            let ready_marker = if t.ready && t.status == "pending" {
                " [ready]"
            } else {
                ""
            };
            println!("  {} [{}] {} ({}){}", status_icon, t.id, t.title, t.priority, ready_marker);
            if !t.blocked_by.is_empty() {
                println!("      blocked by: {}", t.blocked_by.join(", "));
            }
        }
    }

    fn render_json(&self) {
        println!("{}", serde_json::to_string_pretty(self).unwrap_or_default());
    }
}

impl TaskShowResult {
    /// Render the result based on output mode
    pub fn render(&self, mode: OutputMode) {
        match mode {
            OutputMode::Human => self.render_human(),
            OutputMode::Json => self.render_json(),
        }
    }

    fn render_human(&self) {
        if let Some(t) = &self.task {
            println!("Task: {}", t.id);
            println!("  Title:    {}", t.title);
            println!("  Status:   {}", t.status);
            println!("  Priority: {}", t.priority);
            println!("  Ready:    {}", if t.ready { "yes" } else { "no" });
            if !t.blocked_by.is_empty() {
                println!("  Blocked by: {}", t.blocked_by.join(", "));
            }
            if let Some(notes) = &t.notes {
                println!("  Notes:    {notes}");
            }
            println!("  Created:  {}", t.created_at);
        } else {
            println!("Task not found.");
        }
    }

    fn render_json(&self) {
        println!("{}", serde_json::to_string_pretty(self).unwrap_or_default());
    }
}
