# Test Strategy

Comprehensive test plan for all new modules introduced by the setup migration. Every test follows noslop's existing patterns exactly: inline `#[cfg(test)]` unit tests in service modules, `mockall::automock`-annotated traits, `test-case` for parameterized tests, `proptest` for property-based tests, `TempGitRepo`/`tempfile::TempDir` for git operations, `assert_cmd` for CLI integration tests, and builder fixtures in `tests/common/`.

## Test Infrastructure Extensions

### New Fixtures (`tests/common/fixtures.rs`)

Add builders alongside the existing `CheckBuilder` and `AckBuilder`:

```rust
/// Builder for creating test Plan instances
pub struct PlanBuilder {
    format: PlanFormat,
    path: PathBuf,
    tasks: Vec<Task>,
    raw_content: String,
}

impl PlanBuilder {
    pub fn markdown() -> Self { /* default 3 tasks, 1 complete */ }
    pub fn json() -> Self { /* default 2 tasks, 0 complete */ }
    pub fn all_complete(mut self) -> Self { /* mark all tasks complete */ }
    pub fn add_task(mut self, desc: &str, status: TaskStatus) -> Self { /* append task */ }
    pub fn build(self) -> Plan { /* construct Plan */ }
}

/// Builder for creating test IterationRecord instances
pub struct IterationRecordBuilder {
    iteration: u32,
    tasks_before: usize,
    tasks_after: usize,
    lines_changed: u32,
    task_completed: bool,
}

impl IterationRecordBuilder {
    pub fn new(iteration: u32) -> Self { /* defaults: no progress */ }
    pub fn with_progress(mut self, before: usize, after: usize) -> Self { /* set counts */ }
    pub fn with_lines_changed(mut self, n: u32) -> Self { /* set churn */ }
    pub fn build(self) -> IterationRecord { /* construct record */ }
}

/// Builder for creating test ProgressFile instances
pub struct ProgressFileBuilder { /* ... */ }

/// Builder for creating test FailureLog instances
pub struct FailureLogBuilder { /* ... */ }

/// Builder for creating test RunConfig instances
pub struct RunConfigBuilder { /* ... */ }

/// Builder for creating test InvocationResult instances
pub struct InvocationResultBuilder { /* ... */ }
```

### Extended `TempGitRepo` (`tests/common/git_repo.rs`)

Add methods to support checkpoint and worktree testing:

```rust
impl TempGitRepo {
    // --- existing methods: new(), path(), write_file(), stage(), commit(), git() ---

    /// Stage all files and create initial commit (required before worktree ops)
    pub fn init_with_commit(&self) {
        self.write_file("README.md", "# test");
        self.stage("README.md");
        self.commit("Initial commit");
    }

    /// Write a file and stage it in one call
    pub fn write_and_stage(&self, name: &str, content: &str) {
        self.write_file(name, content);
        self.stage(name);
    }

    /// Get the HEAD short SHA
    pub fn head_sha(&self) -> String { /* git rev-parse --short HEAD */ }

    /// Check if working tree is clean
    pub fn is_clean(&self) -> bool { /* git status --porcelain is empty */ }

    /// Count commits on current branch
    pub fn commit_count(&self) -> usize { /* git rev-list --count HEAD */ }

    /// Create a branch from current HEAD
    pub fn create_branch(&self, name: &str) { /* git branch <name> */ }

    /// Get the worktree base dir path (for assertions)
    pub fn worktree_base_dir(&self) -> PathBuf {
        let name = self.path().file_name().unwrap().to_string_lossy();
        self.path().parent().unwrap().join(format!("{name}-worktrees"))
    }
}
```

### New Mock Implementations (`tests/common/mocks.rs`)

Extend the existing manual mocks with new port trait implementations:

```rust
/// Mock implementation of PlanStore
pub struct MockPlanStoreManual {
    plan: RefCell<Plan>,
}

impl MockPlanStoreManual {
    pub fn with_plan(plan: Plan) -> Self { /* ... */ }
}

impl PlanStore for MockPlanStoreManual {
    fn load(&self, _path: &Path) -> anyhow::Result<Plan> { /* return clone */ }
    fn save(&self, plan: &Plan) -> anyhow::Result<()> { /* update internal */ }
    fn update_task_status(&self, _path: &Path, idx: usize, status: TaskStatus) -> anyhow::Result<()> { /* ... */ }
}

/// Mock implementation of TestRunner
pub struct MockTestRunnerManual {
    result: TestResult,
}

impl MockTestRunnerManual {
    pub fn passing() -> Self { /* result.passed = true */ }
    pub fn failing(output: &str) -> Self { /* result.passed = false, output = provided */ }
}

impl TestRunner for MockTestRunnerManual {
    fn run_tests(&self, _working_dir: &Path) -> anyhow::Result<TestResult> { /* return clone */ }
    fn detect(_working_dir: &Path) -> Option<Box<dyn TestRunner>> { None }
}
```

These manual mocks complement the `mockall::automock`-generated mocks (which are used when fine-grained expectation control is needed).

---

## Module 1: Agent Port Traits (`src/core/ports/agent.rs`)

### Unit Tests (inline `#[cfg(test)]`)

Located in `src/core/ports/agent.rs` under `#[cfg(test)] mod tests`.

| Test Function                               | What It Asserts                                                                                              |
| ------------------------------------------- | ------------------------------------------------------------------------------------------------------------ |
| `test_agent_type_from_str_claude`           | `"claude".parse::<AgentType>()` returns `Ok(AgentType::Claude)`                                              |
| `test_agent_type_from_str_claude_code`      | `"claude-code".parse::<AgentType>()` returns `Ok(AgentType::Claude)`                                         |
| `test_agent_type_from_str_codex`            | `"codex".parse::<AgentType>()` returns `Ok(AgentType::Codex)`                                                |
| `test_agent_type_from_str_openai`           | `"openai".parse::<AgentType>()` returns `Ok(AgentType::Codex)`                                               |
| `test_agent_type_from_str_case_insensitive` | `"CLAUDE".parse::<AgentType>()` returns `Ok(AgentType::Claude)`                                              |
| `test_agent_type_from_str_unknown_errors`   | `"gpt".parse::<AgentType>()` returns `Err` containing "Unknown agent type"                                   |
| `test_agent_type_display_claude`            | `AgentType::Claude.to_string()` is `"claude"`                                                                |
| `test_agent_type_display_codex`             | `AgentType::Codex.to_string()` is `"codex"`                                                                  |
| `test_agent_type_command_name`              | `AgentType::Claude.command_name()` is `"claude"`, `AgentType::Codex.command_name()` is `"codex"`             |
| `test_agent_type_display_name`              | `AgentType::Claude.display_name()` is `"Claude Code"`, `AgentType::Codex.display_name()` is `"Codex CLI"`    |
| `test_invocation_config_default`            | `InvocationConfig::default()` has empty prompt, `"."` working_dir, `None` timeout, `true` bypass_permissions |
| `test_agent_config_bundle_default`          | `AgentConfigBundle::default()` has `None` global_instructions, `None` settings, empty hooks and commands     |

### Parameterized Tests (`tests/unit/agent_type_test.rs`)

Uses `test-case` for exhaustive `FromStr` coverage:

```rust
use noslop::core::ports::AgentType;
use test_case::test_case;

#[test_case("claude", AgentType::Claude ; "lowercase claude")]
#[test_case("Claude", AgentType::Claude ; "titlecase claude")]
#[test_case("CLAUDE", AgentType::Claude ; "uppercase claude")]
#[test_case("claude-code", AgentType::Claude ; "hyphenated claude")]
#[test_case("codex", AgentType::Codex ; "lowercase codex")]
#[test_case("Codex", AgentType::Codex ; "titlecase codex")]
#[test_case("openai", AgentType::Codex ; "openai alias")]
fn test_agent_type_parsing(input: &str, expected: AgentType) {
    let parsed: AgentType = input.parse().unwrap();
    assert_eq!(parsed, expected);
}

#[test_case("gpt" ; "gpt is unknown")]
#[test_case("cursor" ; "cursor is unknown")]
#[test_case("" ; "empty string")]
#[test_case("aider" ; "aider is unknown")]
fn test_agent_type_parsing_errors(input: &str) {
    let result: Result<AgentType, _> = input.parse();
    assert!(result.is_err());
}
```

### Unit Tests for Mock Verification

Verify that `MockAgentConfig` and `MockAgentRuntime` (auto-generated by `mockall`) work correctly:

| Test Function                                   | What It Asserts                                                                                                            |
| ----------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------- |
| `test_mock_agent_config_generate`               | `MockAgentConfig` with `expect_generate_config` returning a bundle with one hook produces the expected `AgentConfigBundle` |
| `test_mock_agent_config_is_available`           | `MockAgentConfig` with `expect_is_available().return_const(true)` returns `true`                                           |
| `test_mock_agent_runtime_invoke`                | `MockAgentRuntime` with `expect_invoke` returning a success `InvocationResult` produces expected output                    |
| `test_mock_agent_runtime_parse_output_complete` | `MockAgentRuntime` with `expect_parse_output` returning `plan_complete: true` signals completion                           |

---

## Module 2: Plan Parser (`src/core/services/plan_parser.rs`)

### Unit Tests (inline `#[cfg(test)]`)

All pure function tests -- no mocks, no I/O.

| Test Function                                  | What It Asserts                                                                                                                               |
| ---------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------- |
| `test_parse_markdown_basic`                    | `"- [ ] Task 1\n- [x] Task 2\n- [ ] Task 3\n"` parses to 3 tasks, format `Markdown`, `done_count() == 1`, `total_count() == 3`                |
| `test_parse_markdown_with_headers`             | `"# Plan\n\nSome text\n\n- [ ] Task 1\n\nMore text\n\n- [x] Task 2\n"` correctly extracts 2 tasks, ignoring non-checkbox lines                |
| `test_parse_markdown_uppercase_x`              | `"- [X] Task 1\n"` parses `TaskStatus::Complete` (uppercase X accepted)                                                                       |
| `test_parse_markdown_indented`                 | `"  - [ ] Indented task\n"` parses as 1 incomplete task (leading whitespace handled)                                                          |
| `test_parse_markdown_preserves_description`    | Task description is `"Implement the parser module"`, not `"- [ ] Implement the parser module"` (checkbox prefix stripped)                     |
| `test_parse_markdown_empty_description`        | `"- [ ] \n"` parses as a task with empty description (whitespace trimmed)                                                                     |
| `test_parse_json_array`                        | `[{"description": "Setup", "passes": true}, {"description": "Build", "passes": false}]` parses to 2 tasks, format `Json`, `done_count() == 1` |
| `test_parse_json_nested`                       | `{"tasks": [{"description": "A", "passes": false}]}` recursively finds the task inside nested object                                          |
| `test_parse_json_with_steps`                   | Task with `"steps": ["write code", "run tests"]` has `acceptance_criteria.len() == 2`                                                         |
| `test_parse_json_unnamed_task`                 | `{"passes": false}` without `"description"` gets description `"(unnamed task)"`                                                               |
| `test_parse_empty_file_errors`                 | Empty string `""` returns `Err` with message containing "No tasks found"                                                                      |
| `test_parse_no_tasks_errors`                   | `"# Just a header\n\nSome text but no checkboxes\n"` returns `Err` with message containing "No tasks found"                                   |
| `test_parse_format_detection_prefers_markdown` | Content containing both `- [ ]` checkboxes and `"passes"` JSON returns `PlanFormat::Markdown` (markdown takes priority)                       |
| `test_plan_is_complete_all_done`               | Plan with all `Complete` tasks returns `is_complete() == true`                                                                                |
| `test_plan_is_complete_with_skipped`           | Plan with 1 `Complete` and 1 `Skipped` returns `is_complete() == true` (skipped are excluded)                                                 |
| `test_plan_is_complete_with_incomplete`        | Plan with 1 `Complete` and 1 `Incomplete` returns `is_complete() == false`                                                                    |
| `test_plan_next_incomplete`                    | Plan with tasks `[Complete, Incomplete, Incomplete]` returns second task from `next_incomplete()`                                             |
| `test_plan_next_incomplete_none`               | All-complete plan returns `None` from `next_incomplete()`                                                                                     |
| `test_plan_total_count_excludes_skipped`       | Plan with 3 tasks (1 complete, 1 incomplete, 1 skipped) returns `total_count() == 2`                                                          |

### Parameterized Tests (`tests/unit/plan_parser_test.rs`)

```rust
use noslop::core::services::plan_parser::parse_plan;
use noslop::core::models::PlanFormat;
use std::path::Path;
use test_case::test_case;

#[test_case(
    "- [ ] A\n- [x] B\n",
    PlanFormat::Markdown, 2, 1
    ; "two markdown tasks one done"
)]
#[test_case(
    "- [ ] A\n- [ ] B\n- [ ] C\n",
    PlanFormat::Markdown, 3, 0
    ; "three markdown tasks none done"
)]
#[test_case(
    "- [x] A\n- [x] B\n",
    PlanFormat::Markdown, 2, 2
    ; "two markdown tasks all done"
)]
#[test_case(
    r#"[{"description":"A","passes":true},{"description":"B","passes":false}]"#,
    PlanFormat::Json, 2, 1
    ; "json array two tasks one done"
)]
fn test_plan_parsing(content: &str, expected_format: PlanFormat, total: usize, done: usize) {
    let plan = parse_plan(Path::new("plan.md"), content).unwrap();
    assert_eq!(plan.format, expected_format);
    assert_eq!(plan.tasks.len(), total);
    assert_eq!(plan.done_count(), done);
}
```

### Property-Based Tests (`tests/unit/proptest_plan_parser.rs`)

```rust
use noslop::core::services::plan_parser::parse_plan;
use noslop::core::models::TaskStatus;
use proptest::prelude::*;
use std::path::Path;

proptest! {
    /// Any sequence of markdown checkboxes should parse to the correct count
    #[test]
    fn markdown_task_count_matches_checkboxes(
        complete in 0u32..10,
        incomplete in 0u32..10,
    ) {
        prop_assume!(complete + incomplete > 0);
        let mut content = String::from("# Plan\n\n");
        for i in 0..complete {
            content.push_str(&format!("- [x] Complete task {i}\n"));
        }
        for i in 0..incomplete {
            content.push_str(&format!("- [ ] Incomplete task {i}\n"));
        }
        let plan = parse_plan(Path::new("plan.md"), &content).unwrap();
        prop_assert_eq!(plan.tasks.len(), (complete + incomplete) as usize);
        prop_assert_eq!(plan.done_count(), complete as usize);
    }

    /// Round-trip: descriptions survive parsing
    #[test]
    fn markdown_descriptions_preserved(desc in "[A-Za-z ]{3,50}") {
        let content = format!("- [ ] {desc}\n");
        let plan = parse_plan(Path::new("plan.md"), &content).unwrap();
        prop_assert_eq!(&plan.tasks[0].description, desc.trim());
    }
}
```

---

## Module 3: Stuck Detector (`src/core/services/stuck_detector.rs`)

### Unit Tests (inline `#[cfg(test)]`)

All pure logic tests -- no mocks, no I/O.

| Test Function                                         | What It Asserts                                                                                                                                                                                    |
| ----------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `test_escalation_level_none_at_zero`                  | `EscalationLevel::from_stuck_count_with_policy(0, &default)` is `None`                                                                                                                             |
| `test_escalation_level_none_at_one`                   | `EscalationLevel::from_stuck_count_with_policy(1, &default)` is `None`                                                                                                                             |
| `test_escalation_level_suggest_at_two`                | `from_stuck_count_with_policy(2, &default)` is `Suggest`                                                                                                                                           |
| `test_escalation_level_suggest_at_three`              | `from_stuck_count_with_policy(3, &default)` is `Suggest`                                                                                                                                           |
| `test_escalation_level_analyze_at_four`               | `from_stuck_count_with_policy(4, &default)` is `AnalyzeRootCause`                                                                                                                                  |
| `test_escalation_level_force_at_six`                  | `from_stuck_count_with_policy(6, &default)` is `ForceStrategyChange`                                                                                                                               |
| `test_escalation_level_diagnostic_at_eight`           | `from_stuck_count_with_policy(8, &default)` is `DiagnosticExit`                                                                                                                                    |
| `test_escalation_level_diagnostic_above_eight`        | `from_stuck_count_with_policy(15, &default)` is still `DiagnosticExit`                                                                                                                             |
| `test_escalation_level_custom_policy`                 | Custom `StuckPolicy { level_1: 3, level_2: 5, level_3: 7, level_4: 10 }`: `from_stuck_count_with_policy(3, &custom)` is `Suggest`, `from_stuck_count_with_policy(10, &custom)` is `DiagnosticExit` |
| `test_assess_stuck_progress_resets_count`             | `assess_stuck` with `tasks_after > tasks_before` returns `stuck_count == 0`, `level == None`                                                                                                       |
| `test_assess_stuck_no_progress_increments`            | `assess_stuck` with `tasks_after == tasks_before` and prior `stuck_count == 1` returns `stuck_count == 2`, `level == Suggest`                                                                      |
| `test_assess_stuck_code_churn_detected`               | `assess_stuck` with `lines_changed > 0` but `tasks_after == tasks_before` returns `has_code_churn == true`                                                                                         |
| `test_assess_stuck_no_churn_no_progress`              | `assess_stuck` with `lines_changed == 0` and `tasks_after == tasks_before` returns `has_code_churn == false`                                                                                       |
| `test_assess_stuck_should_terminate_at_level4`        | `assess_stuck` with stuck_count reaching level_4 returns `should_terminate == true`                                                                                                                |
| `test_assess_stuck_should_not_terminate_below_level4` | `assess_stuck` with stuck_count at level_3 returns `should_terminate == false`                                                                                                                     |
| `test_build_escalation_prompt_none`                   | Level `None` returns `prompt_injection == None`                                                                                                                                                    |
| `test_build_escalation_prompt_suggest`                | Level `Suggest` returns prompt containing "DIFFERENT approach"                                                                                                                                     |
| `test_build_escalation_prompt_analyze`                | Level `AnalyzeRootCause` returns prompt containing "Root Cause Analysis Required" and "3 possible root causes"                                                                                     |
| `test_build_escalation_prompt_force`                  | Level `ForceStrategyChange` returns prompt containing "Must Change Strategy" and "Skip", "Decompose", "Simplify"                                                                                   |
| `test_build_escalation_prompt_diagnostic`             | Level `DiagnosticExit` returns prompt containing "FINAL ATTEMPT" and "stuck-report.md"                                                                                                             |
| `test_build_escalation_prompt_with_churn`             | Any non-None level with `has_code_churn == true` includes "churn without progress" warning                                                                                                         |
| `test_build_escalation_prompt_without_churn`          | Non-None level with `has_code_churn == false` does NOT contain "churn without progress"                                                                                                            |

### Parameterized Tests (`tests/unit/stuck_detector_test.rs`)

```rust
use noslop::core::models::{EscalationLevel, StuckPolicy};
use test_case::test_case;

#[test_case(0, EscalationLevel::None ; "zero stuck")]
#[test_case(1, EscalationLevel::None ; "one stuck")]
#[test_case(2, EscalationLevel::Suggest ; "two stuck")]
#[test_case(3, EscalationLevel::Suggest ; "three stuck")]
#[test_case(4, EscalationLevel::AnalyzeRootCause ; "four stuck")]
#[test_case(5, EscalationLevel::AnalyzeRootCause ; "five stuck")]
#[test_case(6, EscalationLevel::ForceStrategyChange ; "six stuck")]
#[test_case(7, EscalationLevel::ForceStrategyChange ; "seven stuck")]
#[test_case(8, EscalationLevel::DiagnosticExit ; "eight stuck")]
#[test_case(100, EscalationLevel::DiagnosticExit ; "hundred stuck")]
fn test_escalation_levels(stuck_count: u32, expected: EscalationLevel) {
    let policy = StuckPolicy::default();
    assert_eq!(
        EscalationLevel::from_stuck_count_with_policy(stuck_count, &policy),
        expected
    );
}
```

---

## Module 4: Prompt Builder (`src/core/services/prompt_builder.rs`)

### Unit Tests (inline `#[cfg(test)]`)

Pure string-construction tests. Uses `PlanBuilder` fixture.

| Test Function                                 | What It Asserts                                                                                                                                              |
| --------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `test_build_prompt_contains_plan_path`        | Output contains `"Plan file: /path/to/plan.md"`                                                                                                              |
| `test_build_prompt_contains_iteration_number` | Output contains `"This is iteration 3 of 20."`                                                                                                               |
| `test_build_prompt_contains_task_counts`      | Output contains `"Tasks: 2 / 5 complete"`                                                                                                                    |
| `test_build_prompt_initializer_template`      | When `is_initializer == true`, output contains `"Initialization Phase"` and `"Explore the repository structure"`                                             |
| `test_build_prompt_standard_template`         | When `is_initializer == false`, output contains `"Find the first incomplete task"` and does NOT contain `"Initialization Phase"`                             |
| `test_build_prompt_includes_escalation`       | When `escalation_prompt == Some("## Stuck...")`, output contains that string verbatim                                                                        |
| `test_build_prompt_no_escalation`             | When `escalation_prompt == None`, output does NOT contain `"## Stuck"` or `"## Attention"`                                                                   |
| `test_build_prompt_includes_failure_context`  | When `failure_log` has entries for the current task, output contains `"Previous Failed Approaches"`                                                          |
| `test_build_prompt_no_failure_context`        | When `failure_log` has no entries for the current task, output does NOT contain `"Previous Failed Approaches"`                                               |
| `test_build_prompt_includes_test_failures`    | When `test_failures == Some("FAILED tests::my_test")`, output contains the test output inside a code block and `"Fix these failures BEFORE proceeding"`      |
| `test_build_prompt_no_test_failures`          | When `test_failures == None`, output does NOT contain `"Test Failures From Previous Iteration"`                                                              |
| `test_build_prompt_confidence_gate`           | When `confidence_threshold == Some(4)`, output contains `"[CONFIDENCE: N]"` and `"below 4"`                                                                  |
| `test_build_prompt_no_confidence_gate`        | When `confidence_threshold == None`, output does NOT contain `"CONFIDENCE"`                                                                                  |
| `test_build_prompt_includes_progress_history` | When `progress` has 3 iteration records, output contains `"Recent Iteration History"` with summaries                                                         |
| `test_build_prompt_includes_git_activity`     | When `git_activity` is non-empty, output contains `"Recent Git Activity"` followed by the activity string                                                    |
| `test_build_prompt_static_before_dynamic`     | The static template text (e.g., `"Read the plan file"`) appears before the dynamic context (e.g., `"This is iteration"`) -- verifies prompt caching ordering |
| `test_build_prompt_includes_next_focus`       | When `progress.next_focus == Some("Implement parser")`, output contains `"Suggested Focus"` and `"Implement parser"`                                         |

### `parse_confidence` Tests (inline `#[cfg(test)]`)

| Test Function                                | What It Asserts                                                              |
| -------------------------------------------- | ---------------------------------------------------------------------------- |
| `test_parse_confidence_valid`                | `parse_confidence("output [CONFIDENCE: 4] more")` returns `Some(4)`          |
| `test_parse_confidence_missing`              | `parse_confidence("no confidence here")` returns `None`                      |
| `test_parse_confidence_out_of_range_high`    | `parse_confidence("[CONFIDENCE: 6]")` returns `None`                         |
| `test_parse_confidence_out_of_range_zero`    | `parse_confidence("[CONFIDENCE: 0]")` returns `None`                         |
| `test_parse_confidence_boundary_one`         | `parse_confidence("[CONFIDENCE: 1]")` returns `Some(1)`                      |
| `test_parse_confidence_boundary_five`        | `parse_confidence("[CONFIDENCE: 5]")` returns `Some(5)`                      |
| `test_parse_confidence_with_spaces`          | `parse_confidence("[CONFIDENCE:  3]")` returns `Some(3)` (extra space)       |
| `test_parse_confidence_multiple_takes_first` | `parse_confidence("[CONFIDENCE: 3] text [CONFIDENCE: 5]")` returns `Some(3)` |

---

## Module 5: Failure Memory (`src/core/models/failure.rs`)

### Unit Tests (inline `#[cfg(test)]`)

| Test Function                                         | What It Asserts                                                                                                                                             |
| ----------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `test_failure_log_for_task_filters`                   | Log with entries for tasks A, B, A: `for_task("A")` returns 2 entries, `for_task("B")` returns 1                                                            |
| `test_failure_log_for_task_empty`                     | Log with entries for task A: `for_task("C")` returns empty vec                                                                                              |
| `test_failure_log_for_task_on_empty_log`              | Empty log: `for_task("A")` returns empty vec                                                                                                                |
| `test_failure_log_as_prompt_context_formats`          | Log with 2 entries for task A: `as_prompt_context("A")` returns `Some(...)` containing `"Previous Failed Approaches"`, `"Iteration 1"`, and `"Iteration 2"` |
| `test_failure_log_as_prompt_context_no_match`         | Log with entries for task A: `as_prompt_context("B")` returns `None`                                                                                        |
| `test_failure_log_as_prompt_context_includes_lessons` | Output string contains each entry's `lesson` field verbatim                                                                                                 |
| `test_failure_log_default_empty`                      | `FailureLog::default()` has `entries.len() == 0`                                                                                                            |

---

## Module 6: Progress File (`src/core/models/progress.rs`)

### Unit Tests (inline `#[cfg(test)]`)

| Test Function                                             | What It Asserts                                                                                                                                      |
| --------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| `test_progress_file_new`                                  | `ProgressFile::new("plan.md", "claude", 20)` has `iterations_completed == 0`, `stuck_count == 0`, `escalation_level == None`, empty `iterations` vec |
| `test_record_iteration_with_progress_resets_stuck`        | After `record_iteration` where `tasks_after > tasks_before`: `stuck_count == 0`, `escalation_level == None`, `iterations_completed == 1`             |
| `test_record_iteration_no_progress_increments_stuck`      | After `record_iteration` where `tasks_after == tasks_before`: `stuck_count == 1`                                                                     |
| `test_record_iteration_consecutive_no_progress_escalates` | After 3 no-progress iterations: `stuck_count == 3`, `escalation_level == Suggest`                                                                    |
| `test_record_iteration_progress_after_stuck_resets`       | After 3 no-progress iterations then 1 progress iteration: `stuck_count == 0`, `escalation_level == None`                                             |
| `test_record_iteration_pushes_to_vec`                     | After recording 5 iterations: `iterations.len() == 5` and `iterations_completed == 5`                                                                |

---

## Module 7: Run Configuration (`src/core/models/run_config.rs`)

### Unit Tests (inline `#[cfg(test)]`)

| Test Function               | What It Asserts                                                                                                                                                        |
| --------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `test_run_config_default`   | `RunConfig::default()` has `max_iterations == 20`, `post_loop_review == true`, `use_worktree == false`, `confidence_threshold == Some(4)`, `initializer_phase == true` |
| `test_stuck_policy_default` | `StuckPolicy::default()` has thresholds `2, 4, 6, 8`                                                                                                                   |
| `test_test_gate_default`    | `TestGate::default()` has empty command, `timeout_secs == 300`, `max_output_lines == 50`                                                                               |

---

## Module 8: Checkpoint Adapter (`src/adapters/git/checkpoint.rs`)

### Unit Tests (inline `#[cfg(test)]`)

Each test creates a `TempDir` with `git2::Repository::init`. These are adapter-level tests that exercise real `git2` operations against temporary repos.

```rust
fn setup_repo() -> (TempDir, Repository) {
    let dir = TempDir::new().unwrap();
    let repo = Repository::init(dir.path()).unwrap();
    let mut index = repo.index().unwrap();
    let tree_oid = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_oid).unwrap();
    let sig = Signature::now("test", "test@test.com").unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[]).unwrap();
    (dir, repo)
}
```

| Test Function                              | What It Asserts                                                                                                                       |
| ------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------- |
| `test_is_clean_on_fresh_repo`              | `is_clean(&repo)` returns `true` on a repo with only the initial commit                                                               |
| `test_is_clean_with_untracked_file`        | After writing a file (not staged): `is_clean(&repo)` returns `false`                                                                  |
| `test_is_clean_with_staged_file`           | After staging a file: `is_clean(&repo)` returns `false`                                                                               |
| `test_is_clean_with_modified_tracked_file` | After modifying a committed file: `is_clean(&repo)` returns `false`                                                                   |
| `test_checkpoint_creates_commit`           | After writing a file: `checkpoint(&repo, "test")` returns `Some(sha)` where `sha.len() == 7`                                          |
| `test_checkpoint_stages_untracked`         | After writing an untracked file: `checkpoint(&repo, "test")` succeeds, and then `is_clean(&repo)` returns `true`                      |
| `test_checkpoint_stages_modified`          | After modifying a committed file: `checkpoint(&repo, "test")` succeeds, and then `is_clean(&repo)` returns `true`                     |
| `test_checkpoint_handles_deleted_files`    | After deleting a committed file: `checkpoint(&repo, "test")` succeeds (the deletion is committed)                                     |
| `test_checkpoint_returns_none_on_clean`    | On a clean repo: `checkpoint(&repo, "nothing")` returns `None`                                                                        |
| `test_checkpoint_message_in_commit`        | After checkpoint: `repo.head().peel_to_commit().message()` contains the provided message                                              |
| `test_checkpoint_does_not_invoke_hooks`    | Implicit: `git2` commits bypass hooks by design. Verified by the fact that `checkpoint` succeeds even when no hooks directory exists. |

### Adapter Tests (`tests/adapter/checkpoint_test.rs`)

Tests that exercise the `GitVersionControl` adapter's checkpoint integration:

| Test Function                             | What It Asserts                                                                                |
| ----------------------------------------- | ---------------------------------------------------------------------------------------------- |
| `test_git_vcs_is_clean`                   | `GitVersionControl::new(path).is_clean()` returns correct results for clean/dirty repos        |
| `test_git_vcs_checkpoint`                 | `GitVersionControl::new(path).checkpoint("msg")` returns `Some(sha)` and leaves the repo clean |
| `test_git_vcs_checkpoint_default_message` | Checkpoint with a timestamped message: commit message starts with `"checkpoint: "`             |

---

## Module 9: Worktree Adapter (`src/adapters/git/worktree.rs`)

### Unit Tests (inline `#[cfg(test)]`)

Each test creates a `TempDir` with `git2::Repository::init` and an initial commit.

```rust
fn setup_repo_with_commit() -> (TempDir, Repository) {
    let dir = TempDir::new().unwrap();
    let repo = Repository::init(dir.path()).unwrap();
    std::fs::write(dir.path().join("README.md"), "# test").unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(Path::new("README.md")).unwrap();
    index.write().unwrap();
    let tree_oid = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_oid).unwrap();
    let sig = Signature::now("test", "test@test.com").unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[]).unwrap();
    (dir, repo)
}
```

| Test Function                                | What It Asserts                                                                                                                 |
| -------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| `test_worktree_base_dir_appends_suffix`      | `worktree_base_dir(Path::new("/code/noslop"))` returns `/code/noslop-worktrees`                                                 |
| `test_worktree_base_dir_preserves_parent`    | `worktree_base_dir(Path::new("/a/b/myrepo"))` returns `/a/b/myrepo-worktrees`                                                   |
| `test_worktree_path_composes`                | `worktree_path(Path::new("/code/repo"), "feat-a")` returns `/code/repo-worktrees/feat-a`                                        |
| `test_list_worktrees_returns_main`           | On a fresh repo: `list_worktrees(&repo)` returns 1 entry with `is_main == true`                                                 |
| `test_list_worktrees_main_has_branch`        | Main worktree entry has `branch == Some("main")` or `Some("master")`                                                            |
| `test_create_worktree_new_branch`            | `create_worktree(&repo, &wt_path, "feat-a", "HEAD")` succeeds; `wt_path.exists()` is `true`; `list_worktrees` returns 2 entries |
| `test_create_worktree_existing_branch`       | After creating branch `feat-b`: `create_worktree(&repo, &wt_path, "feat-b", "HEAD")` succeeds (reuses existing branch)          |
| `test_create_worktree_creates_parent_dirs`   | Worktree path with missing parent directory: `create_worktree` creates the parent automatically                                 |
| `test_remove_worktree_cleans_up_dir`         | After creating and then removing a worktree: `wt_path.exists()` is `false`                                                      |
| `test_remove_worktree_deletes_merged_branch` | Branch created at HEAD (trivially merged): `remove_worktree` returns `BranchCleanupResult::Deleted`                             |
| `test_remove_worktree_keeps_unmerged_branch` | Branch with additional commits beyond HEAD: `remove_worktree` returns `BranchCleanupResult::Kept("not merged")`                 |
| `test_clean_worktrees_removes_all`           | After creating 3 worktrees: `clean_worktrees` returns `CleanResult { removed: 3 }`; worktree base dir is removed                |
| `test_clean_worktrees_noop_when_none`        | On a fresh repo: `clean_worktrees` returns `CleanResult { removed: 0 }`                                                         |
| `test_is_branch_merged_detects_ancestor`     | Branch at HEAD: `is_branch_merged` returns `true`                                                                               |
| `test_is_branch_merged_detects_diverged`     | Branch with commits not in HEAD: `is_branch_merged` returns `false`                                                             |

### Adapter Tests (`tests/adapter/worktree_test.rs`)

Test the `GitVersionControl` adapter's worktree method delegation:

| Test Function                             | What It Asserts                                                                                                 |
| ----------------------------------------- | --------------------------------------------------------------------------------------------------------------- |
| `test_git_vcs_list_worktrees`             | `GitVersionControl::new(path).list_worktrees()` returns at least the main worktree                              |
| `test_git_vcs_create_and_remove_worktree` | Create via `GitVersionControl`, verify `list_worktrees` count increases, then remove and verify count decreases |

---

## Module 10: Run Loop Orchestrator (`src/core/services/run_loop.rs`)

### Unit Tests with Mocks (inline `#[cfg(test)]`)

These tests inject `MockAgentRuntime`, `MockPlanStore`, `MockTestRunner`, and `MockVersionControl` (all auto-generated by `mockall`). They verify orchestration logic without any I/O.

| Test Function                                  | What It Asserts                                                                                                                                                                  |
| ---------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `test_run_loop_completes_in_one_iteration`     | Mock runtime returns `plan_complete: true`; mock plan store returns all-complete plan on reload. `execute()` returns `RunOutcome::Complete { iterations_used: 1 }`               |
| `test_run_loop_completes_in_three_iterations`  | Mock plan store returns incrementally more complete plans across 3 `load` calls. Runtime signals complete on 3rd. Returns `RunOutcome::Complete { iterations_used: 3 }`          |
| `test_run_loop_max_iterations_reached`         | Runtime never signals complete. `max_iterations == 5`. Returns `RunOutcome::MaxIterationsReached { iterations_used: 5, tasks_completed: 0, tasks_total: 3 }`                     |
| `test_run_loop_stuck_terminates`               | Runtime returns no progress for `level_4_threshold` consecutive iterations. Returns `RunOutcome::Stuck`                                                                          |
| `test_run_loop_stuck_signal_in_output`         | Runtime output contains `<promise>STUCK</promise>`. Returns `RunOutcome::Stuck` immediately                                                                                      |
| `test_run_loop_checkpoints_between_iterations` | Mock `vcs.checkpoint` is called exactly once per iteration (verified with `expect_checkpoint().times(n)`)                                                                        |
| `test_run_loop_test_gate_failures_injected`    | Mock test runner returns `passed: false` with output `"FAILED test_foo"`. On next iteration, the prompt (captured from mock runtime's `invoke` arg) contains `"FAILED test_foo"` |
| `test_run_loop_test_gate_passing_no_injection` | Mock test runner returns `passed: true`. Next iteration's prompt does NOT contain `"Test Failures"`                                                                              |
| `test_run_loop_skips_tests_when_none`          | With `test_runner: None`, the loop runs without calling any test runner. No `"Test Failures"` in prompts.                                                                        |
| `test_run_loop_false_completion_continues`     | Runtime signals `plan_complete: true` but plan store returns plan with incomplete tasks. Loop continues to next iteration instead of returning `Complete`.                       |
| `test_run_loop_post_loop_review_called`        | With `post_loop_review: true`, after completion, runtime `invoke` is called one extra time with a prompt containing `"Review all changes"`.                                      |
| `test_run_loop_post_loop_review_skipped`       | With `post_loop_review: false`, after completion, runtime `invoke` is called exactly N times (one per iteration, no extra review call).                                          |
| `test_run_loop_initializer_phase_iteration_1`  | With `initializer_phase: true`, the first iteration's prompt contains `"Initialization Phase"`. Iteration 2's prompt does NOT.                                                   |
| `test_run_loop_progress_recorded`              | After each iteration, the internal `ProgressFile` has the correct `iterations_completed` count and the latest `IterationRecord` matches the agent's output.                      |

---

## Module 11: Config Parsing Extension (`.noslop.toml` agent/run sections)

### Unit Tests (`tests/adapter/toml_agent_config_test.rs`)

Tests for parsing the new `[agent]` and `[run]` sections from `.noslop.toml`.

| Test Function                      | What It Asserts                                                                                                                         |
| ---------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------- |
| `test_parse_agent_section`         | TOML with `[agent]\ntype = "claude"` parses `agent_type == AgentType::Claude`                                                           |
| `test_parse_agent_section_codex`   | TOML with `[agent]\ntype = "codex"` parses `agent_type == AgentType::Codex`                                                             |
| `test_parse_agent_section_missing` | TOML without `[agent]` section: `agent_type` is `None`                                                                                  |
| `test_parse_run_section_defaults`  | TOML with empty `[run]` section: all fields take default values                                                                         |
| `test_parse_run_section_custom`    | TOML with `max_iterations = 30`, `confidence_threshold = 3`: parsed values match                                                        |
| `test_parse_stuck_policy`          | TOML with `[run.stuck_policy]\nlevel_1 = 3\nlevel_4 = 12`: thresholds are `3, 4, 6, 12` (only overridden values change)                 |
| `test_parse_test_gate`             | TOML with `[run.test_gate]\ncommand = "cargo test"\ntimeout_secs = 600`: parsed correctly                                               |
| `test_parse_test_gate_missing`     | TOML without `[run.test_gate]`: `test_gate` is `None`                                                                                   |
| `test_parse_full_config`           | Complete TOML with all sections (`[project]`, `[agent]`, `[run]`, `[run.stuck_policy]`, `[run.test_gate]`): all fields parsed correctly |

### Test Fixture

```toml
# tests/fixtures/full_config.toml
[project]
prefix = "TST"

[agent]
type = "claude"

[[check]]
target = "*.rs"
message = "Review Rust"
severity = "block"

[run]
max_iterations = 30
confidence_threshold = 3
post_loop_review = true
initializer_phase = false

[run.stuck_policy]
level_1 = 3
level_2 = 5
level_3 = 7
level_4 = 10

[run.test_gate]
command = "cargo test"
timeout_secs = 600
max_output_lines = 100
```

---

## Module 12: Agent Adapters (`src/adapters/agent/`)

### Claude Config Adapter Tests (`tests/adapter/claude_config_test.rs`)

| Test Function                                         | What It Asserts                                                                                                             |
| ----------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------- |
| `test_claude_config_agent_type`                       | `ClaudeConfig::new().agent_type()` returns `AgentType::Claude`                                                              |
| `test_claude_config_generate_has_global_instructions` | `generate_config(root)` returns bundle where `global_instructions.relative_path` ends with `"CLAUDE.md"`                    |
| `test_claude_config_generate_has_settings`            | `generate_config(root)` returns bundle where `settings.relative_path` ends with `"settings.json"` and content is valid JSON |
| `test_claude_config_generate_has_hooks`               | `generate_config(root)` returns bundle with at least one hook for `"PreToolUse"` event                                      |
| `test_claude_config_generate_has_checkpoint_command`  | Bundle `commands` includes a `SlashCommand` with `name == "checkpoint"`                                                     |
| `test_claude_config_install_instructions`             | `install_instructions()` returns a non-empty string containing a URL or install command                                     |
| `test_claude_config_project_instructions`             | `generate_project_instructions(root)` returns `Some(content)` containing noslop-specific conventions                        |

### Claude Runtime Adapter Tests (`tests/adapter/claude_runtime_test.rs`)

| Test Function                                        | What It Asserts                                                                                                                                                                                |
| ---------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `test_claude_runtime_agent_type`                     | `ClaudeRuntime::new().agent_type()` returns `AgentType::Claude`                                                                                                                                |
| `test_claude_runtime_build_command`                  | `build_command(&config)` returns `("claude", args)` where `args` contains `-p`, `--output-format`, `text`, `--permission-mode`, `bypassPermissions`                                            |
| `test_claude_runtime_build_command_respects_timeout` | With `timeout_secs: Some(60)`, args contain `--timeout` and `60`                                                                                                                               |
| `test_claude_runtime_parse_output_complete`          | Output containing `<promise>COMPLETE</promise>` and `[RALPH:DONE task="X" iteration=1]`: `parse_output` returns `plan_complete == true`, `completed_task == Some("X")`, `iteration == Some(1)` |
| `test_claude_runtime_parse_output_incomplete`        | Output without completion markers: `parse_output` returns `plan_complete == false`, `completed_task == None`                                                                                   |
| `test_claude_runtime_is_plan_complete`               | `is_plan_complete("text <promise>COMPLETE</promise> text")` returns `true`                                                                                                                     |
| `test_claude_runtime_is_plan_complete_absent`        | `is_plan_complete("text without marker")` returns `false`                                                                                                                                      |
| `test_claude_runtime_extract_status_line`            | `extract_status_line("...[RALPH:DONE task=\"Setup\" iteration=3]...")` returns `Some(...)` containing the full match                                                                           |
| `test_claude_runtime_extract_status_line_absent`     | `extract_status_line("no status here")` returns `None`                                                                                                                                         |

### Codex Config/Runtime Tests (parallel structure)

Same test function patterns as Claude, adapted for Codex-specific command names and output formats.

---

## CLI Integration Tests (`tests/integration/`)

### Checkpoint CLI Tests (`tests/integration/checkpoint_test.rs`)

All tests use `TempGitRepo` and `assert_cmd`.

| Test Function                         | What It Asserts                                                                                                          |
| ------------------------------------- | ------------------------------------------------------------------------------------------------------------------------ |
| `test_cli_checkpoint_with_changes`    | `noslop checkpoint "test"` in a repo with untracked file: exits 0, stdout contains `"Checkpoint:"` and a 7-char SHA      |
| `test_cli_checkpoint_clean_tree`      | `noslop checkpoint` in a clean repo: exits 0, stdout contains `"Nothing to checkpoint"`                                  |
| `test_cli_checkpoint_default_message` | `noslop checkpoint` (no message arg) in a dirty repo: commit message matches `"checkpoint: YYYY-MM-DD HH:MM:SS"` pattern |
| `test_cli_checkpoint_custom_message`  | `noslop checkpoint "before refactor"`: commit message is `"before refactor"`                                             |
| `test_cli_checkpoint_json_output`     | `noslop checkpoint --json "test"` with changes: stdout is valid JSON with `"success": true` and `"sha"` field            |
| `test_cli_checkpoint_json_clean`      | `noslop checkpoint --json` with clean tree: stdout is valid JSON with `"clean": true`                                    |
| `test_cli_checkpoint_multiple_files`  | Write 3 files, run checkpoint: all 3 are committed (verified by `git status --porcelain` being empty)                    |

### Worktree CLI Tests (`tests/integration/worktree_test.rs`)

| Test Function                                 | What It Asserts                                                                                                                                                    |
| --------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `test_cli_worktree_create`                    | `noslop worktree create feature-test` in repo with initial commit: exits 0, stdout contains `"Branch: feature-test"` and a path, worktree directory exists on disk |
| `test_cli_worktree_list_shows_main`           | `noslop worktree list`: stdout contains the repo path and `"(main)"`                                                                                               |
| `test_cli_worktree_list_shows_created`        | After creating `feat-a`: `noslop worktree list` stdout contains `"feat-a"`                                                                                         |
| `test_cli_worktree_remove`                    | After creating `feat-b`: `noslop worktree remove feat-b` exits 0, stdout contains `"Removed:"`, worktree directory no longer exists                                |
| `test_cli_worktree_remove_nonexistent_errors` | `noslop worktree remove nonexistent`: exits non-zero, stderr contains `"not found"`                                                                                |
| `test_cli_worktree_clean`                     | After creating 2 worktrees: `noslop worktree clean` exits 0, stdout contains `"Cleaned 2 worktree(s)"`, worktree base directory removed                            |
| `test_cli_worktree_clean_noop`                | On repo with no worktrees: `noslop worktree clean` exits 0, stdout contains `"No worktrees"`                                                                       |
| `test_cli_worktree_create_with_base`          | `noslop worktree create feat-c HEAD~1`: worktree is created at the specified base commit                                                                           |
| `test_cli_worktree_json_output`               | `noslop worktree list --json`: stdout is valid JSON array                                                                                                          |

### Run CLI Tests (`tests/integration/run_test.rs`)

These tests are `#[ignore]`-tagged because they require agent CLIs to be installed. Run with `cargo test -- --ignored`.

| Test Function                    | What It Asserts                                                                                                |
| -------------------------------- | -------------------------------------------------------------------------------------------------------------- |
| `test_cli_run_missing_plan_file` | `noslop run nonexistent.md`: exits non-zero, stderr contains error about file not found                        |
| `test_cli_run_empty_plan_file`   | `noslop run empty.md` (file exists but has no tasks): exits non-zero, stderr contains `"No tasks found"`       |
| `test_cli_run_shows_banner`      | `noslop run plan.md --max-iterations 1`: stdout contains `"noslop run"`, `"Plan:"`, `"Agent:"`, `"Max iters:"` |

---

## Test File Organization Summary

```
tests/
├── common/
│   ├── mod.rs              # (extend: add PlanBuilder, etc.)
│   ├── fixtures.rs         # (extend: PlanBuilder, IterationRecordBuilder,
│   │                       #  ProgressFileBuilder, FailureLogBuilder,
│   │                       #  RunConfigBuilder, InvocationResultBuilder)
│   ├── git_repo.rs         # (extend: init_with_commit, write_and_stage,
│   │                       #  head_sha, is_clean, commit_count,
│   │                       #  create_branch, worktree_base_dir)
│   └── mocks.rs            # (extend: MockPlanStoreManual,
│                           #  MockTestRunnerManual)
├── unit/
│   ├── agent_type_test.rs          # NEW: AgentType parsing parameterized tests
│   ├── plan_parser_test.rs         # NEW: plan parsing parameterized tests
│   ├── stuck_detector_test.rs      # NEW: escalation level parameterized tests
│   └── proptest_plan_parser.rs     # NEW: property-based plan parsing tests
├── adapter/
│   ├── checkpoint_test.rs          # NEW: GitVersionControl checkpoint tests
│   ├── worktree_test.rs            # NEW: GitVersionControl worktree tests
│   ├── claude_config_test.rs       # NEW: ClaudeConfig adapter tests
│   ├── claude_runtime_test.rs      # NEW: ClaudeRuntime adapter tests
│   ├── codex_config_test.rs        # NEW: CodexConfig adapter tests
│   ├── codex_runtime_test.rs       # NEW: CodexRuntime adapter tests
│   └── toml_agent_config_test.rs   # NEW: .noslop.toml [agent]/[run] parsing tests
└── integration/
    ├── checkpoint_test.rs          # NEW: CLI checkpoint integration tests
    ├── worktree_test.rs            # NEW: CLI worktree integration tests
    └── run_test.rs                 # NEW: CLI run integration tests (mostly #[ignore])
```

Inline `#[cfg(test)]` modules within source files:

```
src/core/ports/agent.rs           # AgentType unit tests
src/core/models/failure.rs        # FailureLog unit tests
src/core/models/progress.rs       # ProgressFile unit tests
src/core/models/run_config.rs     # RunConfig/StuckPolicy/TestGate defaults
src/core/services/plan_parser.rs  # Plan parsing unit tests
src/core/services/stuck_detector.rs # Stuck assessment unit tests
src/core/services/prompt_builder.rs # Prompt construction unit tests
src/core/services/run_loop.rs     # Run loop orchestration unit tests (with mocks)
src/adapters/git/checkpoint.rs    # git2 checkpoint unit tests
src/adapters/git/worktree.rs      # git2 worktree unit tests
```

## Dev Dependencies

No new dev dependencies required. All test functionality is covered by the existing `Cargo.toml`:

| Crate                     | Used For                                                                                            |
| ------------------------- | --------------------------------------------------------------------------------------------------- |
| `mockall` (0.13)          | Auto-generated mocks for `AgentConfig`, `AgentRuntime`, `PlanStore`, `TestRunner`, `VersionControl` |
| `tempfile` (3.23)         | `TempDir` for git repo tests, TOML file tests, plan file tests                                      |
| `assert_cmd` (2.1)        | CLI integration tests for `noslop checkpoint`, `noslop worktree`, `noslop run`                      |
| `predicates` (3.1)        | Stdout/stderr assertions in integration tests                                                       |
| `pretty_assertions` (1.4) | Readable diff output for plan parsing and prompt builder tests                                      |
| `test-case` (3.3)         | Parameterized tests for agent type parsing, plan formats, escalation levels, pattern matching       |
| `proptest` (1.5)          | Property-based tests for plan parser (task count invariants, description preservation)              |

## Test Execution

```bash
# All unit tests (fast, no I/O)
cargo test --lib

# All unit + adapter tests (uses tempfile, no external deps)
cargo test

# Integration tests that require agent CLIs
cargo test -- --ignored

# Specific module tests
cargo test plan_parser
cargo test stuck_detector
cargo test checkpoint
cargo test worktree
cargo test run_loop

# Property-based tests with more cases
PROPTEST_CASES=1000 cargo test proptest
```
