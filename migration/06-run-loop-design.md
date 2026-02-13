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

- Structured progress file (`.noslop/progress.json`) instead of git-history-only memory
- Failure memory (`.noslop/failures.jsonl`) to prevent repeating failed approaches
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
      progress.rs       # ProgressFile, IterationRecord
      failure.rs        # FailureEntry
      run_config.rs     # RunConfig, StuckPolicy, TestGate
    ports/
      agent.rs          # AgentRuntime, AgentConfig (from 03-agent-port-design)
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

### Progress File (`src/core/models/progress.rs`)

The structured progress file replaces ralph's reliance on git history alone. It is the cross-iteration memory that Anthropic recommends.

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Record of a single iteration's work
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IterationRecord {
    /// 1-based iteration number
    pub iteration: u32,
    /// Timestamp when the iteration started
    pub started_at: DateTime<Utc>,
    /// Timestamp when the iteration ended
    pub ended_at: DateTime<Utc>,
    /// Task the agent worked on (description)
    pub task_attempted: Option<String>,
    /// Whether the agent reported task completion
    pub task_completed: bool,
    /// Confidence score (1-5) if the agent provided one
    pub confidence: Option<u8>,
    /// Files modified during this iteration (from git diff)
    pub files_modified: Vec<String>,
    /// Lines changed (insertions + deletions, excluding plan/progress files)
    pub lines_changed: u32,
    /// Tasks done before this iteration
    pub tasks_before: usize,
    /// Tasks done after this iteration
    pub tasks_after: usize,
    /// Exit code from the agent invocation
    pub agent_exit_code: i32,
    /// Duration of agent invocation in seconds
    pub duration_secs: f64,
    /// Whether external test gate passed after this iteration
    pub tests_passed: Option<bool>,
    /// Summary of what happened (extracted from agent output or generated)
    pub summary: String,
}

/// The structured progress file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressFile {
    /// Plan file path (relative to repo root)
    pub plan_file: String,
    /// Agent type used
    pub agent_type: String,
    /// When the run started
    pub started_at: DateTime<Utc>,
    /// Total iterations executed so far
    pub iterations_completed: u32,
    /// Maximum allowed iterations
    pub max_iterations: u32,
    /// Current stuck count
    pub stuck_count: u32,
    /// Current stuck escalation level
    pub escalation_level: EscalationLevel,
    /// All iteration records
    pub iterations: Vec<IterationRecord>,
    /// Decisions and architectural notes accumulated across iterations
    pub decisions: Vec<String>,
    /// What the next iteration should focus on
    pub next_focus: Option<String>,
}

impl ProgressFile {
    /// Create a new progress file for a fresh run
    pub fn new(plan_file: &str, agent_type: &str, max_iterations: u32) -> Self {
        Self {
            plan_file: plan_file.to_string(),
            agent_type: agent_type.to_string(),
            started_at: Utc::now(),
            iterations_completed: 0,
            max_iterations,
            stuck_count: 0,
            escalation_level: EscalationLevel::None,
            iterations: Vec::new(),
            decisions: Vec::new(),
            next_focus: None,
        }
    }

    /// Push a completed iteration record and update counters
    pub fn record_iteration(&mut self, record: IterationRecord) {
        let made_progress = record.tasks_after > record.tasks_before;
        if made_progress {
            self.stuck_count = 0;
            self.escalation_level = EscalationLevel::None;
        } else {
            self.stuck_count += 1;
            self.escalation_level = EscalationLevel::from_stuck_count(self.stuck_count);
        }
        self.iterations_completed += 1;
        self.iterations.push(record);
    }
}
```

### Failure Memory (`src/core/models/failure.rs`)

Append-only log inspired by the Reflexion framework. Prevents agents from repeating failed approaches.

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A single recorded failure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailureEntry {
    /// Iteration where the failure occurred
    pub iteration: u32,
    /// Task description that was attempted
    pub task: String,
    /// Description of the approach taken
    pub approach: String,
    /// Error message or failure description
    pub error: String,
    /// Lesson learned (what NOT to do again)
    pub lesson: String,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// Collection of failures for a run, backed by a JSONL file
#[derive(Debug, Clone, Default)]
pub struct FailureLog {
    pub entries: Vec<FailureEntry>,
}

impl FailureLog {
    /// Filter failures relevant to a specific task
    #[must_use]
    pub fn for_task(&self, task_description: &str) -> Vec<&FailureEntry> {
        self.entries
            .iter()
            .filter(|e| e.task == task_description)
            .collect()
    }

    /// Format failures as context for the agent prompt
    #[must_use]
    pub fn as_prompt_context(&self, task_description: &str) -> Option<String> {
        let relevant = self.for_task(task_description);
        if relevant.is_empty() {
            return None;
        }
        let mut ctx = String::from("### Previous Failed Approaches for This Task\n\n");
        for entry in &relevant {
            ctx.push_str(&format!(
                "- **Iteration {}**: Tried: {}. Failed: {}. Lesson: {}\n",
                entry.iteration, entry.approach, entry.error, entry.lesson
            ));
        }
        Some(ctx)
    }
}
```

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

## Core Services

### Stuck Detector (`src/core/services/stuck_detector.rs`)

Pure logic for graduated escalation. No I/O.

```rust
use crate::core::models::{
    EscalationLevel, IterationRecord, ProgressFile, StuckPolicy,
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

/// Assess whether the run is stuck after an iteration completes.
///
/// Combines task-count progress (did done_count increase?) with
/// diff-based progress detection (did meaningful code change?).
pub fn assess_stuck(
    progress: &ProgressFile,
    latest: &IterationRecord,
    policy: &StuckPolicy,
) -> StuckAssessment {
    let made_task_progress = latest.tasks_after > latest.tasks_before;

    // Diff-based: meaningful code changes = lines_changed > 0 on non-plan files
    let has_code_churn = latest.lines_changed > 0 && !made_task_progress;

    let stuck_count = if made_task_progress {
        0
    } else {
        progress.stuck_count + 1
    };

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
             It is unacceptable to retry the same approach. Read .noslop/failures.jsonl \
             to see what has already been tried.\
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
             This is your last iteration. Write a diagnostic report to \
             .noslop/stuck-report.md covering:\n\
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
    EscalationLevel, FailureLog, Plan, ProgressFile, RunConfig, Task,
};

/// Context for building a single iteration's prompt
pub struct PromptContext<'a> {
    /// Current iteration number (1-based)
    pub iteration: u32,
    /// Maximum iterations
    pub max_iterations: u32,
    /// The parsed plan
    pub plan: &'a Plan,
    /// The progress file (if exists)
    pub progress: Option<&'a ProgressFile>,
    /// The failure log (if exists)
    pub failure_log: Option<&'a FailureLog>,
    /// Recent git activity (formatted string)
    pub git_activity: String,
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
/// 3. Failure context for current task (semi-stable)
/// 4. Escalation prompt (changes only when stuck)
/// 5. Dynamic context: iteration number, plan status, git activity, test failures
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

    // Failure context for the current task
    if let (Some(failure_log), Some(task)) =
        (&ctx.failure_log, ctx.plan.next_incomplete())
    {
        if let Some(failure_ctx) = failure_log.as_prompt_context(&task.description) {
            prompt.push_str("\n\n");
            prompt.push_str(&failure_ctx);
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

    // Progress file summary
    if let Some(progress) = ctx.progress {
        if !progress.iterations.is_empty() {
            prompt.push_str("### Recent Iteration History\n\n");
            for record in progress.iterations.iter().rev().take(5) {
                prompt.push_str(&format!(
                    "- Iteration {}: {} ({})\n",
                    record.iteration,
                    record.summary,
                    if record.task_completed {
                        "completed"
                    } else {
                        "no completion"
                    }
                ));
            }
            prompt.push('\n');
        }
        if let Some(focus) = &progress.next_focus {
            prompt.push_str(&format!("### Suggested Focus\n{focus}\n\n"));
        }
    }

    prompt.push_str("### Recent Git Activity\n");
    prompt.push_str(&ctx.git_activity);
    prompt.push('\n');

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
2. Check the iteration context below. Review what previous iterations committed.
   If the same task was attempted before, read .noslop/failures.jsonl to understand
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

## Step 6: Update progress

Update .noslop/progress.json with:
- What you attempted this iteration
- What succeeded or failed, and why
- Key decisions made
- What the next iteration should focus on

If your implementation attempt FAILED, append to .noslop/failures.jsonl:
{"iteration": N, "task": "...", "approach": "...", "error": "...", "lesson": "..."}

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
4. Create .noslop/progress.json documenting:
   - Repository structure overview
   - Test framework and how to run tests
   - Build system and dependencies
   - Key architectural patterns observed
5. Then proceed to the first incomplete task following the standard workflow below.

## Standard Workflow

Find the first incomplete task and complete it. One task per iteration, no more.

### Orient yourself

1. Read the plan file completely.
2. Check the iteration context below.
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

### Update progress

Update .noslop/progress.json with what you learned and did.

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
    EscalationLevel, FailureEntry, FailureLog, IterationRecord,
    Plan, ProgressFile, RunConfig,
};
use crate::core::ports::{AgentRuntime, InvocationConfig, PlanStore, TestRunner};
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
        diagnostic_report: Option<String>,
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
/// Coordinates plan parsing, agent invocation, progress tracking,
/// stuck detection, test gates, and post-loop review. All I/O is
/// delegated to the injected trait objects.
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
    pub fn execute(&self) -> anyhow::Result<RunOutcome> {
        // 1. Load initial state
        let mut plan = self.plan_store.load(&self.config.plan_file)?;
        let mut progress = self.load_or_create_progress(&plan)?;
        let mut failure_log = self.load_failure_log()?;
        let mut last_test_failures: Option<String> = None;

        // 2. Main iteration loop
        for iteration in 1..=self.config.max_iterations {
            let started_at = chrono::Utc::now();

            // 2a. Reload plan (agent may have modified it)
            plan = self.plan_store.load(&self.config.plan_file)?;
            let tasks_before = plan.done_count();

            // 2b. Build prompt context
            let git_activity = self.vcs.recent_commits(5)?;
            let escalation_prompt = progress
                .escalation_level
                .then(|| {
                    stuck_detector::assess_stuck(
                        &progress,
                        progress.iterations.last().unwrap(),
                        &self.config.stuck_policy,
                    )
                    .prompt_injection
                })
                .flatten();

            let ctx = prompt_builder::PromptContext {
                iteration,
                max_iterations: self.config.max_iterations,
                plan: &plan,
                progress: Some(&progress),
                failure_log: Some(&failure_log),
                git_activity,
                test_failures: last_test_failures.take(),
                escalation_prompt,
                confidence_threshold: self.config.confidence_threshold,
                is_initializer: iteration == 1 && self.config.initializer_phase,
                plan_file_path: self.config.plan_file.display().to_string(),
            };

            let prompt = prompt_builder::build_iteration_prompt(&ctx);

            // 2c. Invoke the agent
            let invocation_config = InvocationConfig {
                prompt,
                working_dir: self.config.working_dir.clone(),
                bypass_permissions: true,
                ..Default::default()
            };

            let result = self.runtime.invoke(&invocation_config)?;
            let output = self.runtime.parse_output(&result);
            let ended_at = chrono::Utc::now();

            // 2d. Log output
            self.append_to_log(iteration, &output.raw_output)?;

            // 2e. Safety checkpoint
            self.vcs.checkpoint(&format!(
                "ralph: iteration {} checkpoint",
                iteration
            ))?;

            // 2f. Check for plan completion signal
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

            // 2g. Check for stuck diagnostic signal
            if output.raw_output.contains("<promise>STUCK</promise>") {
                let diagnostic = self.read_diagnostic_report();
                return Ok(RunOutcome::Stuck {
                    iterations_used: iteration,
                    diagnostic_report: diagnostic,
                });
            }

            // 2h. Post-iteration test gate
            let tests_passed = if let Some(test_runner) = self.test_runner {
                let test_result = test_runner.run_tests(&self.config.working_dir)?;
                if !test_result.passed {
                    last_test_failures = Some(test_result.output.clone());
                }
                Some(test_result.passed)
            } else {
                None
            };

            // 2i. Measure progress (task-count + diff-based)
            plan = self.plan_store.load(&self.config.plan_file)?;
            let tasks_after = plan.done_count();
            let diff_stats = self.vcs.diff_stat_since_last_checkpoint()?;

            // 2j. Record iteration
            let record = IterationRecord {
                iteration,
                started_at,
                ended_at,
                task_attempted: output.completed_task.clone(),
                task_completed: tasks_after > tasks_before,
                confidence: parse_confidence(&output.raw_output),
                files_modified: diff_stats.files,
                lines_changed: diff_stats.lines_changed,
                tasks_before,
                tasks_after,
                agent_exit_code: result.exit_code,
                duration_secs: result.duration.as_secs_f64(),
                tests_passed,
                summary: output
                    .completed_task
                    .clone()
                    .unwrap_or_else(|| "no task completed".to_string()),
            };

            // 2k. Stuck assessment
            let assessment = stuck_detector::assess_stuck(
                &progress,
                &record,
                &self.config.stuck_policy,
            );

            progress.record_iteration(record);
            self.save_progress(&progress)?;

            if assessment.should_terminate {
                return Ok(RunOutcome::Stuck {
                    iterations_used: iteration,
                    diagnostic_report: self.read_diagnostic_report(),
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

    // --- Private helper methods (each delegates to a port trait) ---

    fn load_or_create_progress(&self, plan: &Plan) -> anyhow::Result<ProgressFile> {
        // Load from .noslop/progress.json or create new
        todo!("delegate to filesystem via VersionControl or a dedicated port")
    }

    fn save_progress(&self, progress: &ProgressFile) -> anyhow::Result<()> {
        todo!("serialize to .noslop/progress.json")
    }

    fn load_failure_log(&self) -> anyhow::Result<FailureLog> {
        // Load from .noslop/failures.jsonl (one JSON object per line)
        todo!("delegate to filesystem")
    }

    fn append_to_log(&self, iteration: u32, output: &str) -> anyhow::Result<()> {
        todo!("append to .ralph-log")
    }

    fn read_diagnostic_report(&self) -> Option<String> {
        todo!("read .noslop/stuck-report.md if it exists")
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
            diagnostic_report,
        } => {
            println!(
                "=== Stuck after {} iteration(s). ===",
                iterations_used
            );
            if let Some(report) = diagnostic_report {
                println!("\nDiagnostic report:\n{}", report);
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
  progress.json          # NEW: structured cross-iteration memory
  failures.jsonl         # NEW: append-only failure log (one JSON per line)
  stuck-report.md        # NEW: generated when escalation reaches level 4
  review.md              # NEW: post-loop review findings
```

### `progress.json` Example

```json
{
  "plan_file": "plan.md",
  "agent_type": "claude",
  "started_at": "2026-02-13T10:30:00Z",
  "iterations_completed": 3,
  "max_iterations": 20,
  "stuck_count": 0,
  "escalation_level": "None",
  "iterations": [
    {
      "iteration": 1,
      "started_at": "2026-02-13T10:30:00Z",
      "ended_at": "2026-02-13T10:35:22Z",
      "task_attempted": "Set up project structure with Cargo.toml",
      "task_completed": true,
      "confidence": 5,
      "files_modified": ["Cargo.toml", "src/main.rs", "src/lib.rs"],
      "lines_changed": 45,
      "tasks_before": 0,
      "tasks_after": 1,
      "agent_exit_code": 0,
      "duration_secs": 322.0,
      "tests_passed": true,
      "summary": "Set up project structure with Cargo.toml"
    }
  ],
  "decisions": ["Using workspace layout with separate crates for core and cli"],
  "next_focus": "Implement the parser module (task 2)"
}
```

### `failures.jsonl` Example

```jsonl
{"iteration":4,"task":"Implement rate limiter","approach":"Used token bucket with mutex","error":"Deadlock under concurrent access in test_parallel_requests","lesson":"Mutex-based approach deadlocks when handler holds lock across await point. Use atomic operations or actor pattern instead.","timestamp":"2026-02-13T11:02:00Z"}
{"iteration":5,"task":"Implement rate limiter","approach":"Switched to atomic counter with CAS loop","error":"Race condition: counter resets between check and decrement","lesson":"Atomic CAS loop needs to include both check and decrement in a single compare_exchange. Cannot split into separate load + store.","timestamp":"2026-02-13T11:15:00Z"}
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
         +--------------+--------------+
         |              |              |
   AgentRuntime    PlanStore     TestRunner
   (port trait)    (port trait)  (port trait)
         |              |              |
         v              v              v
  +------+------+ +----+----+ +-------+-------+
  |ClaudeRuntime| |Markdown | |CargoTestRunner|
  |CodexRuntime | |JsonPlan | |NpmTestRunner  |
  +------+------+ +----+----+ +-------+-------+
         |              |              |
         +--------------+--------------+
                        |
                        v
              +---------+---------+
              |    RunLoop        |
              |  (core service)   |
              |                   |
              |  For each iter:   |
              |  1. Load plan     |  <--- PlanStore.load()
              |  2. Build prompt  |  <--- prompt_builder (pure)
              |  3. Invoke agent  |  <--- AgentRuntime.invoke()
              |  4. Log output    |
              |  5. Checkpoint    |  <--- VersionControl.checkpoint()
              |  6. Check done    |  <--- AgentRuntime.parse_output()
              |  7. Run tests     |  <--- TestRunner.run_tests()
              |  8. Assess stuck  |  <--- stuck_detector (pure)
              |  9. Record iter   |
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

| Quick Win                           | Where It Appears                                                                                            | Reference                       |
| ----------------------------------- | ----------------------------------------------------------------------------------------------------------- | ------------------------------- |
| **Structured progress file**        | `ProgressFile` model, `.noslop/progress.json`, prompt builder reads it each iteration                       | Research finding 1              |
| **Graduated escalation (4 levels)** | `EscalationLevel` enum, `StuckPolicy` config, `stuck_detector::assess_stuck()`, `build_escalation_prompt()` | Research finding 2              |
| **Diff-based progress detection**   | `IterationRecord.lines_changed`, `vcs.diff_stat_since_last_checkpoint()`, `StuckAssessment.has_code_churn`  | Research finding 3              |
| **Post-iteration test gates**       | `TestRunner` port trait, `RunLoop` step 2h, `last_test_failures` fed into next prompt                       | Research finding 4              |
| **Prompt restructure for caching**  | `build_iteration_prompt()` puts static template first, dynamic context last                                 | Research finding 5              |
| **Confidence-gated completion**     | `confidence_threshold` in `RunConfig`, `parse_confidence()`, prompt instructs `[CONFIDENCE: N]`             | Research finding 6              |
| **Failure memory**                  | `FailureEntry`, `FailureLog`, `.noslop/failures.jsonl`, injected into prompt via `as_prompt_context()`      | Research finding 9              |
| **Initializer phase**               | `is_initializer` flag, `initializer_template()` for iteration 1, two-agent harness pattern                  | Research finding 10             |
| **Post-loop review**                | `run_post_loop_review()`, BugBot-inspired review pass after completion                                      | Research finding 8              |
| **Strongly-worded constraints**     | Escalation prompts use "unacceptable", "MUST", absolute prohibitions                                        | Research finding from Anthropic |

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
    fn confidence_parsing() {
        assert_eq!(parse_confidence("some output [CONFIDENCE: 4] more"), Some(4));
        assert_eq!(parse_confidence("no confidence here"), None);
        assert_eq!(parse_confidence("[CONFIDENCE: 6]"), None); // out of range
    }

    #[test]
    fn failure_log_filters_by_task() {
        let log = FailureLog {
            entries: vec![
                FailureEntry {
                    iteration: 1,
                    task: "task A".to_string(),
                    approach: "approach 1".to_string(),
                    error: "err".to_string(),
                    lesson: "lesson".to_string(),
                    timestamp: Utc::now(),
                },
                FailureEntry {
                    iteration: 2,
                    task: "task B".to_string(),
                    approach: "approach 2".to_string(),
                    error: "err".to_string(),
                    lesson: "lesson".to_string(),
                    timestamp: Utc::now(),
                },
            ],
        };
        assert_eq!(log.for_task("task A").len(), 1);
        assert_eq!(log.for_task("task C").len(), 0);
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

1. **Separates concerns**: Pure domain models, port traits for I/O, core services for logic, CLI for wiring
2. **Is agent-agnostic**: The `RunLoop` calls `AgentRuntime.invoke()` without knowing if it is talking to Claude, Codex, or a future agent
3. **Incorporates research findings**: All 7 quick wins from the research are integrated into the design (structured progress, graduated escalation, diff-based detection, test gates, prompt caching, confidence gating, failure memory) plus the near-term improvements (post-loop review, initializer phase)
4. **Is testable**: Core services are pure functions, all I/O goes through mockable trait objects, unit tests require no filesystem or network
5. **Follows noslop conventions**: Hexagonal architecture, `anyhow::Result`, `mockall` annotations, `Send + Sync` traits, one file per concern
