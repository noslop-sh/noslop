# Init Command Expansion Design

Design specification for expanding `noslop init` to support agent-specific configuration via `noslop init <agent>`.

## Overview

noslop is a local code review tool for agent-generated code. The `init` command currently handles noslop-only setup: creating `.noslop.toml`, the `.noslop/` directory, and installing git hooks. This design expands it with an optional positional argument that configures a **review agent** -- the AI agent that noslop uses to run LLM-based analyzers in the review pipeline.

**Key distinction**: The developer uses Claude Code (or another agent) separately for coding. `noslop init claude` configures Claude as the **review** agent -- the agent that powers `agent:security`, `agent:quality`, and other LLM-based `ReviewAnalyzer` implementations.

**Invariant**: Bare `noslop init` continues to work exactly as it does today. Agent configuration is purely additive.

### Command Forms

| Command              | What it does                                                                      |
| -------------------- | --------------------------------------------------------------------------------- |
| `noslop init`        | Git hooks + `.noslop.toml` + `.noslop/` directory (unchanged)                     |
| `noslop init claude` | All of the above + Claude as review agent + `[review]` pipeline in `.noslop.toml` |
| `noslop init codex`  | All of the above + Codex as review agent + `[review]` pipeline in `.noslop.toml`  |

## CLI Changes

### `app.rs` -- Command Enum

The `Init` variant gains an optional positional `agent` argument. This uses clap's standard optional positional pattern.

```rust
#[derive(Subcommand, Debug)]
pub enum Command {
    /// Initialize noslop in the current repository
    Init {
        /// AI agent to use for LLM-based review (e.g., claude, codex)
        agent: Option<String>,

        /// Force re-initialization
        #[arg(short, long)]
        force: bool,
    },

    // ... remaining variants unchanged ...
}
```

The `agent` field is `Option<String>` rather than `Option<AgentType>` because clap parsing happens before we can produce a helpful error message. We parse it manually inside the command handler to control the error format.

### `app.rs` -- Dispatch

The `run()` function passes the new argument through to the command handler:

```rust
match cli.command {
    Some(Command::Init { agent, force }) => commands::init(agent, force, output_mode),
    // ... remaining arms unchanged ...
}
```

Full diff for `app.rs`:

```diff
 #[derive(Subcommand, Debug)]
 pub enum Command {
     /// Initialize noslop in the current repository
     Init {
+        /// AI agent to use for LLM-based review (e.g., claude, codex)
+        agent: Option<String>,
+
         /// Force re-initialization
         #[arg(short, long)]
         force: bool,
     },
```

```diff
 match cli.command {
-    Some(Command::Init { force }) => commands::init(force, output_mode),
+    Some(Command::Init { agent, force }) => commands::init(agent, force, output_mode),
```

## `.noslop.toml` Schema Extension

### `[agent]` and `[review]` Sections

When `noslop init <agent>` runs, it writes `[agent]` and `[review]` sections into `.noslop.toml`. The `[agent]` section records which agent is available for review inference. The `[review]` section configures the analyzer pipeline.

```toml
[project]
prefix = "NOS"

[agent]
type = "claude"

[review]
analyzers = ["conventions", "agent:security", "agent:quality"]

[review.agent:security]
agent = "claude"

[review.agent:quality]
agent = "claude"
```

The `[agent]` section has one required field:

| Field  | Type   | Description                               |
| ------ | ------ | ----------------------------------------- |
| `type` | string | Agent identifier: `"claude"` or `"codex"` |

The `[review]` section configures the analyzer pipeline:

| Field       | Type          | Description                                                    |
| ----------- | ------------- | -------------------------------------------------------------- |
| `analyzers` | array[string] | Ordered list of analyzers to run. Prefix `agent:` = LLM-based. |

Each `[review.agent:<name>]` sub-table binds an LLM analyzer to a specific agent:

| Field   | Type   | Description                                      |
| ------- | ------ | ------------------------------------------------ |
| `agent` | string | Which agent to use for this analyzer's inference |

### Analyzer Types

The pipeline supports three kinds of analyzers:

1. **Static analyzers** (e.g., `conventions`): Pattern-matching on diffs. No agent needed.
2. **Computed analyzers** (e.g., `complexity`): Computes metrics from code. No agent needed.
3. **Agent analyzers** (e.g., `agent:security`, `agent:quality`): Send diff context to an LLM agent via `AgentRuntime` for review inference. Require an `[review.agent:<name>]` binding.

### Parser Changes

The `NoslopFile` struct in `src/adapters/toml/parser.rs` gains optional `agent` and `review` fields:

```rust
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

/// Agent configuration section
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AgentSection {
    /// Agent type identifier (e.g., "claude", "codex")
    #[serde(rename = "type")]
    pub agent_type: String,
}

/// Review pipeline configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ReviewSection {
    /// Ordered list of analyzers to run
    pub analyzers: Vec<String>,
}
```

The `#[serde(default)]` on both fields means existing `.noslop.toml` files without these sections parse without error. The fields deserialize to `None`.

## Init Command Implementation

### Full `init.rs`

The expanded `init.rs` follows the existing pattern: sequential setup steps with progress messages. Agent setup is an optional final phase that only runs when an agent argument is provided.

```rust
//! Initialize noslop in a repository

use std::fs;
use std::path::Path;

use crate::{git, noslop_file};
use noslop::core::ports::AgentType;
use noslop::output::OutputMode;

/// Resolve an AgentType from a user-provided string, or return a helpful error
fn parse_agent_type(s: &str) -> anyhow::Result<AgentType> {
    s.parse::<AgentType>().map_err(|e| anyhow::anyhow!("{e}"))
}

/// Get the AgentConfig implementation for a given agent type
fn get_agent_config(agent_type: AgentType) -> Box<dyn noslop::core::ports::AgentConfig> {
    match agent_type {
        AgentType::Claude => Box::new(noslop::adapters::agent::ClaudeConfig::new()),
        AgentType::Codex => Box::new(noslop::adapters::agent::CodexConfig::new()),
    }
}

/// Initialize noslop in the current repository
pub fn init(agent: Option<String>, force: bool, _mode: OutputMode) -> anyhow::Result<()> {
    let noslop_path = Path::new(".noslop.toml");

    if noslop_path.exists() && !force {
        println!("Already initialized (.noslop.toml exists).");
        println!("Use --force to reinitialize.");
        return Ok(());
    }

    // --- Phase 1: Core noslop setup (unchanged) ---

    println!("Initializing noslop...\n");

    let prefix = noslop_file::generate_prefix_from_repo();
    println!("  Generated project prefix: {prefix}");

    // Build .noslop.toml content -- agent + review sections included if specified
    let agent_type = agent
        .as_deref()
        .map(parse_agent_type)
        .transpose()?;

    let noslop_toml = build_noslop_toml(&prefix, agent_type);
    fs::write(noslop_path, noslop_toml)?;
    println!("  Created .noslop.toml");

    fs::create_dir_all(".noslop")?;
    fs::write(".noslop/.gitkeep", "")?;
    println!("  Created .noslop/");

    git::hooks::install_pre_commit()?;
    println!("  Installed pre-commit hook");
    git::hooks::install_commit_msg()?;
    println!("  Installed commit-msg hook");
    git::hooks::install_post_commit()?;
    println!("  Installed post-commit hook");

    // Install pre-push hook for review gating
    if agent_type.is_some() {
        git::hooks::install_pre_push()?;
        println!("  Installed pre-push hook (runs noslop review --check)");
    }

    // Check for existing docs
    let docs = ["CLAUDE.md", "AGENTS.md", "ARCHITECTURE.md"];
    let found: Vec<_> = docs.iter().filter(|d| Path::new(d).exists()).collect();
    if !found.is_empty() {
        let found_str: Vec<&str> = found.into_iter().copied().collect();
        println!("\n  Found: {}", found_str.join(", "));
        println!("  Run 'noslop check import' to extract checks");
    }

    // --- Phase 2: Agent setup (only if agent specified) ---

    if let Some(agent_type) = agent_type {
        println!();
        configure_agent(agent_type)?;
    }

    // --- Done ---

    println!("\nnoslop initialized!");
    println!("\nNext steps:");
    println!("  noslop check add <target> -m \"message\"");
    println!("  git commit  # checks will be validated");

    if agent_type.is_some() {
        println!("  noslop review  # review current branch before push");
    }

    Ok(())
}

/// Build .noslop.toml content, optionally including [agent] and [review] sections
fn build_noslop_toml(prefix: &str, agent_type: Option<AgentType>) -> String {
    let mut toml = format!(
        r#"# noslop configuration

[project]
prefix = "{prefix}"
"#
    );

    if let Some(agent) = agent_type {
        toml.push_str(&format!(
            r#"
[agent]
type = "{agent}"

[review]
analyzers = ["conventions", "agent:security", "agent:quality"]

[review."agent:security"]
agent = "{agent}"

[review."agent:quality"]
agent = "{agent}"
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

/// Configure agent-specific files via the AgentConfig trait
fn configure_agent(agent_type: AgentType) -> anyhow::Result<()> {
    let config = get_agent_config(agent_type);

    println!("Configuring {} as review agent...\n", agent_type.display_name());

    // Validate agent CLI is available
    config.validate()?;

    // Get project root (current directory)
    let project_root = std::env::current_dir()?;

    // Generate and write the configuration bundle
    let bundle = config.generate_config(&project_root)?;
    write_config_bundle(&bundle)?;

    // Generate project-local instructions referencing review workflow
    if let Some(instructions) = config.generate_project_instructions(&project_root)? {
        write_project_instructions(agent_type, &instructions)?;
    }

    Ok(())
}

/// Write all files from an AgentConfigBundle to disk
fn write_config_bundle(bundle: &noslop::core::ports::AgentConfigBundle) -> anyhow::Result<()> {
    let home = dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?;

    // Write global instructions
    if let Some(ref instructions) = bundle.global_instructions {
        let path = home.join(&instructions.relative_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, &instructions.content)?;
        println!("  Created {}", instructions.relative_path.display());
    }

    // Write settings file
    if let Some(ref settings) = bundle.settings {
        let path = home.join(&settings.relative_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, &settings.content)?;
        println!("  Created {}", settings.relative_path.display());
    }

    Ok(())
}

/// Write project-local agent instructions (e.g., CLAUDE.md in repo root)
///
/// The generated instructions reference the review workflow, not code generation.
fn write_project_instructions(agent_type: AgentType, content: &str) -> anyhow::Result<()> {
    let filename = match agent_type {
        AgentType::Claude => "CLAUDE.md",
        AgentType::Codex => "AGENTS.md",
    };

    let path = Path::new(filename);
    if path.exists() {
        println!("  {filename} already exists, skipping");
    } else {
        fs::write(path, content)?;
        println!("  Created {filename}");
    }

    Ok(())
}
```

### Pre-Push Hook

When an agent is configured, `noslop init` installs a pre-push hook that gates pushes on review status. This ensures agent-generated code is reviewed before it reaches the remote.

```bash
#!/usr/bin/env bash
# noslop pre-push hook: gate pushes on unresolved review findings

# Run noslop review in check mode (non-zero exit blocks push)
noslop review --check

exit $?
```

The `noslop review --check` command exits non-zero if there are unresolved findings at severity `error` or above, blocking the push. Findings at `warn` severity are displayed but do not block.

#### `git::hooks::install_pre_push`

```rust
/// Install pre-push hook that runs noslop review --check
pub fn install_pre_push() -> anyhow::Result<()> {
    let hook_path = Path::new(".git/hooks/pre-push");
    let content = r#"#!/usr/bin/env bash
# noslop pre-push hook: gate pushes on unresolved review findings
noslop review --check
exit $?
"#;

    fs::write(hook_path, content)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(hook_path, fs::Permissions::from_mode(0o755))?;
    }

    Ok(())
}
```

## Dispatch Flow

The full flow from user input to file generation:

```
User runs: noslop init claude
                │
                ▼
┌──────────────────────────────────────────────────────────────────┐
│  CLI Parsing (app.rs)                                            │
│                                                                  │
│  Cli::parse() → Command::Init { agent: Some("claude"), force }   │
│  Dispatch: commands::init(Some("claude"), false, Human)          │
└──────────────────────────────┬───────────────────────────────────┘
                               │
                               ▼
┌──────────────────────────────────────────────────────────────────┐
│  init() -- Phase 1: Core noslop setup                            │
│                                                                  │
│  1. Check .noslop.toml exists (bail unless --force)              │
│  2. generate_prefix_from_repo() → "NOS"                          │
│  3. parse_agent_type("claude") → Ok(AgentType::Claude)           │
│  4. build_noslop_toml("NOS", Some(Claude))                       │
│     → writes .noslop.toml with:                                  │
│       [project]  — prefix                                        │
│       [agent]    — type = "claude"                                │
│       [review]   — analyzer pipeline with agent bindings          │
│  5. Create .noslop/ directory + .gitkeep                         │
│  6. Install git hooks (pre-commit, commit-msg, post-commit)      │
│  7. Install pre-push hook (noslop review --check)                │
│  8. Check for existing docs                                      │
└──────────────────────────────┬───────────────────────────────────┘
                               │
                               ▼
┌──────────────────────────────────────────────────────────────────┐
│  init() -- Phase 2: Agent setup                                  │
│                                                                  │
│  configure_agent(AgentType::Claude)                              │
│                                                                  │
│  1. get_agent_config(Claude) → Box<dyn AgentConfig>              │
│     Returns: ClaudeConfig (from adapters::agent::claude)         │
│                                                                  │
│  2. config.validate()                                            │
│     Checks: `which claude` succeeds                              │
│     Error:  "Claude Code CLI not found. Install: ..."            │
│                                                                  │
│  3. config.generate_config(&project_root)                        │
│     Returns: AgentConfigBundle {                                 │
│       global_instructions: ~/.claude/CLAUDE.md                   │
│       settings: ~/.claude/settings.json                          │
│     }                                                            │
│                                                                  │
│  4. write_config_bundle(&bundle)                                 │
│     Writes global instruction + settings files                   │
│                                                                  │
│  5. config.generate_project_instructions(&project_root)          │
│     Returns: Some("# Review Configuration\n...")                 │
│     → writes CLAUDE.md to repo root (if not exists)              │
│     → content references review workflow, not code generation    │
└──────────────────────────────────────────────────────────────────┘
```

### Bare `noslop init` Flow (Unchanged)

When no agent is specified, Phase 2 is skipped entirely:

```
User runs: noslop init
                │
                ▼
  agent = None → agent_type = None
                │
                ▼
  build_noslop_toml("NOS", None)
  → .noslop.toml without [agent] or [review] sections
                │
                ▼
  Phase 2: if let Some(agent_type) = None → skipped
  Pre-push hook: not installed (no review agent configured)
```

The generated `.noslop.toml` is identical to what `init` produces today:

```toml
# noslop configuration

[project]
prefix = "NOS"

# Checks are auto-assigned IDs like NOS-1, NOS-2, etc.
# You can also specify custom IDs:
#   id = "my-custom-id"

# Example check (uncomment to use):
# [[check]]
# target = "*.rs"
# message = "Consider impact on public API"
# severity = "warn"
```

### `noslop init claude` Generated `.noslop.toml`

With an agent specified, the `[agent]` and `[review]` sections are inserted between `[project]` and the check comments:

```toml
# noslop configuration

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

# Checks are auto-assigned IDs like NOS-1, NOS-2, etc.
# You can also specify custom IDs:
#   id = "my-custom-id"

# Example check (uncomment to use):
# [[check]]
# target = "*.rs"
# message = "Consider impact on public API"
# severity = "warn"
```

### Generated Project Instructions (CLAUDE.md)

When `noslop init claude` creates a project-local `CLAUDE.md`, the content references the review workflow:

```markdown
# noslop Review Configuration

This project uses noslop for automated code review of agent-generated changes.

## Review Workflow

1. Create changes on a feature branch (using Claude Code or any agent)
2. Run `noslop review` to analyze changes before push
3. Address findings at severity `error` or above
4. Push -- the pre-push hook runs `noslop review --check` automatically

## Review Pipeline

The configured analyzers run in order:

- `conventions` -- static pattern matching against project conventions
- `agent:security` -- LLM-based security review (powered by Claude)
- `agent:quality` -- LLM-based code quality review (powered by Claude)

## Commands

- `noslop review` -- run the review pipeline on current branch
- `noslop review --check` -- exit non-zero if unresolved error findings
- `noslop review --base main` -- review against a specific base branch
- `noslop checkpoint` -- save current state before fixing findings
```

## Factory Function

The `get_agent_config` function lives in `init.rs` (the command handler) because it is CLI-layer wiring. It bridges the port trait to concrete adapter implementations. This follows noslop's pattern where `app.rs` and command handlers are the only places that know about concrete adapter types.

```rust
fn get_agent_config(agent_type: AgentType) -> Box<dyn noslop::core::ports::AgentConfig> {
    match agent_type {
        AgentType::Claude => Box::new(noslop::adapters::agent::ClaudeConfig::new()),
        AgentType::Codex => Box::new(noslop::adapters::agent::CodexConfig::new()),
    }
}
```

This is the same pattern used elsewhere in noslop: the CLI layer instantiates concrete adapters and passes them as trait objects. The core never knows which adapter is in use.

## File Layout After Changes

```
src/
├── cli/
│   ├── app.rs                    # Modified: Init variant gains `agent` field
│   └── commands/
│       ├── mod.rs                # Modified: init signature gains `agent` param
│       └── init.rs               # Modified: agent parsing + Phase 2
├── core/
│   └── ports/
│       ├── mod.rs                # Modified: re-exports agent types
│       └── agent.rs              # New: AgentConfig + AgentRuntime traits
│                                 #   (defined in 03-agent-port-design.md)
├── adapters/
│   ├── toml/
│   │   └── parser.rs            # Modified: NoslopFile gains agent + review fields
│   └── agent/                   # New: agent adapter implementations
│       ├── mod.rs               #   Module re-exports
│       ├── claude.rs            #   ClaudeConfig (impl AgentConfig)
│       └── codex.rs             #   CodexConfig (impl AgentConfig)
```

## Commands Module Change

The `mod.rs` re-export updates to match the new signature:

```rust
pub use init::init;
```

No change to the `pub use` line itself since the function name stays `init`. The signature change is internal to `init.rs`:

```diff
-pub fn init(force: bool, _mode: OutputMode) -> anyhow::Result<()> {
+pub fn init(agent: Option<String>, force: bool, _mode: OutputMode) -> anyhow::Result<()> {
```

## Error Handling

### Unknown Agent Type

```
$ noslop init cursor
Error: Unknown agent type: cursor. Supported: claude, codex
```

The error comes from `AgentType::from_str` (defined in 03-agent-port-design.md). It is surfaced by `parse_agent_type()` in `init.rs` before any file operations begin.

### Agent CLI Not Installed

```
$ noslop init claude
Initializing noslop...

  Generated project prefix: NOS
  Created .noslop.toml
  Created .noslop/
  Installed pre-commit hook
  Installed commit-msg hook
  Installed post-commit hook
  Installed pre-push hook (runs noslop review --check)

Configuring Claude Code as review agent...

Error: Claude Code CLI not found in PATH. Install it first: npm install -g @anthropic-ai/claude-code
```

Core noslop setup completes successfully. The agent validation failure does not roll back the core setup because noslop works independently of any agent. The user can re-run `noslop init claude --force` after installing the CLI.

### Re-initialization

```
$ noslop init claude
Already initialized (.noslop.toml exists).
Use --force to reinitialize.
```

Same behavior as today. `--force` overwrites everything including the `[agent]` and `[review]` sections.

## Backward Compatibility

| Scenario                                                    | Behavior                                                                   |
| ----------------------------------------------------------- | -------------------------------------------------------------------------- |
| Existing `.noslop.toml` without `[agent]`                   | Parses fine (`agent: None`). `noslop review` uses static analyzers only.   |
| `noslop init` (no agent)                                    | Produces `.noslop.toml` without `[agent]` or `[review]`. Current behavior. |
| `noslop init claude --force` on existing repo               | Overwrites `.noslop.toml` with `[agent]` + `[review]`. Reinstalls hooks.   |
| Old noslop binary reading new `.noslop.toml` with `[agent]` | TOML parser ignores unknown sections by default. No error.                 |

The last point is important: because the existing `NoslopFile` struct uses `#[serde(default)]` for the new fields, and TOML deserialization with serde ignores unknown keys by default when `#[serde(deny_unknown_fields)]` is NOT set (which it is not in the current code), an old binary can read a new config file without error.

## Test Strategy

### Unit Tests

Tests for `parse_agent_type`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_claude() {
        assert_eq!(parse_agent_type("claude").unwrap(), AgentType::Claude);
    }

    #[test]
    fn parse_codex() {
        assert_eq!(parse_agent_type("codex").unwrap(), AgentType::Codex);
    }

    #[test]
    fn parse_unknown_errors() {
        assert!(parse_agent_type("cursor").is_err());
    }

    #[test]
    fn parse_case_insensitive() {
        assert_eq!(parse_agent_type("Claude").unwrap(), AgentType::Claude);
        assert_eq!(parse_agent_type("CODEX").unwrap(), AgentType::Codex);
    }
}
```

Tests for `build_noslop_toml`:

```rust
#[test]
fn build_toml_without_agent() {
    let toml = build_noslop_toml("NOS", None);
    assert!(toml.contains("[project]"));
    assert!(toml.contains("prefix = \"NOS\""));
    assert!(!toml.contains("[agent]"));
    assert!(!toml.contains("[review]"));
}

#[test]
fn build_toml_with_claude() {
    let toml = build_noslop_toml("NOS", Some(AgentType::Claude));
    assert!(toml.contains("[project]"));
    assert!(toml.contains("[agent]"));
    assert!(toml.contains("type = \"claude\""));
    assert!(toml.contains("[review]"));
    assert!(toml.contains("\"conventions\""));
    assert!(toml.contains("\"agent:security\""));
    assert!(toml.contains("\"agent:quality\""));
}

#[test]
fn build_toml_parses_back() {
    let toml_str = build_noslop_toml("NOS", Some(AgentType::Claude));
    let parsed: NoslopFile = toml::from_str(&toml_str).unwrap();
    assert_eq!(parsed.project.prefix, "NOS");
    assert_eq!(parsed.agent.unwrap().agent_type, "claude");
    assert_eq!(parsed.review.unwrap().analyzers.len(), 3);
}
```

### Integration Tests

Full CLI integration test using `TempGitRepo`:

```rust
#[test]
fn init_bare_creates_noslop_toml_without_agent() {
    let repo = TempGitRepo::new();
    let output = Command::cargo_bin("noslop")
        .unwrap()
        .current_dir(repo.path())
        .arg("init")
        .assert()
        .success();

    let toml_content = fs::read_to_string(repo.path().join(".noslop.toml")).unwrap();
    assert!(toml_content.contains("[project]"));
    assert!(!toml_content.contains("[agent]"));
    assert!(!toml_content.contains("[review]"));

    // pre-push hook should NOT be installed without agent
    assert!(!repo.path().join(".git/hooks/pre-push").exists());
}

#[test]
fn init_claude_creates_agent_and_review_sections() {
    let repo = TempGitRepo::new();
    // Note: this test requires `claude` CLI in PATH or uses a mock
    let output = Command::cargo_bin("noslop")
        .unwrap()
        .current_dir(repo.path())
        .arg("init")
        .arg("claude")
        .assert();

    let toml_content = fs::read_to_string(repo.path().join(".noslop.toml")).unwrap();
    assert!(toml_content.contains("[agent]"));
    assert!(toml_content.contains("type = \"claude\""));
    assert!(toml_content.contains("[review]"));
    assert!(toml_content.contains("agent:security"));

    // pre-push hook should be installed
    assert!(repo.path().join(".git/hooks/pre-push").exists());
}

#[test]
fn init_unknown_agent_fails() {
    let repo = TempGitRepo::new();
    Command::cargo_bin("noslop")
        .unwrap()
        .current_dir(repo.path())
        .arg("init")
        .arg("cursor")
        .assert()
        .failure()
        .stderr(predicates::str::contains("Unknown agent type"));
}
```

## Summary

The init expansion follows noslop's existing patterns, reframed for the review tool:

1. **CLI is a thin dispatch layer**: `app.rs` adds one field to the `Init` variant and passes it through
2. **Command handlers do the work**: `init.rs` orchestrates two-phase setup (core + agent)
3. **Ports define interfaces**: `AgentConfig` trait (from 03-agent-port-design.md) abstracts agent differences
4. **Adapters implement I/O**: `ClaudeConfig` and `CodexConfig` generate agent-specific files
5. **Config is TOML**: The `[agent]` and `[review]` sections extend `.noslop.toml` without breaking existing parsers
6. **Error handling uses `anyhow::Result`**: Consistent with every other function in the codebase

The key differences from the previous (code generation) design:

- **`noslop init claude` configures Claude as the review agent**, not the coding agent
- **`.noslop.toml` gains a `[review]` section** with the analyzer pipeline configuration
- **Generated project instructions reference the review workflow**: `noslop review`, not `noslop run`
- **Pre-push hook is installed** to gate pushes on unresolved review findings
- **No references to code generation, plan files, or iteration loops**

The changes are minimal and additive:

- `app.rs`: 3 lines changed (add field, update dispatch)
- `init.rs`: ~120 lines added (agent parsing, Phase 2, helpers, pre-push hook)
- `parser.rs`: ~15 lines added (`AgentSection`, `ReviewSection` structs, fields on `NoslopFile`)
- No changes to any existing core, adapter, or service code
