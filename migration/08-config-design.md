# Configuration & File Format Design

Design specification for the complete `.noslop.toml` schema, `.noslop/` directory file formats, and the TOML parser changes needed to support agent configuration and the iteration loop.

## Overview

This document consolidates all configuration surfaces introduced across the migration design docs into a single reference. It defines:

1. The full `.noslop.toml` schema -- existing `[project]` and `[[check]]` sections plus new `[agent]`, `[run]`, `[run.stuck_policy]`, and `[run.test_gate]` sections
2. All files in the `.noslop/` directory -- existing and new -- with their exact formats
3. The parser changes needed in `src/adapters/toml/parser.rs` to deserialize the expanded schema
4. How CLI flags override TOML values (precedence rules)

**Invariant**: Existing `.noslop.toml` files without the new sections parse without error. All new fields use `#[serde(default)]` so the parser remains backward-compatible.

## `.noslop.toml` Full Schema

### Annotated Reference

```toml
# ============================================================
# .noslop.toml -- project configuration for noslop
# ============================================================

# --- Project metadata (existing, unchanged) ---

[project]
prefix = "NOS"          # 3-letter prefix for auto-generated check IDs

# --- Agent configuration (new, optional) ---
# Written by `noslop init <agent>`. Read by `noslop run` to determine
# which AgentRuntime to use. Omitted when no agent is configured.

[agent]
type = "claude"          # Required. "claude" | "codex"

# --- Run loop configuration (new, optional) ---
# Read by `noslop run`. All fields have defaults; the entire [run]
# section can be omitted.

[run]
max_iterations = 20      # Maximum iterations before the loop exits
confidence_threshold = 4 # 1-5. Agent must provide evidence if below this
post_loop_review = true  # Run a review agent pass after completion
initializer_phase = true # First iteration gets a setup-focused prompt

# --- Stuck detection thresholds (new, optional) ---
# Graduated 4-level escalation. Each value is the number of consecutive
# iterations without task-count progress before that level activates.

[run.stuck_policy]
level_1 = 2              # Suggest a different approach
level_2 = 4              # Mandate root cause analysis
level_3 = 6              # Force skip/decompose/simplify
level_4 = 8              # Generate diagnostic report + terminate

# --- External test gate (new, optional) ---
# When present, noslop runs this command after each iteration and feeds
# failures into the next iteration's prompt.

[run.test_gate]
command = "cargo test"   # Shell command to execute
timeout_secs = 300       # Kill after this many seconds
max_output_lines = 50    # Truncate captured output beyond this

# --- Check definitions (existing, unchanged) ---

[[check]]
id = "NOS-1"                              # Optional (auto-generated if omitted)
target = "src/adapters/git/hooks.rs"      # Path, glob, or fragment reference
message = "Verify hook installation"       # Human-readable check message
severity = "block"                         # "info" | "warn" | "block"
tags = ["hooks"]                           # Optional tags for filtering

[[check]]
target = "*.rs"
message = "Consider impact on public API"
severity = "warn"
```

### Section Reference

| Section              | Status   | Required | Written By            | Read By                         |
| -------------------- | -------- | -------- | --------------------- | ------------------------------- |
| `[project]`          | Existing | Yes      | `noslop init`         | `noslop check`, `noslop run`    |
| `[agent]`            | New      | No       | `noslop init <agent>` | `noslop run`                    |
| `[run]`              | New      | No       | User (manual edit)    | `noslop run`                    |
| `[run.stuck_policy]` | New      | No       | User (manual edit)    | `noslop run`                    |
| `[run.test_gate]`    | New      | No       | User (manual edit)    | `noslop run`                    |
| `[[check]]`          | Existing | No       | `noslop check add`    | `noslop check`, pre-commit hook |

### Field Definitions

#### `[project]`

| Field    | Type   | Default | Description                                  |
| -------- | ------ | ------- | -------------------------------------------- |
| `prefix` | string | `"CHK"` | 3-letter prefix for auto-generated check IDs |

#### `[agent]`

| Field  | Type   | Default | Description                               |
| ------ | ------ | ------- | ----------------------------------------- |
| `type` | string | --      | Agent identifier: `"claude"` or `"codex"` |

#### `[run]`

| Field                  | Type | Default | Description                                             |
| ---------------------- | ---- | ------- | ------------------------------------------------------- |
| `max_iterations`       | u32  | `20`    | Maximum iterations before the loop exits                |
| `confidence_threshold` | u8   | `4`     | Agent must provide evidence below this confidence level |
| `post_loop_review`     | bool | `true`  | Run a review pass after all tasks complete              |
| `initializer_phase`    | bool | `true`  | First iteration uses setup-focused prompt               |

#### `[run.stuck_policy]`

| Field     | Type | Default | Description                                |
| --------- | ---- | ------- | ------------------------------------------ |
| `level_1` | u32  | `2`     | Iterations without progress before Level 1 |
| `level_2` | u32  | `4`     | Iterations without progress before Level 2 |
| `level_3` | u32  | `6`     | Iterations without progress before Level 3 |
| `level_4` | u32  | `8`     | Iterations without progress before Level 4 |

#### `[run.test_gate]`

| Field              | Type   | Default | Description                   |
| ------------------ | ------ | ------- | ----------------------------- |
| `command`          | string | --      | Shell command to run tests    |
| `timeout_secs`     | u64    | `300`   | Kill after this many seconds  |
| `max_output_lines` | usize  | `50`    | Truncate captured test output |

#### `[[check]]`

| Field      | Type         | Default   | Description                               |
| ---------- | ------------ | --------- | ----------------------------------------- |
| `id`       | string (opt) | auto      | Custom ID; auto-generated as `PREFIX-N`   |
| `target`   | string       | --        | Path, glob pattern, or fragment reference |
| `message`  | string       | --        | Human-readable check message              |
| `severity` | string       | `"block"` | `"info"`, `"warn"`, or `"block"`          |
| `tags`     | string[]     | `[]`      | Optional tags for filtering               |

### Minimal Configuration Examples

Bare `noslop init` (no agent):

```toml
[project]
prefix = "NOS"
```

With agent (`noslop init claude`):

```toml
[project]
prefix = "NOS"

[agent]
type = "claude"
```

Full configuration with all options:

```toml
[project]
prefix = "NOS"

[agent]
type = "claude"

[run]
max_iterations = 30
confidence_threshold = 3
post_loop_review = true
initializer_phase = true

[run.stuck_policy]
level_1 = 3
level_2 = 5
level_3 = 7
level_4 = 10

[run.test_gate]
command = "cargo test --workspace"
timeout_secs = 600
max_output_lines = 100

[[check]]
target = "src/core/**/*.rs"
message = "Verify core logic has no I/O dependencies"
severity = "block"
tags = ["architecture"]
```

## CLI Flag Precedence

CLI flags override TOML values. The resolution order (highest priority first):

```
1. CLI flag         (e.g., --max-iterations 50)
2. .noslop.toml     (e.g., [run] max_iterations = 30)
3. Compiled default (e.g., 20)
```

The precedence is implemented in `src/cli/commands/run.rs` where the `RunConfig` is constructed:

```rust
let max_iterations = args.max_iterations
    .unwrap_or_else(|| toml_config.run.max_iterations);
```

CLI flags that override TOML values use `Option<T>` in clap so that "not provided" is distinguishable from "provided with a value":

| CLI Flag                   | TOML Path                    | Default |
| -------------------------- | ---------------------------- | ------- |
| `--max-iterations N`       | `[run] max_iterations`       | `20`    |
| `--agent claude\|codex`    | `[agent] type`               | --      |
| `--no-review`              | `[run] post_loop_review`     | `true`  |
| `--no-tests`               | (disables `[run.test_gate]`) | --      |
| `--stuck-limit N`          | `[run.stuck_policy] level_4` | `8`     |
| `--confidence-threshold N` | `[run] confidence_threshold` | `4`     |

## `.noslop/` Directory Layout

### Full Layout

```
.noslop/
├── .gitkeep            # Ensures directory exists in git (existing)
├── staged-acks.json    # Pending acknowledgments for commit (existing)
├── reviews/            # Review session JSON files (existing)
│   └── REV-xxxx.json
└── review.md           # Post-loop review findings (new)
```

> **Progress tracking**: Iteration progress is tracked through git commit history via structured commit messages, not through separate state files. Each `noslop run` iteration produces an atomic commit with metadata tags like `[TASK: N]`, `[CONFIDENCE: N]`, `[STATUS: complete/failed]`. The loop reads `git log` to reconstruct iteration history, stuck counts, and failure context. This eliminates `progress.json`, `failures.jsonl`, and `stuck-report.md` -- git history is the single source of truth.

### File Ownership

| File               | Created By        | Updated By      | Read By              | In Git |
| ------------------ | ----------------- | --------------- | -------------------- | ------ |
| `.gitkeep`         | `noslop init`     | --              | --                   | Yes    |
| `staged-acks.json` | `noslop ack`      | `noslop ack`    | pre-commit hook      | No     |
| `reviews/*.json`   | `noslop review`   | `noslop review` | `noslop review show` | Yes    |
| `review.md`        | Agent (post-loop) | --              | User                 | Yes    |

### `.gitignore` Entries

The `.noslop/` directory is committed to git so that review results and configuration are available to other developers. The only file excluded is the transient acknowledgment staging file:

```gitignore
# In project .gitignore
.noslop/staged-acks.json
```

## File Formats

### `review.md`

Post-loop review findings written by the agent after all tasks complete. The review pass is triggered by `noslop run` when `post_loop_review = true`.

**Location**: `.noslop/review.md`

**Expected Structure** (enforced by prompt, not by schema):

```markdown
# Post-Loop Review

## Critical Issues

- <issue with severity justification>

## Warnings

- <issue>

## Informational

- <observation>

## Summary

<overall assessment>
```

**Lifecycle**: Written once by the review agent pass. Read by the user. May be committed if the user chooses to keep it.

### `staged-acks.json` (Existing)

Unchanged. Pending acknowledgments staged for the next commit.

```json
[
  {
    "check_id": "NOS-1",
    "message": "Reviewed hook permissions",
    "acknowledged_by": "dev"
  }
]
```

### `reviews/REV-xxxx.json` (Existing)

Unchanged. Code review session files.

## TOML Parser Changes

### Current Parser (`src/adapters/toml/parser.rs`)

The existing parser defines two structs: `NoslopFile` and `ProjectConfig`. It does not know about `[agent]`, `[run]`, or any sub-tables of `[run]`.

### Updated Structs

The following changes are needed in `src/adapters/toml/parser.rs`. All new fields are `#[serde(default)]` so existing `.noslop.toml` files without the new sections continue to parse without error.

```rust
//! TOML parser for .noslop.toml files
//!
//! Handles reading and deserializing noslop configuration files.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// A .noslop.toml file structure
#[derive(Debug, Deserialize)]
pub struct NoslopFile {
    /// Project configuration
    #[serde(default)]
    pub project: ProjectConfig,

    /// Agent configuration (set by `noslop init <agent>`)
    #[serde(default)]
    pub agent: Option<AgentSection>,

    /// Run loop configuration
    #[serde(default)]
    pub run: Option<RunSection>,

    /// Checks in this file
    #[serde(default, rename = "check")]
    pub checks: Vec<CheckEntry>,
}

/// Project-level configuration
#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct ProjectConfig {
    /// 3-letter prefix for check IDs (e.g., "NOS" for NOS-1, NOS-2)
    pub prefix: String,

    /// Next check ID number (computed at runtime, not in TOML)
    #[serde(skip)]
    pub next_id: u32,
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            prefix: "CHK".to_string(),
            next_id: 1,
        }
    }
}

/// Agent configuration section
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AgentSection {
    /// Agent type identifier (e.g., "claude", "codex")
    #[serde(rename = "type")]
    pub agent_type: String,
}

/// Run loop configuration section
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct RunSection {
    /// Maximum number of iterations
    pub max_iterations: u32,

    /// Confidence threshold for task completion (1-5)
    pub confidence_threshold: u8,

    /// Whether to run a post-loop review
    pub post_loop_review: bool,

    /// Whether to run an initializer phase on iteration 1
    pub initializer_phase: bool,

    /// Stuck detection thresholds
    pub stuck_policy: Option<StuckPolicySection>,

    /// External test gate
    pub test_gate: Option<TestGateSection>,
}

impl Default for RunSection {
    fn default() -> Self {
        Self {
            max_iterations: 20,
            confidence_threshold: 4,
            post_loop_review: true,
            initializer_phase: true,
            stuck_policy: None,
            test_gate: None,
        }
    }
}

/// Stuck detection thresholds
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct StuckPolicySection {
    /// Iterations without progress before Level 1 (suggest)
    pub level_1: u32,
    /// Iterations without progress before Level 2 (root cause analysis)
    pub level_2: u32,
    /// Iterations without progress before Level 3 (force strategy change)
    pub level_3: u32,
    /// Iterations without progress before Level 4 (diagnostic + exit)
    pub level_4: u32,
}

impl Default for StuckPolicySection {
    fn default() -> Self {
        Self {
            level_1: 2,
            level_2: 4,
            level_3: 6,
            level_4: 8,
        }
    }
}

/// External test gate configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct TestGateSection {
    /// Shell command to run tests (e.g., "cargo test")
    pub command: String,
    /// Timeout in seconds
    pub timeout_secs: u64,
    /// Maximum lines of test output to capture
    pub max_output_lines: usize,
}

impl Default for TestGateSection {
    fn default() -> Self {
        Self {
            command: String::new(),
            timeout_secs: 300,
            max_output_lines: 50,
        }
    }
}

/// A check entry in .noslop.toml (unchanged)
#[derive(Debug, Deserialize)]
pub struct CheckEntry {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub target: String,
    pub message: String,
    #[serde(default = "default_severity")]
    pub severity: String,
    #[serde(default)]
    pub tags: Vec<String>,
}

fn default_severity() -> String {
    "block".to_string()
}
```

### What Changed (Diff Summary)

```diff
 /// A .noslop.toml file structure
 #[derive(Debug, Deserialize)]
 pub struct NoslopFile {
     /// Project configuration
     #[serde(default)]
     pub project: ProjectConfig,

+    /// Agent configuration (set by `noslop init <agent>`)
+    #[serde(default)]
+    pub agent: Option<AgentSection>,
+
+    /// Run loop configuration
+    #[serde(default)]
+    pub run: Option<RunSection>,
+
     /// Checks in this file
     #[serde(default, rename = "check")]
     pub checks: Vec<CheckEntry>,
 }
+
+/// Agent configuration section
+#[derive(Debug, Clone, Deserialize, Serialize)]
+pub struct AgentSection {
+    #[serde(rename = "type")]
+    pub agent_type: String,
+}
+
+/// Run loop configuration section
+#[derive(Debug, Clone, Deserialize, Serialize)]
+#[serde(default)]
+pub struct RunSection { ... }
+
+/// Stuck detection thresholds
+#[derive(Debug, Clone, Deserialize, Serialize)]
+#[serde(default)]
+pub struct StuckPolicySection { ... }
+
+/// External test gate configuration
+#[derive(Debug, Clone, Deserialize, Serialize)]
+#[serde(default)]
+pub struct TestGateSection { ... }
```

### Backward Compatibility

| Scenario                                                    | Behavior                                                                    |
| ----------------------------------------------------------- | --------------------------------------------------------------------------- |
| Existing `.noslop.toml` without `[agent]` or `[run]`        | Parses fine. `agent` is `None`, `run` is `None`.                            |
| New `.noslop.toml` with `[agent]` read by old noslop binary | TOML serde ignores unknown keys (no `deny_unknown_fields`). No error.       |
| `[run]` section with only some fields specified             | Missing fields use `Default::default()`. Partial config works.              |
| `[run.stuck_policy]` present but `[run]` omitted            | Not valid TOML. `stuck_policy` is a sub-table of `[run]`. Requires `[run]`. |
| `[run.test_gate]` with empty command string                 | Parses, but `noslop run` treats empty command as "no test gate".            |

### No Changes to `load_file` or `find_noslop_files`

The existing `load_file` and `find_noslop_files` functions remain unchanged. They already use `toml::from_str` which handles the expanded struct automatically:

```rust
pub fn load_file(path: &Path) -> anyhow::Result<NoslopFile> {
    let content = fs::read_to_string(path)?;
    let file: NoslopFile = toml::from_str(&content)?;
    Ok(file)
}
```

The `find_noslop_files` function walks up the directory tree and loads each `.noslop.toml`. It is unaffected because it returns `Vec<PathBuf>` -- the caller loads each file individually.

### No Changes to `TomlCheckRepository`

The `TomlCheckRepository` adapter in `src/adapters/toml/repository.rs` only accesses `noslop_file.checks` and `noslop_file.project`. It does not read `agent` or `run`. No changes needed.

## Conversion: TOML Sections to Domain Models

The TOML parser produces intermediate structs (`AgentSection`, `RunSection`, etc.) that are then converted to the core domain models defined in `src/core/models/`. This conversion happens in the CLI layer (`src/cli/commands/run.rs`) where concrete adapters are wired together.

### `AgentSection` to `AgentType`

```rust
use crate::core::ports::AgentType;

impl AgentSection {
    /// Convert to the core AgentType enum
    pub fn to_agent_type(&self) -> anyhow::Result<AgentType> {
        self.agent_type
            .parse::<AgentType>()
            .map_err(|e| anyhow::anyhow!("{e}"))
    }
}
```

### `RunSection` to `RunConfig`

```rust
use crate::core::models::{RunConfig, StuckPolicy, TestGate};

impl RunSection {
    /// Convert to the core RunConfig, merging with CLI overrides
    pub fn to_run_config(
        &self,
        plan_file: PathBuf,
        working_dir: PathBuf,
        cli_overrides: &CliOverrides,
    ) -> RunConfig {
        let stuck_policy = self.stuck_policy
            .as_ref()
            .map(StuckPolicySection::to_stuck_policy)
            .unwrap_or_default();

        let test_gate = self.test_gate
            .as_ref()
            .and_then(TestGateSection::to_test_gate);

        RunConfig {
            plan_file,
            working_dir,
            max_iterations: cli_overrides.max_iterations
                .unwrap_or(self.max_iterations),
            stuck_policy: StuckPolicy {
                level_4_threshold: cli_overrides.stuck_limit
                    .unwrap_or(stuck_policy.level_4_threshold),
                ..stuck_policy
            },
            test_gate: if cli_overrides.no_tests { None } else { test_gate },
            post_loop_review: if cli_overrides.no_review {
                false
            } else {
                self.post_loop_review
            },
            confidence_threshold: cli_overrides.confidence_threshold
                .or(Some(self.confidence_threshold)),
            initializer_phase: self.initializer_phase,
            ..Default::default()
        }
    }
}

impl StuckPolicySection {
    fn to_stuck_policy(&self) -> StuckPolicy {
        StuckPolicy {
            level_1_threshold: self.level_1,
            level_2_threshold: self.level_2,
            level_3_threshold: self.level_3,
            level_4_threshold: self.level_4,
        }
    }
}

impl TestGateSection {
    /// Convert to TestGate, returning None if command is empty
    fn to_test_gate(&self) -> Option<TestGate> {
        if self.command.is_empty() {
            return None;
        }
        Some(TestGate {
            command: self.command.clone(),
            working_dir: None,
            max_output_lines: self.max_output_lines,
            timeout_secs: self.timeout_secs,
        })
    }
}
```

### CLI Overrides Struct

The CLI override values that can shadow TOML settings:

```rust
/// Values from CLI flags that override TOML configuration
pub struct CliOverrides {
    pub max_iterations: Option<u32>,
    pub stuck_limit: Option<u32>,
    pub confidence_threshold: Option<u8>,
    pub no_review: bool,
    pub no_tests: bool,
}
```

## `.noslop.toml` Writer Changes

When `noslop init <agent>` runs, it writes the `.noslop.toml` file with the `[agent]` section. The existing `build_noslop_toml` function (from 07-init-expansion-design.md) handles this:

```rust
fn build_noslop_toml(prefix: &str, agent_type: Option<AgentType>) -> String {
    let mut toml = format!(
        r#"# noslop checks

[project]
prefix = "{prefix}"
"#
    );

    if let Some(agent) = agent_type {
        toml.push_str(&format!(
            r#"
[agent]
type = "{agent}"
"#
        ));
    }

    toml.push_str(&format!(
        r#"
# Checks are auto-assigned IDs like {prefix}-1, {prefix}-2, etc.
# You can also specify custom IDs:
#   id = "my-custom-id"

# Example check (uncomment to use):
# [[check]]
# target = "*.rs"
# message = "Consider impact on public API"
# severity = "warn"
"#
    ));

    toml
}
```

The `[run]` section is NOT written by `noslop init`. It is user-configured. Users add it manually when they want to customize the defaults. This keeps the generated config minimal and avoids polluting the file with defaults that match the compiled-in values.

## Validation Rules

The TOML parser deserializes permissively. Validation of semantic constraints happens in the CLI command handlers, not in the parser. This follows noslop's existing pattern where `parser.rs` handles deserialization and `commands/*.rs` handles validation.

### Agent Type Validation

```rust
// In cli/commands/run.rs
if let Some(agent) = &noslop_file.agent {
    agent.to_agent_type()?; // Fails with "Unknown agent type: X"
}
```

### Stuck Policy Ordering

The stuck policy thresholds must be monotonically increasing. Validated at run time:

```rust
fn validate_stuck_policy(policy: &StuckPolicy) -> anyhow::Result<()> {
    if !(policy.level_1_threshold
        <= policy.level_2_threshold
        && policy.level_2_threshold <= policy.level_3_threshold
        && policy.level_3_threshold <= policy.level_4_threshold)
    {
        anyhow::bail!(
            "Stuck policy levels must be monotonically increasing: \
             level_1={} <= level_2={} <= level_3={} <= level_4={}",
            policy.level_1_threshold,
            policy.level_2_threshold,
            policy.level_3_threshold,
            policy.level_4_threshold,
        );
    }
    Ok(())
}
```

### Confidence Threshold Range

```rust
if let Some(threshold) = config.confidence_threshold {
    if !(1..=5).contains(&threshold) {
        anyhow::bail!("confidence_threshold must be between 1 and 5, got {threshold}");
    }
}
```

### Test Gate Command Non-Empty

An empty `command` field in `[run.test_gate]` means "no test gate". The conversion function returns `None` in this case (see `TestGateSection::to_test_gate` above).

## File Layout After Changes

```
src/
├── adapters/
│   └── toml/
│       ├── parser.rs       # Modified: NoslopFile gains agent + run fields;
│       │                   #   new structs AgentSection, RunSection,
│       │                   #   StuckPolicySection, TestGateSection
│       ├── repository.rs   # Unchanged
│       └── writer.rs       # Unchanged (build_noslop_toml in init.rs)
├── cli/
│   └── commands/
│       └── run.rs          # Reads NoslopFile.agent and NoslopFile.run,
│                           #   converts to domain models, applies CLI overrides
└── core/
    └── models/
        └── run_config.rs   # RunConfig, StuckPolicy, TestGate (defined in 06)
```

## Test Strategy

### Parser Round-Trip Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_minimal_config() {
        let toml_str = r#"
            [project]
            prefix = "NOS"
        "#;
        let file: NoslopFile = toml::from_str(toml_str).unwrap();
        assert_eq!(file.project.prefix, "NOS");
        assert!(file.agent.is_none());
        assert!(file.run.is_none());
        assert!(file.checks.is_empty());
    }

    #[test]
    fn parse_with_agent_section() {
        let toml_str = r#"
            [project]
            prefix = "NOS"

            [agent]
            type = "claude"
        "#;
        let file: NoslopFile = toml::from_str(toml_str).unwrap();
        assert_eq!(file.agent.unwrap().agent_type, "claude");
    }

    #[test]
    fn parse_with_full_run_config() {
        let toml_str = r#"
            [project]
            prefix = "NOS"

            [agent]
            type = "codex"

            [run]
            max_iterations = 30
            confidence_threshold = 3
            post_loop_review = false
            initializer_phase = true

            [run.stuck_policy]
            level_1 = 3
            level_2 = 5
            level_3 = 7
            level_4 = 10

            [run.test_gate]
            command = "npm test"
            timeout_secs = 120
            max_output_lines = 25
        "#;
        let file: NoslopFile = toml::from_str(toml_str).unwrap();

        let run = file.run.unwrap();
        assert_eq!(run.max_iterations, 30);
        assert_eq!(run.confidence_threshold, 3);
        assert!(!run.post_loop_review);

        let stuck = run.stuck_policy.unwrap();
        assert_eq!(stuck.level_1, 3);
        assert_eq!(stuck.level_4, 10);

        let gate = run.test_gate.unwrap();
        assert_eq!(gate.command, "npm test");
        assert_eq!(gate.timeout_secs, 120);
    }

    #[test]
    fn parse_partial_run_config_uses_defaults() {
        let toml_str = r#"
            [project]
            prefix = "NOS"

            [run]
            max_iterations = 50
        "#;
        let file: NoslopFile = toml::from_str(toml_str).unwrap();

        let run = file.run.unwrap();
        assert_eq!(run.max_iterations, 50);
        assert_eq!(run.confidence_threshold, 4); // default
        assert!(run.post_loop_review);            // default
        assert!(run.stuck_policy.is_none());      // not specified
        assert!(run.test_gate.is_none());         // not specified
    }

    #[test]
    fn parse_with_checks_and_agent() {
        let toml_str = r#"
            [project]
            prefix = "NOS"

            [agent]
            type = "claude"

            [[check]]
            target = "*.rs"
            message = "Check Rust files"
            severity = "warn"

            [[check]]
            id = "NOS-1"
            target = "src/main.rs"
            message = "Verify entry point"
        "#;
        let file: NoslopFile = toml::from_str(toml_str).unwrap();
        assert!(file.agent.is_some());
        assert_eq!(file.checks.len(), 2);
    }

    #[test]
    fn existing_config_without_new_sections_parses() {
        // This is the actual content from noslop's own .noslop.toml
        let toml_str = r#"
            [project]
            prefix = "NOS"

            [[check]]
            id = "NOS-1"
            target = "src/adapters/git/hooks.rs"
            message = "Verify hook installation preserves permissions"
            severity = "block"

            [[check]]
            id = "NOS-2"
            target = "src/core/services/checker.rs"
            message = "Verify acknowledgment matching logic correctness"
            severity = "block"
        "#;
        let file: NoslopFile = toml::from_str(toml_str).unwrap();
        assert_eq!(file.project.prefix, "NOS");
        assert!(file.agent.is_none());
        assert!(file.run.is_none());
        assert_eq!(file.checks.len(), 2);
    }

    #[test]
    fn agent_section_to_agent_type() {
        let section = AgentSection { agent_type: "claude".to_string() };
        assert_eq!(
            section.to_agent_type().unwrap().command_name(),
            "claude"
        );
    }

    #[test]
    fn agent_section_unknown_type_errors() {
        let section = AgentSection { agent_type: "cursor".to_string() };
        assert!(section.to_agent_type().is_err());
    }

    #[test]
    fn test_gate_empty_command_returns_none() {
        let section = TestGateSection::default();
        assert!(section.to_test_gate().is_none());
    }

    #[test]
    fn test_gate_with_command_returns_some() {
        let section = TestGateSection {
            command: "cargo test".to_string(),
            ..Default::default()
        };
        let gate = section.to_test_gate().unwrap();
        assert_eq!(gate.command, "cargo test");
        assert_eq!(gate.timeout_secs, 300);
    }
}
```

### Validation Tests

```rust
#[test]
fn stuck_policy_valid_ordering() {
    let policy = StuckPolicy {
        level_1_threshold: 2,
        level_2_threshold: 4,
        level_3_threshold: 6,
        level_4_threshold: 8,
    };
    assert!(validate_stuck_policy(&policy).is_ok());
}

#[test]
fn stuck_policy_invalid_ordering() {
    let policy = StuckPolicy {
        level_1_threshold: 5,
        level_2_threshold: 3, // lower than level_1
        level_3_threshold: 6,
        level_4_threshold: 8,
    };
    assert!(validate_stuck_policy(&policy).is_err());
}

#[test]
fn confidence_threshold_in_range() {
    for i in 1..=5 {
        // Should not panic or error
        assert!((1..=5).contains(&i));
    }
}

#[test]
fn confidence_threshold_out_of_range() {
    assert!(!(1..=5).contains(&0));
    assert!(!(1..=5).contains(&6));
}
```

## Summary

This design consolidates all configuration surfaces into a single reference:

1. **`.noslop.toml` schema**: Three new optional sections (`[agent]`, `[run]`, `[run.stuck_policy]`, `[run.test_gate]`) alongside the existing `[project]` and `[[check]]` sections. All new fields default gracefully.

2. **`.noslop/` directory**: One new file (`review.md`) with a precise format definition. Iteration progress and failure context are tracked through git commit history via structured commit messages -- no separate state files. All files are committed to git except `staged-acks.json`.

3. **Parser changes**: Four new structs (`AgentSection`, `RunSection`, `StuckPolicySection`, `TestGateSection`) added to `src/adapters/toml/parser.rs`. Two new fields on `NoslopFile`. Zero changes to existing functions or the `TomlCheckRepository` adapter.

4. **Precedence rules**: CLI flags override TOML values override compiled defaults. Implemented via `Option<T>` in clap args and explicit merging in the command handler.

5. **Backward compatibility**: Existing `.noslop.toml` files parse without error. Old noslop binaries ignore unknown TOML sections. No migration step required.
