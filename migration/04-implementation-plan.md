# Implementation Plan

Actionable implementation plan for the review pipeline architecture. A developer should be able to pick up any phase and start coding immediately.

## Table of Contents

- [Dependency Analysis](#dependency-analysis)
- [Current Codebase Baseline](#current-codebase-baseline)
- [Phase Overview](#phase-overview)
- [Phase 1: Core Types](#phase-1-core-types)
- [Phase 2: VCS Extensions](#phase-2-vcs-extensions)
- [Phase 3: Agent Ports and Adapters](#phase-3-agent-ports-and-adapters)
- [Phase 4: Init Expansion](#phase-4-init-expansion)
- [Phase 5: Review Pipeline](#phase-5-review-pipeline)
- [Phase 6: Review CLI](#phase-6-review-cli)
- [Phase 7: Integration Testing](#phase-7-integration-testing)
- [Quick Reference: All New Files](#quick-reference-all-new-files)
- [Quick Reference: All Modified Files](#quick-reference-all-modified-files)
- [Risk Register](#risk-register)

---

## Dependency Analysis

**Existing dependencies (no changes)**: `git2` 0.20.2, `chrono` 0.4.42, `clap` 4.5.51, `regex` 1.12, `serde` 1.0.228, `serde_json` 1.0.145, `toml` 0.9.8, `anyhow` 1.0.100, `colored` 3.0.0, `glob` 0.3.3, `log` 0.4.28. Dev: `mockall`, `tempfile`, `assert_cmd`, `predicates`, `pretty_assertions`, `test-case`, `proptest`.

**One new dependency**:

```toml
dirs = "6.0"
```

Needed by `configure_agent()` to resolve `$HOME` for writing agent config files. Small crate, no transitive deps beyond `libc`/`windows-sys`. Required because `std::env::home_dir()` is deprecated.

**No new feature flags**. Agent invocation uses `std::process::Command` (synchronous). The existing `llm` feature (tokio + reqwest) is orthogonal.

---

## Current Codebase Baseline

47 source .rs files, 21 test .rs files. Key existing types that map to the new model:

- `Severity` (Info/Warn/Block) -- moves to primitives.rs, gains `PartialOrd/Ord/Hash`
- `Check` (id, target: String, message, severity) -- target becomes `Target`, drops `introduced_by`/`created_at`, gains `tags` and `into_finding()`
- `Target` (PathSpec + GlobPattern + Fragment) -- replaced by simplified struct `{ path, span, commit }`
- `Review` (Vec\<ReviewComment\>) -- flattened to `Vec<Finding>`
- `Acknowledgment` -- deleted, replaced by `FindingStatus`
- `ReviewComment`, `DiffPosition`, `DiffSide`, `CommentStatus` -- all deleted

---

## Phase Overview

```
P1 (core types) --> P2 (VCS) -----+
                    P3 (agents) ---+--> P5 (pipeline) --> P6 (CLI) --> P7 (testing)
                    P4 (init) -----+
```

Phases 2, 3, 4 can run in parallel after Phase 1. Phase 5 needs Phase 1 only. Phase 6 needs 2+3+5. Phase 7 needs all.

| Phase | Description              | New Files | Modified Files | Est. Lines |
| ----- | ------------------------ | --------- | -------------- | ---------- |
| 1     | Core types               | 3         | 7              | ~400       |
| 2     | VCS extensions           | 3         | 4              | ~400       |
| 3     | Agent ports and adapters | 12        | 2              | ~1,000     |
| 4     | Init expansion           | 0         | 5              | ~200       |
| 5     | Review pipeline          | 5         | 2              | ~650       |
| 6     | Review CLI               | 3         | 4              | ~500       |
| 7     | Integration testing      | 5         | 2              | ~400       |
| Total |                          | 31        | ~18            | ~3,550     |

---

## Phase 1: Core Types

**Goal**: Rewrite domain models so all subsequent phases compile against stable type definitions. **Depends on**: Nothing.

### NEW `src/core/models/primitives.rs` (~60 lines)

Replaces both `severity.rs` and `target.rs`. Contains:

- **Severity** enum (Info, Warn, Block) -- same 3 levels. Add `PartialOrd`, `Ord`, `Hash` derives. Copy `Display` + `FromStr` impls from current `severity.rs`.
- **Span** struct `{ start: u32, end: u32 }` -- constructors: `Span::line(n)`, `Span::range(s, e)`.
- **Target** struct `{ path: String, span: Option<Span>, commit: Option<String> }` -- replaces PathSpec/GlobPattern/Fragment. Constructors: `Target::pattern("src/**/*.rs")`, `Target::file("src/auth.rs")`. Methods: `is_glob()` (checks for `*`, `?`, `[`), `with_span()`, `with_commit()`. Serde: `skip_serializing_if` on span and commit.
- **FindingSource** enum `{ Check(String), Script(String), Agent(String), Human }` -- serde: `tag = "kind"`, `content = "name"`.

### NEW `src/core/models/finding.rs` (~50 lines)

- **Finding** struct: id, target: Target, severity, message, source: FindingSource, status: FindingStatus, suggestion: Option\<String\>, created_at: String. Replaces ReviewComment + CheckItemResult.
- **FindingStatus** enum `{ Open, Resolved, Dismissed }` -- default: Open. Replaces CommentStatus + Acknowledgment.

### NEW `src/core/models/agent.rs` (~15 lines)

- **AgentKind** enum `{ Claude, Codex }` -- derives: Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize. Serde: `rename_all = "lowercase"`. Impl `Display`, `FromStr`.

### MODIFY `src/core/models/check.rs`

- Change `target: String` to `target: Target`
- Drop `introduced_by: Option<String>` and `created_at: String` fields
- Add `tags: Vec<String>` with `#[serde(default, skip_serializing_if = "Vec::is_empty")]`
- Add `into_finding(matched_file: &str) -> Finding` method
- Update `new()`: remove timestamp. Update `applies_to()`: delegate to `target.is_glob()` + matching.

### REWRITE `src/core/models/review.rs`

- Replace `comments: Vec<ReviewComment>` with `findings: Vec<Finding>`
- Rename `base_sha`/`head_sha` to `base`/`head`. Drop `description`.
- **Delete** entirely: `ReviewComment`, `DiffPosition`, `DiffSide`, `CommentStatus`
- Keep `ReviewStatus` enum (Open, Closed)
- New methods: `add_finding()`, `blocking_findings()` (status=Open + severity=Block), `is_blocked()`, `findings_for_file(path)`
- Update all inline tests

### DELETE files

- `src/core/models/acknowledgment.rs` -- replaced by FindingStatus
- `src/core/models/severity.rs` -- moved to primitives.rs
- `src/core/models/target.rs` -- simplified into primitives.rs

### UPDATE `src/core/models/mod.rs`

```rust
mod agent;
mod check;
mod finding;
mod primitives;
mod review;

pub use agent::AgentKind;
pub use check::Check;
pub use finding::{Finding, FindingStatus};
pub use primitives::{FindingSource, Severity, Span, Target};
pub use review::{Review, ReviewStatus};
```

### UPDATE imports across codebase

| File                                     | Change                                                                                      |
| ---------------------------------------- | ------------------------------------------------------------------------------------------- |
| `src/lib.rs`                             | Remove `Acknowledgment`, `Fragment`, `ParseError`, `PathSpec`, `GlobPattern`; add new types |
| `src/core/services/checker.rs`           | Remove `Acknowledgment` import; adapt to new Check shape                                    |
| `src/core/ports/acknowledgment_store.rs` | Mark deprecated or gate behind cfg                                                          |
| `src/adapters/review.rs`                 | Update to new Review shape (findings not comments)                                          |
| `src/cli/commands/review.rs`             | Update to new types (no ReviewComment/DiffPosition)                                         |

**Verify**: `cargo check && cargo test`

---

## Phase 2: VCS Extensions

**Goal**: Extend `VersionControl` trait with diff/history methods. Add checkpoint adapter. **Depends on**: Phase 1.

### NEW `src/core/models/diff.rs` (~90 lines)

- **FileDiff** `{ path: PathBuf, old_path: Option<PathBuf>, status: DiffStatus, hunks: Vec<DiffHunk> }`
- **DiffStatus** enum `{ Added, Modified, Deleted, Renamed }`
- **DiffHunk** `{ old_start: u32, old_count: u32, new_start: u32, new_count: u32, lines: Vec<DiffLine> }`
- **DiffLine** `{ kind: DiffLineKind, content: String }`, **DiffLineKind** `{ Context, Addition, Deletion }`
- **CommitInfo** `{ sha, short_sha, summary, message, author, timestamp: i64 }`
- **DiffStat** `{ files_changed, insertions, deletions }`

All derive `Debug, Clone, PartialEq, Eq`. Add to `src/core/models/mod.rs`.

### MODIFY `src/core/ports/vcs.rs` (+40 lines)

Extend `VersionControl` trait:

```rust
fn is_clean(&self) -> anyhow::Result<bool>;
fn commit_all(&self, message: &str) -> anyhow::Result<Option<String>>;
fn diff_between(&self, base: &str, head: &str) -> anyhow::Result<Vec<FileDiff>>;
fn commits_between(&self, base: &str, head: &str) -> anyhow::Result<Vec<CommitInfo>>;
fn file_at_revision(&self, path: &Path, rev: &str) -> anyhow::Result<String>;
fn recent_commits(&self, count: usize) -> anyhow::Result<Vec<CommitInfo>>;
fn diff_stat_since(&self, since: &str) -> anyhow::Result<DiffStat>;
```

### NEW `src/adapters/git/checkpoint.rs` (~60 lines)

Functions using `git2`: `is_clean(repo)` via StatusOptions, `commit_all(repo, message)` via index.add_all/write_tree/commit, `default_signature(repo)`.

### MODIFY `src/adapters/git/mod.rs` (+120 lines)

Add `pub mod checkpoint;`. Implement 7 new VersionControl methods: `is_clean`/`commit_all` delegate to checkpoint module; `diff_between` uses `diff_tree_to_tree`; `commits_between` uses `revwalk` with push/hide; `file_at_revision` uses tree.get_path + find_blob; `recent_commits` uses revwalk from HEAD; `diff_stat_since` uses diff.stats.

### NEW `src/cli/commands/checkpoint.rs` (~50 lines)

Handler: resolve message (default = timestamp), call `vcs.commit_all(&msg)`, output result.

### MODIFY `src/cli/app.rs` (+10 lines)

Add `Checkpoint { message: Option<String> }` variant and dispatch arm.

### MODIFY `src/cli/commands/mod.rs` (+2 lines)

Add `mod checkpoint; pub use checkpoint::checkpoint;`

**Verify**: `cargo test checkpoint && cargo test diff_between`

---

## Phase 3: Agent Ports and Adapters

**Goal**: Define agent port traits, implement Claude + Codex adapters with shared output parsing. **Depends on**: Phase 1.

### NEW `src/core/ports/agent.rs` (~200 lines)

**Config types**: GlobalInstructions, HookDefinition, SettingsFile, SlashCommand, AgentConfigBundle. **Invocation types**: InvocationConfig, InvocationResult, ReviewOutput.

**Traits** (both with `#[cfg_attr(test, mockall::automock)]`):

- `AgentConfig` -- agent_kind, generate_config, is_available, validate, install_instructions, generate_project_instructions
- `AgentRuntime` -- agent_kind, invoke, parse_output, is_available, build_command

### NEW `src/core/ports/analyzer.rs` (~35 lines)

- **ReviewContext** `{ diff, changed_files, repo_root, commit_range }`
- **ContextKind** enum `{ Diff, ChangedFiles, FileContents, CommitHistory }`
- **ReviewAnalyzer** trait (mockall): name, required_context, analyze(context, prior_findings)

### MODIFY `src/core/ports/mod.rs` (+8 lines)

Add module declarations and re-exports for `agent` and `analyzer`.

### NEW `src/adapters/agents/mod.rs` (~30 lines)

Factory functions: `get_agent_config(AgentKind) -> Box<dyn AgentConfig>`, `get_agent_runtime(AgentKind) -> Box<dyn AgentRuntime>`.

### NEW `src/adapters/agents/parsing.rs` (~80 lines)

Shared parser: `extract_findings(output) -> ReviewOutput` (bracket-matching JSON extractor, parses `Vec<Finding>`), `extract_json_array(output) -> Option<String>`, `extract_errors(output) -> Vec<String>`.

### NEW `src/adapters/agents/claude.rs` (~350 lines)

**ClaudeConfig**: generate_config produces AgentConfigBundle (settings.json with 97 allow rules, CLAUDE.md, hooks, slash commands). is_available checks `claude --version`.

**ClaudeRuntime**: build_command produces `claude -p "$PROMPT" --output-format text --permission-mode bypassPermissions`. invoke via `std::process::Command`. parse_output delegates to parsing module.

### NEW `src/adapters/agents/codex.rs` (~200 lines)

**CodexConfig**: generate_config produces bundle with instructions.md only. **CodexRuntime**: build_command produces `codex --quiet --approval-mode full-auto "$PROMPT"`.

### NEW template files (6 files, embedded via `include_str!`)

```
src/adapters/agents/templates/
  claude_global_instructions.md     (~40 lines)
  claude_branch_protection.sh       (~15 lines)
  claude_auto_format.sh             (~30 lines)
  claude_cmd_checkpoint.md          (~30 lines)
  claude_cmd_worktree.md            (~20 lines)
  codex_instructions.md             (~30 lines)
```

### MODIFY `src/adapters/mod.rs` (+2 lines)

Add `pub mod agents;` and re-export factory functions.

**Verify**: `cargo test claude_config && cargo test extract_findings`

---

## Phase 4: Init Expansion

**Goal**: Expand `noslop init` to accept agent argument, write `[agent]` to `.noslop.toml`, configure agent, install pre-push hook. **Depends on**: Phase 1 + Phase 3.

### MODIFY `src/cli/app.rs` (+5 lines)

Add `agent: Option<String>` positional arg to `Command::Init`. Update dispatch.

### MODIFY `src/cli/commands/init.rs` (+120 lines)

Signature becomes `init(agent: Option<&str>, force, mode)`. If agent specified: parse via `AgentKind::from_str`, call `get_agent_config(kind)`, validate, generate_config, write bundle to `$HOME/<agent-dir>/` (uses `dirs::home_dir()`), generate project instructions (CLAUDE.md/AGENTS.md), install pre-push hook. Write `[agent] type = "<kind>"` section to `.noslop.toml`.

### MODIFY `src/adapters/toml/parser.rs` (+50 lines)

Add optional fields to `NoslopFile`: `agent: Option<AgentSection>`, `review: Option<ReviewSection>`. New structs: `AgentSection { agent_type }`, `ReviewSection { analyzers, co_change, formatting }`, `CoChangeConfig { min_correlation, lookback }`, `FormattingConfig { tools }`. All with `#[serde(default)]` for backward compat.

### MODIFY `src/adapters/git/hooks.rs` (+30 lines)

Add `install_pre_push()`: writes `#!/bin/sh\nnoslop review --check\nexit $?`. Same pattern as existing `install_pre_commit()`.

### MODIFY `Cargo.toml` (+1 line)

Add `dirs = "6.0"` to `[dependencies]`.

**Verify**: `cargo test init && cargo test parse_agent`

---

## Phase 5: Review Pipeline

**Goal**: Implement ReviewPipeline orchestrator, built-in analyzers, script analyzer, AgentAnalyzer bridge. Pure domain services. **Depends on**: Phase 1.

### NEW `src/core/services/pipeline.rs` (~80 lines)

`ReviewPipeline { analyzers: Vec<Box<dyn ReviewAnalyzer>> }`. Method `run(context) -> Result<Vec<Finding>>`: iterates analyzers in order, each receives all prior findings (fold semantics).

### NEW `src/core/services/analyzers.rs` (~200 lines)

- **ConventionAnalyzer** `{ checks }` -- matches changed files against check targets, produces findings via `check.into_finding()`. Name: "conventions".
- **FormattingAnalyzer** `{ config }` -- runs formatters on changed files, flags dirty files. Name: "formatting".
- **CoChangeAnalyzer** `{ config }` -- scans git history for co-change patterns, flags missing companions. Name: "co-change".

### NEW `src/core/services/script_analyzer.rs` (~60 lines)

**ScriptAnalyzer** `{ name, command }` -- runs external command, pipes diff to stdin, parses JSON findings from stdout. Name: "custom:\<name\>".

### NEW `src/core/services/agent_analyzer.rs` (~120 lines)

**AgentAnalyzer** `{ runtime, focus, prompt_template }` -- bridge between AgentRuntime and ReviewAnalyzer. `build_prompt()` replaces `{diff}`, `{prior_findings}`, `{focus}`, `{changed_files}` in template. Name: "agent:\<focus\>".

### NEW `src/core/services/pipeline_builder.rs` (~100 lines)

`build_pipeline(config, checks, agent_kind, prompts_dir) -> ReviewPipeline`. Iterates config.analyzer_entries, instantiates correct analyzer for each. `load_prompt_template()` checks `.noslop/prompts/<focus>.md` first, falls back to built-in.

### MODIFY `src/core/services/mod.rs` (+5 lines)

Add module declarations for all 5 new service files.

**Verify**: `cargo test pipeline && cargo test convention_analyzer && cargo test agent_analyzer`

---

## Phase 6: Review CLI

**Goal**: Wire up `noslop review` and `noslop findings` commands, implement ReviewStore adapter. **Depends on**: Phase 2 + 3 + 5.

### NEW `src/adapters/review_store.rs` (~100 lines)

**JsonReviewStore** `{ reviews_dir }` -- replaces existing `adapters/review.rs`. Methods: save (to `.noslop/reviews/<id>.json`), load, list_all, list_open, delete, find_blocking_for_file. Implements `ReviewStore` trait.

### NEW `src/cli/commands/review_pipeline.rs` (~200 lines)

Handler for `noslop review`. Clap args: `--base`, `--head` (default HEAD), `--analyzer`, `--check`. Flow: load config -> open VCS -> diff_between -> build ReviewContext -> build_pipeline -> run -> save Review -> display findings. In `--check` mode: exit 1 if blocking findings.

### NEW `src/cli/commands/findings.rs` (~150 lines)

Handler for `noslop findings`. Subcommands: list (default), `resolve <id>`, `dismiss <id> --reason`. Uses ReviewStore to load latest review and operate on findings.

### MODIFY `src/cli/app.rs` (+25 lines)

Replace existing Review command variant. Add: `Review { base, head, analyzer, check }` and `Findings { action }`. Update dispatch.

### MODIFY `src/cli/commands/mod.rs` (+4 lines)

Add modules for `review_pipeline` and `findings`. Existing `review.rs` removed or deprecated.

### MODIFY `src/adapters/mod.rs` (+1 line)

Update review adapter reference to new `review_store.rs`.

### MODIFY `src/core/ports/review_store.rs`

Verify trait works with new Review shape (Vec\<Finding\> not Vec\<ReviewComment\>). Method signatures stay same.

**Verify**: `cargo test review_pipeline && cargo test findings && cargo test json_review_store`

---

## Phase 7: Integration Testing

**Goal**: End-to-end tests, mock pipeline tests, clippy/doc cleanup. **Depends on**: All.

### NEW test files (5)

- `tests/adapter/checkpoint_test.rs` (~80 lines) -- is_clean, commit_all using tempfile+git2
- `tests/adapter/agent_test.rs` (~100 lines) -- extract_findings, config generation
- `tests/unit/pipeline_test.rs` (~100 lines) -- mock-based: order, fold semantics, prompt building
- `tests/unit/finding_test.rs` (~60 lines) -- serialization roundtrip, Target.is_glob, Span constructors
- `tests/integration/review_test.rs` (~100 lines) -- full CLI: review with findings, --check exit code, checkpoint

### MODIFY `tests/common/mocks.rs` (+40 lines)

Add 7 new MockVersionControl methods for Phase 2 trait extensions.

### MODIFY `tests/adapter/mod.rs` (+2 lines)

Add `mod checkpoint_test; mod agent_test;`

### Polish

1. `cargo clippy -- -D warnings`
2. Doc comments on all public types
3. Debug/Clone derives on all new types
4. Consistent human/JSON output

**Verify**: `cargo test && cargo clippy -- -D warnings && cargo doc --no-deps`

---

## Quick Reference: All New Files

```
Phase 1:  src/core/models/primitives.rs          Severity, Span, Target, FindingSource
          src/core/models/finding.rs             Finding, FindingStatus
          src/core/models/agent.rs               AgentKind enum
Phase 2:  src/core/models/diff.rs                FileDiff, DiffHunk, CommitInfo, DiffStat
          src/adapters/git/checkpoint.rs         is_clean(), commit_all() via git2
          src/cli/commands/checkpoint.rs         noslop checkpoint handler
Phase 3:  src/core/ports/agent.rs                AgentConfig, AgentRuntime traits + types
          src/core/ports/analyzer.rs             ReviewAnalyzer trait, ReviewContext
          src/adapters/agents/mod.rs             Factory functions
          src/adapters/agents/parsing.rs         JSON finding extractor
          src/adapters/agents/claude.rs          ClaudeConfig, ClaudeRuntime
          src/adapters/agents/codex.rs           CodexConfig, CodexRuntime
          src/adapters/agents/templates/         6 template files (include_str!)
Phase 5:  src/core/services/pipeline.rs          ReviewPipeline orchestrator
          src/core/services/analyzers.rs         Convention/Formatting/CoChange analyzers
          src/core/services/script_analyzer.rs   ScriptAnalyzer
          src/core/services/agent_analyzer.rs    AgentAnalyzer bridge
          src/core/services/pipeline_builder.rs  build_pipeline() factory
Phase 6:  src/adapters/review_store.rs           JsonReviewStore
          src/cli/commands/review_pipeline.rs    noslop review handler
          src/cli/commands/findings.rs           noslop findings handler
Phase 7:  tests/adapter/checkpoint_test.rs       Checkpoint tests
          tests/adapter/agent_test.rs            Agent adapter tests
          tests/unit/pipeline_test.rs            Pipeline mock tests
          tests/unit/finding_test.rs             Model type tests
          tests/integration/review_test.rs       CLI integration tests
```

Total: 31 new files (25 Rust + 6 templates).

---

## Quick Reference: All Modified Files

```
Phase 1:  src/core/models/mod.rs           Remove old modules, add primitives/finding/agent
          src/core/models/check.rs         target: String -> Target, drop fields, add into_finding()
          src/core/models/review.rs        Flatten to Vec<Finding>, delete 4 types
          src/lib.rs                       Update re-exports
          src/core/services/checker.rs     Adapt to new Check shape
          src/adapters/review.rs           Adapt to new Review shape
          src/cli/commands/review.rs       Adapt to new Review shape
Phase 2:  src/core/models/mod.rs           Add diff module
          src/core/ports/vcs.rs            Add 7 new trait methods
          src/adapters/git/mod.rs          Add checkpoint + implement new methods
          src/cli/app.rs                   Add Checkpoint command
          src/cli/commands/mod.rs          Add checkpoint module
Phase 3:  src/core/ports/mod.rs            Add agent + analyzer modules
          src/adapters/mod.rs              Add agents module
Phase 4:  src/cli/app.rs                   Add agent arg to Init
          src/cli/commands/init.rs         Agent setup logic (+120 lines)
          src/adapters/toml/parser.rs      AgentSection, ReviewSection structs
          src/adapters/git/hooks.rs        Add install_pre_push()
          Cargo.toml                       Add dirs dependency
Phase 5:  src/core/services/mod.rs         Add 5 new service modules
Phase 6:  src/cli/app.rs                   Replace Review command, add Findings
          src/cli/commands/mod.rs          Add review_pipeline + findings
          src/adapters/mod.rs              Update review adapter ref
          src/core/ports/review_store.rs   Verify trait with new Review shape
Phase 7:  tests/common/mocks.rs            Add 7 new mock methods
          tests/adapter/mod.rs             Add test modules
```

---

## Risk Register

| Risk                                        | Likelihood | Impact | Mitigation                                                                                                        |
| ------------------------------------------- | ---------- | ------ | ----------------------------------------------------------------------------------------------------------------- |
| **git2 diff API produces incomplete diffs** | Medium     | Medium | Fall back to `Command::new("git diff ...")` in diff_between(). Adapter pattern makes the swap local to one file.  |
| **Agent CLI flags change**                  | Low        | Medium | Each agent has its own adapter file. Flag change only affects claude.rs or codex.rs.                              |
| **Existing tests break in Phase 1**         | High       | Low    | Phase 1 updates all impacted tests. Type changes are structural, not behavioral. Run `cargo test` before Phase 2. |
| **Agent output parsing is brittle**         | Medium     | Medium | extract_findings() returns empty vec on parse failure, preserving raw output. Property tests in Phase 7.          |
| **Pre-push hook blocks pushes**             | Medium     | High   | Only blocks on severity=Block + status=Open. Users can dismiss. Hook removable via `git config core.hooksPath`.   |
