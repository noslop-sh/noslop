# Run Loop Design

Design specification for `noslop run` -- the Rust orchestrator that translates ralph's bash iteration loop into noslop's hexagonal architecture.

## Overview

The run loop is the core of noslop's autonomous agent capability. It reads a plan file, invokes an AI agent repeatedly via the `AgentRuntime` trait, tracks progress, detects stuck states with graduated escalation, gates iterations on external test results, and optionally runs a post-loop review. All orchestration logic lives in `src/core/services/` as pure business logic; all I/O goes through port traits.

### What ralph does today (288 lines of bash)

```
1. Parse arguments, resolve plan file
2. Optional worktree creation
3. For each iteration 1..max:
   a. Count tasks (before)
   b. Generate context (iteration number, plan status, git activity)
   c. If stuck >= 3: append stuck recovery prompt
   d. Invoke: claude -p "$PROMPT" --output-format text --permission-mode bypassPermissions
   e. Log output to .ralph-log
   f. Run checkpoint
   g. Check for <promise>COMPLETE</promise> -> exit success
   h. Count tasks (after), update stuck counter
   i. If stuck >= 5 -> exit failure
4. Exit with "max iterations reached"
```

### What `noslop run` adds

- Git-history-only progress tracking -- git commits ARE the iteration memory
- Atomic commit convention -- each iteration produces structured commits with metadata tags
- Git-based failure memory -- `[STATUS: failed]` commits with `[ERROR]` and `[LESSON]` tags
- Stacked PR flow -- clean, reviewable, revertable commit history
- Graduated 4-level stuck escalation instead of binary 3/5 thresholds
- Diff-based progress detection alongside task-count detection
- Post-iteration test gates that feed failures into next iteration's context
- Confidence-gated completion requiring evidence before marking tasks done
- Post-loop review agent pass
- Agent-agnostic design via `AgentRuntime` trait

## Module Organization

```
src/
  core/
    models/
      plan.rs           # Plan, Task, TaskStatus, PlanFormat
      commit_info.rs    # CommitInfo, DiffStat (parsed from git history)
      run_config.rs     # RunConfig, StuckPolicy, TestGate
    ports/
      agent.rs          # AgentRuntime, AgentConfig (from 03-agent-port-design)
      vcs.rs            # VersionControl trait (extended with commit history methods)
      plan_store.rs     # PlanStore trait -- read/write plan files
      test_runner.rs    # TestRunner trait -- execute project tests
    services/
      run_loop.rs       # RunLoop orchestrator (pure logic)
      plan_parser.rs    # Plan file parsing (JSON + Markdown)
      stuck_detector.rs # Graduated escalation logic
      prompt_builder.rs # Iteration prompt construction
  adapters/
    plan/
      markdown.rs       # MarkdownPlanStore
      json.rs           # JsonPlanStore
    test_runner/
      detect.rs         # Auto-detect test framework
      cargo.rs          # Cargo test adapter
      npm.rs            # npm test adapter
      pytest.rs         # pytest adapter
  cli/
    commands/
      run.rs            # CLI entry point for `noslop run`
```

## Domain Models

### Plan (`src/core/models/plan.rs`)

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Format of the plan file
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanFormat {
    /// Markdown checklists: `- [ ]` / `- [x]`
    Markdown,
    /// JSON with `"passes": bool` fields
    Json,
}

/// Status of a single task
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    /// Not yet completed
    Incomplete,
    /// Completed successfully
    Complete,
    /// Explicitly skipped with a reason
    Skipped,
}

/// A single task in the plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// Zero-based index in the plan file
    pub index: usize,
    /// Task description (first line of the checklist item, or JSON description field)
    pub description: String,
    /// Current completion status
    pub status: TaskStatus,
    /// Optional acceptance criteria (from JSON `steps` array)
    pub acceptance_criteria: Vec<String>,
    /// Optional notes added during execution (e.g., blocker explanations)
    pub notes: Vec<String>,
}

/// Parsed plan file
#[derive(Debug, Clone)]
pub struct Plan {
    /// Detected format
    pub format: PlanFormat,
    /// Absolute path to the plan file
    pub path: std::path::PathBuf,
    /// All tasks in order
    pub tasks: Vec<Task>,
    /// Raw file content (preserved for round-trip fidelity)
    pub raw_content: String,
}

impl Plan {
    /// Count of completed tasks
    #[must_use]
    pub fn done_count(&self) -> usize {
        self.tasks
            .iter()
            .filter(|t| t.status == TaskStatus::Complete)
            .count()
    }

    /// Count of total tasks (excludes skipped)
    #[must_use]
    pub fn total_count(&self) -> usize {
        self.tasks
            .iter()
            .filter(|t| t.status != TaskStatus::Skipped)
            .count()
    }

    /// Get the first incomplete task
    #[must_use]
    pub fn next_incomplete(&self) -> Option<&Task> {
        self.tasks
            .iter()
            .find(|t| t.status == TaskStatus::Incomplete)
    }

    /// Whether all non-skipped tasks are complete
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.tasks
            .iter()
            .all(|t| t.status != TaskStatus::Incomplete)
    }
}
```

### Commit Info (`src/core/models/commit_info.rs`)

Git history is the single source of truth for iteration progress. Instead of maintaining separate state files, each iteration produces an atomic commit with structured metadata tags in the commit message. The `CommitInfo` struct represents parsed information from these commits.

```rust
use chrono::{DateTime, Utc};

/// Structured information parsed from a git commit
#[derive(Debug, Clone)]
pub struct CommitInfo {
    /// Short commit hash
    pub id: String,
    /// Full commit message
    pub message: String,
    /// Commit timestamp
    pub timestamp: DateTime<Utc>,
    /// Files changed in this commit
    pub files_changed: Vec<String>,
    /// Lines added
    pub insertions: u32,
    /// Lines removed
    pub deletions: u32,
}

/// Aggregated diff statistics since a given commit
#[derive(Debug, Clone, Default)]
pub struct DiffStat {
    /// Files changed
    pub files: Vec<String>,
    /// Total lines added
    pub insertions: u32,
    /// Total lines removed
    pub deletions: u32,
}

impl CommitInfo {
    /// Parse the `[TASK: N]` tag from the commit message, if present
    #[must_use]
    pub fn task_number(&self) -> Option<u32> {
        let re = regex::Regex::new(r"\[TASK:\s*(\d+)\]").ok()?;
        re.captures(&self.message)
            .and_then(|c| c[1].parse().ok())
    }

    /// Parse the `[STATUS: ...]` tag from the commit message
    #[must_use]
    pub fn status(&self) -> Option<String> {
        let re = regex::Regex::new(r"\[STATUS:\s*(\w+)\]").ok()?;
        re.captures(&self.message)
            .map(|c| c[1].to_string())
    }

    /// Whether this commit represents a failed iteration
    #[must_use]
    pub fn is_failure(&self) -> bool {
        self.status().as_deref() == Some("failed")
    }

    /// Parse the `[ERROR: ...]` tag from the commit message
    #[must_use]
    pub fn error_message(&self) -> Option<String> {
        let re = regex::Regex::new(r"\[ERROR:\s*(.+)\]").ok()?;
        re.captures(&self.message)
            .map(|c| c[1].to_string())
    }

    /// Parse the `[LESSON: ...]` tag from the commit message
    #[must_use]
    pub fn lesson(&self) -> Option<String> {
        let re = regex::Regex::new(r"\[LESSON:\s*(.+)\]").ok()?;
        re.captures(&self.message)
            .map(|c| c[1].to_string())
    }

    /// Parse the `[CONFIDENCE: N]` tag from the commit message
    #[must_use]
    pub fn confidence(&self) -> Option<u8> {
        let re = regex::Regex::new(r"\[CONFIDENCE:\s*(\d)\]").ok()?;
        re.captures(&self.message)
            .and_then(|c| c[1].parse::<u8>().ok())
            .filter(|&n| (1..=5).contains(&n))
    }
}
```

#### Atomic Commit Convention

Each iteration's checkpoint produces a structured commit message. The commit log doubles as a progress report.

Successful iteration:

```
noslop: iteration 3 -- Add validation to user input

[TASK: 3]
[CONFIDENCE: 4]
[STATUS: complete]
[TEST: pass]
```

Failed iteration:

```
noslop: iteration 4 -- Implement rate limiter [FAILED]

[TASK: 3]
[STATUS: failed]
[ERROR: Deadlock under concurrent access]
[LESSON: Mutex-based approach deadlocks across await points]
```

The beauty of this approach: git history IS the memory. No state files to get out of sync. Each iteration's work is a clean atomic commit. You get stacked PR flow for free.

### Run Configuration (`src/core/models/run_config.rs`)

```rust
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

/// Configuration for a `noslop run` invocation
#[derive(Debug, Clone)]
pub struct RunConfig {
    /// Absolute path to the plan file
    pub plan_file: PathBuf,
    /// Working directory for agent invocations
    pub working_dir: PathBuf,
    /// Maximum number of iterations
    pub max_iterations: u32,
    /// Stuck detection policy
    pub stuck_policy: StuckPolicy,
    /// Test gate configuration
    pub test_gate: Option<TestGate>,
    /// Whether to run a post-loop review
    pub post_loop_review: bool,
    /// Whether to use a worktree
    pub use_worktree: bool,
    /// Optional worktree branch name (auto-generated if None)
    pub worktree_branch: Option<String>,
    /// Confidence threshold for task completion (1-5, None = disabled)
    pub confidence_threshold: Option<u8>,
    /// Whether to run an initializer phase on iteration 1
    pub initializer_phase: bool,
}

impl Default for RunConfig {
    fn default() -> Self {
        Self {
            plan_file: PathBuf::new(),
            working_dir: PathBuf::from("."),
            max_iterations: 20,
            stuck_policy: StuckPolicy::default(),
            test_gate: None,
            post_loop_review: true,
            use_worktree: false,
            worktree_branch: None,
            confidence_threshold: Some(4),
            initializer_phase: true,
        }
    }
}

/// Stuck detection thresholds (graduated 4-level escalation)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StuckPolicy {
    /// Iterations without progress before Level 1 (suggest different approach)
    pub level_1_threshold: u32,
    /// Iterations before Level 2 (mandate root cause analysis)
    pub level_2_threshold: u32,
    /// Iterations before Level 3 (force skip/decompose)
    pub level_3_threshold: u32,
    /// Iterations before Level 4 (diagnostic report + exit)
    pub level_4_threshold: u32,
}

impl Default for StuckPolicy {
    fn default() -> Self {
        Self {
            level_1_threshold: 2,
            level_2_threshold: 4,
            level_3_threshold: 6,
            level_4_threshold: 8,
        }
    }
}

/// External test gate configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestGate {
    /// Command to run tests (e.g., "cargo test", "npm test")
    pub command: String,
    /// Working directory for test execution (relative to repo root)
    pub working_dir: Option<PathBuf>,
    /// Maximum lines of test output to capture
    pub max_output_lines: usize,
    /// Timeout in seconds
    pub timeout_secs: u64,
}

impl Default for TestGate {
    fn default() -> Self {
        Self {
            command: String::new(),
            working_dir: None,
            max_output_lines: 50,
            timeout_secs: 300,
        }
    }
}
```

### Escalation Level (`src/core/models/run_config.rs`, continued)

```rust
/// The four graduated escalation levels for stuck detection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EscalationLevel {
    /// No intervention needed
    None,
    /// Level 1: Suggest a different approach
    Suggest,
    /// Level 2: Mandate root cause analysis before continuing
    AnalyzeRootCause,
    /// Level 3: Force skip, decompose, or simplify
    ForceStrategyChange,
    /// Level 4: Generate diagnostic report and terminate
    DiagnosticExit,
}

impl EscalationLevel {
    /// Determine escalation level from stuck count using the default thresholds
    #[must_use]
    pub fn from_stuck_count(stuck_count: u32) -> Self {
        Self::from_stuck_count_with_policy(stuck_count, &StuckPolicy::default())
    }

    /// Determine escalation level from stuck count using custom thresholds
    #[must_use]
    pub fn from_stuck_count_with_policy(stuck_count: u32, policy: &StuckPolicy) -> Self {
        if stuck_count >= policy.level_4_threshold {
            Self::DiagnosticExit
        } else if stuck_count >= policy.level_3_threshold {
            Self::ForceStrategyChange
        } else if stuck_count >= policy.level_2_threshold {
            Self::AnalyzeRootCause
        } else if stuck_count >= policy.level_1_threshold {
            Self::Suggest
        } else {
            Self::None
        }
    }
}
```

## Port Traits

### Plan Store (`src/core/ports/plan_store.rs`)

```rust
use std::path::Path;
use crate::core::models::{Plan, Task, TaskStatus};

/// Read and write plan files in any supported format
#[cfg_attr(test, mockall::automock)]
pub trait PlanStore: Send + Sync {
    /// Parse a plan file from disk
    fn load(&self, path: &Path) -> anyhow::Result<Plan>;

    /// Write an updated plan back to disk (preserves format)
    fn save(&self, plan: &Plan) -> anyhow::Result<()>;

    /// Update a single task's status in the plan file
    ///
    /// This performs a targeted edit rather than rewriting the entire file,
    /// reducing the risk of accidental corruption.
    fn update_task_status(
        &self,
        path: &Path,
        task_index: usize,
        new_status: TaskStatus,
    ) -> anyhow::Result<()>;
}
```

### Test Runner (`src/core/ports/test_runner.rs`)

```rust
use std::path::Path;

/// Result of running external tests
#[derive(Debug, Clone)]
pub struct TestResult {
    /// Whether all tests passed
    pub passed: bool,
    /// Exit code of the test process
    pub exit_code: i32,
    /// Captured output (stdout + stderr, truncated)
    pub output: String,
    /// Number of tests run (if parseable)
    pub tests_run: Option<u32>,
    /// Number of tests failed (if parseable)
    pub tests_failed: Option<u32>,
    /// Duration in seconds
    pub duration_secs: f64,
}

/// Execute project tests as an external gate
#[cfg_attr(test, mockall::automock)]
pub trait TestRunner: Send + Sync {
    /// Run the test suite and capture results
    fn run_tests(&self, working_dir: &Path) -> anyhow::Result<TestResult>;

    /// Detect the test framework from project files
    ///
    /// Returns None if no recognized test framework is found.
    fn detect(working_dir: &Path) -> Option<Box<dyn TestRunner>>
    where
        Self: Sized;
}
```

### VersionControl Trait Extensions (`src/core/ports/vcs.rs`)

The existing `VersionControl` trait (defined in the VCS port design) is extended with methods for reading commit history. This is how the run loop accesses iteration memory -- through git history rather than state files.

```rust
use crate::core::models::{CommitInfo, DiffStat};

// New methods added to the existing VersionControl trait:

#[cfg_attr(test, mockall::automock)]
pub trait VersionControl: Send + Sync {
    // ... existing methods (checkpoint, etc.) ...

    /// Retrieve the N most recent commits as structured data.
    ///
    /// Parses commit messages, timestamps, and diff stats for each commit.
    /// Used by the run loop to build iteration context from git history.
    fn recent_commits(&self, count: usize) -> anyhow::Result<Vec<CommitInfo>>;

    /// Check whether the working tree is clean (no uncommitted changes).
    fn is_clean(&self) -> anyhow::Result<bool>;

    /// Stage all changes and commit with the given message.
    ///
    /// This is the atomic checkpoint that records each iteration's work.
    /// The message should follow the structured commit convention:
    /// `noslop: iteration N -- <summary>\n\n[TASK: N]\n[STATUS: ...]\n...`
    fn commit_all(&self, message: &str) -> anyhow::Result<()>;

    /// Get diff statistics since a given commit.
    ///
    /// Used for diff-based progress detection (detecting code churn
    /// when task counts are not advancing).
    fn diff_stat_since(&self, commit_id: &str) -> anyhow::Result<DiffStat>;
}
```

## Core Services

### Stuck Detector (`src/core/services/stuck_detector.rs`)

Pure logic for graduated escalation. No I/O. Operates on `&[CommitInfo]` parsed from git history rather than a `ProgressFile`.

```rust
use crate::core::models::{
    CommitInfo, EscalationLevel, StuckPolicy,
};

/// Outcome of stuck detection for a single iteration
#[derive(Debug, Clone)]
pub struct StuckAssessment {
    /// Current escalation level
    pub level: EscalationLevel,
    /// Number of consecutive iterations without task-count progress
    pub stuck_count: u32,
    /// Whether diff-based analysis detected meaningful code changes
    pub has_code_churn: bool,
    /// Whether the loop should terminate
    pub should_terminate: bool,
    /// Prompt section to inject into the next iteration
    pub prompt_injection: Option<String>,
}

/// Assess whether the run is stuck based on recent commit history.
///
/// Parses commit messages for `[TASK: N]` and `[STATUS: ...]` markers.
/// Counts consecutive commits without task-count progress.
/// Detects code churn (commits with file changes but no task completion).
pub fn assess_stuck(
    recent_commits: &[CommitInfo],
    current_task: u32,
    policy: &StuckPolicy,
) -> StuckAssessment {
    // Count consecutive commits on the same task without completion
    let stuck_count = recent_commits
        .iter()
        .take_while(|c| {
            c.task_number() == Some(current_task)
                && c.status().as_deref() != Some("complete")
        })
        .count() as u32;

    // Detect code churn: commits with file changes but no task completion
    let has_code_churn = recent_commits
        .first()
        .map(|c| {
            (c.insertions > 0 || c.deletions > 0)
                && c.status().as_deref() != Some("complete")
        })
        .unwrap_or(false);

    let level = EscalationLevel::from_stuck_count_with_policy(stuck_count, policy);
    let should_terminate = level == EscalationLevel::DiagnosticExit;
    let prompt_injection = build_escalation_prompt(level, stuck_count, has_code_churn);

    StuckAssessment {
        level,
        stuck_count,
        has_code_churn,
        should_terminate,
        prompt_injection,
    }
}

/// Generate the escalation prompt section for the given level.
///
/// Each level is progressively more forceful, following the research finding
/// that "strongly-worded constraints" are more effective.
fn build_escalation_prompt(
    level: EscalationLevel,
    stuck_count: u32,
    has_code_churn: bool,
) -> Option<String> {
    let churn_warning = if has_code_churn {
        "\n\nWARNING: Code is changing but no tasks are completing. \
         This suggests churn without progress. Stop modifying code \
         until you have identified the root cause."
    } else {
        ""
    };

    match level {
        EscalationLevel::None => None,

        EscalationLevel::Suggest => Some(format!(
            "## Attention\n\n\
             Your last {stuck_count} iterations made no task progress. \
             Review what was tried and choose a genuinely DIFFERENT approach.\
             {churn_warning}"
        )),

        EscalationLevel::AnalyzeRootCause => Some(format!(
            "## Stuck -- Root Cause Analysis Required\n\n\
             {stuck_count} iterations without progress. \
             Before doing ANYTHING else, you MUST:\n\
             1. List 3 possible root causes for why previous attempts failed\n\
             2. Pick the cause most different from previous approaches\n\
             3. Only then implement a fix targeting that specific cause\n\
             \n\
             It is unacceptable to retry the same approach. Review recent git commits \
             for [STATUS: failed] entries to see what has already been tried.\
             {churn_warning}"
        )),

        EscalationLevel::ForceStrategyChange => Some(format!(
            "## Stuck -- Must Change Strategy\n\n\
             {stuck_count} iterations without progress. You MUST do ONE of:\n\
             a) **Skip** this task: add a blocker note and move to the next incomplete task\n\
             b) **Decompose**: break this task into 2-4 smaller sub-tasks in the plan file\n\
             c) **Simplify**: implement a minimal version that partially satisfies the requirement\n\
             \n\
             Continuing the current approach is not an option.\
             {churn_warning}"
        )),

        EscalationLevel::DiagnosticExit => Some(format!(
            "## FINAL ATTEMPT -- Diagnostic Mode\n\n\
             This is your last iteration. Output a diagnostic report covering:\n\
             - Task attempted and its requirements\n\
             - All approaches tried across previous iterations\n\
             - Specific error messages encountered\n\
             - Files involved and their current state\n\
             - Your assessment of what is fundamentally blocking progress\n\
             \n\
             Then output <promise>STUCK</promise>\
             {churn_warning}"
        )),
    }
}
```

### Plan Parser (`src/core/services/plan_parser.rs`)

Pure parsing logic for both Markdown and JSON plan formats.

```rust
use crate::core::models::{Plan, PlanFormat, Task, TaskStatus};
use std::path::Path;

/// Parse a plan file from its content string.
///
/// Supports two formats:
/// - **Markdown**: Lines matching `^\s*- \[[ x]\]` are tasks
/// - **JSON**: Objects with `"passes": bool` fields are tasks
///
/// The parser auto-detects format by checking for Markdown checkboxes first,
/// then falling back to JSON `"passes"` fields.
pub fn parse_plan(path: &Path, content: &str) -> anyhow::Result<Plan> {
    let md_tasks = parse_markdown_tasks(content);
    let json_tasks = parse_json_tasks(content);

    if !md_tasks.is_empty() {
        Ok(Plan {
            format: PlanFormat::Markdown,
            path: path.to_path_buf(),
            tasks: md_tasks,
            raw_content: content.to_string(),
        })
    } else if !json_tasks.is_empty() {
        Ok(Plan {
            format: PlanFormat::Json,
            path: path.to_path_buf(),
            tasks: json_tasks,
            raw_content: content.to_string(),
        })
    } else {
        anyhow::bail!(
            "No tasks found in plan file: {}. \
             Expected markdown checklists (- [ ]) or JSON (\"passes\": false).",
            path.display()
        )
    }
}

/// Parse markdown checklist items
fn parse_markdown_tasks(content: &str) -> Vec<Task> {
    let checkbox_re = regex::Regex::new(r"^\s*- \[([ xX])\]\s*(.+)$").unwrap();
    let mut tasks = Vec::new();

    for (line_idx, line) in content.lines().enumerate() {
        if let Some(caps) = checkbox_re.captures(line) {
            let status = match &caps[1] {
                "x" | "X" => TaskStatus::Complete,
                _ => TaskStatus::Incomplete,
            };
            tasks.push(Task {
                index: tasks.len(),
                description: caps[2].trim().to_string(),
                status,
                acceptance_criteria: Vec::new(),
                notes: Vec::new(),
            });
        }
    }

    tasks
}

/// Parse JSON tasks with `"passes"` field
///
/// Supports both array-of-objects and objects with `"passes"` anywhere in
/// the structure. Extracts `description`, `passes`, and optional `steps`.
fn parse_json_tasks(content: &str) -> Vec<Task> {
    // Try parsing as a JSON value and walk for "passes" fields
    let Ok(value) = serde_json::from_str::<serde_json::Value>(content) else {
        return Vec::new();
    };
    let mut tasks = Vec::new();
    collect_json_tasks(&value, &mut tasks);
    tasks
}

fn collect_json_tasks(value: &serde_json::Value, tasks: &mut Vec<Task>) {
    match value {
        serde_json::Value::Array(arr) => {
            for item in arr {
                collect_json_tasks(item, tasks);
            }
        }
        serde_json::Value::Object(map) => {
            if let Some(passes) = map.get("passes") {
                let status = if passes.as_bool().unwrap_or(false) {
                    TaskStatus::Complete
                } else {
                    TaskStatus::Incomplete
                };
                let description = map
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("(unnamed task)")
                    .to_string();
                let acceptance_criteria = map
                    .get("steps")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();
                tasks.push(Task {
                    index: tasks.len(),
                    description,
                    status,
                    acceptance_criteria,
                    notes: Vec::new(),
                });
            }
            // Recurse into nested objects
            for (_, v) in map {
                if v.is_object() || v.is_array() {
                    collect_json_tasks(v, tasks);
                }
            }
        }
        _ => {}
    }
}
```

### Prompt Builder (`src/core/services/prompt_builder.rs`)

Constructs the per-iteration prompt. Static parts come first (for prompt caching), dynamic context comes last.

````rust
use crate::core::models::{
    CommitInfo, EscalationLevel, Plan, RunConfig, Task,
};

/// Context for building a single iteration's prompt
pub struct PromptContext<'a> {
    /// Current iteration number (1-based)
    pub iteration: u32,
    /// Maximum iterations
    pub max_iterations: u32,
    /// The parsed plan
    pub plan: &'a Plan,
    /// Recent commits from git history (from vcs.recent_commits(N))
    pub recent_commits: &'a [CommitInfo],
    /// Test failure output from previous iteration (if any)
    pub test_failures: Option<String>,
    /// Stuck escalation prompt to inject (if any)
    pub escalation_prompt: Option<String>,
    /// Confidence threshold for completion
    pub confidence_threshold: Option<u8>,
    /// Whether this is the initializer phase (iteration 1)
    pub is_initializer: bool,
    /// Absolute path to the plan file
    pub plan_file_path: String,
}

/// Build the complete prompt for an agent iteration.
///
/// Prompt structure (ordered for prompt caching -- static first, dynamic last):
///
/// 1. Static prompt template (cacheable across all iterations)
/// 2. Plan file path (stable)
/// 3. Failure context extracted from recent commits (semi-stable)
/// 4. Escalation prompt (changes only when stuck)
/// 5. Dynamic context: iteration number, plan status, recent commits, test failures
pub fn build_iteration_prompt(ctx: &PromptContext<'_>) -> String {
    let mut prompt = String::with_capacity(8192);

    // --- Static section (cacheable) ---
    if ctx.is_initializer {
        prompt.push_str(&initializer_template());
    } else {
        prompt.push_str(&standard_template());
    }

    prompt.push_str(&format!(
        "\n\nPlan file: {}\n",
        ctx.plan_file_path
    ));

    // --- Semi-static section ---

    // Failure context extracted from recent commits with [STATUS: failed]
    let failed_commits: Vec<_> = ctx.recent_commits
        .iter()
        .filter(|c| c.is_failure())
        .collect();
    if !failed_commits.is_empty() {
        prompt.push_str("\n\n### Previous Failed Approaches\n\n");
        for commit in &failed_commits {
            let error = commit.error_message().unwrap_or_default();
            let lesson = commit.lesson().unwrap_or_default();
            prompt.push_str(&format!(
                "- **{}**: Error: {}. Lesson: {}\n",
                commit.id, error, lesson
            ));
        }
    }

    // Confidence gate
    if let Some(threshold) = ctx.confidence_threshold {
        prompt.push_str(&format!(
            "\n\n## Confidence Requirement\n\n\
             Before marking any task complete, state your confidence level (1-5).\n\
             If your confidence is below {threshold}, you MUST provide test evidence \
             (actual test output showing passage) before marking the task done.\n\
             Format: [CONFIDENCE: N]\n"
        ));
    }

    // Escalation prompt (only present when stuck)
    if let Some(escalation) = &ctx.escalation_prompt {
        prompt.push_str("\n\n");
        prompt.push_str(escalation);
    }

    // --- Dynamic section (changes every iteration) ---
    prompt.push_str("\n\n## Iteration Context\n\n");
    prompt.push_str(&format!(
        "This is iteration {} of {}.\n\n",
        ctx.iteration, ctx.max_iterations
    ));

    prompt.push_str("### Plan Status\n");
    prompt.push_str(&format!(
        "Tasks: {} / {} complete\n\n",
        ctx.plan.done_count(),
        ctx.plan.total_count()
    ));

    // Recent iteration history from git commits
    if !ctx.recent_commits.is_empty() {
        prompt.push_str("### Recent Iteration History\n\n");
        for commit in ctx.recent_commits.iter().take(10) {
            let status = commit.status().unwrap_or_else(|| "unknown".to_string());
            let first_line = commit.message.lines().next().unwrap_or("");
            prompt.push_str(&format!(
                "- `{}` {} (status: {}, +{} -{}, {} files)\n",
                commit.id,
                first_line,
                status,
                commit.insertions,
                commit.deletions,
                commit.files_changed.len(),
            ));
        }
        prompt.push('\n');
    }

    // Test failures from previous iteration
    if let Some(test_output) = &ctx.test_failures {
        prompt.push_str("\n### Test Failures From Previous Iteration\n\n");
        prompt.push_str("```\n");
        prompt.push_str(test_output);
        prompt.push_str("\n```\n\n");
        prompt.push_str(
            "Fix these failures BEFORE proceeding to new tasks. \
             It is unacceptable to mark a task complete while tests are failing.\n",
        );
    }

    prompt
}

/// The standard per-iteration prompt template (cacheable).
fn standard_template() -> String {
    r#"Read the plan file specified below.

## Your job

Find the first incomplete task and complete it. One task per iteration, no more.

## Step 1: Orient yourself

Before writing any code:

1. Read the plan file completely. Understand the task you are about to work on.
2. Check the iteration context below. Review recent git commits for
   `[STATUS: failed]` entries. If the same task was attempted before, understand
   what went wrong. It is unacceptable to repeat a previously failed approach.
3. Discover the repository layout. Note which directories are git repos.
4. Identify where tests live for this task. Run the relevant test suite BEFORE
   making changes to establish a baseline.

## Step 2: Implement

Make your changes in small, atomic increments. Commit after each logical unit.

Commit rules:
- One logical change per commit. Each commit must leave tests passing.
- Include test changes with the code they test (one commit, not two).
- Write commit messages that explain WHY, not just what.
- Commit in the correct git repo.
- Do NOT batch everything into one giant commit at the end.
- Do NOT git push.

## Step 3: Verify

After implementing, run the acceptance criteria for this task (tests, build, etc.).
If tests fail, fix them before proceeding. Do NOT mark a task complete with
failing tests.

## Step 4: Mark complete

Update the plan file to mark the task as done:
- Markdown checklists: change `- [ ]` to `- [x]`
- JSON tasks: change `"passes": false` to `"passes": true`

Commit the plan file update as a separate commit.

## Step 5: Report

Output a status line:
[RALPH:DONE task="<task description (first 80 chars)>" iteration=N]

Your changes will be committed atomically. Include [CONFIDENCE: N] in your output.

## Completion

When ALL tasks in the plan are complete, output exactly:
<promise>COMPLETE</promise>"#
        .to_string()
}

/// The initializer prompt for iteration 1 (two-agent harness pattern).
fn initializer_template() -> String {
    r#"Read the plan file specified below.

## Your job -- Initialization Phase

This is iteration 1. Before implementing any tasks, set up the working context:

1. Read the plan file completely. Understand the full scope of work.
2. Explore the repository structure. Identify key files, test locations,
   and build systems.
3. Run the existing test suite to establish a baseline.
4. Review the git log to understand what has been done.
   Your work will be committed atomically.
5. Then proceed to the first incomplete task following the standard workflow below.

## Standard Workflow

Find the first incomplete task and complete it. One task per iteration, no more.

### Orient yourself

1. Read the plan file completely.
2. Check the iteration context below. Review recent git commits for context.
3. Identify where tests live for this task. Run them BEFORE making changes.

### Implement

Make changes in small, atomic commits. One logical change per commit.
Do NOT batch everything into one giant commit. Do NOT git push.

### Verify

Run acceptance criteria. Fix failures before proceeding.

### Mark complete

Update the plan file (- [ ] to - [x], or "passes": false to true).
Commit the plan file update separately.

### Report

Output: [RALPH:DONE task="<description>" iteration=1]

Your changes will be committed atomically. Include [CONFIDENCE: N] in your output.

## Completion

When ALL tasks in the plan are complete, output exactly:
<promise>COMPLETE</promise>"#
        .to_string()
}
````

### Run Loop Orchestrator (`src/core/services/run_loop.rs`)

The central orchestration service. This is the Rust translation of ralph's main loop. It coordinates all the pieces but performs no I/O directly -- all I/O goes through the injected port trait references.

```rust
use std::path::Path;
use crate::core::models::{
    CommitInfo, EscalationLevel, Plan, RunConfig,
};
use crate::core::ports::{AgentRuntime, InvocationConfig, PlanStore, TestRunner, VersionControl};
use crate::core::services::{prompt_builder, stuck_detector};

/// Outcome of the entire run loop
#[derive(Debug)]
pub enum RunOutcome {
    /// All tasks completed successfully
    Complete {
        iterations_used: u32,
        plan: Plan,
    },
    /// Agent reported it is stuck and cannot proceed
    Stuck {
        iterations_used: u32,
        diagnostic_output: Option<String>,
    },
    /// Reached maximum iterations without completion
    MaxIterationsReached {
        iterations_used: u32,
        tasks_completed: usize,
        tasks_total: usize,
    },
    /// Run was interrupted or failed
    Failed {
        iterations_used: u32,
        error: String,
    },
}

/// The main orchestrator for `noslop run`.
///
/// Coordinates plan parsing, agent invocation, stuck detection, test gates,
/// and post-loop review. All I/O is delegated to the injected trait objects.
///
/// Progress is tracked entirely through git history. No separate state files.
/// Each iteration produces an atomic commit with structured metadata tags.
/// The commit log doubles as the progress report.
pub struct RunLoop<'a> {
    runtime: &'a dyn AgentRuntime,
    plan_store: &'a dyn PlanStore,
    test_runner: Option<&'a dyn TestRunner>,
    vcs: &'a dyn VersionControl,
    config: RunConfig,
}

impl<'a> RunLoop<'a> {
    pub fn new(
        runtime: &'a dyn AgentRuntime,
        plan_store: &'a dyn PlanStore,
        test_runner: Option<&'a dyn TestRunner>,
        vcs: &'a dyn VersionControl,
        config: RunConfig,
    ) -> Self {
        Self {
            runtime,
            plan_store,
            test_runner,
            vcs,
            config,
        }
    }

    /// Execute the full run loop.
    ///
    /// This is the Rust equivalent of ralph's main `for` loop.
    /// All progress tracking is through git commits -- no state files.
    pub fn execute(&self) -> anyhow::Result<RunOutcome> {
        // 1. Load initial state (plan only -- progress comes from git history)
        let mut plan = self.plan_store.load(&self.config.plan_file)?;
        let mut last_test_failures: Option<String> = None;

        // 2. Main iteration loop
        for iteration in 1..=self.config.max_iterations {
            // 2a. Reload plan (agent may have modified it)
            plan = self.plan_store.load(&self.config.plan_file)?;
            let tasks_before = plan.done_count();

            // 2b. Read recent git history for context and stuck detection
            let recent_commits = self.vcs.recent_commits(10)?;
            let current_task = plan.done_count() as u32 + 1;

            let assessment = stuck_detector::assess_stuck(
                &recent_commits,
                current_task,
                &self.config.stuck_policy,
            );
            let escalation_prompt = assessment.prompt_injection.clone();

            // 2c. Build prompt context (from git history, not state files)
            let ctx = prompt_builder::PromptContext {
                iteration,
                max_iterations: self.config.max_iterations,
                plan: &plan,
                recent_commits: &recent_commits,
                test_failures: last_test_failures.take(),
                escalation_prompt,
                confidence_threshold: self.config.confidence_threshold,
                is_initializer: iteration == 1 && self.config.initializer_phase,
                plan_file_path: self.config.plan_file.display().to_string(),
            };

            let prompt = prompt_builder::build_iteration_prompt(&ctx);

            // 2d. Invoke the agent
            let invocation_config = InvocationConfig {
                prompt,
                working_dir: self.config.working_dir.clone(),
                bypass_permissions: true,
                ..Default::default()
            };

            let result = self.runtime.invoke(&invocation_config)?;
            let output = self.runtime.parse_output(&result);

            // 2e. Log output
            self.append_to_log(iteration, &output.raw_output)?;

            // 2f. Reload plan to measure progress
            plan = self.plan_store.load(&self.config.plan_file)?;
            let tasks_after = plan.done_count();
            let made_progress = tasks_after > tasks_before;

            // 2g. Build structured commit message and commit atomically
            let confidence = parse_confidence(&output.raw_output);
            let task_desc = output
                .completed_task
                .clone()
                .unwrap_or_else(|| "work in progress".to_string());

            let status = if output.raw_output.contains("<promise>STUCK</promise>") {
                "stuck"
            } else if made_progress {
                "complete"
            } else {
                "failed"
            };

            let mut commit_msg = format!(
                "noslop: iteration {} -- {}",
                iteration,
                task_desc,
            );
            if status == "failed" {
                commit_msg.push_str(" [FAILED]");
            }
            commit_msg.push_str(&format!("\n\n[TASK: {}]", current_task));
            if let Some(c) = confidence {
                commit_msg.push_str(&format!("\n[CONFIDENCE: {}]", c));
            }
            commit_msg.push_str(&format!("\n[STATUS: {}]", status));

            // Extract error/lesson from agent output for failed iterations
            if status == "failed" {
                if let Some(error) = extract_error(&output.raw_output) {
                    commit_msg.push_str(&format!("\n[ERROR: {}]", error));
                }
                if let Some(lesson) = extract_lesson(&output.raw_output) {
                    commit_msg.push_str(&format!("\n[LESSON: {}]", lesson));
                }
            }

            self.vcs.commit_all(&commit_msg)?;

            // 2h. Check for plan completion signal
            if output.plan_complete {
                // Reload plan to verify
                plan = self.plan_store.load(&self.config.plan_file)?;
                if plan.is_complete() {
                    self.run_post_loop_review()?;
                    return Ok(RunOutcome::Complete {
                        iterations_used: iteration,
                        plan,
                    });
                }
                // Agent claimed complete but tasks remain -- continue
            }

            // 2i. Check for stuck diagnostic signal
            if output.raw_output.contains("<promise>STUCK</promise>") {
                return Ok(RunOutcome::Stuck {
                    iterations_used: iteration,
                    diagnostic_output: Some(output.raw_output),
                });
            }

            // 2j. Post-iteration test gate
            if let Some(test_runner) = self.test_runner {
                let test_result = test_runner.run_tests(&self.config.working_dir)?;
                if !test_result.passed {
                    last_test_failures = Some(test_result.output.clone());
                }
            }

            // 2k. Stuck termination check
            if assessment.should_terminate {
                return Ok(RunOutcome::Stuck {
                    iterations_used: iteration,
                    diagnostic_output: Some(output.raw_output),
                });
            }
        }

        // 3. Max iterations reached
        plan = self.plan_store.load(&self.config.plan_file)?;
        Ok(RunOutcome::MaxIterationsReached {
            iterations_used: self.config.max_iterations,
            tasks_completed: plan.done_count(),
            tasks_total: plan.total_count(),
        })
    }

    // --- Private helper methods ---

    fn append_to_log(&self, iteration: u32, output: &str) -> anyhow::Result<()> {
        todo!("append to .ralph-log")
    }

    fn run_post_loop_review(&self) -> anyhow::Result<()> {
        if !self.config.post_loop_review {
            return Ok(());
        }

        let review_prompt = "\
            Review all changes on this branch compared to the base.\n\
            Run: git log --oneline main..HEAD\n\
            Run: git diff main..HEAD --stat\n\n\
            For each changed file, check for:\n\
            1. Bugs or logic errors\n\
            2. Security vulnerabilities\n\
            3. Missing error handling\n\
            4. Test coverage gaps\n\
            5. Code quality issues\n\n\
            Write your findings to .noslop/review.md with severity ratings \
            (critical/warning/info). If you find critical issues, list them \
            clearly at the top.";

        let config = InvocationConfig {
            prompt: review_prompt.to_string(),
            working_dir: self.config.working_dir.clone(),
            bypass_permissions: true,
            ..Default::default()
        };

        let _result = self.runtime.invoke(&config)?;
        Ok(())
    }
}

/// Parse a confidence score from agent output.
///
/// Looks for `[CONFIDENCE: N]` where N is 1-5.
fn parse_confidence(output: &str) -> Option<u8> {
    let re = regex::Regex::new(r"\[CONFIDENCE:\s*(\d)\]").ok()?;
    re.captures(output)
        .and_then(|c| c[1].parse::<u8>().ok())
        .filter(|&n| (1..=5).contains(&n))
}

/// Extract an error description from agent output.
fn extract_error(output: &str) -> Option<String> {
    // Look for error patterns in agent output
    let re = regex::Regex::new(r"(?i)error:\s*(.+)").ok()?;
    re.captures(output).map(|c| c[1].trim().to_string())
}

/// Extract a lesson learned from agent output.
fn extract_lesson(output: &str) -> Option<String> {
    let re = regex::Regex::new(r"(?i)lesson:\s*(.+)").ok()?;
    re.captures(output).map(|c| c[1].trim().to_string())
}
```

## CLI Integration

### `noslop run` Command (`src/cli/commands/run.rs`)

This is where dependency injection happens. The CLI layer constructs concrete adapters and passes them to the core `RunLoop`.

```rust
use clap::Args;
use std::path::PathBuf;
use crate::core::models::RunConfig;
use crate::core::ports::{AgentRuntime, AgentType};
use crate::core::services::run_loop::{RunLoop, RunOutcome};
use crate::adapters::agent::{get_agent_runtime};
use crate::adapters::plan::{detect_plan_store};
use crate::adapters::test_runner::{detect_test_runner};

/// Run arguments for clap
#[derive(Args, Debug)]
pub struct RunArgs {
    /// Path to the plan file (markdown or JSON)
    pub plan_file: PathBuf,

    /// Maximum iterations
    #[arg(short, long, default_value_t = 20)]
    pub max_iterations: u32,

    /// Run in an isolated git worktree
    #[arg(long)]
    pub worktree: bool,

    /// Agent type (auto-detected from .noslop.toml if not specified)
    #[arg(short, long)]
    pub agent: Option<String>,

    /// Disable post-loop review
    #[arg(long)]
    pub no_review: bool,

    /// Disable test gates
    #[arg(long)]
    pub no_tests: bool,

    /// Stuck detection level 4 threshold (iterations without progress)
    #[arg(long, default_value_t = 8)]
    pub stuck_limit: u32,
}

/// Entry point for `noslop run <plan_file>`
pub fn run(args: RunArgs) -> anyhow::Result<()> {
    // 1. Resolve agent type
    let agent_type = resolve_agent_type(args.agent.as_deref())?;

    // 2. Construct concrete adapters (dependency injection)
    let runtime = get_agent_runtime(agent_type);
    let plan_store = detect_plan_store(&args.plan_file)?;
    let vcs = crate::adapters::git::GitVersionControl::new()?;

    let test_runner = if args.no_tests {
        None
    } else {
        detect_test_runner(&args.plan_file.parent().unwrap_or(Path::new(".")))
    };

    // 3. Build config
    let config = RunConfig {
        plan_file: std::fs::canonicalize(&args.plan_file)?,
        working_dir: std::env::current_dir()?,
        max_iterations: args.max_iterations,
        post_loop_review: !args.no_review,
        use_worktree: args.worktree,
        stuck_policy: StuckPolicy {
            level_4_threshold: args.stuck_limit,
            ..Default::default()
        },
        ..Default::default()
    };

    // 4. Print banner
    println!("=== noslop run ===");
    println!("Plan:       {}", config.plan_file.display());
    println!("Agent:      {}", agent_type.display_name());
    println!("Max iters:  {}", config.max_iterations);
    println!("Worktree:   {}", config.use_worktree);
    println!("==================");
    println!();

    // 5. Execute the loop
    let loop_runner = RunLoop::new(
        runtime.as_ref(),
        plan_store.as_ref(),
        test_runner.as_deref(),
        &vcs,
        config,
    );

    let outcome = loop_runner.execute()?;

    // 6. Report outcome
    match &outcome {
        RunOutcome::Complete { iterations_used, .. } => {
            println!(
                "=== All tasks complete after {} iteration(s). ===",
                iterations_used
            );
        }
        RunOutcome::Stuck {
            iterations_used,
            diagnostic_output,
        } => {
            println!(
                "=== Stuck after {} iteration(s). ===",
                iterations_used
            );
            if let Some(output) = diagnostic_output {
                println!("\nDiagnostic output:\n{}", output);
            }
        }
        RunOutcome::MaxIterationsReached {
            iterations_used,
            tasks_completed,
            tasks_total,
        } => {
            println!(
                "=== Reached max iterations ({}) without completion. ===",
                iterations_used
            );
            println!(
                "Progress: {} / {} tasks completed",
                tasks_completed, tasks_total
            );
        }
        RunOutcome::Failed { error, .. } => {
            eprintln!("=== Run failed: {} ===", error);
        }
    }

    // 7. Exit code
    match outcome {
        RunOutcome::Complete { .. } => Ok(()),
        _ => std::process::exit(1),
    }
}

fn resolve_agent_type(arg: Option<&str>) -> anyhow::Result<AgentType> {
    if let Some(name) = arg {
        return name
            .parse::<AgentType>()
            .map_err(|e| anyhow::anyhow!(e));
    }
    // Try to read from .noslop.toml [agent] section
    // Fall back to auto-detection (check which CLIs are in PATH)
    todo!("read from .noslop.toml or auto-detect")
}
```

### Clap Integration (`src/cli/app.rs` addition)

The `Run` variant added to the existing `Command` enum:

```rust
// Added to the Command enum in src/cli/app.rs:

/// Run an autonomous iteration loop on a plan file
Run(run::RunArgs),

// Added to the match in run():
Some(Command::Run(args)) => commands::run::run(args),
```

## File Layout: `.noslop/` Directory Extension

```
.noslop/
  .gitkeep
  staged-acks.json       # (existing) pending acknowledgments
  reviews/               # (existing) review session files
  review.md              # NEW: post-loop review findings
```

No progress or failure state files. All iteration memory lives in git commit history via the atomic commit convention described above. The `VersionControl` trait methods (`recent_commits`, `commit_all`, `diff_stat_since`) provide access to this data.

### Example: Reading Progress From Git History

```
$ git log --oneline --grep="noslop: iteration"

a1b2c3d noslop: iteration 3 -- Add validation to user input
f4e5d6c noslop: iteration 2 -- Implement parser module
9a8b7c6 noslop: iteration 1 -- Set up project structure
```

Each commit message contains structured tags that `CommitInfo` methods can parse:

```
$ git log -1 --format="%B" a1b2c3d

noslop: iteration 3 -- Add validation to user input

[TASK: 3]
[CONFIDENCE: 4]
[STATUS: complete]
[TEST: pass]
```

Failed iterations are equally visible in the log:

```
$ git log -1 --format="%B" d7e8f9a

noslop: iteration 4 -- Implement rate limiter [FAILED]

[TASK: 3]
[STATUS: failed]
[ERROR: Deadlock under concurrent access in test_parallel_requests]
[LESSON: Mutex-based approach deadlocks when handler holds lock across await point]
```

## Data Flow

```
                 noslop run plan.md
                        |
                        v
              +---------+---------+
              |   CLI (run.rs)    |
              |  Dependency       |
              |  Injection        |
              +---------+---------+
                        |
      +--------+--------+--------+--------+
      |        |                 |        |
 AgentRuntime PlanStore  VersionControl TestRunner
 (port trait) (port)     (port trait)   (port trait)
      |        |                 |        |
      v        v                 v        v
  +---+----+ +-+------+ +-------+-----+ +-+----------+
  |Claude  | |Markdown| |GitVersion   | |CargoTest   |
  |Codex   | |JsonPlan| |Control      | |NpmTest     |
  +---+----+ +-+------+ +-------+-----+ +-+----------+
      |        |                 |        |
      +--------+--------+--------+--------+
                        |
                        v
              +---------+---------+
              |    RunLoop        |
              |  (core service)   |
              |                   |
              |  For each iter:   |
              |  1. Load plan     |  <--- PlanStore.load()
              |  2. Read history  |  <--- VersionControl.recent_commits()
              |  3. Assess stuck  |  <--- stuck_detector (pure, from commits)
              |  4. Build prompt  |  <--- prompt_builder (pure, from commits)
              |  5. Invoke agent  |  <--- AgentRuntime.invoke()
              |  6. Log output    |
              |  7. Commit atomic |  <--- VersionControl.commit_all()
              |  8. Check done    |  <--- AgentRuntime.parse_output()
              |  9. Run tests     |  <--- TestRunner.run_tests()
              |                   |
              |  Post-loop:       |
              |  10. Review agent |  <--- AgentRuntime.invoke()
              +---------+---------+
                        |
                        v
              +---------+---------+
              |    RunOutcome     |
              |  Complete |       |
              |  Stuck |          |
              |  MaxIters |       |
              |  Failed           |
              +-------------------+
```

## How `noslop run` Calls `AgentRuntime`

The agent invocation point is step 2c in the main loop. The `RunLoop` never knows which agent it is talking to. The interaction is:

```
RunLoop                         AgentRuntime (trait object)
  |                                     |
  |  build_iteration_prompt()           |
  |  (pure function, no trait call)     |
  |                                     |
  |  InvocationConfig {                 |
  |    prompt: "...",                   |
  |    working_dir: "/path/to/repo",   |
  |    bypass_permissions: true,        |
  |  }                                  |
  |                                     |
  | ---- invoke(&config) ------------> |
  |                                     |  ClaudeRuntime: builds
  |                                     |    `claude -p "$prompt"
  |                                     |     --output-format text
  |                                     |     --permission-mode bypassPermissions`
  |                                     |  executes as child process
  |                                     |  captures stdout/stderr
  | <--- InvocationResult ------------ |
  |   { output, exit_code,             |
  |     completed, duration }           |
  |                                     |
  | ---- parse_output(&result) ------> |
  |                                     |  ClaudeRuntime: checks for
  |                                     |    <promise>COMPLETE</promise>
  |                                     |    [RALPH:DONE task="..." iteration=N]
  | <--- IterationOutput ------------- |
  |   { plan_complete,                  |
  |     completed_task,                 |
  |     raw_output, errors }            |
  |                                     |
```

For a different agent (e.g., Codex), the `build_command` method produces different CLI arguments, and `parse_output` looks for different completion signals. The `RunLoop` orchestrator is unchanged.

## Incorporated Research Quick Wins

| Quick Win                           | Where It Appears                                                                                                     | Reference                       |
| ----------------------------------- | -------------------------------------------------------------------------------------------------------------------- | ------------------------------- |
| **Git-history-based progress**      | `CommitInfo` model, `VersionControl.recent_commits()`, commit messages encode iteration metadata                     | Research finding 1              |
| **Graduated escalation (4 levels)** | `EscalationLevel` enum, `StuckPolicy` config, `stuck_detector::assess_stuck()`, `build_escalation_prompt()`          | Research finding 2              |
| **Diff-based progress detection**   | `CommitInfo.insertions`/`deletions`, `vcs.diff_stat_since()`, `StuckAssessment.has_code_churn`                       | Research finding 3              |
| **Post-iteration test gates**       | `TestRunner` port trait, `RunLoop` step 2j, `last_test_failures` fed into next prompt                                | Research finding 4              |
| **Prompt restructure for caching**  | `build_iteration_prompt()` puts static template first, dynamic context last                                          | Research finding 5              |
| **Confidence-gated completion**     | `confidence_threshold` in `RunConfig`, `parse_confidence()`, prompt instructs `[CONFIDENCE: N]`                      | Research finding 6              |
| **Git-based failure memory**        | `[STATUS: failed]` commits with `[ERROR]` and `[LESSON]` tags, parsed by `CommitInfo` methods, injected into prompts | Research finding 9              |
| **Initializer phase**               | `is_initializer` flag, `initializer_template()` for iteration 1, two-agent harness pattern                           | Research finding 10             |
| **Post-loop review**                | `run_post_loop_review()`, BugBot-inspired review pass after completion                                               | Research finding 8              |
| **Strongly-worded constraints**     | Escalation prompts use "unacceptable", "MUST", absolute prohibitions                                                 | Research finding from Anthropic |

## Test Strategy

### Unit Tests (no I/O)

All core services are pure functions testable without mocks:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_markdown_plan() {
        let content = "# Plan\n\n- [x] Task 1\n- [ ] Task 2\n- [ ] Task 3\n";
        let plan = parse_plan(Path::new("plan.md"), content).unwrap();
        assert_eq!(plan.format, PlanFormat::Markdown);
        assert_eq!(plan.done_count(), 1);
        assert_eq!(plan.total_count(), 3);
    }

    #[test]
    fn parse_json_plan() {
        let content = r#"[
            {"description": "Setup", "passes": true},
            {"description": "Implement", "passes": false, "steps": ["write code", "run tests"]}
        ]"#;
        let plan = parse_plan(Path::new("plan.json"), content).unwrap();
        assert_eq!(plan.format, PlanFormat::Json);
        assert_eq!(plan.done_count(), 1);
        assert_eq!(plan.tasks[1].acceptance_criteria.len(), 2);
    }

    #[test]
    fn stuck_detection_graduated_escalation() {
        let policy = StuckPolicy::default();
        assert_eq!(EscalationLevel::from_stuck_count_with_policy(0, &policy), EscalationLevel::None);
        assert_eq!(EscalationLevel::from_stuck_count_with_policy(1, &policy), EscalationLevel::None);
        assert_eq!(EscalationLevel::from_stuck_count_with_policy(2, &policy), EscalationLevel::Suggest);
        assert_eq!(EscalationLevel::from_stuck_count_with_policy(4, &policy), EscalationLevel::AnalyzeRootCause);
        assert_eq!(EscalationLevel::from_stuck_count_with_policy(6, &policy), EscalationLevel::ForceStrategyChange);
        assert_eq!(EscalationLevel::from_stuck_count_with_policy(8, &policy), EscalationLevel::DiagnosticExit);
    }

    #[test]
    fn stuck_detection_from_commit_history() {
        let policy = StuckPolicy::default();

        // 3 consecutive failed commits on the same task
        let commits = vec![
            CommitInfo {
                id: "abc123".to_string(),
                message: "noslop: iteration 5 -- Rate limiter [FAILED]\n\n\
                          [TASK: 3]\n[STATUS: failed]\n\
                          [ERROR: Deadlock]\n[LESSON: Use actors]".to_string(),
                timestamp: Utc::now(),
                files_changed: vec!["src/lib.rs".to_string()],
                insertions: 20,
                deletions: 5,
            },
            CommitInfo {
                id: "def456".to_string(),
                message: "noslop: iteration 4 -- Rate limiter [FAILED]\n\n\
                          [TASK: 3]\n[STATUS: failed]\n\
                          [ERROR: Race condition]".to_string(),
                timestamp: Utc::now(),
                files_changed: vec!["src/lib.rs".to_string()],
                insertions: 15,
                deletions: 10,
            },
            CommitInfo {
                id: "789abc".to_string(),
                message: "noslop: iteration 3 -- Rate limiter [FAILED]\n\n\
                          [TASK: 3]\n[STATUS: failed]".to_string(),
                timestamp: Utc::now(),
                files_changed: vec!["src/lib.rs".to_string()],
                insertions: 30,
                deletions: 0,
            },
        ];

        let assessment = assess_stuck(&commits, 3, &policy);
        assert_eq!(assessment.stuck_count, 3);
        assert_eq!(assessment.level, EscalationLevel::Suggest);
        assert!(assessment.has_code_churn);
        assert!(!assessment.should_terminate);
    }

    #[test]
    fn commit_info_parses_metadata_tags() {
        let commit = CommitInfo {
            id: "abc123".to_string(),
            message: "noslop: iteration 3 -- Add validation\n\n\
                      [TASK: 3]\n[CONFIDENCE: 4]\n[STATUS: complete]\n[TEST: pass]"
                .to_string(),
            timestamp: Utc::now(),
            files_changed: vec![],
            insertions: 0,
            deletions: 0,
        };
        assert_eq!(commit.task_number(), Some(3));
        assert_eq!(commit.confidence(), Some(4));
        assert_eq!(commit.status().as_deref(), Some("complete"));
        assert!(!commit.is_failure());
    }

    #[test]
    fn commit_info_parses_failure_tags() {
        let commit = CommitInfo {
            id: "def456".to_string(),
            message: "noslop: iteration 4 -- Rate limiter [FAILED]\n\n\
                      [TASK: 3]\n[STATUS: failed]\n\
                      [ERROR: Deadlock under concurrent access]\n\
                      [LESSON: Mutex deadlocks across await points]"
                .to_string(),
            timestamp: Utc::now(),
            files_changed: vec![],
            insertions: 0,
            deletions: 0,
        };
        assert!(commit.is_failure());
        assert_eq!(commit.error_message().as_deref(), Some("Deadlock under concurrent access"));
        assert_eq!(commit.lesson().as_deref(), Some("Mutex deadlocks across await points"));
    }

    #[test]
    fn confidence_parsing() {
        assert_eq!(parse_confidence("some output [CONFIDENCE: 4] more"), Some(4));
        assert_eq!(parse_confidence("no confidence here"), None);
        assert_eq!(parse_confidence("[CONFIDENCE: 6]"), None); // out of range
    }
}
```

### Integration Tests (with mocks)

```rust
#[cfg(test)]
mod integration_tests {
    use super::*;
    use mockall::predicate::*;

    #[test]
    fn run_loop_completes_in_one_iteration() {
        let mut mock_runtime = MockAgentRuntime::new();
        mock_runtime
            .expect_invoke()
            .returning(|_| Ok(InvocationResult {
                output: "[RALPH:DONE task=\"setup\" iteration=1]\n\
                         [CONFIDENCE: 5]\n\
                         <promise>COMPLETE</promise>"
                    .to_string(),
                exit_code: 0,
                completed: true,
                status_line: None,
                duration: Duration::from_secs(10),
            }));
        mock_runtime
            .expect_parse_output()
            .returning(|result| IterationOutput {
                plan_complete: true,
                completed_task: Some("setup".to_string()),
                iteration: Some(1),
                raw_output: result.output.clone(),
                errors: vec![],
            });

        let mut mock_plan = MockPlanStore::new();
        mock_plan.expect_load().returning(|_| {
            Ok(Plan {
                format: PlanFormat::Markdown,
                path: PathBuf::from("plan.md"),
                tasks: vec![Task {
                    index: 0,
                    description: "setup".to_string(),
                    status: TaskStatus::Complete,
                    acceptance_criteria: vec![],
                    notes: vec![],
                }],
                raw_content: "- [x] setup\n".to_string(),
            })
        });

        let mut mock_vcs = MockVersionControl::new();
        // Verify that commit_all is called with structured message
        mock_vcs
            .expect_recent_commits()
            .returning(|_| Ok(vec![]));
        mock_vcs
            .expect_commit_all()
            .withf(|msg: &str| {
                msg.starts_with("noslop: iteration 1")
                    && msg.contains("[STATUS: complete]")
                    && msg.contains("[CONFIDENCE: 5]")
            })
            .returning(|_| Ok(()));

        // ... construct RunLoop with mocks and assert Complete outcome
    }
}
```

## Configuration in `.noslop.toml`

The run loop reads its defaults from the project config:

```toml
[project]
prefix = "NOS"

[agent]
type = "claude"

[run]
max_iterations = 20
confidence_threshold = 4
post_loop_review = true
initializer_phase = true

[run.stuck_policy]
level_1 = 2
level_2 = 4
level_3 = 6
level_4 = 8

[run.test_gate]
command = "cargo test"
timeout_secs = 300
max_output_lines = 50
```

CLI flags override TOML values. Agent type can be overridden with `--agent codex`.

## Summary

The `noslop run` command translates ralph's 288-line bash script into a structured Rust orchestrator that:

1. **Uses git as the single source of truth**: No separate state files for progress or failures. Each iteration produces an atomic commit with structured metadata tags. The commit log IS the progress report. No files to get out of sync.
2. **Separates concerns**: Pure domain models, port traits for I/O, core services for logic, CLI for wiring
3. **Is agent-agnostic**: The `RunLoop` calls `AgentRuntime.invoke()` without knowing if it is talking to Claude, Codex, or a future agent
4. **Incorporates research findings**: All quick wins from the research are integrated (git-history-based progress, graduated escalation, diff-based detection, test gates, prompt caching, confidence gating, git-based failure memory) plus the near-term improvements (post-loop review, initializer phase)
5. **Is testable**: Core services are pure functions, all I/O goes through mockable trait objects (`MockVersionControl` verifies `commit_all()` is called with structured messages), unit tests require no filesystem or network
6. **Follows noslop conventions**: Hexagonal architecture, `anyhow::Result`, `mockall` annotations, `Send + Sync` traits, one file per concern
