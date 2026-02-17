# 03 — VCS Extensions, Configuration Schema, and CLI Surface

Design specification consolidating three concerns that share a boundary: VCS trait extensions that power the review pipeline, the `.noslop.toml` configuration schema that controls it, and the complete CLI surface that exposes it.

## Scope

| Area             | What Ships                                                     | Rust Target                                  |
| ---------------- | -------------------------------------------------------------- | -------------------------------------------- |
| VCS extensions   | 7 new methods on `VersionControl` trait + VCS-internal types   | `src/core/ports/vcs.rs`, `src/adapters/git/` |
| Checkpoint       | `noslop checkpoint [message]` command                          | `src/cli/commands/checkpoint.rs`             |
| Configuration    | Full `.noslop.toml` schema with `[agent]`, `[review]` sections | `src/adapters/toml/parser.rs`                |
| Directory layout | `.noslop/` directory structure                                 | Runtime, not code                            |
| Init expansion   | `noslop init [agent]` with agent configuration                 | `src/cli/commands/init.rs`                   |
| CLI reference    | Complete command surface                                       | `src/cli/app.rs`                             |

### What Does NOT Ship

- Worktree management (deferred)
- Review memory / pattern graduation (future consideration)
- Multi-repo detection (dropped permanently)

---

## 1. VCS Extensions

### Extended `VersionControl` Trait

The `VersionControl` trait in `src/core/ports/vcs.rs` gains two categories of methods: checkpoint operations and review pipeline queries.

```rust
#[cfg_attr(test, mockall::automock)]
pub trait VersionControl: Send + Sync {
    // --- existing methods (unchanged) ---
    fn staged_files(&self) -> anyhow::Result<Vec<String>>;
    fn repo_name(&self) -> String;
    fn repo_root(&self) -> anyhow::Result<std::path::PathBuf>;
    fn install_hooks(&self, force: bool) -> anyhow::Result<()>;
    fn is_inside_repo(&self, path: &Path) -> bool;
    fn current_branch(&self) -> anyhow::Result<Option<String>>;

    // --- checkpoint methods ---

    /// Returns true if no staged, unstaged, or untracked changes exist.
    fn is_clean(&self) -> anyhow::Result<bool>;

    /// Stage all changes and commit with the given message, bypassing hooks.
    /// Returns the short SHA of the new commit, or None if the tree was clean.
    fn commit_all(&self, message: &str) -> anyhow::Result<Option<String>>;

    // --- review pipeline methods ---

    /// Diff between two refs. Primary input to the review pipeline.
    fn diff_between(&self, base: &str, head: &str) -> anyhow::Result<Vec<FileDiff>>;

    /// List commits between two refs (for commit message analysis).
    fn commits_between(&self, base: &str, head: &str) -> anyhow::Result<Vec<CommitInfo>>;

    /// Read file content at a specific revision (for full-file context).
    fn file_at_revision(&self, path: &Path, rev: &str) -> anyhow::Result<String>;

    /// Recent commits from HEAD (for context).
    fn recent_commits(&self, count: usize) -> anyhow::Result<Vec<CommitInfo>>;

    /// Summary statistics for changes since a ref.
    fn diff_stat_since(&self, since: &str) -> anyhow::Result<DiffStat>;
}
```

### VCS-Internal Types

These types live in `src/adapters/git/diff_types.rs`. They are VCS-specific data structures, not domain models. Domain types (`Finding`, `Review`, `Target`, etc.) are defined in `core::models` (see doc 00).

```rust
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileDiff {
    pub path: PathBuf,
    pub old_path: Option<PathBuf>,
    pub status: DiffStatus,
    pub hunks: Vec<DiffHunk>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffStatus { Added, Modified, Deleted, Renamed }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffHunk {
    pub old_start: u32,
    pub old_lines: u32,
    pub new_start: u32,
    pub new_lines: u32,
    pub lines: Vec<DiffLine>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffLine {
    pub kind: DiffLineKind,
    pub content: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffLineKind { Context, Addition, Deletion }

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

### Git Adapter Implementation

All methods implemented on `GitVersionControl` using `git2` (already a dependency at v0.20.2). Review pipeline methods are read-only queries; checkpoint methods stage and commit.

**Module registration** (`src/adapters/git/mod.rs`):

```rust
pub mod hooks;
pub mod staging;
pub mod checkpoint;    // NEW
pub mod diff_types;    // NEW
```

**Key implementation patterns**:

| Method             | git2 API                                        |
| ------------------ | ----------------------------------------------- |
| `is_clean`         | `repo.statuses()` with untracked                |
| `commit_all`       | `index.add_all` + `update_all` + `repo.commit`  |
| `diff_between`     | `repo.diff_tree_to_tree` then parse hunks       |
| `commits_between`  | `repo.revwalk` with `push(head)` + `hide(base)` |
| `file_at_revision` | `tree.get_path` + `repo.find_blob`              |
| `recent_commits`   | `repo.revwalk` from HEAD with `.take(count)`    |
| `diff_stat_since`  | `repo.diff_tree_to_tree` then `diff.stats()`    |

**Checkpoint adapter** (`src/adapters/git/checkpoint.rs`):

```rust
use git2::{Repository, Signature, StatusOptions};

pub fn is_clean(repo: &Repository) -> anyhow::Result<bool> {
    let mut opts = StatusOptions::new();
    opts.include_untracked(true).recurse_untracked_dirs(true);
    Ok(repo.statuses(Some(&mut opts))?.is_empty())
}

pub fn commit_all(repo: &Repository, message: &str) -> anyhow::Result<Option<String>> {
    if is_clean(repo)? { return Ok(None); }
    let mut index = repo.index()?;
    index.add_all(["*"], git2::IndexAddOption::DEFAULT, None)?;
    index.update_all(["*"], None)?;
    index.write()?;
    let tree = repo.find_tree(index.write_tree()?)?;
    let parent = repo.head()?.peel_to_commit()?;
    let sig = repo.signature()
        .or_else(|_| Signature::now("noslop", "noslop@localhost"))?;
    let oid = repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &[&parent])?;
    Ok(Some(oid.to_string()[..7].to_string()))
}
```

### Why One Trait (not a separate port)

All methods operate on the same git repository. The review pipeline calls `diff_between`, `commits_between`, and `file_at_revision` on the same VCS instance in a single invocation. `MockVersionControl` already covers everything.

---

## 2. Checkpoint Command

```
noslop checkpoint                          # "checkpoint: 2026-02-16 12:30:00"
noslop checkpoint "before fixing findings" # custom message
noslop checkpoint --json                   # JSON output
```

A developer runs `noslop review`, sees findings, checkpoints current state, then fixes. That is the entire use case. The command handler in `src/cli/commands/checkpoint.rs` calls `GitVersionControl::commit_all()` with either the provided message or a timestamp-based default.

**Output**: `Checkpoint: a1b2c3d <message>` or `Nothing to checkpoint (working tree clean).`

---

## 3. Configuration Schema

### Complete `.noslop.toml` Reference

```toml
# --- Project metadata (existing, unchanged) ---
[project]
prefix = "NOS"                  # 3-letter prefix for auto-generated check IDs

# --- Agent configuration (new, optional) ---
[agent]
type = "claude"                  # "claude" | "codex"

# --- Review pipeline (new, optional) ---
[review]
analyzers = ["conventions", "formatting", "co-change", "agent:security", "agent:quality"]

[review.co-change]
min_correlation = 0.7            # Minimum co-change frequency (0.0-1.0)
lookback = 500                   # Commits to scan for correlation
min_commits = 10                 # Minimum co-occurrences before flagging

[review.formatting]
tools = ["rustfmt", "prettier"]

[review."agent:security"]
agent = "claude"                                # Override default agent
prompt_template = ".noslop/prompts/security.md" # Optional custom prompt

[review."agent:quality"]
agent = "claude"

[review."custom:migrations"]
type = "script"
command = "python ./scripts/check-migrations.py"

# --- Check definitions (existing, unchanged) ---
[[check]]
id = "NOS-1"
target = "src/adapters/git/hooks.rs"
message = "Verify hook installation"
severity = "block"
tags = ["hooks"]
```

### Section Reference

| Section               | Status   | Required | Written By            | Read By                         |
| --------------------- | -------- | -------- | --------------------- | ------------------------------- |
| `[project]`           | Existing | Yes      | `noslop init`         | `noslop check`, `noslop review` |
| `[agent]`             | New      | No       | `noslop init <agent>` | AgentAnalyzer                   |
| `[review]`            | New      | No       | User (manual edit)    | `noslop review`                 |
| `[review.co-change]`  | New      | No       | User (manual edit)    | CoChangeAnalyzer                |
| `[review.formatting]` | New      | No       | User (manual edit)    | FormattingAnalyzer              |
| `[review."agent:*"]`  | New      | No       | User (manual edit)    | AgentAnalyzer                   |
| `[review."custom:*"]` | New      | No       | User (manual edit)    | ScriptAnalyzer                  |
| `[[check]]`           | Existing | No       | `noslop check add`    | `noslop check`, pre-commit hook |

### Field Definitions

**`[project]`**: `prefix` (string, default `"CHK"`) -- prefix for auto-generated check IDs.

**`[agent]`**: `type` (string, required) -- `"claude"` or `"codex"`.

**`[review]`**: `analyzers` (string[], default `["conventions"]`) -- ordered list for the pipeline.

Analyzer naming convention:

- **Plain name** = built-in: `"conventions"`, `"formatting"`, `"co-change"`
- **`agent:` prefix** = LLM-based: `"agent:security"`, `"agent:quality"`
- **`custom:` prefix** = external script: `"custom:migrations"`

**`[review.co-change]`**: `min_correlation` (f64, 0.7), `lookback` (u32, 500), `min_commits` (u32, 10).

**`[review.formatting]`**: `tools` (string[], []) -- formatter commands to check.

**`[review."agent:*"]`**: `agent` (string, falls back to `[agent].type`), `prompt_template` (string, optional path to custom prompt).

**`[review."custom:*"]`**: `type` (string, must be `"script"`), `command` (string, shell command).

**`[[check]]`** (unchanged): `id` (optional, auto-generated), `target` (string), `message` (string), `severity` (string, default `"block"`), `tags` (string[], []).

### CLI Override Precedence

```
1. CLI flag           --analyzer conventions,formatting
2. .noslop.toml       [review] analyzers = [...]
3. Compiled default   ["conventions"]
```

| CLI Flag                    | TOML Override        | Compiled Default  |
| --------------------------- | -------------------- | ----------------- |
| `--analyzer name[,name...]` | `[review] analyzers` | `["conventions"]` |
| `--agent claude\|codex`     | `[agent] type`       | --                |
| `--base main`               | (inferred from git)  | default branch    |
| `--check`                   | (none, CLI-only)     | `false`           |

### Parser Structs (`src/adapters/toml/parser.rs`)

```rust
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct NoslopFile {
    #[serde(default)]
    pub project: ProjectConfig,
    #[serde(default)]
    pub agent: Option<AgentSection>,
    #[serde(default)]
    pub review: Option<ReviewSection>,
    #[serde(default, rename = "check")]
    pub checks: Vec<CheckEntry>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AgentSection {
    #[serde(rename = "type")]
    pub agent_type: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct ReviewSection {
    pub analyzers: Vec<String>,
    #[serde(rename = "co-change")]
    pub co_change: Option<CoChangeConfig>,
    pub formatting: Option<FormattingConfig>,
    /// Catch-all for agent:* and custom:* sub-tables.
    #[serde(flatten)]
    pub analyzer_configs: HashMap<String, toml::Value>,
}

impl Default for ReviewSection {
    fn default() -> Self {
        Self {
            analyzers: vec!["conventions".to_string()],
            co_change: None,
            formatting: None,
            analyzer_configs: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct CoChangeConfig {
    pub min_correlation: f64,   // default 0.7
    pub lookback: u32,          // default 500
    pub min_commits: u32,       // default 10
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct FormattingConfig {
    pub tools: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AgentAnalyzerConfig {
    pub agent: Option<String>,
    pub prompt_template: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ScriptAnalyzerConfig {
    #[serde(rename = "type")]
    pub analyzer_type: String,
    pub command: String,
}
```

All new fields on `NoslopFile` use `#[serde(default)]`, so existing `.noslop.toml` files parse without error. No changes to `load_file`, `find_noslop_files`, or `TomlCheckRepository`.

---

## 4. `.noslop/` Directory Layout

```
.noslop/
  .gitkeep              # Ensures directory exists in git (existing)
  staged-acks.json      # Pending acknowledgments for commit (existing)
  reviews/              # Review session JSON files (new)
    {review-id}.json
  prompts/              # Custom prompt templates for agent analyzers (new)
    security.md
    quality.md
```

| File               | Created By      | Updated By        | Read By                  | In Git |
| ------------------ | --------------- | ----------------- | ------------------------ | ------ |
| `.gitkeep`         | `noslop init`   | --                | --                       | Yes    |
| `staged-acks.json` | `noslop ack`    | `noslop ack`      | pre-commit hook          | No     |
| `reviews/*.json`   | `noslop review` | `noslop findings` | `noslop findings`, hooks | Yes    |
| `prompts/*.md`     | User            | User              | AgentAnalyzer            | Yes    |

Only `staged-acks.json` is gitignored. Review sessions use the `Review` domain model from `core::models` serialized as JSON via `JsonReviewStore`. Prompt templates are user-authored markdown referenced by `prompt_template` in config.

---

## 5. Init Expansion

### Command Forms

| Command              | What it does                                                     |
| -------------------- | ---------------------------------------------------------------- |
| `noslop init`        | Git hooks + `.noslop.toml` + `.noslop/` directory (unchanged)    |
| `noslop init claude` | All above + Claude as review agent + `[agent]` in `.noslop.toml` |
| `noslop init codex`  | All above + Codex as review agent + `[agent]` in `.noslop.toml`  |

**Invariant**: Bare `noslop init` continues to work exactly as today.

### CLI Change

The `Init` variant gains an optional positional `agent: Option<String>` field. Parsed as `String` (not `AgentKind`) for error message control.

### Init Flow

```
noslop init claude
    |
    v
Phase 1: Core noslop setup
    1. Check .noslop.toml exists (bail unless --force)
    2. generate_prefix_from_repo() -> "NOS"
    3. parse_agent_type("claude") -> Ok(AgentKind::Claude)
    4. build_noslop_toml("NOS", Some(Claude))
       -> .noslop.toml with [project] + [agent] + [review]
    5. Create .noslop/ directory + .gitkeep
    6. Install hooks (pre-commit, commit-msg, post-commit)
    7. Install pre-push hook (only when agent specified)
    |
    v
Phase 2: Agent setup (only if agent specified)
    1. get_agent_config(Claude) -> Box<dyn AgentConfig>
    2. config.validate() -- checks `which claude` succeeds
    3. config.generate_config(&project_root) -> AgentConfigBundle
    4. Write bundle files (global instructions, settings)
    5. Generate project instructions (CLAUDE.md if not exists)
```

When no agent is specified, Phase 2 is skipped. The pre-push hook is not installed. The `.noslop.toml` omits `[agent]` and `[review]`.

### Generated `.noslop.toml` (with agent)

```toml
[project]
prefix = "NOS"

[agent]
type = "claude"

[review]
analyzers = ["conventions", "agent:security", "agent:quality"]

[review."agent:security"]
agent = "claude"

[review."agent:quality"]
agent = "claude"
```

### Pre-Push Hook

Installed only when an agent is configured:

```bash
#!/usr/bin/env bash
noslop review --check
exit $?
```

### Error Handling

| Scenario                | Behavior                                                               |
| ----------------------- | ---------------------------------------------------------------------- |
| `noslop init cursor`    | `Error: Unknown agent type: cursor. Supported: claude, codex`          |
| Agent CLI not installed | Core setup completes. Agent config fails. User re-runs with `--force`. |
| Already initialized     | `Already initialized. Use --force to reinitialize.`                    |

### Backward Compatibility

Existing `.noslop.toml` without `[agent]` parses fine (`agent: None`). Old noslop binaries ignore unknown TOML sections (no `deny_unknown_fields`).

---

## 6. CLI Command Reference

### `noslop init [agent]`

```
noslop init                # Core setup only
noslop init claude         # Core + Claude as review agent
noslop init codex          # Core + Codex as review agent
noslop init --force        # Reinitialize
```

Creates `.noslop.toml`, `.noslop/` directory, installs git hooks. With agent: also writes `[agent]` + `[review]` sections, installs pre-push hook, generates agent config via `AgentConfig::generate_config()`.

### `noslop check`

```
noslop check               # Validate checks against staged files
noslop check --json        # JSON output
```

Existing, unchanged. Runs `[[check]]` rules against staged files. Used by pre-commit hook.

### `noslop review`

```
noslop review                          # Review current branch vs default base
noslop review --base main              # Explicit base ref
noslop review --head feature/x         # Explicit head ref
noslop review --analyzer conventions   # Specific analyzers only
noslop review --agent codex            # Override agent for this run
noslop review --json                   # JSON output
noslop review --check                  # Exit 1 if unresolved blockers
```

Runs the review pipeline: `diff_between(base, head)`, then each analyzer in order with fold semantics. Results saved to `.noslop/reviews/{id}.json`. The `--check` flag is used by the pre-push hook -- exits non-zero if any `Finding` has `severity = Block` and `status = Open`.

### `noslop findings`

```
noslop findings                                    # List from latest review
noslop findings list                               # Same
noslop findings resolve F-001                      # Mark resolved
noslop findings dismiss F-001 --reason "false pos" # Dismiss with reason
noslop findings --json                             # JSON output
```

Operates on the latest review in `.noslop/reviews/`. Updates `FindingStatus` on the `Finding`.

### `noslop checkpoint`

```
noslop checkpoint                          # Auto-timestamp message
noslop checkpoint "before fixing findings" # Custom message
noslop checkpoint --json                   # JSON output
```

Safety commit via `commit_all()`. Bypasses hooks (git2 does not invoke them).

### `noslop ack`

```
noslop ack NOS-1 -m "Reviewed"     # Acknowledge a check
```

Existing, unchanged. Stages acknowledgment in `.noslop/staged-acks.json`.

---

## 7. File Layout

```
src/
  cli/
    app.rs                      # Modified: Init gains agent, add Checkpoint variant
    commands/
      mod.rs                    # Modified: add checkpoint re-export
      init.rs                   # Modified: agent parsing, Phase 2, pre-push hook
      checkpoint.rs             # NEW: checkpoint command handler
      review.rs                 # NEW (later phase): review pipeline CLI
      findings.rs               # NEW (later phase): findings management CLI
  core/
    ports/
      vcs.rs                    # Modified: 7 new methods on VersionControl
    models/                     # Unchanged (domain types in doc 00)
  adapters/
    git/
      mod.rs                    # Modified: add checkpoint, diff_types modules
      checkpoint.rs             # NEW: is_clean, commit_all
      diff_types.rs             # NEW: FileDiff, CommitInfo, DiffStat, etc.
      hooks.rs                  # Modified: add install_pre_push
    toml/
      parser.rs                 # Modified: +AgentSection, +ReviewSection,
                                #   +CoChangeConfig, +FormattingConfig,
                                #   +AgentAnalyzerConfig, +ScriptAnalyzerConfig
      repository.rs             # Unchanged
```

---

## 8. Test Strategy

### Unit Tests: Checkpoint Adapter

Tests use `tempfile::TempDir` + `git2::Repository::init` with an initial commit.

| Test                               | Assertion                                |
| ---------------------------------- | ---------------------------------------- |
| `is_clean_on_fresh_repo`           | Returns `true`                           |
| `is_clean_with_untracked_file`     | Returns `false` after writing a file     |
| `commit_all_stages_and_commits`    | Returns `Some(sha)`, tree is clean after |
| `commit_all_returns_none_on_clean` | Returns `None` on already-clean repo     |

### Unit Tests: TOML Parser

| Test                                   | What it covers                                      |
| -------------------------------------- | --------------------------------------------------- |
| `parse_minimal_config`                 | `[project]` only, `agent`/`review` are `None`       |
| `parse_with_agent_section`             | `[agent]` deserializes `agent_type`                 |
| `parse_full_config`                    | All sections including co-change, agent:_, custom:_ |
| `existing_config_without_new_sections` | Backward compat: old TOML parses without error      |
| `co_change_config_defaults`            | Default values match spec                           |
| `review_section_default_analyzers`     | Default is `["conventions"]`                        |

### Integration Tests: CLI

| Test                          | What it covers                             |
| ----------------------------- | ------------------------------------------ |
| `cli_checkpoint_with_changes` | `noslop checkpoint` succeeds, outputs sha  |
| `cli_checkpoint_clean_tree`   | Outputs "Nothing to checkpoint"            |
| `init_bare_without_agent`     | No `[agent]`, no pre-push hook             |
| `init_claude_creates_agent`   | `[agent]` in TOML, pre-push hook installed |
| `init_unknown_agent_fails`    | Exit 1 with "Unknown agent type"           |

---

## 9. Dependencies

No new crate dependencies. All already in `Cargo.toml`:

| Crate        | Version | Used For                                   |
| ------------ | ------- | ------------------------------------------ |
| `git2`       | 0.20.2  | All git operations (diff, log, checkpoint) |
| `chrono`     | 0.4.42  | Default checkpoint timestamp               |
| `clap`       | 4.5.51  | CLI parsing for new subcommands            |
| `serde`      | 1.x     | TOML deserialization for config structs    |
| `toml`       | 0.8.x   | TOML parsing                               |
| `serde_json` | 1.x     | JSON output mode                           |
| `anyhow`     | 1.0.100 | Error handling                             |
