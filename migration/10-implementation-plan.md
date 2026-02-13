# Implementation Plan: Setup Migration into Noslop

Synthesis of design documents 01-07 into an ordered, actionable implementation plan. A developer should be able to pick up this document and start coding Phase 1 immediately.

## Table of Contents

- [Dependency Analysis](#dependency-analysis)
- [Current Codebase Baseline](#current-codebase-baseline)
- [Phase Overview](#phase-overview)
- [Phase 1: Core Types and Port Traits](#phase-1-core-types-and-port-traits)
- [Phase 2: Checkpoint and Worktree](#phase-2-checkpoint-and-worktree)
- [Phase 3: Agent Adapters](#phase-3-agent-adapters)
- [Phase 4: Init Expansion](#phase-4-init-expansion)
- [Phase 5: Run Loop Core](#phase-5-run-loop-core)
- [Phase 6: Run Loop Adapters and CLI](#phase-6-run-loop-adapters-and-cli)
- [Phase 7: Integration Testing and Polish](#phase-7-integration-testing-and-polish)
- [Setup Repo Deprecation Path](#setup-repo-deprecation-path)
- [Risk Register](#risk-register)

---

## Dependency Analysis

### Cargo.toml: Current Dependencies (No Changes Required)

Every crate needed for the migration is already in `Cargo.toml`. No new runtime or dev dependencies.

| Crate        | Current Version           | Used By (New Code)                                                                           |
| ------------ | ------------------------- | -------------------------------------------------------------------------------------------- |
| `git2`       | 0.20.2 (vendored-openssl) | Checkpoint (stage-all, commit), Worktree (create, list, remove, prune, branch ops)           |
| `chrono`     | 0.4.42 (serde)            | Checkpoint timestamps, ProgressFile/IterationRecord/FailureEntry timestamps                  |
| `clap`       | 4.5.51 (derive, cargo)    | New subcommands: Checkpoint, Worktree, Run; Init agent argument                              |
| `regex`      | 1.12                      | Plan parser (markdown checkboxes), output parsing (RALPH:DONE, CONFIDENCE), stuck prompts    |
| `serde`      | 1.0.228 (derive)          | AgentType, Plan models, ProgressFile, FailureEntry, RunConfig, TestGate, AgentSection        |
| `serde_json` | 1.0.145                   | Settings.json generation, progress.json, failures.jsonl, JSON plan parsing, JSON output mode |
| `toml`       | 0.9.8                     | NoslopFile `[agent]` section parsing, `.noslop.toml` generation                              |
| `anyhow`     | 1.0.100                   | All new functions return `anyhow::Result<T>`                                                 |
| `thiserror`  | 2.0.17                    | Custom error types if needed (none required in current designs)                              |
| `colored`    | 3.0.0                     | Run loop progress output                                                                     |
| `log`        | 0.4.28                    | Debug logging in adapters                                                                    |
| `walkdir`    | 2.5.0                     | Not directly needed by new code, but already available                                       |

**Dev dependencies** -- all already present:

| Crate                   | Used By (New Tests)                                              |
| ----------------------- | ---------------------------------------------------------------- |
| `mockall` 0.13          | MockAgentRuntime, MockAgentConfig, MockPlanStore, MockTestRunner |
| `tempfile` 3.23.0       | Checkpoint/worktree adapter tests, integration tests             |
| `assert_cmd` 2.1.1      | CLI integration tests for checkpoint, worktree, run              |
| `predicates` 3.1.3      | Output assertions in integration tests                           |
| `pretty_assertions` 1.4 | Struct comparison in unit tests                                  |
| `test-case` 3.3         | Parameterized tests for plan parser, stuck detector              |
| `proptest` 1.5          | Property tests for plan parser edge cases                        |

### Cargo.toml: One Optional New Dependency

The `dirs` crate is needed by `configure_agent()` to resolve `$HOME` for writing agent config files (`~/.claude/settings.json`, etc.). Currently the codebase does not use `dirs`.

```toml
# Add to [dependencies]:
dirs = "6.0"
```

This is the **only** new dependency. It is small (no transitive deps beyond `libc`/`windows-sys`), widely used, and required because `std::env::home_dir()` is deprecated.

### Feature Flags

No new feature flags are needed. The existing `llm` feature (tokio + reqwest) is orthogonal to the migration. Agent invocation uses `std::process::Command` (synchronous), not async HTTP.

---

## Current Codebase Baseline

### Source Files (45 .rs files)

```
src/
  main.rs, lib.rs, output.rs, noslop_file.rs
  cli/
    mod.rs, app.rs
    commands/
      mod.rs, init.rs, ack.rs, check_validate.rs, check_manage.rs,
      add_trailers.rs, clear_staged.rs, review.rs
  core/
    mod.rs
    models/
      mod.rs, check.rs, acknowledgment.rs, severity.rs, target.rs, review.rs
    ports/
      mod.rs, check_repo.rs, acknowledgment_store.rs, vcs.rs, review_store.rs
    services/
      mod.rs, checker.rs, matcher.rs
  adapters/
    mod.rs, review.rs
    git/
      mod.rs, hooks.rs, staging.rs
    toml/
      mod.rs, parser.rs, repository.rs, writer.rs
    file/
      mod.rs
    trailer/
      mod.rs
  shared/
    mod.rs, resolver.rs
    parser/
      mod.rs, token.rs, pattern.rs
  storage/
    mod.rs
  git/
    mod.rs
```

### Test Files (21 .rs files)

```
tests/
  unit.rs, adapter.rs
  common/
    mod.rs, fixtures.rs, git_repo.rs, mocks.rs
  unit/
    common/mod.rs, check_test.rs, cli_test.rs, output_test.rs,
    storage_test.rs, parser_test.rs, parameterized_test.rs,
    proptest_matcher.rs, resolver_test.rs, target_test.rs
  adapter/
    mod.rs, git_hooks_test.rs, toml_test.rs
  integration/
    main.rs, lifecycle_test.rs
```

---

## Phase Overview

| Phase     | Description                     | New Files | Modified Files | Est. Lines | Depends On    |
| --------- | ------------------------------- | --------- | -------------- | ---------- | ------------- |
| 1         | Core types and port traits      | 8         | 3              | ~650       | Nothing       |
| 2         | Checkpoint and worktree         | 6         | 4              | ~750       | Phase 1       |
| 3         | Agent adapters (Claude + Codex) | 10        | 2              | ~900       | Phase 1       |
| 4         | Init expansion                  | 0         | 5              | ~200       | Phase 1, 3    |
| 5         | Run loop core services          | 4         | 2              | ~700       | Phase 1       |
| 6         | Run loop adapters + CLI         | 7         | 3              | ~600       | Phase 2, 3, 5 |
| 7         | Integration testing + polish    | 5         | 2              | ~500       | All           |
| **Total** |                                 | **40**    | **~15**        | **~4,300** |               |

### Dependency Graph

```
Phase 1 (types + ports)
  |
  +---> Phase 2 (checkpoint + worktree)
  |       |
  +---> Phase 3 (agent adapters)
  |       |
  |       +---> Phase 4 (init expansion) <--- Phase 3
  |
  +---> Phase 5 (run loop core)
          |
          +---> Phase 6 (run loop adapters + CLI) <--- Phase 2, 3
                  |
                  +---> Phase 7 (integration testing)
```

Phases 2, 3, and 5 can be worked on **in parallel** after Phase 1 completes. Phase 4 requires both 1 and 3. Phase 6 requires 2, 3, and 5. Phase 7 is the final sweep.

---

## Phase 1: Core Types and Port Traits

**Goal**: Define all new domain models and port trait interfaces so that subsequent phases can code against stable contracts.

**Source docs**: 03-agent-port-design.md, 06-run-loop-design.md (models only), 05-checkpoint-worktree-design.md (WorktreeInfo model)

### File-by-File Changes

#### NEW: `src/core/models/agent.rs` (~60 lines)

`AgentType` enum with `FromStr`, `Display`, `Serialize`, `Deserialize`. Factory helpers `command_name()` and `display_name()`.

Source: 03-agent-port-design.md, "Type Definitions" section.

```rust
// Key types:
// - AgentType { Claude, Codex }
// - impl FromStr, Display, Serialize, Deserialize
```

#### NEW: `src/core/models/worktree.rs` (~20 lines)

`WorktreeInfo` struct with `path`, `branch`, `head`, `is_main` fields.

Source: 05-checkpoint-worktree-design.md, "New Model Type" section.

#### NEW: `src/core/models/plan.rs` (~80 lines)

`PlanFormat`, `TaskStatus`, `Task`, `Plan` structs with helper methods (`done_count`, `total_count`, `next_incomplete`, `is_complete`).

Source: 06-run-loop-design.md, "Plan" section.

#### NEW: `src/core/models/progress.rs` (~70 lines)

`IterationRecord`, `ProgressFile`, `EscalationLevel` types. `ProgressFile::record_iteration()` method.

Source: 06-run-loop-design.md, "Progress File" and "Escalation Level" sections.

#### NEW: `src/core/models/failure.rs` (~50 lines)

`FailureEntry`, `FailureLog` types. `FailureLog::for_task()` and `as_prompt_context()` methods.

Source: 06-run-loop-design.md, "Failure Memory" section.

#### NEW: `src/core/models/run_config.rs` (~80 lines)

`RunConfig`, `StuckPolicy`, `TestGate` structs with `Default` impls. `EscalationLevel::from_stuck_count_with_policy()`.

Source: 06-run-loop-design.md, "Run Configuration" and "Escalation Level" sections.

#### NEW: `src/core/ports/agent.rs` (~200 lines)

All agent port types and both trait definitions:

- Configuration types: `GlobalInstructions`, `HookDefinition`, `SettingsFile`, `SlashCommand`, `AgentConfigBundle`
- Invocation types: `InvocationConfig`, `InvocationResult`, `IterationOutput`
- Traits: `AgentConfig`, `AgentRuntime`

Source: 03-agent-port-design.md, "Complete Module Definition" section.

#### NEW: `src/core/ports/plan_store.rs` (~25 lines)

`PlanStore` trait with `load()`, `save()`, `update_task_status()`.

Source: 06-run-loop-design.md, "Plan Store" section.

#### NEW: `src/core/ports/test_runner.rs` (~30 lines)

`TestResult` struct, `TestRunner` trait with `run_tests()` and `detect()`.

Source: 06-run-loop-design.md, "Test Runner" section.

#### MODIFY: `src/core/models/mod.rs` (+8 lines)

Add module declarations and re-exports for new model files:

```rust
mod agent;
mod failure;
mod plan;
mod progress;
mod run_config;
mod worktree;

pub use agent::AgentType;
pub use failure::{FailureEntry, FailureLog};
pub use plan::{Plan, PlanFormat, Task, TaskStatus};
pub use progress::{EscalationLevel, IterationRecord, ProgressFile};
pub use run_config::{RunConfig, StuckPolicy, TestGate};
pub use worktree::WorktreeInfo;
```

#### MODIFY: `src/core/ports/mod.rs` (+6 lines)

Add module declarations and re-exports:

```rust
mod agent;
mod plan_store;
mod test_runner;

pub use agent::{
    AgentConfig, AgentConfigBundle, AgentRuntime, AgentType,
    GlobalInstructions, HookDefinition, InvocationConfig,
    InvocationResult, IterationOutput, SettingsFile, SlashCommand,
};
pub use plan_store::PlanStore;
pub use test_runner::{TestResult, TestRunner};
```

#### MODIFY: `src/lib.rs` (+0-5 lines)

Verify that `core::models` and `core::ports` re-exports are already public. They should be, but check.

### Phase 1 Verification

After Phase 1, `cargo check` must pass. No functionality is implemented yet, but all type definitions and trait signatures compile. Downstream phases can `use` these types.

```bash
cargo check 2>&1 | head -20
# Expected: Compiling noslop v0.1.2
# No errors
```

### Phase 1 Scope

- **New files**: 8
- **Modified files**: 3
- **Estimated lines**: ~650
- **Tests**: None yet (types and traits only; tests come with implementations)

---

## Phase 2: Checkpoint and Worktree

**Goal**: Port the bash `checkpoint` (56 lines) and `worktree` (223 lines) scripts to Rust subcommands, using `git2`.

**Source docs**: 05-checkpoint-worktree-design.md

**Depends on**: Phase 1 (for `WorktreeInfo` model and `VersionControl` trait extensions)

### File-by-File Changes

#### NEW: `src/adapters/git/checkpoint.rs` (~60 lines)

Functions: `is_clean(&Repository) -> Result<bool>`, `checkpoint(&Repository, &str) -> Result<Option<String>>`, `default_signature()`.

Uses `git2::StatusOptions`, `Index::add_all`, `Index::update_all`, `Repository::commit`. No hooks invoked (git2 skips hooks by default).

Source: 05-checkpoint-worktree-design.md, "Adapter Implementations" > checkpoint.

#### NEW: `src/adapters/git/worktree.rs` (~200 lines)

Functions:

- `worktree_base_dir(repo_root) -> PathBuf`
- `worktree_path(repo_root, branch) -> PathBuf`
- `list_worktrees(repo) -> Vec<WorktreeInfo>`
- `create_worktree(repo, path, branch, base)`
- `remove_worktree(repo, path, branch, force) -> BranchCleanupResult`
- `clean_worktrees(repo, repo_root) -> CleanResult`
- `prune_worktrees(repo)`
- `is_branch_merged(repo, branch) -> bool`
- `cleanup_branch(repo, branch) -> BranchCleanupResult`

Types: `BranchCleanupResult` enum, `CleanResult` struct.

Source: 05-checkpoint-worktree-design.md, "Adapter Implementations" > worktree.

#### NEW: `src/cli/commands/checkpoint.rs` (~50 lines)

Command handler: resolves message (default = timestamp), calls `vcs.checkpoint()`, outputs human/JSON result.

Source: 05-checkpoint-worktree-design.md, "Command Implementations" > checkpoint.

#### NEW: `src/cli/commands/worktree.rs` (~130 lines)

Command handler: dispatches `WorktreeAction` to create/list/remove/clean, each with human/JSON output.

Source: 05-checkpoint-worktree-design.md, "Command Implementations" > worktree.

#### MODIFY: `src/core/ports/vcs.rs` (~+40 lines)

Extend `VersionControl` trait with 8 new methods:

```rust
fn is_clean(&self) -> anyhow::Result<bool>;
fn checkpoint(&self, message: &str) -> anyhow::Result<Option<String>>;
fn list_worktrees(&self) -> anyhow::Result<Vec<WorktreeInfo>>;
fn create_worktree(&self, path: &Path, branch: &str, base: &str) -> anyhow::Result<()>;
fn remove_worktree(&self, path: &Path, force: bool) -> anyhow::Result<()>;
fn prune_worktrees(&self) -> anyhow::Result<()>;
fn is_branch_merged(&self, branch: &str) -> anyhow::Result<bool>;
fn delete_branch(&self, branch: &str, force: bool) -> anyhow::Result<()>;
```

Source: 05-checkpoint-worktree-design.md, "Port Trait Extension".

#### MODIFY: `src/adapters/git/mod.rs` (~+50 lines)

- Add `pub mod checkpoint;` and `pub mod worktree;`
- Implement the 8 new `VersionControl` methods on `GitVersionControl`, each delegating to the corresponding module function.

#### MODIFY: `src/cli/app.rs` (~+25 lines)

Add `Checkpoint` and `Worktree` variants to the `Command` enum. Add `WorktreeAction` subcommand enum. Add dispatch arms in `run()`.

Source: 05-checkpoint-worktree-design.md, "CLI Definitions".

#### MODIFY: `src/cli/commands/mod.rs` (~+4 lines)

Add `mod checkpoint;` and `mod worktree;` declarations and `pub use` re-exports.

### Design Decisions

- **Multi-repo mode dropped**. Single-repo only. Every other noslop command operates on one repo.
- **git2 preferred over Command**. Checkpoint is hot-path (runs between every ralph iteration). git2 avoids subprocess overhead and hook execution.
- **VersionControl trait extended** (not a new trait). Checkpoint and worktree are git operations that share the same repository context.

### Phase 2 Verification

```bash
# Unit tests
cargo test checkpoint -- --nocapture
cargo test worktree -- --nocapture

# Manual CLI test
cd /tmp && git init test-repo && cd test-repo
echo "hello" > file.txt
noslop checkpoint "test"
noslop worktree create feature-a
noslop worktree list
noslop worktree remove feature-a
noslop worktree clean
```

### Phase 2 Scope

- **New files**: 4 (+ 2 test files in Phase 7)
- **Modified files**: 4
- **Estimated lines**: ~750 (implementation + inline tests)
- **Behavior parity**: All single-repo behaviors from bash scripts covered. Multi-repo intentionally dropped.

---

## Phase 3: Agent Adapters

**Goal**: Implement `ClaudeConfig`, `ClaudeRuntime`, `CodexConfig`, `CodexRuntime` adapters with embedded template content.

**Source docs**: 04-agent-adapters-design.md

**Depends on**: Phase 1 (for port traits and types)

### File-by-File Changes

#### NEW: `src/adapters/agent/mod.rs` (~30 lines)

Module declarations, re-exports, factory functions:

- `get_agent_config(AgentType) -> Box<dyn AgentConfig>`
- `get_agent_runtime(AgentType) -> Box<dyn AgentRuntime>`

Source: 04-agent-adapters-design.md, "Module Root" section.

#### NEW: `src/adapters/agent/claude.rs` (~400 lines)

`ClaudeConfig` struct implementing `AgentConfig`:

- `generate_config()` produces `AgentConfigBundle` with settings.json, CLAUDE.md, hooks, slash commands
- `build_settings()` constructs settings.json via `serde_json::json!`
- `build_permissions()`, `build_hooks()`, `build_sandbox()` helpers
- `is_available()` checks for `claude --version`
- `generate_project_instructions()` returns CLAUDE.md content for repo root

`ClaudeRuntime` struct implementing `AgentRuntime`:

- `build_command()` produces `claude -p "$PROMPT" --output-format text --permission-mode bypassPermissions`
- `invoke()` executes via `std::process::Command`, captures stdout/stderr, records duration
- `parse_output()` extracts `<promise>COMPLETE</promise>`, `[RALPH:DONE ...]`, error patterns
- `extract_errors()` scans for error/failure patterns

Source: 04-agent-adapters-design.md, full "Claude Adapter" section.

#### NEW: `src/adapters/agent/codex.rs` (~200 lines)

`CodexConfig` struct implementing `AgentConfig`:

- `generate_config()` produces bundle with `instructions.md` only (no settings, hooks, or commands)
- `is_available()` checks for `codex --version`
- `generate_project_instructions()` returns AGENTS.md content

`CodexRuntime` struct implementing `AgentRuntime`:

- `build_command()` produces `codex --quiet --approval-mode full-auto "$PROMPT"`
- `invoke()` and `parse_output()` use same patterns as Claude (completion signals are prompt conventions)

Source: 04-agent-adapters-design.md, full "Codex Adapter" section.

#### NEW: Template files (6 files, ~300 lines total)

```
src/adapters/agent/templates/
  claude_global_instructions.md     (~40 lines)
  claude_branch_protection.sh       (~15 lines)
  claude_auto_format.sh             (~30 lines)
  claude_cmd_checkpoint.md          (~30 lines)
  claude_cmd_worktree.md            (~20 lines)
  codex_instructions.md             (~30 lines)
```

Embedded at compile time via `include_str!()`. Content extracted from setup repo's existing files.

Source: 04-agent-adapters-design.md, "Generated Content" subsections.

#### MODIFY: `src/adapters/mod.rs` (+3 lines)

Add `pub mod agent;` and re-export key types.

#### MODIFY: `src/lib.rs` (+0-2 lines)

Ensure `adapters::agent` is accessible from binary crate.

### Phase 3 Verification

```bash
# Unit tests (no agent CLI required)
cargo test claude_config -- --nocapture
cargo test codex_config -- --nocapture
cargo test claude_runtime -- --nocapture
cargo test parse_output -- --nocapture

# Integration tests (require agent CLIs, run with --ignored)
cargo test -- --ignored agent
```

### Phase 3 Scope

- **New files**: 10 (4 Rust + 6 templates)
- **Modified files**: 2
- **Estimated lines**: ~900 (Rust) + ~165 (templates)

---

## Phase 4: Init Expansion

**Goal**: Expand `noslop init` to accept an optional agent argument and generate agent-specific config files.

**Source docs**: 07-init-expansion-design.md

**Depends on**: Phase 1 (AgentType), Phase 3 (ClaudeConfig, CodexConfig adapters)

### File-by-File Changes

#### MODIFY: `src/cli/app.rs` (~+5 lines)

Add `agent: Option<String>` field to `Command::Init` variant. Update dispatch to pass `agent` to handler.

```diff
 Init {
+    /// AI agent to configure (e.g., claude, codex)
+    agent: Option<String>,
+
     #[arg(short, long)]
     force: bool,
 },
```

```diff
-Some(Command::Init { force }) => commands::init(force, output_mode),
+Some(Command::Init { agent, force }) => commands::init(agent, force, output_mode),
```

#### MODIFY: `src/cli/commands/init.rs` (~+100 lines)

Major changes:

1. Signature becomes `pub fn init(agent: Option<String>, force: bool, _mode: OutputMode)`
2. `parse_agent_type()` function validates agent string
3. `build_noslop_toml()` conditionally includes `[agent]` section
4. `configure_agent()` calls `AgentConfig::validate()`, `generate_config()`, writes bundle
5. `write_config_bundle()` writes all files to `$HOME/<agent-dir>/`
6. `write_project_instructions()` creates CLAUDE.md or AGENTS.md at repo root

Source: 07-init-expansion-design.md, full "Init Command Implementation" section.

#### MODIFY: `src/cli/commands/mod.rs` (+0 lines, signature change only)

The `pub use init::init;` line stays the same. Only the internal signature changes.

#### MODIFY: `src/adapters/toml/parser.rs` (~+10 lines)

Add `AgentSection` struct and `agent: Option<AgentSection>` field to `NoslopFile`:

```rust
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AgentSection {
    #[serde(rename = "type")]
    pub agent_type: String,
}
```

#### MODIFY: `Cargo.toml` (+1 line)

Add `dirs = "6.0"` to `[dependencies]`.

### Phase 4 Verification

```bash
# Unit tests
cargo test init -- --nocapture
cargo test build_noslop_toml -- --nocapture
cargo test parse_agent_type -- --nocapture

# Integration tests
cd /tmp && git init test-init && cd test-init
noslop init              # No agent -- same as before
cat .noslop.toml         # Should NOT have [agent]

cd /tmp && git init test-init-claude && cd test-init-claude
noslop init claude       # With agent (requires claude CLI)
cat .noslop.toml         # Should have [agent] type = "claude"

cd /tmp && git init test-init-bad && cd test-init-bad
noslop init cursor       # Unknown agent -- should error
```

### Backward Compatibility

- `noslop init` (no agent) produces identical output to today
- Old `.noslop.toml` files without `[agent]` parse without error (`#[serde(default)]`)
- Old noslop binaries can read new `.noslop.toml` files with `[agent]` (serde ignores unknown fields)

### Phase 4 Scope

- **New files**: 0
- **Modified files**: 5
- **Estimated lines**: ~200 (net additions)

---

## Phase 5: Run Loop Core

**Goal**: Implement the pure domain services for the iteration loop: plan parsing, stuck detection, and prompt building. No I/O, no CLI.

**Source docs**: 06-run-loop-design.md (services sections)

**Depends on**: Phase 1 (all model types)

### File-by-File Changes

#### NEW: `src/core/services/plan_parser.rs` (~100 lines)

Pure parsing functions:

- `parse_plan(path, content) -> Result<Plan>` -- auto-detects markdown vs JSON
- `parse_markdown_tasks(content) -> Vec<Task>` -- regex `^\s*- \[([ xX])\]\s*(.+)$`
- `parse_json_tasks(content) -> Vec<Task>` -- walks JSON for `"passes"` fields
- `collect_json_tasks(value, tasks)` -- recursive JSON walker

Source: 06-run-loop-design.md, "Plan Parser" section.

#### NEW: `src/core/services/stuck_detector.rs` (~80 lines)

Pure stuck assessment:

- `assess_stuck(progress, latest, policy) -> StuckAssessment` -- combines task-count and diff-based detection
- `build_escalation_prompt(level, stuck_count, has_code_churn) -> Option<String>` -- 4-level graduated prompts
- `StuckAssessment` struct: level, stuck_count, has_code_churn, should_terminate, prompt_injection

Source: 06-run-loop-design.md, "Stuck Detector" section.

#### NEW: `src/core/services/prompt_builder.rs` (~150 lines)

Prompt construction:

- `PromptContext` struct (all inputs for one iteration's prompt)
- `build_iteration_prompt(ctx) -> String` -- ordered for prompt caching: static first, dynamic last
- `standard_template() -> String` -- the main per-iteration prompt (~60 lines of static text)
- `initializer_template() -> String` -- iteration 1 setup prompt (~40 lines of static text)

Source: 06-run-loop-design.md, "Prompt Builder" section.

#### NEW: `src/core/services/run_loop.rs` (~250 lines)

The orchestrator:

- `RunOutcome` enum: Complete, Stuck, MaxIterationsReached, Failed
- `RunLoop` struct holding `&dyn AgentRuntime`, `&dyn PlanStore`, `Option<&dyn TestRunner>`, `&dyn VersionControl`, `RunConfig`
- `RunLoop::execute() -> Result<RunOutcome>` -- the main loop (translates ralph's bash `for` loop)
- `parse_confidence(output) -> Option<u8>` -- extracts `[CONFIDENCE: N]`
- Private helpers: `load_or_create_progress`, `save_progress`, `load_failure_log`, `append_to_log`, `read_diagnostic_report`, `run_post_loop_review`

Source: 06-run-loop-design.md, "Run Loop Orchestrator" section.

Note: The private helpers contain `todo!()` stubs initially. They require filesystem adapters that are built in Phase 6.

#### MODIFY: `src/core/services/mod.rs` (+4 lines)

```rust
pub mod plan_parser;
pub mod prompt_builder;
pub mod run_loop;
pub mod stuck_detector;
```

#### MODIFY: `src/core/mod.rs` (+0-2 lines)

Verify services module is re-exported.

### Phase 5 Verification

All services are pure functions. Tests require no I/O:

```bash
cargo test plan_parser -- --nocapture
cargo test stuck_detector -- --nocapture
cargo test prompt_builder -- --nocapture
cargo test parse_confidence -- --nocapture
```

The `RunLoop::execute()` method will not compile without the Phase 6 adapters filling in the `todo!()` stubs, but `cargo check` should pass for the individual services.

### Phase 5 Scope

- **New files**: 4
- **Modified files**: 2
- **Estimated lines**: ~700 (including inline unit tests)

---

## Phase 6: Run Loop Adapters and CLI

**Goal**: Implement the I/O adapters for the run loop (plan store, test runner) and the `noslop run` CLI command. Wire everything together.

**Source docs**: 06-run-loop-design.md (adapters, CLI sections)

**Depends on**: Phase 2 (checkpoint via VersionControl), Phase 3 (AgentRuntime), Phase 5 (RunLoop, plan parser)

### File-by-File Changes

#### NEW: `src/adapters/plan/mod.rs` (~15 lines)

Module declarations, `detect_plan_store()` factory.

#### NEW: `src/adapters/plan/markdown.rs` (~80 lines)

`MarkdownPlanStore` implementing `PlanStore`:

- `load()` reads file, delegates to `plan_parser::parse_plan()`
- `save()` writes raw content back
- `update_task_status()` performs targeted regex replacement (`- [ ]` to `- [x]` or vice versa)

#### NEW: `src/adapters/plan/json.rs` (~80 lines)

`JsonPlanStore` implementing `PlanStore`:

- `load()` reads file, delegates to `plan_parser::parse_plan()`
- `save()` serializes and writes
- `update_task_status()` updates `"passes"` field in JSON

#### NEW: `src/adapters/test_runner/mod.rs` (~30 lines)

Module declarations, `detect_test_runner()` factory that checks for `Cargo.toml`, `package.json`, `pytest.ini`/`pyproject.toml`.

#### NEW: `src/adapters/test_runner/cargo.rs` (~50 lines)

`CargoTestRunner` implementing `TestRunner`:

- `run_tests()` executes `cargo test`, captures output, parses pass/fail counts
- `detect()` checks for `Cargo.toml` in working dir

#### NEW: `src/adapters/test_runner/npm.rs` (~50 lines)

`NpmTestRunner` implementing `TestRunner`:

- `run_tests()` executes `npm test`
- `detect()` checks for `package.json` with `scripts.test`

#### NEW: `src/cli/commands/run.rs` (~150 lines)

CLI entry point:

- `RunArgs` struct with clap derive (plan_file, max_iterations, --worktree, --agent, --no-review, --no-tests, --stuck-limit)
- `run(args)` function: resolves agent type, constructs concrete adapters, builds RunConfig, instantiates RunLoop, calls execute(), reports outcome
- `resolve_agent_type()` reads from CLI arg or `.noslop.toml [agent]` section

Source: 06-run-loop-design.md, "CLI Integration" section.

#### MODIFY: `src/adapters/mod.rs` (+2 lines)

Add `pub mod plan;` and `pub mod test_runner;`.

#### MODIFY: `src/cli/app.rs` (~+15 lines)

Add `Run(run::RunArgs)` variant to `Command` enum. Add dispatch arm.

#### MODIFY: `src/cli/commands/mod.rs` (+2 lines)

Add `pub mod run;` and re-export.

### Run Loop I/O Completion

Phase 6 also fills in the `todo!()` stubs in `RunLoop`:

- `load_or_create_progress()` -- reads `.noslop/progress.json` or creates new `ProgressFile`
- `save_progress()` -- serializes to `.noslop/progress.json`
- `load_failure_log()` -- reads `.noslop/failures.jsonl` (one JSON per line)
- `append_to_log()` -- appends to `.ralph-log`
- `read_diagnostic_report()` -- reads `.noslop/stuck-report.md` if exists

These are direct filesystem operations in the `RunLoop` impl (not behind a port trait, since they are noslop-internal files, not a swappable boundary).

### Phase 6 Verification

```bash
# Unit tests for adapters
cargo test markdown_plan -- --nocapture
cargo test json_plan -- --nocapture
cargo test cargo_test_runner -- --nocapture

# Full integration (requires an agent CLI)
cd /tmp && git init test-run && cd test-run
noslop init claude
echo "- [ ] Create README\n- [ ] Add tests" > plan.md
noslop run plan.md --max-iterations 2
```

### Phase 6 Scope

- **New files**: 7
- **Modified files**: 3
- **Estimated lines**: ~600

---

## Phase 7: Integration Testing and Polish

**Goal**: End-to-end tests, mock-based RunLoop tests, update test infrastructure, and documentation.

**Source docs**: All design docs (test sections)

**Depends on**: All previous phases

### File-by-File Changes

#### NEW: `tests/adapter/checkpoint_test.rs` (~80 lines)

Tests for `checkpoint::is_clean()`, `checkpoint::checkpoint()` using `tempfile::TempDir` + `git2::Repository::init()`.

Source: 05-checkpoint-worktree-design.md, "Unit Tests" section.

#### NEW: `tests/adapter/worktree_test.rs` (~100 lines)

Tests for `worktree_base_dir()`, `list_worktrees()`, `create_worktree()`, `is_branch_merged()` using temp repos.

Source: 05-checkpoint-worktree-design.md, "Worktree adapter tests".

#### NEW: `tests/unit/plan_parser_test.rs` (~80 lines)

Tests for markdown and JSON plan parsing, edge cases (empty plans, mixed formats, nested JSON).

Source: 06-run-loop-design.md, "Unit Tests" section.

#### NEW: `tests/unit/stuck_detector_test.rs` (~60 lines)

Tests for graduated escalation levels, stuck count transitions, code churn detection.

#### NEW: `tests/integration/run_test.rs` (~100 lines)

Full `noslop run` integration tests using mock agents (via env var to swap in a test harness) or `#[ignore]` tests that require real agent CLIs.

#### MODIFY: `tests/common/mocks.rs` (~+30 lines)

Add `MockVersionControl` methods for the 8 new `VersionControl` trait methods.

#### MODIFY: `tests/adapter/mod.rs` (+2 lines)

Add `mod checkpoint_test;` and `mod worktree_test;`.

### Polish Tasks

1. **Clippy clean**: Run `cargo clippy -- -D warnings` and fix all new warnings
2. **Doc comments**: Ensure all public types and functions have `///` doc comments
3. **Missing trait impls**: Verify `Debug`, `Clone`, `Copy` where appropriate (clippy will catch missing_debug_implementations)
4. **Error messages**: Review all `anyhow::bail!` messages for clarity
5. **Output formatting**: Consistent human/JSON output across checkpoint, worktree, run commands

### Phase 7 Scope

- **New files**: 5
- **Modified files**: 2
- **Estimated lines**: ~500

---

## Setup Repo Deprecation Path

### Timeline

| Stage                 | When          | Action                                                                                                                |
| --------------------- | ------------- | --------------------------------------------------------------------------------------------------------------------- |
| 1. Parallel operation | After Phase 4 | Both repos work. `noslop init claude` generates what `setup/install.sh` used to symlink. Users can migrate gradually. |
| 2. Feature parity     | After Phase 6 | `noslop run` replaces `ralph`. `noslop checkpoint` and `noslop worktree` replace the bash scripts.                    |
| 3. Deprecation notice | After Phase 7 | Add notice to setup's README: "This repo is deprecated. Use `noslop init <agent>` instead."                           |
| 4. Migration guide    | +1 week       | Document exact steps for existing setup users to switch to noslop.                                                    |
| 5. Archive            | +1 month      | Archive the setup repo. Remove symlinks documentation.                                                                |

### Migration Steps for Existing Setup Users

```bash
# 1. Remove setup symlinks
cd ~/Code/setup
./install.sh --uninstall  # or manually remove symlinks

# 2. Remove PATH entry
# Edit ~/.zshrc or ~/.bashrc, remove the line:
#   export PATH="$HOME/.claude/scripts:$PATH"

# 3. Install noslop (if not already)
cargo install noslop

# 4. Initialize with agent support
cd ~/Code/your-project
noslop init claude --force

# 5. Verify
noslop checkpoint "test migration"
noslop worktree list
```

### What Gets Replaced

| Setup Component      | Noslop Replacement        | Notes                                                                                      |
| -------------------- | ------------------------- | ------------------------------------------------------------------------------------------ |
| `scripts/ralph`      | `noslop run`              | Same iteration loop, more features (stuck escalation, test gates, failure memory)          |
| `scripts/checkpoint` | `noslop checkpoint`       | Same interface. Drops multi-repo mode.                                                     |
| `scripts/worktree`   | `noslop worktree`         | Same interface. Drops multi-repo mode and `--repo` flag.                                   |
| `scripts/_lib.sh`    | Built into adapters       | `is_clean` -> checkpoint adapter; `count_tasks` -> plan_parser; `git_activity` -> run loop |
| `install.sh`         | `noslop init claude`      | Generates all config files instead of symlinking                                           |
| `settings.json`      | Generated by ClaudeConfig | Identical content, written to `~/.claude/settings.json`                                    |
| `CLAUDE.global.md`   | Generated by ClaudeConfig | Updated content referencing noslop commands                                                |
| `hooks/*.sh`         | Generated by ClaudeConfig | Identical content, written to `~/.claude/hooks/`                                           |
| `commands/*.md`      | Generated by ClaudeConfig | Updated content referencing noslop commands                                                |

### What Is NOT Replaced

| Setup Component              | Status                        | Reason                                                                     |
| ---------------------------- | ----------------------------- | -------------------------------------------------------------------------- |
| `ralph-research-findings.md` | Incorporated into design docs | Research is consumed, not migrated                                         |
| `ralph-research-plan.md`     | Consumed                      | Planning artifact                                                          |
| Multi-repo mode              | Intentionally dropped         | noslop is single-repo; use git submodules or scripted loops for multi-repo |
| `--repo` flag                | Intentionally dropped         | Consequence of dropping multi-repo                                         |

---

## Risk Register

| Risk                                                       | Likelihood | Impact | Mitigation                                                                                              |
| ---------------------------------------------------------- | ---------- | ------ | ------------------------------------------------------------------------------------------------------- |
| `git2` worktree API limitations                            | Medium     | High   | Fall back to `std::process::Command("git worktree ...")` for operations git2 does not support cleanly   |
| Agent CLI interface changes                                | Low        | Medium | Adapter pattern isolates changes to one file per agent                                                  |
| `dirs` crate version conflicts                             | Low        | Low    | `dirs` v6.0 has no transitive deps that conflict with existing Cargo.toml                               |
| Clippy pedantic rejections on new code                     | High       | Low    | Address iteratively; `#[allow]` pragmatically where clippy is wrong                                     |
| Test flakiness in worktree tests (filesystem timing)       | Medium     | Low    | Use `tempfile::TempDir` with unique paths; add cleanup in test teardown                                 |
| Progress/failure files corrupted by concurrent noslop runs | Low        | Medium | Document that `noslop run` should not be invoked concurrently on the same repo. Future: add a lockfile. |

---

## Quick Reference: All New Files

Sorted by implementation order within each phase.

```
Phase 1 (8 new files):
  src/core/models/agent.rs
  src/core/models/worktree.rs
  src/core/models/plan.rs
  src/core/models/progress.rs
  src/core/models/failure.rs
  src/core/models/run_config.rs
  src/core/ports/agent.rs
  src/core/ports/plan_store.rs
  src/core/ports/test_runner.rs

Phase 2 (4 new files):
  src/adapters/git/checkpoint.rs
  src/adapters/git/worktree.rs
  src/cli/commands/checkpoint.rs
  src/cli/commands/worktree.rs

Phase 3 (10 new files):
  src/adapters/agent/mod.rs
  src/adapters/agent/claude.rs
  src/adapters/agent/codex.rs
  src/adapters/agent/templates/claude_global_instructions.md
  src/adapters/agent/templates/claude_branch_protection.sh
  src/adapters/agent/templates/claude_auto_format.sh
  src/adapters/agent/templates/claude_cmd_checkpoint.md
  src/adapters/agent/templates/claude_cmd_worktree.md
  src/adapters/agent/templates/codex_instructions.md

Phase 4 (0 new files):
  (modifications only)

Phase 5 (4 new files):
  src/core/services/plan_parser.rs
  src/core/services/stuck_detector.rs
  src/core/services/prompt_builder.rs
  src/core/services/run_loop.rs

Phase 6 (7 new files):
  src/adapters/plan/mod.rs
  src/adapters/plan/markdown.rs
  src/adapters/plan/json.rs
  src/adapters/test_runner/mod.rs
  src/adapters/test_runner/cargo.rs
  src/adapters/test_runner/npm.rs
  src/cli/commands/run.rs

Phase 7 (5 new files):
  tests/adapter/checkpoint_test.rs
  tests/adapter/worktree_test.rs
  tests/unit/plan_parser_test.rs
  tests/unit/stuck_detector_test.rs
  tests/integration/run_test.rs
```

**Total**: 38 new files + ~15 modified files = ~53 file touches across 7 phases.

---

## Quick Reference: All Modified Files

```
Phase 1:
  src/core/models/mod.rs       (add module declarations + re-exports)
  src/core/ports/mod.rs        (add module declarations + re-exports)
  src/lib.rs                   (verify re-exports)

Phase 2:
  src/core/ports/vcs.rs        (extend VersionControl trait)
  src/adapters/git/mod.rs      (add modules + implement trait methods)
  src/cli/app.rs               (add Checkpoint + Worktree commands)
  src/cli/commands/mod.rs      (add module declarations)

Phase 3:
  src/adapters/mod.rs          (add agent module)
  src/lib.rs                   (ensure agent re-exports)

Phase 4:
  src/cli/app.rs               (add agent arg to Init)
  src/cli/commands/init.rs     (Phase 2 agent setup)
  src/cli/commands/mod.rs      (signature update)
  src/adapters/toml/parser.rs  (AgentSection)
  Cargo.toml                   (add dirs dependency)

Phase 5:
  src/core/services/mod.rs     (add module declarations)
  src/core/mod.rs              (verify re-exports)

Phase 6:
  src/adapters/mod.rs          (add plan + test_runner modules)
  src/cli/app.rs               (add Run command)
  src/cli/commands/mod.rs      (add run module)

Phase 7:
  tests/common/mocks.rs        (update MockVersionControl)
  tests/adapter/mod.rs         (add test modules)
```
