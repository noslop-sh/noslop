# Implementation Plan: Review Pipeline Architecture

Synthesis of design documents 03-08 into an ordered, actionable implementation plan. A developer should be able to pick up this document and start coding Phase 1 immediately.

## Table of Contents

- [Dependency Analysis](#dependency-analysis)
- [Current Codebase Baseline](#current-codebase-baseline)
- [Phase Overview](#phase-overview)
- [Phase 1: Core Types and Port Traits](#phase-1-core-types-and-port-traits)
- [Phase 2: Checkpoint and VCS Extensions](#phase-2-checkpoint-and-vcs-extensions)
- [Phase 3: Agent Adapters](#phase-3-agent-adapters)
- [Phase 4: Init Expansion](#phase-4-init-expansion)
- [Phase 5: Review Pipeline Core](#phase-5-review-pipeline-core)
- [Phase 6: Review CLI and Finding Management](#phase-6-review-cli-and-finding-management)
- [Phase 7: Integration Testing and Polish](#phase-7-integration-testing-and-polish)
- [Quick Reference: All New Files](#quick-reference-all-new-files)
- [Quick Reference: All Modified Files](#quick-reference-all-modified-files)
- [Risk Register](#risk-register)

---

## Dependency Analysis

### Cargo.toml: Current Dependencies (No Changes Required)

Every crate needed for the review pipeline is already in `Cargo.toml`. No new runtime or dev dependencies.

| Crate        | Current Version           | Used By (New Code)                                                                               |
| ------------ | ------------------------- | ------------------------------------------------------------------------------------------------ |
| `git2`       | 0.20.2 (vendored-openssl) | Checkpoint (stage-all, commit), VCS extensions (diff_between, commits_between, file_at_revision) |
| `chrono`     | 0.4.42 (serde)            | Checkpoint timestamps, CommitInfo timestamps, review session timestamps                          |
| `clap`       | 4.5.51 (derive, cargo)    | New subcommands: Checkpoint, Review, Findings; Init agent argument                               |
| `regex`      | 1.12                      | Output parsing (finding extraction), analyzer pattern matching                                   |
| `serde`      | 1.0.228 (derive)          | AgentType, Finding, ReviewSession, ReviewConfig, AgentSection, ReviewSection                     |
| `serde_json` | 1.0.145                   | Settings.json generation, finding JSON parsing, review session files, JSON output mode           |
| `toml`       | 0.9.8                     | NoslopFile `[agent]`/`[review]` section parsing, `.noslop.toml` generation                       |
| `anyhow`     | 1.0.100                   | All new functions return `anyhow::Result<T>`                                                     |
| `thiserror`  | 2.0.17                    | Custom error types if needed (none required in current designs)                                  |
| `colored`    | 3.0.0                     | Review pipeline output, finding severity coloring                                                |
| `log`        | 0.4.28                    | Debug logging in adapters and pipeline                                                           |
| `walkdir`    | 2.5.0                     | Not directly needed by new code, but already available                                           |
| `glob`       | 0.3.3                     | ConventionAnalyzer target matching (existing usage, no changes)                                  |

**Dev dependencies** -- all already present:

| Crate                   | Used By (New Tests)                                                     |
| ----------------------- | ----------------------------------------------------------------------- |
| `mockall` 0.13          | MockAgentRuntime, MockAgentConfig, MockReviewAnalyzer, MockFindingStore |
| `tempfile` 3.23.0       | Checkpoint adapter tests, VCS extension tests, integration tests        |
| `assert_cmd` 2.1.1      | CLI integration tests for checkpoint, review, findings                  |
| `predicates` 3.1.3      | Output assertions in integration tests                                  |
| `pretty_assertions` 1.4 | Struct comparison in unit tests                                         |
| `test-case` 3.3         | Parameterized tests for output parsing, analyzer configs                |
| `proptest` 1.5          | Property tests for finding parser edge cases                            |

### Cargo.toml: One Optional New Dependency

The `dirs` crate is needed by `configure_agent()` to resolve `$HOME` for writing agent config files (`~/.claude/settings.json`, etc.). Currently the codebase does not use `dirs`.

```toml
# Add to [dependencies]:
dirs = "6.0"
```

This is the **only** new dependency. It is small (no transitive deps beyond `libc`/`windows-sys`), widely used, and required because `std::env::home_dir()` is deprecated.

### Feature Flags

No new feature flags are needed. The existing `llm` feature (tokio + reqwest) is orthogonal to the review pipeline. Agent invocation uses `std::process::Command` (synchronous), not async HTTP.

---

## Current Codebase Baseline

### Source Files (46 .rs files)

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

| Phase     | Description                       | New Files | Modified Files | Est. Lines | Depends On    |
| --------- | --------------------------------- | --------- | -------------- | ---------- | ------------- |
| 1         | Core types and port traits        | 6         | 3              | ~500       | Nothing       |
| 2         | Checkpoint and VCS extensions     | 3         | 4              | ~450       | Phase 1       |
| 3         | Agent adapters (Claude + Codex)   | 12        | 2              | ~1,000     | Phase 1       |
| 4         | Init expansion                    | 0         | 5              | ~200       | Phase 1, 3    |
| 5         | Review pipeline core              | 5         | 2              | ~700       | Phase 1       |
| 6         | Review CLI and finding management | 3         | 4              | ~550       | Phase 2, 3, 5 |
| 7         | Integration testing and polish    | 5         | 2              | ~500       | All           |
| **Total** |                                   | **34**    | **~16**        | **~3,900** |               |

### Dependency Graph

```
Phase 1 (types + ports)
  |
  +---> Phase 2 (checkpoint + VCS extensions)
  |       |
  +---> Phase 3 (agent adapters) ----------+
  |       |                                |
  |       +---> Phase 4 (init expansion) <-+
  |
  +---> Phase 5 (review pipeline core)
          |
          +---> Phase 6 (review CLI + findings) <--- Phase 2, 3
                  |
                  +---> Phase 7 (integration testing)
```

Phases 2, 3, and 5 can be worked on **in parallel** after Phase 1 completes. Phase 4 requires both 1 and 3. Phase 6 requires 2, 3, and 5. Phase 7 is the final sweep.

---

## Phase 1: Core Types and Port Traits

**Goal**: Define all new domain models and port trait interfaces so that subsequent phases can code against stable contracts.

**Source docs**: 03-agent-port-design.md (agent types, Finding, Severity, ReviewContext, ReviewAnalyzer, AgentConfig, AgentRuntime), 05-checkpoint-worktree-design.md (FileDiff, CommitInfo, DiffStat), 08-config-design.md (ReviewConfig, AnalyzerEntry)

### File-by-File Changes

#### NEW: `src/core/models/finding.rs` (~80 lines)

Core review finding types: `Severity` enum (Info, Warning, Error, Critical), `Finding` struct, `FindingSource` struct, `Resolution` struct. All with `Debug`, `Clone`, `Serialize`, `Deserialize`.

Source: 03-agent-port-design.md, "Finding and Review Types" section.

```rust
// Key types:
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Info,
    Warning,
    Error,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub severity: Severity,
    pub file: String,
    pub line: Option<u32>,
    pub message: String,
    pub suggestion: Option<String>,
    pub source: String,
}
```

#### NEW: `src/core/models/review_config.rs` (~100 lines)

Review pipeline configuration types: `ReviewConfig`, `AnalyzerEntry` enum (BuiltIn, Script, Agent variants), `ReviewSession`, `ReviewTarget`. Includes the `ReviewContext` struct used by all analyzers.

Source: 03-agent-port-design.md "ReviewContext"; 08-config-design.md "ReviewSection to ReviewConfig" section.

```rust
// Key types:
#[derive(Debug, Clone)]
pub struct ReviewContext {
    pub diff: String,
    pub changed_files: Vec<String>,
    pub repo_root: PathBuf,
    pub commit_range: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ReviewConfig {
    pub analyzer_entries: Vec<AnalyzerEntry>,
    pub co_change: CoChangeConfig,
    pub formatting: FormattingConfig,
}

#[derive(Debug, Clone)]
pub enum AnalyzerEntry {
    BuiltIn { name: String },
    Script { name: String, command: String },
    Agent { name: String, focus: String, agent: String, prompt_template: Option<String> },
}
```

#### NEW: `src/core/models/diff.rs` (~100 lines)

VCS diff model types for the review pipeline: `FileDiff`, `DiffStatus` enum (Added, Modified, Deleted, Renamed), `DiffHunk`, `DiffLine`, `DiffLineKind` enum (Context, Addition, Deletion), `CommitInfo`, `DiffStat`.

Source: 05-checkpoint-worktree-design.md, "New Model Types" section.

```rust
// Key types:
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileDiff {
    pub path: PathBuf,
    pub old_path: Option<PathBuf>,
    pub status: DiffStatus,
    pub hunks: Vec<DiffHunk>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitInfo {
    pub short_sha: String,
    pub sha: String,
    pub summary: String,
    pub message: String,
    pub author: String,
    pub timestamp: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffStat {
    pub files_changed: usize,
    pub insertions: usize,
    pub deletions: usize,
}
```

#### NEW: `src/core/ports/agent.rs` (~250 lines)

All agent port types and both trait definitions:

- `AgentType` enum with `FromStr`, `Display`, `Serialize`, `Deserialize`
- Configuration types: `GlobalInstructions`, `HookDefinition`, `SettingsFile`, `SlashCommand`, `AgentConfigBundle`
- Invocation types: `InvocationConfig`, `InvocationResult`, `ReviewOutput`
- Traits: `AgentConfig`, `AgentRuntime`

Source: 03-agent-port-design.md, "Complete Module Definition" section.

```rust
// Key trait signatures:
#[cfg_attr(test, mockall::automock)]
pub trait AgentConfig: Send + Sync {
    fn agent_type(&self) -> AgentType;
    fn generate_config(&self, project_root: &Path) -> anyhow::Result<AgentConfigBundle>;
    fn is_available(&self) -> bool;
    fn validate(&self) -> anyhow::Result<()>;
    fn install_instructions(&self) -> &str;
    fn generate_project_instructions(&self, project_root: &Path) -> anyhow::Result<Option<String>>;
}

#[cfg_attr(test, mockall::automock)]
pub trait AgentRuntime: Send + Sync {
    fn agent_type(&self) -> AgentType;
    fn invoke(&self, config: &InvocationConfig) -> anyhow::Result<InvocationResult>;
    fn parse_output(&self, result: &InvocationResult) -> ReviewOutput;
    fn is_available(&self) -> bool;
    fn build_command(&self, config: &InvocationConfig) -> (String, Vec<String>);
}
```

#### NEW: `src/core/ports/analyzer.rs` (~40 lines)

`ReviewAnalyzer` trait and `ContextKind` enum. The core abstraction for the review pipeline: each analyzer receives context and prior findings, returns new findings.

Source: 03-agent-port-design.md, "The ReviewAnalyzer Trait and AgentAnalyzer Bridge" section.

```rust
/// A review analyzer that produces findings from code changes.
#[cfg_attr(test, mockall::automock)]
pub trait ReviewAnalyzer: Send + Sync {
    fn required_context(&self) -> Vec<ContextKind>;
    fn analyze(&self, context: &ReviewContext, prior_findings: &[Finding]) -> anyhow::Result<Vec<Finding>>;
    fn name(&self) -> &str;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextKind {
    Diff,
    ChangedFiles,
    FileContents,
    CommitHistory,
}
```

#### NEW: `src/core/ports/finding_store.rs` (~30 lines)

`FindingStore` trait for persisting review findings. Follows the `CheckRepository` pattern.

Source: 01-noslop-analysis.md, "Key Extension Points" section.

```rust
#[cfg_attr(test, mockall::automock)]
pub trait FindingStore: Send + Sync {
    fn save_session(&self, session: &ReviewSession) -> anyhow::Result<()>;
    fn load_session(&self, id: &str) -> anyhow::Result<Option<ReviewSession>>;
    fn list_sessions(&self) -> anyhow::Result<Vec<String>>;
    fn find_unresolved_blocking(&self) -> anyhow::Result<Vec<Finding>>;
}
```

#### MODIFY: `src/core/models/mod.rs` (+6 lines)

Add module declarations and re-exports for new model files:

```rust
mod diff;
mod finding;
mod review_config;

pub use diff::{FileDiff, DiffStatus, DiffHunk, DiffLine, DiffLineKind, CommitInfo, DiffStat};
pub use finding::{Finding, Severity};
pub use review_config::{ReviewContext, ReviewConfig, AnalyzerEntry, ReviewSession, ReviewTarget};
```

#### MODIFY: `src/core/ports/mod.rs` (+6 lines)

Add module declarations and re-exports:

```rust
mod agent;
mod analyzer;
mod finding_store;

pub use agent::{
    AgentConfig, AgentConfigBundle, AgentRuntime, AgentType,
    GlobalInstructions, HookDefinition, InvocationConfig,
    InvocationResult, ReviewOutput, SettingsFile, SlashCommand,
};
pub use analyzer::{ReviewAnalyzer, ContextKind};
pub use finding_store::FindingStore;
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

- **New files**: 6
- **Modified files**: 3
- **Estimated lines**: ~500
- **Tests**: None yet (types and traits only; tests come with implementations)

---

## Phase 2: Checkpoint and VCS Extensions

**Goal**: Implement the `noslop checkpoint` convenience command and extend the `VersionControl` trait with diff/history methods that serve the review pipeline.

**Source docs**: 05-checkpoint-worktree-design.md

**Depends on**: Phase 1 (for `CommitInfo`, `FileDiff`, `DiffStat` model types and `VersionControl` trait)

### File-by-File Changes

#### NEW: `src/adapters/git/checkpoint.rs` (~60 lines)

Functions: `is_clean(&Repository) -> Result<bool>`, `commit_all(&Repository, &str) -> Result<Option<String>>`, `default_signature()`.

Uses `git2::StatusOptions`, `Index::add_all`, `Index::update_all`, `Repository::commit`. No hooks invoked (git2 skips hooks by default).

Source: 05-checkpoint-worktree-design.md, "Adapter Implementations" > checkpoint.

```rust
pub fn is_clean(repo: &Repository) -> anyhow::Result<bool> {
    let mut opts = StatusOptions::new();
    opts.include_untracked(true).recurse_untracked_dirs(true);
    let statuses = repo.statuses(Some(&mut opts))?;
    Ok(statuses.is_empty())
}

pub fn commit_all(repo: &Repository, message: &str) -> anyhow::Result<Option<String>> {
    if is_clean(repo)? { return Ok(None); }
    let mut index = repo.index()?;
    index.add_all(["*"], git2::IndexAddOption::DEFAULT, None)?;
    index.update_all(["*"], None)?;
    index.write()?;
    let tree_oid = index.write_tree()?;
    let tree = repo.find_tree(tree_oid)?;
    let parent = repo.head()?.peel_to_commit()?;
    let signature = default_signature(repo)?;
    let commit_oid = repo.commit(Some("HEAD"), &signature, &signature, message, &tree, &[&parent])?;
    Ok(Some(commit_oid.to_string()[..7].to_string()))
}
```

#### NEW: `src/cli/commands/checkpoint.rs` (~50 lines)

Command handler: resolves message (default = timestamp), calls `vcs.commit_all()`, outputs human/JSON result.

Source: 05-checkpoint-worktree-design.md, "Command Implementations" > checkpoint.

```rust
pub fn checkpoint(message: Option<&str>, mode: OutputMode) -> anyhow::Result<()> {
    let vcs = GitVersionControl::current_dir()?;
    let msg = message.map(String::from).unwrap_or_else(|| {
        format!("checkpoint: {}", Local::now().format("%Y-%m-%d %H:%M:%S"))
    });
    match vcs.commit_all(&msg)? {
        Some(short_sha) => { /* output success */ }
        None => { /* output "nothing to checkpoint" */ }
    }
    Ok(())
}
```

#### NEW: `src/core/models/diff.rs`

Already created in Phase 1. No additional changes needed in Phase 2.

#### MODIFY: `src/core/ports/vcs.rs` (~+45 lines)

Extend `VersionControl` trait with checkpoint + review pipeline methods:

```rust
// Checkpoint methods
fn is_clean(&self) -> anyhow::Result<bool>;
fn commit_all(&self, message: &str) -> anyhow::Result<Option<String>>;

// Review pipeline methods
fn diff_between(&self, base: &str, head: &str) -> anyhow::Result<Vec<FileDiff>>;
fn commits_between(&self, base: &str, head: &str) -> anyhow::Result<Vec<CommitInfo>>;
fn file_at_revision(&self, path: &Path, rev: &str) -> anyhow::Result<String>;
fn recent_commits(&self, count: usize) -> anyhow::Result<Vec<CommitInfo>>;
fn diff_stat_since(&self, since: &str) -> anyhow::Result<DiffStat>;
```

Source: 05-checkpoint-worktree-design.md, "Extended VersionControl Trait" section.

#### MODIFY: `src/adapters/git/mod.rs` (~+120 lines)

- Add `pub mod checkpoint;`
- Implement the 7 new `VersionControl` methods on `GitVersionControl`:
  - `is_clean()` delegates to `checkpoint::is_clean()`
  - `commit_all()` delegates to `checkpoint::commit_all()`
  - `diff_between()` uses `repo.diff_tree_to_tree()`, converts `git2::Diff` to `Vec<FileDiff>`
  - `commits_between()` uses `repo.revwalk()` with push/hide, converts to `Vec<CommitInfo>`
  - `file_at_revision()` uses `tree.get_path()` + `find_blob()`
  - `recent_commits()` uses `repo.revwalk()` from HEAD with `.take(count)`
  - `diff_stat_since()` uses `diff.stats()`

Source: 05-checkpoint-worktree-design.md, "Review Pipeline Methods" section.

#### MODIFY: `src/cli/app.rs` (~+10 lines)

Add `Checkpoint` variant to the `Command` enum. Add dispatch arm in `run()`.

```rust
/// Safety commit all changes (staged + untracked)
Checkpoint {
    /// Commit message (default: "checkpoint: <timestamp>")
    message: Option<String>,
},
```

#### MODIFY: `src/cli/commands/mod.rs` (~+2 lines)

Add `mod checkpoint;` declaration and `pub use checkpoint::checkpoint;` re-export.

### Design Decisions

- **Worktree management deferred**. The review tool does not need isolated worktrees. See 05-checkpoint-worktree-design.md for rationale.
- **git2 preferred over Command**. Checkpoint and diff operations are hot-path. git2 avoids subprocess overhead and hook execution.
- **VersionControl trait extended** (not a new trait). All operations are reads/writes against a single git repository, sharing the same context as `staged_files`, `repo_root`, and `current_branch`.

### Phase 2 Verification

```bash
# Unit tests
cargo test checkpoint -- --nocapture
cargo test diff_between -- --nocapture
cargo test commits_between -- --nocapture

# Manual CLI test
cd /tmp && git init test-repo && cd test-repo
echo "hello" > file.txt
noslop checkpoint "test"
# Expected: "Checkpoint: a1b2c3d test"

noslop checkpoint
# Expected: "Nothing to checkpoint (working tree clean)."
```

### Phase 2 Scope

- **New files**: 2 (`src/adapters/git/checkpoint.rs`, `src/cli/commands/checkpoint.rs`; `src/core/models/diff.rs` already in Phase 1)
- **Modified files**: 4
- **Estimated lines**: ~450 (implementation + inline tests)

---

## Phase 3: Agent Adapters

**Goal**: Implement `ClaudeConfig`, `ClaudeRuntime`, `CodexConfig`, `CodexRuntime` adapters and the shared finding parser. These adapters power the LLM-based review passes.

**Source docs**: 04-agent-adapters-design.md

**Depends on**: Phase 1 (for port traits and types)

### File-by-File Changes

#### NEW: `src/adapters/agent/mod.rs` (~30 lines)

Module declarations, re-exports, factory functions:

- `get_agent_config(AgentType) -> Box<dyn AgentConfig>`
- `get_agent_runtime(AgentType) -> Box<dyn AgentRuntime>`

Source: 04-agent-adapters-design.md, "Module Root" section.

```rust
pub fn get_agent_config(agent_type: AgentType) -> Box<dyn AgentConfig> {
    match agent_type {
        AgentType::Claude => Box::new(ClaudeConfig::new()),
        AgentType::Codex => Box::new(CodexConfig::new()),
    }
}

pub fn get_agent_runtime(agent_type: AgentType) -> Box<dyn AgentRuntime> {
    match agent_type {
        AgentType::Claude => Box::new(ClaudeRuntime::new()),
        AgentType::Codex => Box::new(CodexRuntime::new()),
    }
}
```

#### NEW: `src/adapters/agent/parsing.rs` (~80 lines)

Shared finding extraction from raw agent output:

- `extract_findings(output: &str) -> ReviewOutput` -- finds JSON array in output, parses as `Vec<Finding>`
- `extract_json_array(output: &str) -> Option<String>` -- bracket-matching JSON extractor
- `extract_errors(output: &str) -> Vec<String>` -- error pattern scanner

Source: 04-agent-adapters-design.md, "Shared Finding Parser" section.

```rust
pub fn extract_findings(output: &str) -> ReviewOutput {
    let findings = extract_json_array(output)
        .and_then(|json_str| serde_json::from_str::<Vec<Finding>>(&json_str).ok())
        .unwrap_or_default();
    let errors = extract_errors(output);
    ReviewOutput { findings, raw_output: output.to_string(), errors }
}
```

#### NEW: `src/adapters/agent/claude.rs` (~400 lines)

`ClaudeConfig` struct implementing `AgentConfig`:

- `generate_config()` produces `AgentConfigBundle` with settings.json, CLAUDE.md, hooks, slash commands
- `build_settings()` constructs settings.json via `serde_json::json!` (97 allow rules, deny rules, hooks, sandbox)
- `build_permissions()`, `build_hooks()`, `build_sandbox()` helpers
- `is_available()` checks for `claude --version`
- `generate_project_instructions()` returns CLAUDE.md content for repo root

`ClaudeRuntime` struct implementing `AgentRuntime`:

- `build_command()` produces `claude -p "$PROMPT" --output-format text --permission-mode bypassPermissions`
- `invoke()` executes via `std::process::Command`, captures stdout/stderr, records duration
- `parse_output()` delegates to `parsing::extract_findings()`

Source: 04-agent-adapters-design.md, full "Claude Adapter" section.

#### NEW: `src/adapters/agent/codex.rs` (~200 lines)

`CodexConfig` struct implementing `AgentConfig`:

- `generate_config()` produces bundle with `instructions.md` only (no settings, hooks, or commands)
- `is_available()` checks for `codex --version`
- `generate_project_instructions()` returns AGENTS.md content

`CodexRuntime` struct implementing `AgentRuntime`:

- `build_command()` produces `codex --quiet --approval-mode full-auto "$PROMPT"`
- `invoke()` and `parse_output()` use same patterns as Claude

Source: 04-agent-adapters-design.md, full "Codex Adapter" section.

#### NEW: Template files (6 files, ~300 lines total)

```
src/adapters/agent/templates/
  claude_global_instructions.md     (~40 lines)  -- CLAUDE.md referencing noslop review commands
  claude_branch_protection.sh       (~15 lines)  -- PreToolUse hook blocking edits on protected branches
  claude_auto_format.sh             (~30 lines)  -- PostToolUse hook routing to rustfmt/prettier/ruff
  claude_cmd_checkpoint.md          (~30 lines)  -- /checkpoint slash command definition
  claude_cmd_worktree.md            (~20 lines)  -- /worktree slash command definition
  codex_instructions.md             (~30 lines)  -- Codex global instructions referencing noslop
```

Embedded at compile time via `include_str!()`. Content extracted from 04-agent-adapters-design.md "Generated Content" subsections.

#### NEW: Prompt template files (3 files, ~100 lines total)

```
src/adapters/agent/templates/prompts/
  security.md                       (~35 lines)  -- Security focus review prompt template
  quality.md                        (~35 lines)  -- Quality focus review prompt template
  architecture.md                   (~35 lines)  -- Architecture focus review prompt template
```

Default prompt templates with `{diff}`, `{prior_findings}`, `{focus}`, `{changed_files}` placeholders. Users can override by placing files in `.noslop/prompts/`.

Source: 04-agent-adapters-design.md, "Review Prompt Templates" section.

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
cargo test extract_findings -- --nocapture
cargo test parse_output -- --nocapture

# Integration tests (require agent CLIs, run with --ignored)
cargo test -- --ignored agent
```

### Phase 3 Scope

- **New files**: 12 (3 Rust + 6 templates + 3 prompt templates)
- **Modified files**: 2
- **Estimated lines**: ~1,000 (Rust) + ~265 (templates)

---

## Phase 4: Init Expansion

**Goal**: Expand `noslop init` to accept an optional agent argument, generate agent-specific config files, and write `[agent]`/`[review]` sections to `.noslop.toml`.

**Source docs**: 07-init-expansion-design.md, 08-config-design.md

**Depends on**: Phase 1 (AgentType), Phase 3 (ClaudeConfig, CodexConfig adapters)

### File-by-File Changes

#### MODIFY: `src/cli/app.rs` (~+5 lines)

Add `agent: Option<String>` field to `Command::Init` variant. Update dispatch to pass `agent` to handler.

```diff
 Init {
+    /// AI agent to use for LLM-based review (e.g., claude, codex)
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

#### MODIFY: `src/cli/commands/init.rs` (~+120 lines)

Major changes:

1. Signature becomes `pub fn init(agent: Option<String>, force: bool, _mode: OutputMode)`
2. `parse_agent_type()` function validates agent string via `AgentType::from_str()`
3. `build_noslop_toml()` conditionally includes `[agent]` and `[review]` sections
4. `configure_agent()` calls `AgentConfig::validate()`, `generate_config()`, writes bundle
5. `write_config_bundle()` writes global instructions and settings to `$HOME/<agent-dir>/`
6. `write_project_instructions()` creates CLAUDE.md or AGENTS.md at repo root
7. `install_pre_push()` installs pre-push hook that runs `noslop review --check`

Source: 07-init-expansion-design.md, full "Init Command Implementation" section.

```rust
pub fn init(agent: Option<String>, force: bool, _mode: OutputMode) -> anyhow::Result<()> {
    // Phase 1: Core noslop setup (unchanged)
    // ...write .noslop.toml, .noslop/ directory, git hooks...

    // Phase 2: Agent setup (only if agent specified)
    if let Some(agent_type) = agent_type {
        configure_agent(agent_type)?;
    }
    Ok(())
}
```

#### MODIFY: `src/cli/commands/mod.rs` (+0 lines, signature change only)

The `pub use init::init;` line stays the same. Only the internal signature changes.

#### MODIFY: `src/adapters/toml/parser.rs` (~+60 lines)

Add new structs for the expanded `.noslop.toml` schema and new fields on `NoslopFile`:

```rust
// New fields on NoslopFile:
pub agent: Option<AgentSection>,
pub review: Option<ReviewSection>,

// New structs:
pub struct AgentSection {
    #[serde(rename = "type")]
    pub agent_type: String,
}

pub struct ReviewSection {
    pub analyzers: Vec<String>,
    #[serde(rename = "co-change")]
    pub co_change: Option<CoChangeConfig>,
    pub formatting: Option<FormattingConfig>,
    #[serde(flatten)]
    pub analyzer_configs: HashMap<String, toml::Value>,
}

pub struct CoChangeConfig { pub min_correlation: f64, pub lookback: u32, pub min_commits: u32 }
pub struct FormattingConfig { pub tools: Vec<String> }
pub struct AgentAnalyzerConfig { pub agent: Option<String>, pub prompt_template: Option<String> }
pub struct ScriptAnalyzerConfig { pub analyzer_type: String, pub command: String }
```

Source: 08-config-design.md, "Updated Structs" section.

#### MODIFY: `Cargo.toml` (+1 line)

Add `dirs = "6.0"` to `[dependencies]`.

### Backward Compatibility

- `noslop init` (no agent) produces identical output to today
- Old `.noslop.toml` files without `[agent]` or `[review]` parse without error (`#[serde(default)]`)
- Old noslop binaries can read new `.noslop.toml` files with `[agent]` (serde ignores unknown fields)
- `[review]` section is NOT written by `noslop init` by default -- it is user-configured when they want to customize the pipeline beyond the default `["conventions"]` analyzer

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

### Phase 4 Scope

- **New files**: 0
- **Modified files**: 5
- **Estimated lines**: ~200 (net additions)

---

## Phase 5: Review Pipeline Core

**Goal**: Implement the pure domain services for the review pipeline: the `ReviewPipeline` orchestrator, built-in analyzers (`ConventionAnalyzer`, `FormattingAnalyzer`, `CoChangeAnalyzer`), `ScriptAnalyzer`, and the `AgentAnalyzer` bridge. No I/O, no CLI.

**Source docs**: 03-agent-port-design.md (AgentAnalyzer bridge), 08-config-design.md (analyzer configuration)

**Depends on**: Phase 1 (all model types, `ReviewAnalyzer` trait)

### File-by-File Changes

#### NEW: `src/core/services/pipeline.rs` (~150 lines)

The `ReviewPipeline` orchestrator. Runs analyzers in order: static first, then script, then agent. Each analyzer receives findings from all prior analyzers (fold semantics).

Source: 03-agent-port-design.md, "Integration Points" > "With noslop review"; 01-noslop-analysis.md, "Pipeline Execution Order".

```rust
/// Orchestrates the review pipeline: runs analyzers in order,
/// collecting and passing forward findings at each step.
pub struct ReviewPipeline {
    analyzers: Vec<Box<dyn ReviewAnalyzer>>,
}

impl ReviewPipeline {
    pub fn new(analyzers: Vec<Box<dyn ReviewAnalyzer>>) -> Self {
        Self { analyzers }
    }

    /// Run all analyzers in order, each seeing prior findings
    pub fn run(&self, context: &ReviewContext) -> anyhow::Result<Vec<Finding>> {
        let mut all_findings: Vec<Finding> = Vec::new();

        for analyzer in &self.analyzers {
            log::info!("Running analyzer: {}", analyzer.name());
            let findings = analyzer.analyze(context, &all_findings)?;
            all_findings.extend(findings);
        }

        Ok(all_findings)
    }
}
```

#### NEW: `src/core/services/analyzers.rs` (~200 lines)

Built-in analyzer implementations:

- `ConventionAnalyzer` -- wraps existing check logic (`CheckRepository`) to produce findings from convention checks on changed files. Implements `ReviewAnalyzer`.
- `FormattingAnalyzer` -- runs external formatters (rustfmt, prettier) on changed files and produces findings for files that would change. Implements `ReviewAnalyzer`.
- `CoChangeAnalyzer` -- scans commit history for files that usually change together and flags missing co-changes. Implements `ReviewAnalyzer`.

Source: 01-noslop-analysis.md, "Key Extension Points" > "New adapters: src/adapters/analyzer/".

```rust
pub struct ConventionAnalyzer {
    checks: Vec<Check>,
}

impl ReviewAnalyzer for ConventionAnalyzer {
    fn name(&self) -> &str { "conventions" }
    fn required_context(&self) -> Vec<ContextKind> { vec![ContextKind::ChangedFiles] }
    fn analyze(&self, ctx: &ReviewContext, _prior: &[Finding]) -> anyhow::Result<Vec<Finding>> {
        // Match changed files against check targets, produce findings
        // for files that match check patterns
    }
}

pub struct CoChangeAnalyzer {
    config: CoChangeConfig,
}

impl ReviewAnalyzer for CoChangeAnalyzer {
    fn name(&self) -> &str { "co-change" }
    fn required_context(&self) -> Vec<ContextKind> { vec![ContextKind::CommitHistory, ContextKind::ChangedFiles] }
    fn analyze(&self, ctx: &ReviewContext, _prior: &[Finding]) -> anyhow::Result<Vec<Finding>> {
        // Scan commit history for co-change patterns,
        // flag missing files from current diff
    }
}
```

#### NEW: `src/core/services/script_analyzer.rs` (~60 lines)

`ScriptAnalyzer` -- runs an external command, reads JSON findings from stdout, returns them. Implements `ReviewAnalyzer`.

```rust
pub struct ScriptAnalyzer {
    name: String,
    command: String,
}

impl ReviewAnalyzer for ScriptAnalyzer {
    fn name(&self) -> &str { &self.name }
    fn required_context(&self) -> Vec<ContextKind> { vec![ContextKind::Diff] }
    fn analyze(&self, ctx: &ReviewContext, prior: &[Finding]) -> anyhow::Result<Vec<Finding>> {
        // Execute command, pipe diff to stdin, parse JSON findings from stdout
    }
}
```

#### NEW: `src/core/services/agent_analyzer.rs` (~120 lines)

The `AgentAnalyzer` bridge. Wraps an `AgentRuntime` to implement `ReviewAnalyzer`. Each instance is configured with a focus area and prompt template.

Source: 03-agent-port-design.md, "The ReviewAnalyzer Trait and AgentAnalyzer Bridge" section.

```rust
pub struct AgentAnalyzer {
    runtime: Box<dyn AgentRuntime>,
    focus: String,
    prompt_template: String,
}

impl AgentAnalyzer {
    pub fn new(
        runtime: Box<dyn AgentRuntime>,
        focus: impl Into<String>,
        prompt_template: impl Into<String>,
    ) -> Self { /* ... */ }

    fn build_prompt(&self, ctx: &ReviewContext, prior: &[Finding]) -> String {
        self.prompt_template
            .replace("{diff}", &ctx.diff)
            .replace("{focus}", &self.focus)
            .replace("{prior_findings}", &format_prior_findings(prior))
            .replace("{changed_files}", &ctx.changed_files.join("\n"))
    }
}

impl ReviewAnalyzer for AgentAnalyzer {
    fn name(&self) -> &str { /* returns "agent:<focus>" */ }
    fn required_context(&self) -> Vec<ContextKind> { vec![ContextKind::Diff, ContextKind::ChangedFiles] }
    fn analyze(&self, ctx: &ReviewContext, prior: &[Finding]) -> anyhow::Result<Vec<Finding>> {
        let prompt = self.build_prompt(ctx, prior);
        let config = InvocationConfig {
            prompt,
            working_dir: ctx.repo_root.clone(),
            timeout_secs: Some(120),
            bypass_permissions: true,
            ..Default::default()
        };
        let result = self.runtime.invoke(&config)?;
        let output = self.runtime.parse_output(&result);
        Ok(output.findings)
    }
}
```

#### NEW: `src/core/services/pipeline_builder.rs` (~100 lines)

Factory functions to construct the analyzer pipeline from `ReviewConfig`:

- `build_pipeline(config: &ReviewConfig, agent_type: Option<AgentType>) -> ReviewPipeline`
- `build_agent_analyzers(agent_type: AgentType, prompts_dir: &Path, passes: &[String]) -> Vec<Box<dyn ReviewAnalyzer>>`

Resolves built-in, script, and agent analyzers from the config and assembles them in order.

```rust
pub fn build_pipeline(
    config: &ReviewConfig,
    checks: Vec<Check>,
    agent_type: Option<AgentType>,
    prompts_dir: &Path,
) -> ReviewPipeline {
    let mut analyzers: Vec<Box<dyn ReviewAnalyzer>> = Vec::new();

    for entry in &config.analyzer_entries {
        match entry {
            AnalyzerEntry::BuiltIn { name } => match name.as_str() {
                "conventions" => analyzers.push(Box::new(ConventionAnalyzer::new(checks.clone()))),
                "formatting" => analyzers.push(Box::new(FormattingAnalyzer::new(config.formatting.clone()))),
                "co-change" => analyzers.push(Box::new(CoChangeAnalyzer::new(config.co_change.clone()))),
                _ => log::warn!("Unknown built-in analyzer: {name}"),
            },
            AnalyzerEntry::Script { name, command } => {
                analyzers.push(Box::new(ScriptAnalyzer::new(name.clone(), command.clone())));
            },
            AnalyzerEntry::Agent { name, focus, agent, prompt_template } => {
                let agent_type = agent.parse::<AgentType>().expect("validated at config load");
                let runtime = get_agent_runtime(agent_type);
                let template = load_prompt_template(focus, prompt_template.as_deref(), prompts_dir);
                analyzers.push(Box::new(AgentAnalyzer::new(runtime, focus.clone(), template)));
            },
        }
    }

    ReviewPipeline::new(analyzers)
}
```

#### MODIFY: `src/core/services/mod.rs` (+5 lines)

```rust
pub mod agent_analyzer;
pub mod analyzers;
pub mod pipeline;
pub mod pipeline_builder;
pub mod script_analyzer;
```

#### MODIFY: `src/core/mod.rs` (+0-2 lines)

Verify services module is re-exported.

### Phase 5 Verification

All services are pure functions. Tests require no I/O:

```bash
cargo test pipeline -- --nocapture
cargo test convention_analyzer -- --nocapture
cargo test co_change_analyzer -- --nocapture
cargo test agent_analyzer -- --nocapture
cargo test build_pipeline -- --nocapture
```

### Phase 5 Scope

- **New files**: 5
- **Modified files**: 2
- **Estimated lines**: ~700 (including inline unit tests)

---

## Phase 6: Review CLI and Finding Management

**Goal**: Implement the `noslop review` command, the `noslop findings` command, the finding store adapter, review session I/O, and the pre-push hook integration.

**Source docs**: 03-agent-port-design.md (review command), 05-checkpoint-worktree-design.md (VCS methods used), 08-config-design.md (review session files, finding management)

**Depends on**: Phase 2 (VersionControl extensions for diff/history), Phase 3 (AgentRuntime), Phase 5 (ReviewPipeline, analyzers)

### File-by-File Changes

#### NEW: `src/adapters/finding_store.rs` (~100 lines)

`FileFindingStore` implementing `FindingStore`:

- `save_session()` writes JSON to `.noslop/reviews/{session-id}.json`
- `load_session()` reads and deserializes a session file
- `list_sessions()` lists files in `.noslop/reviews/`
- `find_unresolved_blocking()` scans sessions for unresolved findings at severity `error` or above

Source: 08-config-design.md, "Review Session Files" section.

```rust
pub struct FileFindingStore {
    reviews_dir: PathBuf,
}

impl FileFindingStore {
    pub fn new(repo_root: &Path) -> Self {
        Self { reviews_dir: repo_root.join(".noslop/reviews") }
    }
}

impl FindingStore for FileFindingStore {
    fn save_session(&self, session: &ReviewSession) -> anyhow::Result<()> {
        std::fs::create_dir_all(&self.reviews_dir)?;
        let path = self.reviews_dir.join(format!("{}.json", session.id));
        let content = serde_json::to_string_pretty(session)?;
        std::fs::write(path, content)?;
        Ok(())
    }
    // ... other methods ...
}
```

#### NEW: `src/cli/commands/review_cmd.rs` (~200 lines)

The `noslop review` CLI command. Orchestrates the full review pipeline:

1. Load `.noslop.toml` to get `ReviewConfig` and `AgentType`
2. Resolve base/head refs (default: default branch..HEAD)
3. Call `vcs.diff_between()` and `vcs.commits_between()` to build `ReviewContext`
4. Build the analyzer pipeline via `pipeline_builder::build_pipeline()`
5. Run the pipeline, collect findings
6. Save review session to `.noslop/reviews/`
7. Display findings with severity coloring
8. In `--check` mode, exit non-zero if unresolved blocking findings exist

Source: 03-agent-port-design.md, "Integration Points" > "With noslop review".

```rust
#[derive(Debug, clap::Args)]
pub struct ReviewArgs {
    /// Base ref to compare against (default: inferred from git)
    #[arg(long)]
    base: Option<String>,

    /// Head ref to review (default: HEAD)
    #[arg(long, default_value = "HEAD")]
    head: String,

    /// Run specific analyzers (overrides .noslop.toml)
    #[arg(long, value_delimiter = ',')]
    analyzer: Option<Vec<String>>,

    /// Include LLM-powered agent review passes
    #[arg(long)]
    agent: bool,

    /// Check mode: exit non-zero if blocking findings exist (for pre-push hook)
    #[arg(long)]
    check: bool,

    /// Override agent type (default: from .noslop.toml)
    #[arg(long)]
    agent_type: Option<String>,
}

pub fn review(args: ReviewArgs, mode: OutputMode) -> anyhow::Result<()> {
    let vcs = GitVersionControl::current_dir()?;
    let repo_root = vcs.repo_root()?;

    // Load config
    let noslop_file = load_noslop_config(&repo_root)?;
    let review_config = resolve_review_config(&noslop_file, &args)?;

    // Build review context from VCS
    let base = args.base.unwrap_or_else(|| detect_default_branch(&vcs));
    let file_diffs = vcs.diff_between(&base, &args.head)?;
    let changed_files: Vec<String> = file_diffs.iter().map(|d| d.path.display().to_string()).collect();
    let diff_text = format_unified_diff(&file_diffs);

    let context = ReviewContext {
        diff: diff_text,
        changed_files,
        repo_root: repo_root.clone(),
        commit_range: Some(format!("{base}..{}", args.head)),
    };

    // Build and run pipeline
    let pipeline = build_pipeline(&review_config, /* ... */);
    let findings = pipeline.run(&context)?;

    // Save session and display results
    // ...

    // Check mode: exit non-zero if blocking findings
    if args.check {
        let blocking: Vec<_> = findings.iter()
            .filter(|f| matches!(f.severity, Severity::Error | Severity::Critical))
            .collect();
        if !blocking.is_empty() {
            std::process::exit(1);
        }
    }

    Ok(())
}
```

#### NEW: `src/cli/commands/findings_cmd.rs` (~150 lines)

The `noslop findings` CLI command for managing review findings:

- `noslop findings` -- list findings from the most recent review session
- `noslop findings resolve <id> [-m <msg>]` -- resolve a finding
- `noslop findings dismiss <id> [-m <reason>]` -- dismiss a finding
- `noslop findings show <session-id>` -- show all findings from a specific session

```rust
#[derive(Debug, clap::Subcommand)]
pub enum FindingsAction {
    /// List findings from the most recent review
    List {
        /// Show only unresolved findings
        #[arg(long)]
        unresolved: bool,
    },
    /// Resolve a finding
    Resolve {
        id: String,
        #[arg(short, long)]
        message: Option<String>,
    },
    /// Dismiss a finding
    Dismiss {
        id: String,
        #[arg(short, long)]
        reason: Option<String>,
    },
    /// Show findings from a specific review session
    Show { session_id: String },
}
```

#### MODIFY: `src/cli/app.rs` (~+20 lines)

Add `Review(review_cmd::ReviewArgs)` and `Findings` variants to the `Command` enum. Add dispatch arms in `run()`.

```rust
/// Run the review pipeline on current changes
Review(review_cmd::ReviewArgs),

/// Manage review findings
Findings {
    #[command(subcommand)]
    action: Option<findings_cmd::FindingsAction>,
},
```

#### MODIFY: `src/cli/commands/mod.rs` (+4 lines)

Add `mod review_cmd;`, `mod findings_cmd;`, and re-exports. The existing `review.rs` (for the older review subcommand) may need renaming or integration.

#### MODIFY: `src/adapters/mod.rs` (+1 line)

Add `pub mod finding_store;`.

#### MODIFY: `src/adapters/git/hooks.rs` (~+20 lines)

Add `install_pre_push()` function that writes the pre-push hook script running `noslop review --check`.

Source: 07-init-expansion-design.md, "Pre-Push Hook" section.

```rust
pub fn install_pre_push() -> anyhow::Result<()> {
    let hook_path = Path::new(".git/hooks/pre-push");
    let content = "#!/usr/bin/env bash\nnoslop review --check\nexit $?\n";
    fs::write(hook_path, content)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(hook_path, fs::Permissions::from_mode(0o755))?;
    }
    Ok(())
}
```

### Phase 6 Verification

```bash
# Unit tests
cargo test review_cmd -- --nocapture
cargo test findings_cmd -- --nocapture
cargo test file_finding_store -- --nocapture

# Integration tests (in a git repo with changes)
cd /tmp && git init test-review && cd test-review
noslop init
echo "fn main() {}" > main.rs
git add main.rs && git commit -m "initial"
echo "fn main() { panic!() }" > main.rs
git add main.rs && git commit -m "add panic"
noslop review --base HEAD~1
noslop findings
```

### Phase 6 Scope

- **New files**: 3
- **Modified files**: 4
- **Estimated lines**: ~550

---

## Phase 7: Integration Testing and Polish

**Goal**: End-to-end tests, mock-based pipeline tests, update test infrastructure, clippy/doc cleanup.

**Source docs**: All design docs (test sections)

**Depends on**: All previous phases

### File-by-File Changes

#### NEW: `tests/adapter/checkpoint_test.rs` (~80 lines)

Tests for `checkpoint::is_clean()`, `checkpoint::commit_all()` using `tempfile::TempDir` + `git2::Repository::init()`.

Source: 05-checkpoint-worktree-design.md, "Unit Tests" section.

```rust
#[test]
fn is_clean_on_fresh_repo() {
    let (_dir, repo) = setup_repo();
    assert!(is_clean(&repo).unwrap());
}

#[test]
fn is_clean_with_untracked_file() {
    let (dir, repo) = setup_repo();
    std::fs::write(dir.path().join("new.txt"), "content").unwrap();
    assert!(!is_clean(&repo).unwrap());
}

#[test]
fn commit_all_stages_and_commits() {
    let (dir, repo) = setup_repo();
    std::fs::write(dir.path().join("file.txt"), "content").unwrap();
    let result = commit_all(&repo, "test checkpoint").unwrap();
    assert!(result.is_some());
    assert!(is_clean(&repo).unwrap());
}
```

#### NEW: `tests/adapter/agent_adapter_test.rs` (~100 lines)

Tests for shared finding parser and adapter configuration generation:

```rust
#[test]
fn extract_findings_from_clean_json() { /* ... */ }

#[test]
fn extract_findings_from_narrative_wrapper() { /* ... */ }

#[test]
fn claude_config_generates_all_files() { /* ... */ }

#[test]
fn claude_settings_includes_noslop_permission() { /* ... */ }

#[test]
fn codex_config_has_no_hooks_or_commands() { /* ... */ }
```

#### NEW: `tests/unit/pipeline_test.rs` (~100 lines)

Mock-based pipeline tests using `MockReviewAnalyzer`:

```rust
#[test]
fn pipeline_runs_analyzers_in_order() {
    // Create mock analyzers that record call order
    // Verify second analyzer receives findings from first
}

#[test]
fn pipeline_passes_prior_findings_to_each_analyzer() {
    // Verify fold semantics: each analyzer sees all prior findings
}

#[test]
fn agent_analyzer_builds_correct_prompt() {
    // Verify {diff}, {prior_findings}, {focus} substitution
}
```

#### NEW: `tests/unit/review_config_test.rs` (~60 lines)

Tests for TOML parser changes:

```rust
#[test]
fn parse_minimal_config() { /* agent: None, review: None */ }

#[test]
fn parse_with_agent_section() { /* agent.type = "claude" */ }

#[test]
fn parse_with_review_pipeline() { /* analyzers list */ }

#[test]
fn parse_with_co_change_config() { /* co-change sub-table */ }

#[test]
fn existing_config_without_new_sections_parses() { /* backward compat */ }
```

#### NEW: `tests/integration/review_test.rs` (~100 lines)

Full `noslop review` integration tests using `TempGitRepo`:

```rust
#[test]
fn cli_review_with_convention_findings() {
    let repo = TempGitRepo::new();
    // Set up .noslop.toml with checks
    // Create a change that triggers a check
    // Run noslop review
    // Verify findings in output
}

#[test]
fn cli_review_check_mode_exits_nonzero_on_blocking() {
    // Set up blocking findings
    // Run noslop review --check
    // Verify exit code != 0
}

#[test]
fn cli_checkpoint_with_changes() {
    let repo = TempGitRepo::new();
    repo.write_file("test.txt", "content");
    Command::cargo_bin("noslop").unwrap()
        .current_dir(repo.path())
        .args(["checkpoint", "test message"])
        .assert()
        .success()
        .stdout(predicates::str::contains("Checkpoint:"));
}
```

#### MODIFY: `tests/common/mocks.rs` (~+40 lines)

Add `MockVersionControl` methods for the 7 new `VersionControl` trait methods (`is_clean`, `commit_all`, `diff_between`, `commits_between`, `file_at_revision`, `recent_commits`, `diff_stat_since`).

#### MODIFY: `tests/adapter/mod.rs` (+2 lines)

Add `mod checkpoint_test;` and `mod agent_adapter_test;`.

### Polish Tasks

1. **Clippy clean**: Run `cargo clippy -- -D warnings` and fix all new warnings
2. **Doc comments**: Ensure all public types and functions have `///` doc comments
3. **Missing trait impls**: Verify `Debug`, `Clone`, `Copy` where appropriate (clippy will catch `missing_debug_implementations`)
4. **Error messages**: Review all `anyhow::bail!` messages for clarity
5. **Output formatting**: Consistent human/JSON output across checkpoint, review, findings commands
6. **Review session format**: Verify review session JSON files round-trip correctly

### Phase 7 Verification

```bash
# Full test suite
cargo test

# Clippy
cargo clippy -- -D warnings

# Doc build
cargo doc --no-deps

# Integration (manual)
cd /tmp && git init test-full && cd test-full
noslop init
echo "fn main() {}" > main.rs
git add . && git commit -m "initial"
noslop check add "*.rs" -m "Review Rust files" -s warn
echo "fn main() { todo!() }" > main.rs
git add . && git commit -m "add todo"
noslop review --base HEAD~1
noslop findings
noslop checkpoint "before fix"
```

### Phase 7 Scope

- **New files**: 5
- **Modified files**: 2
- **Estimated lines**: ~500

---

## Quick Reference: All New Files

Sorted by implementation order within each phase.

```
Phase 1 (6 new files):
  src/core/models/finding.rs
  src/core/models/review_config.rs
  src/core/models/diff.rs
  src/core/ports/agent.rs
  src/core/ports/analyzer.rs
  src/core/ports/finding_store.rs

Phase 2 (2 new files):
  src/adapters/git/checkpoint.rs
  src/cli/commands/checkpoint.rs

Phase 3 (12 new files):
  src/adapters/agent/mod.rs
  src/adapters/agent/parsing.rs
  src/adapters/agent/claude.rs
  src/adapters/agent/codex.rs
  src/adapters/agent/templates/claude_global_instructions.md
  src/adapters/agent/templates/claude_branch_protection.sh
  src/adapters/agent/templates/claude_auto_format.sh
  src/adapters/agent/templates/claude_cmd_checkpoint.md
  src/adapters/agent/templates/claude_cmd_worktree.md
  src/adapters/agent/templates/codex_instructions.md
  src/adapters/agent/templates/prompts/security.md
  src/adapters/agent/templates/prompts/quality.md
  src/adapters/agent/templates/prompts/architecture.md

Phase 4 (0 new files):
  (modifications only)

Phase 5 (5 new files):
  src/core/services/pipeline.rs
  src/core/services/analyzers.rs
  src/core/services/script_analyzer.rs
  src/core/services/agent_analyzer.rs
  src/core/services/pipeline_builder.rs

Phase 6 (3 new files):
  src/adapters/finding_store.rs
  src/cli/commands/review_cmd.rs
  src/cli/commands/findings_cmd.rs

Phase 7 (5 new files):
  tests/adapter/checkpoint_test.rs
  tests/adapter/agent_adapter_test.rs
  tests/unit/pipeline_test.rs
  tests/unit/review_config_test.rs
  tests/integration/review_test.rs
```

**Total**: 33 new files + ~16 modified files = ~49 file touches across 7 phases.

---

## Quick Reference: All Modified Files

```
Phase 1:
  src/core/models/mod.rs       (add module declarations + re-exports)
  src/core/ports/mod.rs        (add module declarations + re-exports)
  src/lib.rs                   (verify re-exports)

Phase 2:
  src/core/ports/vcs.rs        (extend VersionControl trait with 7 methods)
  src/adapters/git/mod.rs      (add checkpoint module + implement trait methods)
  src/cli/app.rs               (add Checkpoint command)
  src/cli/commands/mod.rs      (add module declarations)

Phase 3:
  src/adapters/mod.rs          (add agent module)
  src/lib.rs                   (ensure agent re-exports)

Phase 4:
  src/cli/app.rs               (add agent arg to Init)
  src/cli/commands/init.rs     (agent parsing + Phase 2 agent setup)
  src/cli/commands/mod.rs      (signature update)
  src/adapters/toml/parser.rs  (AgentSection, ReviewSection, config structs)
  Cargo.toml                   (add dirs dependency)

Phase 5:
  src/core/services/mod.rs     (add module declarations)
  src/core/mod.rs              (verify re-exports)

Phase 6:
  src/cli/app.rs               (add Review + Findings commands)
  src/cli/commands/mod.rs      (add review_cmd + findings_cmd modules)
  src/adapters/mod.rs          (add finding_store module)
  src/adapters/git/hooks.rs    (add install_pre_push function)

Phase 7:
  tests/common/mocks.rs        (update MockVersionControl with 7 new methods)
  tests/adapter/mod.rs         (add test modules)
```

---

## Risk Register

| Risk                                                    | Likelihood | Impact | Mitigation                                                                                      |
| ------------------------------------------------------- | ---------- | ------ | ----------------------------------------------------------------------------------------------- |
| `git2` diff API does not produce clean unified diffs    | Medium     | Medium | Fall back to `std::process::Command("git diff ...")` for the `diff_between` method if needed    |
| Agent CLI interface changes (claude/codex flag formats) | Low        | Medium | Adapter pattern isolates changes to one file per agent                                          |
| `dirs` crate version conflicts                          | Low        | Low    | `dirs` v6.0 has no transitive deps that conflict with existing Cargo.toml                       |
| Clippy pedantic rejections on new code                  | High       | Low    | Address iteratively; `#[allow]` pragmatically where clippy is wrong                             |
| Agent output parsing brittle (JSON extraction fails)    | Medium     | Medium | Graceful degradation: return empty findings with raw output preserved for debugging             |
| Co-change analyzer performance on large repos           | Medium     | Low    | `lookback` config limits commit scan depth; default 500 commits is reasonable                   |
| `[review]` TOML section with `#[serde(flatten)]` quirks | Low        | Medium | Test round-trip parsing extensively; `serde(flatten)` with `HashMap` is well-supported for TOML |
| Existing `review.rs` command naming conflict            | Low        | Low    | New review command uses `review_cmd.rs`; existing review.rs handles the older review subcommand |
| Pre-push hook blocks pushes unexpectedly                | Medium     | High   | `noslop review --check` only blocks on `error`/`critical` severity; `warn` is display-only      |
| Test flakiness in git-based tests (filesystem timing)   | Medium     | Low    | Use `tempfile::TempDir` with unique paths; add cleanup in test teardown                         |
