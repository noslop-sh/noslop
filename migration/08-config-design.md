# Configuration & File Format Design

Design specification for the complete `.noslop.toml` schema, `.noslop/` directory file formats, and the TOML parser changes needed to support the review analyzer pipeline.

## Overview

This document consolidates all configuration surfaces for noslop's code review architecture into a single reference. It defines:

1. The full `.noslop.toml` schema -- existing `[project]` and `[[check]]` sections plus new `[agent]`, `[review]`, and analyzer-specific sub-tables
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
# Specifies the default agent used by AgentAnalyzer for inference-based
# review passes. Written by `noslop init <agent>`.

[agent]
type = "claude"          # Required. "claude" | "codex"

# --- Review pipeline configuration (new, optional) ---
# Controls which analyzers run and in what order during `noslop review`.
# Each analyzer sees findings from all prior analyzers (fold semantics).
# Order matters: Static -> Computed -> Tool -> Inference.

[review]
analyzers = ["conventions", "formatting", "co-change", "agent:security", "agent:quality"]

# --- Co-change analyzer configuration ---
# Detects files that historically change together but are missing from
# the current branch diff.

[review.co-change]
min_correlation = 0.7    # Minimum co-change frequency (0.0-1.0)
lookback = 500           # Number of commits to scan for correlation
min_commits = 10         # Minimum co-occurrences before flagging

# --- Agent-based analyzer: security focus ---
# Uses AgentRuntime to perform LLM-based security review of the diff.

[review."agent:security"]
agent = "claude"                                  # Override default agent
prompt_template = ".noslop/prompts/security.md"   # Optional custom prompt

# --- Agent-based analyzer: quality focus ---
# Uses AgentRuntime to perform LLM-based quality review of the diff.

[review."agent:quality"]
agent = "claude"

# --- Script analyzer: user-extensible ---
# External command that reads findings JSON on stdin and writes
# new findings JSON on stdout.

[review."custom:migrations"]
type = "script"
command = "python ./scripts/check-migrations.py"

# --- Formatting tools ---
# Delegates to external formatters and flags files that would change.

[review.formatting]
tools = ["rustfmt", "prettier"]

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

| Section               | Status   | Required | Written By            | Read By                         |
| --------------------- | -------- | -------- | --------------------- | ------------------------------- |
| `[project]`           | Existing | Yes      | `noslop init`         | `noslop check`, `noslop review` |
| `[agent]`             | New      | No       | `noslop init <agent>` | `noslop review` (AgentAnalyzer) |
| `[review]`            | New      | No       | User (manual edit)    | `noslop review`                 |
| `[review.co-change]`  | New      | No       | User (manual edit)    | CoChangeAnalyzer                |
| `[review."agent:*"]`  | New      | No       | User (manual edit)    | AgentAnalyzer                   |
| `[review."custom:*"]` | New      | No       | User (manual edit)    | ScriptAnalyzer                  |
| `[review.formatting]` | New      | No       | User (manual edit)    | FormatAnalyzer                  |
| `[[check]]`           | Existing | No       | `noslop check add`    | `noslop check`, pre-commit hook |

### Field Definitions

#### `[project]`

| Field    | Type   | Default | Description                                  |
| -------- | ------ | ------- | -------------------------------------------- |
| `prefix` | string | `"CHK"` | 3-letter prefix for auto-generated check IDs |

#### `[agent]`

| Field  | Type   | Default | Description                               |
| ------ | ------ | ------- | ----------------------------------------- |
| `type` | string | --      | Agent identifier: `"claude"` or `"codex"` |

#### `[review]`

| Field       | Type     | Default           | Description                                           |
| ----------- | -------- | ----------------- | ----------------------------------------------------- |
| `analyzers` | string[] | `["conventions"]` | Ordered list of analyzer names to run in the pipeline |

Analyzer names follow a convention:

- **Plain names** map to built-in analyzers: `"conventions"`, `"formatting"`, `"co-change"`
- **`agent:` prefix** maps to AgentAnalyzer instances with a focus area: `"agent:security"`, `"agent:quality"`
- **`custom:` prefix** maps to ScriptAnalyzer instances: `"custom:migrations"`, `"custom:lint"`

#### `[review.co-change]`

| Field             | Type | Default | Description                                      |
| ----------------- | ---- | ------- | ------------------------------------------------ |
| `min_correlation` | f64  | `0.7`   | Minimum co-change frequency to flag (0.0-1.0)    |
| `lookback`        | u32  | `500`   | Number of commits to scan for co-change patterns |
| `min_commits`     | u32  | `10`    | Minimum co-occurrences before flagging           |

#### `[review."agent:*"]`

| Field             | Type   | Default             | Description                                            |
| ----------------- | ------ | ------------------- | ------------------------------------------------------ |
| `agent`           | string | from `[agent].type` | Agent to use for this pass (overrides default)         |
| `prompt_template` | string | built-in default    | Path to custom prompt template (relative to repo root) |

#### `[review."custom:*"]`

| Field     | Type   | Default | Description              |
| --------- | ------ | ------- | ------------------------ |
| `type`    | string | --      | Must be `"script"`       |
| `command` | string | --      | Shell command to execute |

#### `[review.formatting]`

| Field   | Type     | Default | Description                                                |
| ------- | -------- | ------- | ---------------------------------------------------------- |
| `tools` | string[] | `[]`    | Formatting tools to check: `"rustfmt"`, `"prettier"`, etc. |

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

Review pipeline with defaults only (convention checks):

```toml
[project]
prefix = "NOS"

[review]
analyzers = ["conventions"]
```

Full configuration with all options:

```toml
[project]
prefix = "NOS"

[agent]
type = "claude"

[review]
analyzers = ["conventions", "formatting", "co-change", "agent:security", "agent:quality", "custom:migrations"]

[review.co-change]
min_correlation = 0.8
lookback = 1000
min_commits = 5

[review."agent:security"]
agent = "claude"
prompt_template = ".noslop/prompts/security.md"

[review."agent:quality"]
agent = "claude"

[review."custom:migrations"]
type = "script"
command = "python ./scripts/check-migrations.py"

[review.formatting]
tools = ["rustfmt", "prettier"]

[[check]]
target = "src/core/**/*.rs"
message = "Verify core logic has no I/O dependencies"
severity = "block"
tags = ["architecture"]
```

## CLI Flag Precedence

CLI flags override TOML values. The resolution order (highest priority first):

```
1. CLI flag         (e.g., --analyzer conventions,formatting)
2. .noslop.toml     (e.g., [review] analyzers = [...])
3. Compiled default (e.g., ["conventions"])
```

The precedence is implemented in `src/cli/commands/review.rs` where the `ReviewConfig` is constructed:

```rust
let analyzers = args.analyzers
    .unwrap_or_else(|| toml_config.review.analyzers.clone());
```

CLI flags that override TOML values use `Option<T>` in clap so that "not provided" is distinguishable from "provided with a value":

| CLI Flag                    | TOML Path                            | Default           |
| --------------------------- | ------------------------------------ | ----------------- |
| `--analyzer name[,name...]` | `[review] analyzers`                 | `["conventions"]` |
| `--agent claude\|codex`     | `[agent] type`                       | --                |
| `--base main`               | (inferred from git)                  | default branch    |
| `--check`                   | (none)                               | `false`           |
| `--min-correlation 0.8`     | `[review.co-change] min_correlation` | `0.7`             |

## `.noslop/` Directory Layout

### Full Layout

```
.noslop/
├── .gitkeep            # Ensures directory exists in git (existing)
├── staged-acks.json    # Pending acknowledgments for commit (existing)
├── reviews/            # Review session JSON files (new)
│   └── {session-id}.json
└── prompts/            # Custom prompt templates for agent analyzers (new)
    ├── security.md
    └── quality.md
```

### File Ownership

| File               | Created By      | Updated By        | Read By                  | In Git |
| ------------------ | --------------- | ----------------- | ------------------------ | ------ |
| `.gitkeep`         | `noslop init`   | --                | --                       | Yes    |
| `staged-acks.json` | `noslop ack`    | `noslop ack`      | pre-commit hook          | No     |
| `reviews/*.json`   | `noslop review` | `noslop findings` | `noslop findings`, hooks | Yes    |
| `prompts/*.md`     | User            | User              | AgentAnalyzer            | Yes    |

### `.gitignore` Entries

The `.noslop/` directory is committed to git so that review results, prompt templates, and configuration are available to other developers. The only file excluded is the transient acknowledgment staging file:

```gitignore
# In project .gitignore
.noslop/staged-acks.json
```

## File Formats

### Review Session Files (`reviews/{session-id}.json`)

Each `noslop review` invocation creates a review session file. The session captures all findings from the pipeline, along with their resolution status. Sessions are git-tracked so the team can see review history.

**Location**: `.noslop/reviews/{session-id}.json`

**Schema**:

```json
{
  "id": "review-abc123",
  "target": {
    "base": "main",
    "head": "feature/x",
    "base_commit": "a1b2c3d",
    "head_commit": "e4f5g6h"
  },
  "started_at": "2026-02-16T10:00:00Z",
  "completed_at": "2026-02-16T10:02:30Z",
  "analyzers_run": ["conventions", "co-change", "agent:security"],
  "findings": [
    {
      "id": "F-001",
      "source": { "type": "analyzer", "name": "co-change" },
      "file": "src/core/services/pipeline.rs",
      "line": null,
      "severity": "warn",
      "message": "src/core/ports/analyzer.rs usually changes with this file (correlation: 0.85)",
      "suggestion": "Review whether analyzer.rs needs updates too",
      "commit": null,
      "category": "co-change"
    },
    {
      "id": "F-002",
      "source": { "type": "check", "id": "NOS-3" },
      "file": "src/adapters/agent/claude.rs",
      "line": 42,
      "severity": "block",
      "message": "Consider impact on public API",
      "suggestion": null,
      "commit": "e4f5g6h",
      "category": "convention"
    }
  ],
  "resolutions": {
    "F-001": {
      "status": "dismissed",
      "reason": "Not applicable to this change",
      "at": "2026-02-16T10:05:00Z"
    },
    "F-002": {
      "status": "resolved",
      "reason": null,
      "at": "2026-02-16T10:06:00Z"
    }
  }
}
```

**Lifecycle**:

1. Created by `noslop review` when the pipeline starts
2. Findings are appended as each analyzer completes
3. Updated by `noslop findings resolve` and `noslop findings dismiss`
4. Read by `noslop findings` for listing
5. Read by pre-push hook to gate on unresolved blocking findings

### Review Memory File (`.noslop/reviews/memory.json`)

Tracks finding patterns across review sessions. Findings that recur frequently are candidates for graduation to permanent `[[check]]` entries.

**Location**: `.noslop/reviews/memory.json`

**Schema**:

```json
{
  "patterns": [
    {
      "category": "security",
      "message_pattern": "SQL injection.*raw query",
      "occurrences": 5,
      "first_seen": "2026-01-15T10:00:00Z",
      "last_seen": "2026-02-16T10:00:00Z",
      "sessions": ["review-abc123", "review-def456", "review-ghi789"],
      "graduated": false
    }
  ],
  "graduated_checks": [
    {
      "pattern_index": 0,
      "check_id": "NOS-15",
      "graduated_at": "2026-02-20T14:00:00Z"
    }
  ]
}
```

### Custom Prompt Templates (`prompts/*.md`)

User-authored prompt templates for agent-based analyzers. Referenced by `prompt_template` in the TOML config.

**Expected Structure** (convention, not schema-enforced):

```markdown
# Security Review

You are reviewing a code diff for security issues.

## Focus Areas

- SQL injection
- Command injection
- Path traversal
- Hardcoded secrets
- Authentication/authorization bypass

## Output Format

For each finding, output a JSON object on its own line:

{"file": "path/to/file.rs", "line": 42, "severity": "block", "message": "...", "suggestion": "..."}
```

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

## TOML Parser Changes

### Current Parser (`src/adapters/toml/parser.rs`)

The existing parser defines two structs: `NoslopFile` and `ProjectConfig`. It does not know about `[agent]`, `[review]`, or any analyzer configuration.

### Updated Structs

The following changes are needed in `src/adapters/toml/parser.rs`. All new fields are `#[serde(default)]` so existing `.noslop.toml` files without the new sections continue to parse without error.

```rust
//! TOML parser for .noslop.toml files
//!
//! Handles reading and deserializing noslop configuration files.

use std::collections::HashMap;
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

    /// Review pipeline configuration
    #[serde(default)]
    pub review: Option<ReviewSection>,

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

/// Review pipeline configuration section
///
/// The `analyzers` list defines which analyzers run and in what order.
/// Additional keys are parsed as analyzer-specific configuration via
/// the `analyzer_configs` catch-all.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct ReviewSection {
    /// Ordered list of analyzer names to run
    pub analyzers: Vec<String>,

    /// Co-change analyzer configuration
    #[serde(rename = "co-change")]
    pub co_change: Option<CoChangeConfig>,

    /// Formatting analyzer configuration
    pub formatting: Option<FormattingConfig>,

    /// Catch-all for analyzer-specific sub-tables (agent:*, custom:*)
    /// Parsed as a flat HashMap; keys like "agent:security" map to their config.
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

/// Co-change analyzer configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct CoChangeConfig {
    /// Minimum co-change frequency to flag (0.0-1.0)
    pub min_correlation: f64,
    /// Number of commits to scan for co-change patterns
    pub lookback: u32,
    /// Minimum co-occurrences before flagging
    pub min_commits: u32,
}

impl Default for CoChangeConfig {
    fn default() -> Self {
        Self {
            min_correlation: 0.7,
            lookback: 500,
            min_commits: 10,
        }
    }
}

/// Formatting analyzer configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct FormattingConfig {
    /// List of formatting tools to check
    pub tools: Vec<String>,
}

impl Default for FormattingConfig {
    fn default() -> Self {
        Self {
            tools: Vec::new(),
        }
    }
}

/// Agent analyzer entry (parsed from catch-all `analyzer_configs`)
///
/// This struct is not directly deserialized from TOML. Instead, entries
/// matching `agent:*` keys in the `analyzer_configs` HashMap are
/// deserialized into this type at validation time.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AgentAnalyzerConfig {
    /// Agent to use (overrides [agent].type)
    pub agent: Option<String>,
    /// Path to custom prompt template
    pub prompt_template: Option<String>,
}

/// Script analyzer entry (parsed from catch-all `analyzer_configs`)
///
/// Entries matching `custom:*` keys with `type = "script"`.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ScriptAnalyzerConfig {
    /// Must be "script"
    #[serde(rename = "type")]
    pub analyzer_type: String,
    /// Shell command to execute
    pub command: String,
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
+    /// Review pipeline configuration
+    #[serde(default)]
+    pub review: Option<ReviewSection>,
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
+/// Review pipeline configuration section
+#[derive(Debug, Clone, Deserialize, Serialize)]
+#[serde(default)]
+pub struct ReviewSection { ... }
+
+/// Co-change analyzer configuration
+#[derive(Debug, Clone, Deserialize, Serialize)]
+#[serde(default)]
+pub struct CoChangeConfig { ... }
+
+/// Formatting analyzer configuration
+#[derive(Debug, Clone, Deserialize, Serialize)]
+#[serde(default)]
+pub struct FormattingConfig { ... }
+
+/// Agent analyzer entry
+#[derive(Debug, Clone, Deserialize, Serialize)]
+pub struct AgentAnalyzerConfig { ... }
+
+/// Script analyzer entry
+#[derive(Debug, Clone, Deserialize, Serialize)]
+pub struct ScriptAnalyzerConfig { ... }
```

### Backward Compatibility

| Scenario                                                    | Behavior                                                                       |
| ----------------------------------------------------------- | ------------------------------------------------------------------------------ |
| Existing `.noslop.toml` without `[agent]` or `[review]`     | Parses fine. `agent` is `None`, `review` is `None`.                            |
| New `.noslop.toml` with `[agent]` read by old noslop binary | TOML serde ignores unknown keys (no `deny_unknown_fields`). No error.          |
| `[review]` section with only `analyzers` specified          | Missing sub-tables use `None`. Partial config works.                           |
| `[review.co-change]` present but `[review]` omitted         | Not valid TOML. `co-change` is a sub-table of `[review]`. Requires `[review]`. |
| `[review."agent:security"]` with no `agent` field           | Parses. Falls back to `[agent].type` at runtime.                               |

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

The `TomlCheckRepository` adapter in `src/adapters/toml/repository.rs` only accesses `noslop_file.checks` and `noslop_file.project`. It does not read `agent` or `review`. No changes needed.

## Conversion: TOML Sections to Domain Models

The TOML parser produces intermediate structs (`AgentSection`, `ReviewSection`, etc.) that are then converted to the core domain models defined in `src/core/models/`. This conversion happens in the CLI layer (`src/cli/commands/review.rs`) where concrete adapters are wired together.

### `ReviewSection` to `ReviewConfig`

```rust
use crate::core::models::{ReviewConfig, AnalyzerEntry};

impl ReviewSection {
    /// Convert to the core ReviewConfig, resolving analyzer entries
    pub fn to_review_config(
        &self,
        default_agent: Option<&str>,
    ) -> anyhow::Result<ReviewConfig> {
        let mut entries = Vec::new();

        for name in &self.analyzers {
            let entry = self.resolve_analyzer_entry(name, default_agent)?;
            entries.push(entry);
        }

        Ok(ReviewConfig {
            analyzer_entries: entries,
            co_change: self.co_change.clone().unwrap_or_default(),
            formatting: self.formatting.clone().unwrap_or_default(),
        })
    }

    /// Resolve a single analyzer name to its full configuration
    fn resolve_analyzer_entry(
        &self,
        name: &str,
        default_agent: Option<&str>,
    ) -> anyhow::Result<AnalyzerEntry> {
        if name.starts_with("agent:") {
            let focus = name.strip_prefix("agent:").unwrap();
            let config = self.analyzer_configs
                .get(name)
                .map(|v| v.clone().try_into::<AgentAnalyzerConfig>())
                .transpose()?;

            let agent = config.as_ref()
                .and_then(|c| c.agent.as_deref())
                .or(default_agent)
                .ok_or_else(|| anyhow::anyhow!(
                    "Analyzer '{name}' requires an agent. Set [agent].type or [review.\"{name}\"].agent"
                ))?
                .to_string();

            let prompt_template = config.as_ref()
                .and_then(|c| c.prompt_template.clone());

            Ok(AnalyzerEntry::Agent {
                name: name.to_string(),
                focus: focus.to_string(),
                agent,
                prompt_template,
            })
        } else if name.starts_with("custom:") {
            let config: ScriptAnalyzerConfig = self.analyzer_configs
                .get(name)
                .ok_or_else(|| anyhow::anyhow!(
                    "Analyzer '{name}' requires [review.\"{name}\"] configuration with type and command"
                ))?
                .clone()
                .try_into()?;

            Ok(AnalyzerEntry::Script {
                name: name.to_string(),
                command: config.command,
            })
        } else {
            // Built-in analyzer: conventions, formatting, co-change
            Ok(AnalyzerEntry::BuiltIn {
                name: name.to_string(),
            })
        }
    }
}
```

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

### CLI Overrides Struct

The CLI override values that can shadow TOML settings:

```rust
/// Values from CLI flags that override TOML configuration
pub struct ReviewCliOverrides {
    pub analyzers: Option<Vec<String>>,
    pub agent: Option<String>,
    pub base: Option<String>,
    pub check_mode: bool,
    pub min_correlation: Option<f64>,
}
```

## `.noslop.toml` Writer Changes

When `noslop init <agent>` runs, it writes the `.noslop.toml` file with the `[agent]` section. The existing `build_noslop_toml` function handles this:

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
# Review pipeline (uncomment to customize):
# [review]
# analyzers = ["conventions", "formatting", "co-change", "agent:security"]

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

The `[review]` section is NOT written by `noslop init`. It is user-configured. Users add it manually when they want to customize the pipeline beyond the default `["conventions"]` analyzer. This keeps the generated config minimal and avoids polluting the file with defaults that match the compiled-in values.

## Validation Rules

The TOML parser deserializes permissively. Validation of semantic constraints happens in the CLI command handlers, not in the parser. This follows noslop's existing pattern where `parser.rs` handles deserialization and `commands/*.rs` handles validation.

### Agent Type Validation

```rust
// In cli/commands/review.rs
if let Some(agent) = &noslop_file.agent {
    agent.to_agent_type()?; // Fails with "Unknown agent type: X"
}
```

### Analyzer List Validation

Verify that every analyzer in the list is either a known built-in or has configuration:

```rust
fn validate_analyzers(review: &ReviewSection) -> anyhow::Result<()> {
    let known_builtins = ["conventions", "formatting", "co-change"];

    for name in &review.analyzers {
        if known_builtins.contains(&name.as_str()) {
            continue;
        }
        if name.starts_with("agent:") || name.starts_with("custom:") {
            // Custom analyzers are validated during resolve_analyzer_entry
            continue;
        }
        anyhow::bail!(
            "Unknown analyzer '{name}'. Built-in: {known_builtins:?}. \
             Use 'agent:' or 'custom:' prefix for extensions."
        );
    }
    Ok(())
}
```

### Co-Change Correlation Range

```rust
if let Some(cc) = &review.co_change {
    if !(0.0..=1.0).contains(&cc.min_correlation) {
        anyhow::bail!(
            "min_correlation must be between 0.0 and 1.0, got {}",
            cc.min_correlation
        );
    }
}
```

### Script Analyzer Command Non-Empty

```rust
// Validated in resolve_analyzer_entry when type == "script"
if config.command.trim().is_empty() {
    anyhow::bail!("Script analyzer '{name}' has empty command");
}
```

## File Layout After Changes

```
src/
├── adapters/
│   └── toml/
│       ├── parser.rs       # Modified: NoslopFile gains agent + review fields;
│       │                   #   new structs AgentSection, ReviewSection,
│       │                   #   CoChangeConfig, FormattingConfig,
│       │                   #   AgentAnalyzerConfig, ScriptAnalyzerConfig
│       ├── repository.rs   # Unchanged
│       └── writer.rs       # Unchanged (build_noslop_toml in init.rs)
├── cli/
│   └── commands/
│       └── review.rs       # Reads NoslopFile.agent and NoslopFile.review,
│                           #   converts to domain models, applies CLI overrides
└── core/
    └── models/
        ├── finding.rs      # Finding, FindingSource, Resolution
        └── review_target.rs # ReviewTarget, ReviewConfig, AnalyzerEntry
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
        assert!(file.review.is_none());
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
    fn parse_with_review_pipeline() {
        let toml_str = r#"
            [project]
            prefix = "NOS"

            [review]
            analyzers = ["conventions", "formatting", "co-change"]
        "#;
        let file: NoslopFile = toml::from_str(toml_str).unwrap();
        let review = file.review.unwrap();
        assert_eq!(review.analyzers.len(), 3);
        assert_eq!(review.analyzers[0], "conventions");
    }

    #[test]
    fn parse_with_co_change_config() {
        let toml_str = r#"
            [project]
            prefix = "NOS"

            [review]
            analyzers = ["co-change"]

            [review.co-change]
            min_correlation = 0.8
            lookback = 1000
            min_commits = 5
        "#;
        let file: NoslopFile = toml::from_str(toml_str).unwrap();
        let cc = file.review.unwrap().co_change.unwrap();
        assert!((cc.min_correlation - 0.8).abs() < f64::EPSILON);
        assert_eq!(cc.lookback, 1000);
        assert_eq!(cc.min_commits, 5);
    }

    #[test]
    fn parse_with_agent_analyzer() {
        let toml_str = r#"
            [project]
            prefix = "NOS"

            [agent]
            type = "claude"

            [review]
            analyzers = ["agent:security"]

            [review."agent:security"]
            agent = "claude"
            prompt_template = ".noslop/prompts/security.md"
        "#;
        let file: NoslopFile = toml::from_str(toml_str).unwrap();
        let review = file.review.unwrap();
        assert!(review.analyzer_configs.contains_key("agent:security"));
    }

    #[test]
    fn parse_with_script_analyzer() {
        let toml_str = r#"
            [project]
            prefix = "NOS"

            [review]
            analyzers = ["custom:migrations"]

            [review."custom:migrations"]
            type = "script"
            command = "python ./scripts/check-migrations.py"
        "#;
        let file: NoslopFile = toml::from_str(toml_str).unwrap();
        let review = file.review.unwrap();
        assert!(review.analyzer_configs.contains_key("custom:migrations"));
    }

    #[test]
    fn parse_with_formatting_config() {
        let toml_str = r#"
            [project]
            prefix = "NOS"

            [review]
            analyzers = ["formatting"]

            [review.formatting]
            tools = ["rustfmt", "prettier"]
        "#;
        let file: NoslopFile = toml::from_str(toml_str).unwrap();
        let fmt = file.review.unwrap().formatting.unwrap();
        assert_eq!(fmt.tools, vec!["rustfmt", "prettier"]);
    }

    #[test]
    fn parse_full_review_config() {
        let toml_str = r#"
            [project]
            prefix = "NOS"

            [agent]
            type = "claude"

            [review]
            analyzers = ["conventions", "formatting", "co-change", "agent:security", "custom:lint"]

            [review.co-change]
            min_correlation = 0.8
            lookback = 1000
            min_commits = 5

            [review."agent:security"]
            agent = "claude"
            prompt_template = ".noslop/prompts/security.md"

            [review."custom:lint"]
            type = "script"
            command = "eslint --format json ."

            [review.formatting]
            tools = ["rustfmt", "prettier"]

            [[check]]
            target = "*.rs"
            message = "Check Rust files"
            severity = "warn"
        "#;
        let file: NoslopFile = toml::from_str(toml_str).unwrap();

        assert!(file.agent.is_some());
        let review = file.review.unwrap();
        assert_eq!(review.analyzers.len(), 5);
        assert!(review.co_change.is_some());
        assert!(review.formatting.is_some());
        assert!(review.analyzer_configs.contains_key("agent:security"));
        assert!(review.analyzer_configs.contains_key("custom:lint"));
        assert_eq!(file.checks.len(), 1);
    }

    #[test]
    fn parse_partial_review_config_uses_defaults() {
        let toml_str = r#"
            [project]
            prefix = "NOS"

            [review]
            analyzers = ["conventions", "formatting"]
        "#;
        let file: NoslopFile = toml::from_str(toml_str).unwrap();
        let review = file.review.unwrap();
        assert_eq!(review.analyzers.len(), 2);
        assert!(review.co_change.is_none());
        assert!(review.formatting.is_none());
    }

    #[test]
    fn existing_config_without_new_sections_parses() {
        // This is the actual content pattern from noslop's own .noslop.toml
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
        assert!(file.review.is_none());
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
    fn co_change_config_defaults() {
        let config = CoChangeConfig::default();
        assert!((config.min_correlation - 0.7).abs() < f64::EPSILON);
        assert_eq!(config.lookback, 500);
        assert_eq!(config.min_commits, 10);
    }

    #[test]
    fn review_section_default_analyzers() {
        let section = ReviewSection::default();
        assert_eq!(section.analyzers, vec!["conventions"]);
    }
}
```

### Validation Tests

```rust
#[test]
fn validate_known_analyzer_names() {
    let review = ReviewSection {
        analyzers: vec![
            "conventions".to_string(),
            "formatting".to_string(),
            "co-change".to_string(),
        ],
        ..Default::default()
    };
    assert!(validate_analyzers(&review).is_ok());
}

#[test]
fn validate_unknown_analyzer_name_errors() {
    let review = ReviewSection {
        analyzers: vec!["nonexistent".to_string()],
        ..Default::default()
    };
    assert!(validate_analyzers(&review).is_err());
}

#[test]
fn validate_agent_prefix_accepted() {
    let review = ReviewSection {
        analyzers: vec!["agent:security".to_string()],
        ..Default::default()
    };
    assert!(validate_analyzers(&review).is_ok());
}

#[test]
fn validate_custom_prefix_accepted() {
    let review = ReviewSection {
        analyzers: vec!["custom:lint".to_string()],
        ..Default::default()
    };
    assert!(validate_analyzers(&review).is_ok());
}

#[test]
fn co_change_correlation_in_range() {
    // Valid
    assert!((0.0..=1.0).contains(&0.7));
    assert!((0.0..=1.0).contains(&0.0));
    assert!((0.0..=1.0).contains(&1.0));
    // Invalid
    assert!(!(0.0..=1.0).contains(&-0.1));
    assert!(!(0.0..=1.0).contains(&1.1));
}
```

## Summary

This design consolidates all configuration surfaces into a single reference:

1. **`.noslop.toml` schema**: Two new optional sections (`[agent]` and `[review]` with sub-tables) alongside the existing `[project]` and `[[check]]` sections. The `[review]` section uses a composable analyzer list with convention-based naming (`agent:*`, `custom:*`) and per-analyzer configuration sub-tables.

2. **`.noslop/` directory**: New `reviews/` directory for review session files and review memory. New `prompts/` directory for custom agent prompt templates. All files are committed to git except `staged-acks.json`.

3. **Parser changes**: Six new structs (`AgentSection`, `ReviewSection`, `CoChangeConfig`, `FormattingConfig`, `AgentAnalyzerConfig`, `ScriptAnalyzerConfig`) added to `src/adapters/toml/parser.rs`. Two new fields on `NoslopFile`. Zero changes to existing functions or the `TomlCheckRepository` adapter.

4. **Precedence rules**: CLI flags override TOML values override compiled defaults. Implemented via `Option<T>` in clap args and explicit merging in the command handler.

5. **Backward compatibility**: Existing `.noslop.toml` files parse without error. Old noslop binaries ignore unknown TOML sections. No migration step required.
