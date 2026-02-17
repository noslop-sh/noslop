# Implementation Plan

Actionable implementation plan for the review pipeline architecture. Zero backwards compatibility — all dead code is deleted, not deprecated. Each phase is a self-contained work package executable by a single agent.

## Table of Contents

- [Dead Code Manifest](#dead-code-manifest)
- [Phase Overview](#phase-overview)
- [Phase 1: Core Types + Dead Code Purge](#phase-1-core-types--dead-code-purge)
- [Phase 2: VCS Extensions](#phase-2-vcs-extensions)
- [Phase 3: Agent Ports and Adapters](#phase-3-agent-ports-and-adapters)
- [Phase 4: Review Pipeline](#phase-4-review-pipeline)
- [Phase 5: Init Expansion](#phase-5-init-expansion)
- [Phase 6: Review CLI](#phase-6-review-cli)
- [Phase 7: Integration Testing](#phase-7-integration-testing)
- [Agent Team Execution Plan](#agent-team-execution-plan)

---

## Dead Code Manifest

Everything listed here is deleted in Phase 1. Zero backwards compatibility — this is an unreleased product.

### Files to DELETE entirely (20 files)

**Core models (3):**

| File                                | What                                                | Replaced by                         |
| ----------------------------------- | --------------------------------------------------- | ----------------------------------- |
| `src/core/models/acknowledgment.rs` | Acknowledgment struct                               | `FindingStatus` on `Finding`        |
| `src/core/models/target.rs`         | Target, PathSpec, GlobPattern, Fragment, ParseError | `Target`, `Span` in `primitives.rs` |
| `src/core/models/severity.rs`       | Severity enum                                       | Moved to `primitives.rs`            |

**Ports (1):**

| File                                     | What                                           | Replaced by                                         |
| ---------------------------------------- | ---------------------------------------------- | --------------------------------------------------- |
| `src/core/ports/acknowledgment_store.rs` | AcknowledgmentStore trait, StorageBackend enum | `ReviewStore` persists `Review` with `Vec<Finding>` |

**Services (2):**

| File                           | What                                        | Replaced by                                 |
| ------------------------------ | ------------------------------------------- | ------------------------------------------- |
| `src/core/services/checker.rs` | check_items(), CheckResult, CheckItemResult | ReviewPipeline + ConventionAnalyzer         |
| `src/core/services/matcher.rs` | matches_target()                            | `Target.matches()` using `glob_match` crate |

**Adapters (2):**

| File                          | What                              | Replaced by             |
| ----------------------------- | --------------------------------- | ----------------------- |
| `src/adapters/file/mod.rs`    | FileStore (JSON ack staging)      | Findings live on Review |
| `src/adapters/trailer/mod.rs` | TrailerAckStore (commit trailers) | Findings live on Review |

**Facades (3):**

| File                 | What                                         | Why dead                         |
| -------------------- | -------------------------------------------- | -------------------------------- |
| `src/storage/mod.rs` | Storage facade, Backend enum, ack_store()    | Entire ack storage layer deleted |
| `src/git/mod.rs`     | Backwards-compat re-exports                  | Zero backwards compat            |
| `src/noslop_file.rs` | Backwards-compat wrapper over adapters::toml | Zero backwards compat            |

**Shared (5):**

| File                           | What                              | Why dead                                        |
| ------------------------------ | --------------------------------- | ----------------------------------------------- |
| `src/shared/mod.rs`            | Shared module root                | Both children dead                              |
| `src/shared/resolver.rs`       | Resolver using Fragment           | Fragment deleted                                |
| `src/shared/parser/mod.rs`     | Parser trait, ParserRegistry      | Not used by any code                            |
| `src/shared/parser/pattern.rs` | Pattern, PatternKind              | Not used by any code                            |
| `src/shared/parser/token.rs`   | Token, TokenKind, Span, TokenTree | Not used by any code (different Span than ours) |

**CLI commands (3):**

| File                               | What                              | Replaced by                              |
| ---------------------------------- | --------------------------------- | ---------------------------------------- |
| `src/cli/commands/ack.rs`          | `noslop ack` command              | `noslop findings resolve/dismiss`        |
| `src/cli/commands/add_trailers.rs` | commit-msg hook trailer insertion | Findings persist on Review, not trailers |
| `src/cli/commands/clear_staged.rs` | post-commit staged ack cleanup    | No staged acks to clear                  |

**Tests (5):**

| File                             | What                      | Why dead              |
| -------------------------------- | ------------------------- | --------------------- |
| `tests/unit/target_test.rs`      | Old Target parsing tests  | Target rewritten      |
| `tests/unit/storage_test.rs`     | AcknowledgmentStore tests | Storage layer deleted |
| `tests/unit/parser_test.rs`      | shared::parser tests      | Parser module deleted |
| `tests/unit/resolver_test.rs`    | Resolver tests            | Resolver deleted      |
| `tests/unit/proptest_matcher.rs` | matches_target proptests  | matcher.rs deleted    |

### Types to DELETE from surviving files

**`src/core/models/review.rs`** — delete these types, keep Review and ReviewStatus:

- `ReviewComment` struct (entire definition)
- `DiffPosition` struct (entire definition)
- `DiffSide` enum (entire definition)
- `CommentStatus` enum (entire definition)
- All methods that reference them: `add_comment()`, `open_comments()`, `open_comments_for_file()`

**`src/output.rs`** — delete ack-related types:

- `AckResult` struct
- `CheckMatch.acknowledged` field
- `CheckResult.acknowledged` field
- All `AckResult` rendering methods

**`src/cli/app.rs`** — delete these Command variants:

- `Command::Ack { id, message }`
- `Command::AddTrailers { commit_msg_file }`
- `Command::ClearStaged`
- `ReviewAction::Comment`, `ReviewAction::Resolve` (move to findings subcommand later)

**`src/cli/commands/check_validate.rs`** — rewrite entirely:

- Remove: ack loading, ack matching, `storage::ack_store()` usage
- Remove: `noslop_file::load_checks_for_files()` usage
- Replace with: direct check loading via `TomlCheckRepository`, matching via `Check.applies_to()`

**`src/adapters/toml/repository.rs`** — update:

- Remove: `use crate::core::services::matches_target`
- Replace: `matches_target(&entry.target, file, ...)` with check-level matching via new Target

**`src/core/mod.rs`** — update doc comments referencing old types

**`src/adapters/mod.rs`** — remove:

- `pub mod file;`
- `pub mod trailer;`
- `pub use file::FileStore;`
- `pub use trailer::{TrailerAckStore, append_trailers};`

**`tests/unit.rs`** — remove dead test modules:

- `mod target_test;`
- `mod storage_test;`
- `mod parser_test;`
- `mod resolver_test;`
- `mod proptest_matcher;`

**`tests/common/fixtures.rs`** — remove Acknowledgment fixtures
**`tests/common/mocks.rs`** — remove AcknowledgmentStore mock methods

### GUI updates (defer to Phase 6)

**`gui/src-tauri/src/dto.rs`** — rewrite ReviewDto/CommentDto to use Finding
**`gui/src-tauri/src/commands.rs`** — rewrite to use new Review/Finding types, remove Acknowledgment/storage usage

---

## Phase Overview

```
P1 (core types + purge) ──→ P2 (VCS) ──────────+
                        ├──→ P3 (agents) ───────+──→ P6 (CLI) ──→ P7 (testing)
                        ├──→ P4 (pipeline) ─────+
                        └──→ P5 (init, needs P3)
```

Phases 2, 3, 4 run in parallel after Phase 1. Phase 5 needs Phase 3. Phase 6 needs 2+3+4. Phase 7 needs all.

| Phase | Description                  | New | Modify | Delete | Est. Lines           |
| ----- | ---------------------------- | --- | ------ | ------ | -------------------- |
| 1     | Core types + dead code purge | 3   | ~12    | 20     | ~250 new, -3000 dead |
| 2     | VCS extensions               | 3   | 4      | 0      | ~400                 |
| 3     | Agent ports and adapters     | 12  | 2      | 0      | ~1,000               |
| 4     | Review pipeline              | 5   | 1      | 0      | ~650                 |
| 5     | Init expansion               | 0   | 5      | 0      | ~200                 |
| 6     | Review CLI + GUI             | 3   | 6      | 0      | ~600                 |
| 7     | Integration testing          | 5   | 3      | 0      | ~400                 |

---

## Phase 1: Core Types + Dead Code Purge

**Goal**: Replace the domain model, delete all dead code, reach `cargo check && cargo test` passing with the new types. This is the critical path — everything else depends on it.

**Depends on**: Nothing.

### Step 1: Create new model files

**NEW `src/core/models/primitives.rs`** (~80 lines)

Contains `Severity`, `Span`, `Target`, `FindingSource`. Full definitions in [00-core-domain-model.md](./00-core-domain-model.md). Key points:

- Severity: same 3 levels (Info/Warn/Block), add `PartialOrd, Ord, Hash` derives, add `is_blocking()` method
- Span: `{ start: u32, end: u32 }`, constructors `line(n)` and `range(s, e)`
- Target: `{ path: String, span: Option<Span>, commit: Option<String> }`, constructors `file()` and `pattern()`, methods `is_glob()`, `matches()`, `with_span()`, `with_commit()`
- FindingSource: tagged enum `Check(String) | Script(String) | Agent(String) | Human`

**NEW `src/core/models/finding.rs`** (~70 lines)

Contains `Finding`, `FindingStatus`. Full definitions in [00-core-domain-model.md](./00-core-domain-model.md). Key points:

- Finding: id, target, severity, message, source, status, suggestion, created_at
- FindingStatus: Open (default), Resolved, Dismissed
- Methods: `new()`, `with_suggestion()`, `is_blocking()`, `resolve()`, `dismiss()`, `is_open()`

**NEW `src/core/models/agent.rs`** (~40 lines)

Contains `AgentKind` enum (Claude, Codex). `command()`, `display_name()`, `Display`, `FromStr`.

### Step 2: Rewrite existing model files

**REWRITE `src/core/models/check.rs`**

- Change `target: String` → `target: Target`
- Drop `introduced_by: Option<String>` and `created_at: String` fields
- Add `tags: Vec<String>` with `#[serde(default, skip_serializing_if = "Vec::is_empty")]`
- Add `into_finding(matched_path: &str) -> Finding` method
- Update `new()`: remove timestamp parameter. Constructor takes `(id, target: Target, message, severity)`.
- Update `applies_to()`: delegate to `self.target.matches(path)`
- Update inline tests

**REWRITE `src/core/models/review.rs`**

- Delete entirely: `ReviewComment`, `DiffPosition`, `DiffSide`, `CommentStatus` (and all their impls)
- Change `comments: Vec<ReviewComment>` → `findings: Vec<Finding>`
- Rename `base_sha` → `base`, `head_sha` → `head`
- Drop `description` field
- Add methods: `add_finding()`, `add_findings()`, `blocking_findings()`, `is_blocked()`, `findings_for_file()`, `open_findings()`, `close()`
- Delete methods: `add_comment()`, `open_comments()`, `open_comments_for_file()`
- Update inline tests

### Step 3: Update `src/core/models/mod.rs`

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

### Step 4: Delete dead files

Delete all files listed in the Dead Code Manifest:

```
rm src/core/models/acknowledgment.rs
rm src/core/models/target.rs
rm src/core/models/severity.rs
rm src/core/ports/acknowledgment_store.rs
rm src/core/services/checker.rs
rm src/core/services/matcher.rs
rm src/adapters/file/mod.rs
rm src/adapters/trailer/mod.rs
rm src/storage/mod.rs
rm src/git/mod.rs
rm src/noslop_file.rs
rm -r src/shared/
rm src/cli/commands/ack.rs
rm src/cli/commands/add_trailers.rs
rm src/cli/commands/clear_staged.rs
rm tests/unit/target_test.rs
rm tests/unit/storage_test.rs
rm tests/unit/parser_test.rs
rm tests/unit/resolver_test.rs
rm tests/unit/proptest_matcher.rs
```

### Step 5: Update all module declarations and re-exports

**`src/core/ports/mod.rs`** — remove acknowledgment_store module and re-exports:

```rust
mod check_repo;
mod review_store;
mod vcs;

pub use check_repo::CheckRepository;
pub use review_store::ReviewStore;
pub use vcs::VersionControl;
```

**`src/core/services/mod.rs`** — empty for now (pipeline services added in Phase 4):

```rust
//! Business logic services
```

**`src/core/mod.rs`** — update doc comments, remove references to old types

**`src/adapters/mod.rs`** — remove file, trailer modules:

```rust
pub mod git;
pub mod review;
pub mod toml;

pub use git::{GitVersionControl, get_repo_name};
pub use review::FileReviewStore;
pub use toml::TomlCheckRepository;
```

**`src/lib.rs`** — complete rewrite of re-exports:

```rust
pub mod adapters;
pub mod core;
pub mod output;

pub use core::models::{
    AgentKind, Check, Finding, FindingSource, FindingStatus,
    Review, ReviewStatus, Severity, Span, Target,
};
pub use core::ports::{CheckRepository, ReviewStore, VersionControl};
```

Remove: `pub mod shared`, `pub mod storage`, `pub use shared::parser`, `pub use shared::resolver`, all old re-exports.

**`src/main.rs`** — remove `mod git`, `mod noslop_file`:

```rust
mod cli;

fn main() {
    if let Err(e) = cli::run() {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
```

### Step 6: Update CLI

**`src/cli/app.rs`** — remove dead Command variants:

- Delete: `Ack`, `AddTrailers`, `ClearStaged`
- Delete: `ReviewAction::Comment`, `ReviewAction::Resolve` (reimplemented as `noslop findings` in Phase 6)
- Update dispatch in `run()` to remove deleted arms
- Update about text: "Pre-commit checks with acknowledgment tracking" → "Local code review for agent-generated code"

**`src/cli/commands/mod.rs`** — remove dead modules:

```rust
mod check_manage;
mod check_validate;
mod init;
mod review;

pub use check_manage::check_manage;
pub use check_validate::check_validate;
pub use init::init;
pub use review::review;
```

**`src/cli/commands/check_validate.rs`** — rewrite without ack logic:

- Remove: `use crate::{git, noslop_file}`, `use noslop::storage`, all ack loading/matching
- Use: `GitVersionControl` directly for staged files, `TomlCheckRepository` for checks
- Match each check against staged files via `check.applies_to(file)`
- Display results (no acknowledgment column)
- Exit 1 if any Block-severity check applies (simple: any match = blocked, until pipeline is wired in Phase 6)

**`src/cli/commands/review.rs`** — simplify to just start/list/show/close (remove comment/resolve which move to findings in Phase 6). Update to use new Review shape (findings not comments).

### Step 7: Update adapters that reference dead types

**`src/adapters/toml/repository.rs`**:

- Remove: `use crate::core::services::matches_target`
- Update Check construction to use `Target::pattern(entry.target)` instead of plain String
- Replace `matches_target(...)` calls with `Target::pattern(&entry.target).matches(file)` or construct the Check first and use `check.applies_to(file)`

**`src/adapters/review.rs`** (FileReviewStore):

- Update to work with new Review shape (findings not comments)
- Rewrite any methods that reference `open_comments_for_file()` to use `review.findings.iter().filter(...)`

**`src/output.rs`**:

- Delete: `AckResult` struct and its render methods
- Remove: `acknowledged` field from `CheckMatch` and `CheckResult`
- Keep: `OutputMode`, `CheckResult` (simplified), `CheckListResult`, `CheckInfo`, `OperationResult`

### Step 8: Update test infrastructure

**`tests/unit.rs`** — remove dead test modules:

```rust
mod common;
mod check_test;
mod cli_test;
mod output_test;
mod parameterized_test;
```

**`tests/common/fixtures.rs`** — remove `Acknowledgment` fixture builders
**`tests/common/mocks.rs`** — remove `AcknowledgmentStore` mock, update `VersionControl` mock if needed

**`tests/unit/check_test.rs`** — update for new Check shape (Target instead of String, no introduced_by/created_at)
**`tests/unit/cli_test.rs`** — update for removed CLI commands
**`tests/unit/output_test.rs`** — update for removed AckResult
**`tests/unit/parameterized_test.rs`** — update if it references dead types

**`tests/integration/main.rs`** — major rewrite:

- Remove all ack-related tests (test*ack*_, test*multiple_acks*_, staged acks tests)
- Remove review comment tests that use old ReviewComment/DiffPosition
- Keep: init tests, check add/list/remove tests, severity tests, CI mode tests, nested .noslop.toml tests
- Update remaining tests: no more "acknowledgment" workflow, just check-matching

**`tests/integration/lifecycle_test.rs`** — rewrite or delete depending on content

### Step 9: Verify

```bash
cargo check
cargo test
cargo clippy -- -D warnings
```

All 3 must pass before proceeding to any subsequent phase.

---

## Phase 2: VCS Extensions

**Goal**: Extend `VersionControl` trait with diff/history methods. Add checkpoint command. **Depends on**: Phase 1.

### NEW `src/core/models/diff.rs` (~90 lines)

VCS-internal types (not part of the 11-type domain model — these are adapter-layer data structures):

- `FileDiff { path, old_path, status: DiffStatus, hunks: Vec<DiffHunk> }`
- `DiffStatus` enum: Added, Modified, Deleted, Renamed
- `DiffHunk { old_start, old_count, new_start, new_count, lines: Vec<DiffLine> }`
- `DiffLine { kind: DiffLineKind, content }`, `DiffLineKind`: Context, Addition, Deletion
- `CommitInfo { sha, short_sha, summary, message, author, timestamp: i64 }`
- `DiffStat { files_changed, insertions, deletions }`

Add to `src/core/models/mod.rs` but do NOT re-export from lib.rs (these are internal types).

### MODIFY `src/core/ports/vcs.rs` (+40 lines)

Extend `VersionControl` trait with 7 new methods:

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

Functions using `git2`: `is_clean(repo)`, `commit_all(repo, message)`, `default_signature(repo)`.

### MODIFY `src/adapters/git/mod.rs` (+120 lines)

Implement 7 new VersionControl methods on GitVersionControl.

### NEW `src/cli/commands/checkpoint.rs` (~50 lines)

`noslop checkpoint [message]` command handler.

### MODIFY `src/cli/app.rs` (+10 lines)

Add `Checkpoint { message: Option<String> }` variant and dispatch.

### MODIFY `src/cli/commands/mod.rs` (+2 lines)

Add `mod checkpoint; pub use checkpoint::checkpoint;`

**Verify**: `cargo check && cargo test`

---

## Phase 3: Agent Ports and Adapters

**Goal**: Define agent port traits, implement Claude + Codex adapters. **Depends on**: Phase 1.

### NEW `src/core/ports/agent.rs` (~200 lines)

Config types: `GlobalInstructions`, `HookDefinition`, `SettingsFile`, `SlashCommand`, `AgentConfigBundle`.
Invocation types: `InvocationConfig`, `InvocationResult`, `ReviewOutput`.

Traits (both with `#[cfg_attr(test, mockall::automock)]`):

- `AgentConfig`: agent_kind, generate_config, is_available, validate, install_instructions, generate_project_instructions
- `AgentRuntime`: agent_kind, invoke, parse_output, is_available, build_command

### NEW `src/core/ports/analyzer.rs` (~35 lines)

- `ReviewContext { diff, changed_files, repo_root, commit_range }`
- `ContextKind` enum
- `ReviewAnalyzer` trait: name, required_context, analyze(context, prior_findings)

### MODIFY `src/core/ports/mod.rs` (+8 lines)

Add module declarations and re-exports for `agent` and `analyzer`.

### NEW `src/adapters/agents/mod.rs` (~30 lines)

Factory: `get_agent_config(AgentKind)`, `get_agent_runtime(AgentKind)`.

### NEW `src/adapters/agents/parsing.rs` (~80 lines)

Shared parser: `extract_findings(output) -> ReviewOutput`, `extract_json_array()`, `extract_errors()`.

### NEW `src/adapters/agents/claude.rs` (~350 lines)

ClaudeConfig + ClaudeRuntime implementations.

### NEW `src/adapters/agents/codex.rs` (~200 lines)

CodexConfig + CodexRuntime implementations.

### NEW template files (6 files, embedded via `include_str!`)

```
src/adapters/agents/templates/
  claude_global_instructions.md
  claude_branch_protection.sh
  claude_auto_format.sh
  claude_cmd_checkpoint.md
  claude_cmd_worktree.md
  codex_instructions.md
```

### MODIFY `src/adapters/mod.rs` (+2 lines)

Add `pub mod agents;` and re-export factory functions.

**Verify**: `cargo check && cargo test`

---

## Phase 4: Review Pipeline

**Goal**: Implement ReviewPipeline orchestrator, built-in analyzers, script analyzer, AgentAnalyzer bridge. **Depends on**: Phase 1 (Phase 3 needed only for AgentAnalyzer, which can use a mock).

### NEW `src/core/services/pipeline.rs` (~80 lines)

`ReviewPipeline { analyzers: Vec<Box<dyn ReviewAnalyzer>> }`. Method `run(context) -> Result<Vec<Finding>>`: fold semantics — each analyzer receives all prior findings.

### NEW `src/core/services/analyzers.rs` (~200 lines)

- `ConventionAnalyzer { checks }` — matches changed files against check targets, calls `check.into_finding()`. Name: "conventions".
- `FormattingAnalyzer { config }` — runs formatters in `--check` mode. Name: "formatting".
- `CoChangeAnalyzer { config }` — git history correlation. Name: "co-change".

### NEW `src/core/services/script_analyzer.rs` (~60 lines)

`ScriptAnalyzer { name, command }` — JSON stdin/stdout protocol. Name: "custom:\<name\>".

### NEW `src/core/services/agent_analyzer.rs` (~120 lines)

`AgentAnalyzer` — bridge between `AgentRuntime` and `ReviewAnalyzer`. Builds prompt with `{diff}`, `{prior_findings}`, `{focus}`, `{changed_files}` placeholders. Name: "agent:\<focus\>".

### NEW `src/core/services/pipeline_builder.rs` (~100 lines)

`build_pipeline(config, checks, agent_kind, prompts_dir) -> ReviewPipeline`. Dispatches to correct analyzer for each config entry.

### MODIFY `src/core/services/mod.rs` (+5 lines)

Add module declarations for all 5 new service files.

**Verify**: `cargo check && cargo test`

---

## Phase 5: Init Expansion

**Goal**: Expand `noslop init` to accept agent argument, write `[agent]` section, configure agent, install pre-push hook. **Depends on**: Phase 1 + Phase 3.

### MODIFY `src/cli/app.rs` (+5 lines)

Add `agent: Option<String>` positional arg to `Command::Init`.

### MODIFY `src/cli/commands/init.rs` (+120 lines)

If agent specified: parse `AgentKind::from_str`, call `get_agent_config(kind)`, validate, generate_config, write bundle, generate project instructions, install pre-push hook. Write `[agent] type = "<kind>"` to `.noslop.toml`.

### MODIFY `src/adapters/toml/parser.rs` (+50 lines)

Add optional fields to `NoslopFile`: `agent: Option<AgentSection>`, `review: Option<ReviewSection>`. New structs: `AgentSection`, `ReviewSection`, `CoChangeConfig`, `FormattingConfig`, `AgentAnalyzerConfig`, `ScriptAnalyzerConfig`.

### MODIFY `src/adapters/git/hooks.rs` (+30 lines)

Add `install_pre_push()`: writes hook script running `noslop review --check`.

### MODIFY `Cargo.toml` (+1 line)

Add `dirs = "6.0"` dependency.

**Verify**: `cargo check && cargo test`

---

## Phase 6: Review CLI

**Goal**: Wire up `noslop review` and `noslop findings` commands, implement JsonReviewStore, update GUI. **Depends on**: Phases 2 + 3 + 4.

### NEW `src/adapters/review_store.rs` (~100 lines)

`JsonReviewStore` — replaces existing `adapters/review.rs`. Persists reviews as `.noslop/reviews/<id>.json`. Implements `ReviewStore` trait.

### NEW `src/cli/commands/review_pipeline.rs` (~200 lines)

Handler for `noslop review`. Flow: load config → diff_between → build ReviewContext → build_pipeline → run → save Review → display findings. `--check` mode: exit 1 if blocking.

### NEW `src/cli/commands/findings.rs` (~150 lines)

Handler for `noslop findings`. Subcommands: list (default), `resolve <id>`, `dismiss <id> --reason`.

### MODIFY `src/cli/app.rs` (+25 lines)

Replace existing Review command. Add `Findings { action }`. Update dispatch.

### MODIFY `src/cli/commands/mod.rs` (+4 lines)

Add modules for `review_pipeline` and `findings`.

### MODIFY `src/adapters/mod.rs` (+1 line)

Update review adapter reference.

### MODIFY GUI files

**`gui/src-tauri/src/dto.rs`** — rewrite:

- `ReviewDto`: `base`/`head` (not `base_sha`/`head_sha`), `findings: Vec<FindingDto>` (not `comments`)
- `FindingDto` replaces `CommentDto`: id, target, severity, message, source, status, suggestion

**`gui/src-tauri/src/commands.rs`** — rewrite:

- Remove: `Acknowledgment`, `DiffPosition`, `storage` imports
- `add_comment()` → `add_finding()` using `Finding::new()`
- `resolve_comment()` → `resolve_finding()` using `Finding.resolve()`
- `close_review()` → remove ack staging, just close

**Verify**: `cargo check && cargo test`

---

## Phase 7: Integration Testing

**Goal**: End-to-end tests, full workflow validation. **Depends on**: All phases.

### NEW test files (5)

- `tests/adapter/checkpoint_test.rs` (~80 lines) — is_clean, commit_all using tempfile+git2
- `tests/adapter/agent_test.rs` (~100 lines) — extract_findings, config generation
- `tests/unit/pipeline_test.rs` (~100 lines) — mock-based: order, fold semantics, prompt building
- `tests/unit/finding_test.rs` (~60 lines) — serialization roundtrip, Target.is_glob, Span constructors
- `tests/integration/review_test.rs` (~100 lines) — full CLI: review with findings, --check exit code, checkpoint

### MODIFY test infrastructure

- `tests/common/mocks.rs` (+40 lines) — add 7 new MockVersionControl methods
- `tests/adapter/mod.rs` (+2 lines) — add new test modules
- `tests/unit.rs` (+2 lines) — add new test modules

### Rewrite integration tests

- `tests/integration/main.rs` — rewrite workflow tests:
  - `noslop init` → create branch → make changes → `noslop review` → findings shown → `noslop findings resolve` → `noslop review --check` passes → push
  - Replace all ack-based test workflows with finding-based workflows

### Polish

1. `cargo clippy -- -D warnings`
2. Doc comments on all public types
3. Consistent human/JSON output

**Verify**: `cargo test && cargo clippy -- -D warnings && cargo doc --no-deps`

---

## Agent Team Execution Plan

The phases map to agent work packages with explicit dependency ordering:

```
┌──────────────────────────────────────────────────────────┐
│  WAVE 1 (serial — critical path)                         │
│                                                          │
│  Agent A: Phase 1 — Core Types + Dead Code Purge         │
│  • Create primitives.rs, finding.rs, agent.rs            │
│  • Rewrite check.rs, review.rs                           │
│  • Delete 20 files                                       │
│  • Fix all compilation + tests                           │
│  • GATE: cargo check && cargo test && cargo clippy pass  │
└──────────────────────┬───────────────────────────────────┘
                       │
    ┌──────────────────┼──────────────────┐
    ▼                  ▼                  ▼
┌────────────┐  ┌──────────────┐  ┌──────────────┐
│ WAVE 2a    │  │ WAVE 2b      │  │ WAVE 2c      │
│            │  │              │  │              │
│ Agent B:   │  │ Agent C:     │  │ Agent D:     │
│ Phase 2    │  │ Phase 3      │  │ Phase 4      │
│ VCS +      │  │ Agent Ports  │  │ Pipeline +   │
│ Checkpoint │  │ + Adapters   │  │ Analyzers    │
└────────────┘  └──────┬───────┘  └──────────────┘
                       │
                       ▼
                ┌──────────────┐
                │ WAVE 3       │
                │              │
                │ Agent E:     │
                │ Phase 5      │
                │ Init         │
                │ Expansion    │
                └──────────────┘
                       │ (+ wait for B, D)
                       ▼
                ┌──────────────┐
                │ WAVE 4       │
                │              │
                │ Agent F:     │
                │ Phase 6      │
                │ Review CLI   │
                │ + GUI        │
                └──────────────┘
                       │
                       ▼
                ┌──────────────┐
                │ WAVE 5       │
                │              │
                │ Agent G:     │
                │ Phase 7      │
                │ Integration  │
                │ Testing      │
                └──────────────┘
```

### Agent instructions

Each agent receives:

1. This implementation plan (their phase section)
2. The relevant design document(s):
   - Agent A: [00-core-domain-model.md](./00-core-domain-model.md) + Dead Code Manifest above
   - Agent B: [03-vcs-config-cli.md](./03-vcs-config-cli.md) (VCS section)
   - Agent C: [02-agent-system.md](./02-agent-system.md)
   - Agent D: [01-review-pipeline.md](./01-review-pipeline.md)
   - Agent E: [03-vcs-config-cli.md](./03-vcs-config-cli.md) (Init section)
   - Agent F: [03-vcs-config-cli.md](./03-vcs-config-cli.md) (CLI section) + GUI files
   - Agent G: [05-test-strategy.md](./05-test-strategy.md)

### Gate conditions

Each wave must pass before the next starts:

- **Wave 1 gate**: `cargo check && cargo test && cargo clippy -- -D warnings`
- **Wave 2 gate**: same (per-agent, all 3 must pass)
- **Wave 3-5 gates**: same

### Dependencies

One new dependency added in Phase 5:

```toml
dirs = "6.0"
```

Phase 1 needs `glob_match` for `Target.matches()`. Check if `glob` crate already covers this or add `glob_match`.
