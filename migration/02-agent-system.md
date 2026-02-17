# Agent System

Agent port traits and Claude/Codex adapter implementations. Covers `AgentConfig` (configuration generation for `noslop init`) and `AgentRuntime` (CLI invocation for `noslop review`), plus shared output parsing.

## Overview

Agents are AI reviewers. The user picks one during setup (`noslop init claude`), noslop generates its configuration files, and then uses it during `noslop review` for LLM-powered review passes. Two port traits define the boundary:

- **`AgentConfig`** -- generates agent-specific configuration files (settings, instructions, hooks, slash commands) during `noslop init <agent>`.
- **`AgentRuntime`** -- invokes an agent CLI with a review prompt and parses structured findings from its output during `noslop review`.

The agent is one kind of `ReviewAnalyzer`. The `AgentAnalyzer` adapter (defined in the pipeline doc) wraps an `AgentRuntime` and bridges it into the review pipeline. Multiple agent passes are possible (security, quality) with different focused prompts.

All domain types come from `core::models` (defined in doc 00): `AgentKind`, `Finding`, `Severity`, `Target`, `Span`, `FindingSource`. This document does NOT redefine them.

```
src/core/ports/agent.rs          <- trait definitions + port-local structs
src/adapters/agents/
  mod.rs                          <- re-exports, factory functions
  claude.rs                       <- ClaudeConfig + ClaudeRuntime
  codex.rs                        <- CodexConfig + CodexRuntime
  parsing.rs                      <- shared finding extraction
  templates/                      <- embedded content (include_str!)
    claude_global_instructions.md
    claude_branch_protection.sh
    claude_auto_format.sh
    claude_cmd_checkpoint.md
    claude_cmd_worktree.md
    codex_instructions.md
```

---

## AgentConfig Trait

Generates configuration files during `noslop init <agent>`. Each agent produces an `AgentConfigBundle` containing all files to write.

```rust
// src/core/ports/agent.rs

use std::path::{Path, PathBuf};
use crate::core::models::AgentKind;

/// Global instructions file (e.g., ~/.claude/CLAUDE.md)
#[derive(Debug, Clone)]
pub struct GlobalInstructions {
    pub relative_path: PathBuf, // relative to $HOME
    pub content: String,
}

/// Agent settings file (e.g., ~/.claude/settings.json)
#[derive(Debug, Clone)]
pub struct SettingsFile {
    pub relative_path: PathBuf, // relative to $HOME
    pub content: String,
}

/// Hook definition for agent configuration
#[derive(Debug, Clone)]
pub struct HookDefinition {
    pub event: String,   // e.g., "PreToolUse"
    pub matcher: String, // e.g., "Edit|Write"
    pub command: String, // script content or path
}

/// Slash command definition
#[derive(Debug, Clone)]
pub struct SlashCommand {
    pub name: String,           // without leading slash
    pub relative_path: PathBuf, // relative to agent config dir
    pub content: String,
}

/// Complete configuration bundle returned by generate_config
#[derive(Debug, Clone, Default)]
pub struct AgentConfigBundle {
    pub global_instructions: Option<GlobalInstructions>,
    pub settings: Option<SettingsFile>,
    pub hooks: Vec<HookDefinition>,
    pub commands: Vec<SlashCommand>,
}

/// Agent configuration generator.
///
/// Implementations produce all files needed for an agent to work with noslop.
/// Called once during `noslop init <agent>`.
#[cfg_attr(test, mockall::automock)]
pub trait AgentConfig: Send + Sync {
    fn agent_type(&self) -> AgentKind;
    fn generate_config(&self, project_root: &Path) -> anyhow::Result<AgentConfigBundle>;
    fn is_available(&self) -> bool;
    fn install_instructions(&self) -> &str;

    /// Default validates CLI availability.
    fn validate(&self) -> anyhow::Result<()> {
        if !self.is_available() {
            anyhow::bail!(
                "{:?} CLI not found in PATH. Install: {}",
                self.agent_type(),
                self.install_instructions()
            );
        }
        Ok(())
    }
}
```

The init command calls `generate_config`, iterates over the bundle, and writes each file. Backup/conflict logic lives in the init command, not the adapter.

---

## AgentRuntime Trait

Invokes an agent CLI for review. Called by `AgentAnalyzer` during `noslop review`.

```rust
// src/core/ports/agent.rs (continued)

use std::collections::HashMap;
use crate::core::models::{AgentKind, Finding};

/// Configuration for a single agent invocation
#[derive(Debug, Clone)]
pub struct InvocationConfig {
    pub prompt: String,
    pub working_dir: PathBuf,
    pub timeout_secs: Option<u64>,
    pub bypass_permissions: bool,
    pub env: HashMap<String, String>,
}

impl Default for InvocationConfig {
    fn default() -> Self {
        Self {
            prompt: String::new(),
            working_dir: PathBuf::from("."),
            timeout_secs: None,
            bypass_permissions: true,
            env: HashMap::new(),
        }
    }
}

/// Raw result from an agent invocation
#[derive(Debug, Clone)]
pub struct InvocationResult {
    pub output: String,
    pub exit_code: i32,
    pub duration: std::time::Duration,
}

/// Parsed output from an agent review pass
#[derive(Debug, Clone, Default)]
pub struct ReviewOutput {
    pub findings: Vec<Finding>,
    pub raw_output: String,
    pub errors: Vec<String>,
}

/// Agent runtime for review invocation.
///
/// Wraps a CLI tool (claude, codex), sends it a review prompt,
/// parses structured findings from the output.
#[cfg_attr(test, mockall::automock)]
pub trait AgentRuntime: Send + Sync {
    fn agent_type(&self) -> AgentKind;
    fn invoke(&self, config: &InvocationConfig) -> anyhow::Result<InvocationResult>;
    fn parse_output(&self, result: &InvocationResult) -> ReviewOutput;
    fn is_available(&self) -> bool;
}
```

### Factory Functions

```rust
// src/adapters/agents/mod.rs

pub fn get_agent_config(kind: AgentKind) -> Box<dyn AgentConfig> {
    match kind {
        AgentKind::Claude => Box::new(ClaudeConfig),
        AgentKind::Codex => Box::new(CodexConfig),
    }
}

pub fn get_agent_runtime(kind: AgentKind) -> Box<dyn AgentRuntime> {
    match kind {
        AgentKind::Claude => Box::new(ClaudeRuntime),
        AgentKind::Codex => Box::new(CodexRuntime),
    }
}
```

---

## Claude Adapter

### ClaudeConfig

Generates `settings.json`, `CLAUDE.md`, PreToolUse/PostToolUse hooks, and slash commands under `~/.claude/`.

| File                 | Relative Path                        | Purpose                                       |
| -------------------- | ------------------------------------ | --------------------------------------------- |
| settings.json        | `.claude/settings.json`              | Permissions, hooks, sandbox, model            |
| CLAUDE.md            | `.claude/CLAUDE.md`                  | Global agent instructions                     |
| branch-protection.sh | `.claude/hooks/branch-protection.sh` | PreToolUse: block edits on protected branches |
| auto-format.sh       | `.claude/hooks/auto-format.sh`       | PostToolUse: auto-format after edits          |
| checkpoint.md        | `.claude/commands/checkpoint.md`     | /checkpoint slash command                     |
| worktree.md          | `.claude/commands/worktree.md`       | /worktree slash command                       |

```rust
// src/adapters/agents/claude.rs

use std::path::{Path, PathBuf};
use std::process::Command;
use crate::core::models::AgentKind;
use crate::core::ports::agent::*;

#[derive(Debug, Clone, Copy)]
pub struct ClaudeConfig;

impl AgentConfig for ClaudeConfig {
    fn agent_type(&self) -> AgentKind { AgentKind::Claude }

    fn generate_config(&self, _project_root: &Path) -> anyhow::Result<AgentConfigBundle> {
        Ok(AgentConfigBundle {
            global_instructions: Some(GlobalInstructions {
                relative_path: PathBuf::from(".claude/CLAUDE.md"),
                content: include_str!("templates/claude_global_instructions.md").to_string(),
            }),
            settings: Some(SettingsFile {
                relative_path: PathBuf::from(".claude/settings.json"),
                content: self.build_settings()?,
            }),
            hooks: vec![
                HookDefinition {
                    event: "PreToolUse".into(),
                    matcher: "Edit|Write".into(),
                    command: include_str!("templates/claude_branch_protection.sh").to_string(),
                },
                HookDefinition {
                    event: "PostToolUse".into(),
                    matcher: "Edit|Write".into(),
                    command: include_str!("templates/claude_auto_format.sh").to_string(),
                },
            ],
            commands: vec![
                SlashCommand {
                    name: "checkpoint".into(),
                    relative_path: PathBuf::from("commands/checkpoint.md"),
                    content: include_str!("templates/claude_cmd_checkpoint.md").to_string(),
                },
                SlashCommand {
                    name: "worktree".into(),
                    relative_path: PathBuf::from("commands/worktree.md"),
                    content: include_str!("templates/claude_cmd_worktree.md").to_string(),
                },
            ],
        })
    }

    fn is_available(&self) -> bool {
        Command::new("claude").arg("--version").output().is_ok_and(|o| o.status.success())
    }

    fn install_instructions(&self) -> &str {
        "npm install -g @anthropic-ai/claude-code"
    }
}
```

#### settings.json Generation

Built programmatically via `serde_json`. Key sections:

```rust
impl ClaudeConfig {
    fn build_settings(&self) -> anyhow::Result<String> {
        let settings = serde_json::json!({
            "$schema": "https://json.schemastore.org/claude-code-settings.json",
            "permissions": {
                "allow": [
                    // Core tools
                    "Read", "Edit", "Write", "Glob", "Grep", "WebFetch", "WebSearch",
                    // Git operations (status, diff, log, branch, add, commit, checkout, etc.)
                    "Bash(git status *)", "Bash(git diff *)", "Bash(git log *)",
                    "Bash(git add *)", "Bash(git commit *)", "Bash(git checkout *)",
                    // Language toolchains (npm, node, python, poetry, pip)
                    "Bash(npm run *)", "Bash(npx *)", "Bash(python -m *)",
                    // Linters/formatters
                    "Bash(ruff *)", "Bash(eslint *)", "Bash(prettier *)",
                    // System utilities (ls, cat, find, tree, jq, curl, etc.)
                    "Bash(ls *)", "Bash(find *)", "Bash(jq *)",
                    // noslop tools
                    "Bash(noslop *)", "Bash(checkpoint *)", "Bash(checkpoint)",
                    "Bash(worktree *)"
                    // Full list: ~50 rules covering git, npm, python, docker, system utils
                ],
                "deny": [
                    "Read(./.env)", "Read(./.env.*)", "Read(./secrets/*.json)",
                    "Read(~/.ssh/**)",
                    "Bash(rm -rf /)", "Bash(git push --force *)",
                    "Bash(git reset --hard *)", "Bash(git clean -f *)"
                ],
                "ask": [
                    "Bash(git push *)", "Bash(docker run *)",
                    "Bash(rm -rf *)", "Bash(rm -r *)"
                ]
            },
            "hooks": {
                "PreToolUse": [{
                    "matcher": "Edit|Write",
                    "hooks": [{
                        "type": "command",
                        "command": "~/.claude/hooks/branch-protection.sh",
                        "statusMessage": "Checking branch protection..."
                    }]
                }],
                "PostToolUse": [{
                    "matcher": "Edit|Write",
                    "hooks": [{
                        "type": "command",
                        "command": "~/.claude/hooks/auto-format.sh",
                        "statusMessage": "Auto-formatting..."
                    }]
                }]
            },
            "model": "opus",
            "sandbox": { "enabled": true, "autoAllowBashIfSandboxed": true },
            "alwaysThinkingEnabled": true,
            "showTurnDuration": true
        });
        serde_json::to_string_pretty(&settings).map_err(Into::into)
    }
}
```

#### Generated Templates

**CLAUDE.md** -- global instructions referencing noslop, checkpoint, worktree, and project conventions (git flow, no generic prefixes, type hints).

**branch-protection.sh** (PreToolUse: Edit|Write) -- blocks file edits on protected branches:

```bash
#!/bin/bash
BRANCH=$(git rev-parse --abbrev-ref HEAD 2>/dev/null)
PROTECTED=("main" "master" "development" "production")
for p in "${PROTECTED[@]}"; do
  [[ "$BRANCH" == "$p" ]] && echo "Blocked: on '$BRANCH'. Create a feature branch." >&2 && exit 2
done
exit 0
```

**auto-format.sh** (PostToolUse: Edit|Write) -- routes to ruff/rustfmt/prettier based on file extension.

**Slash commands** -- `/checkpoint` (safety commit) and `/worktree` (git worktree management), embedded as markdown definitions.

### ClaudeRuntime

Invokes `claude -p` with review prompts, parses JSON findings from output.

```rust
use std::time::Instant;

#[derive(Debug, Clone, Copy)]
pub struct ClaudeRuntime;

impl AgentRuntime for ClaudeRuntime {
    fn agent_type(&self) -> AgentKind { AgentKind::Claude }

    fn invoke(&self, config: &InvocationConfig) -> anyhow::Result<InvocationResult> {
        let start = Instant::now();
        let mut cmd = Command::new("claude");
        cmd.args(["-p", &config.prompt, "--output-format", "text"])
            .current_dir(&config.working_dir)
            .env("CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC", "1");

        if config.bypass_permissions {
            cmd.args(["--permission-mode", "bypassPermissions"]);
        }
        for (k, v) in &config.env { cmd.env(k, v); }

        let output = cmd.output()?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        Ok(InvocationResult {
            output: if stderr.is_empty() { stdout.into() } else { format!("{stdout}\n{stderr}") },
            exit_code: output.status.code().unwrap_or(-1),
            duration: start.elapsed(),
        })
    }

    fn parse_output(&self, result: &InvocationResult) -> ReviewOutput {
        super::parsing::extract_findings(&result.output)
    }

    fn is_available(&self) -> bool {
        Command::new("claude").arg("--version").output().is_ok_and(|o| o.status.success())
    }
}
```

CLI invocation: `claude -p "$PROMPT" --output-format text --permission-mode bypassPermissions`

Environment: `CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC=1`

---

## Codex Adapter

### CodexConfig

Generates only `instructions.md` under `~/.codex/`. Codex CLI has no settings.json, hooks, or slash commands.

```rust
// src/adapters/agents/codex.rs

#[derive(Debug, Clone, Copy)]
pub struct CodexConfig;

impl AgentConfig for CodexConfig {
    fn agent_type(&self) -> AgentKind { AgentKind::Codex }

    fn generate_config(&self, _project_root: &Path) -> anyhow::Result<AgentConfigBundle> {
        Ok(AgentConfigBundle {
            global_instructions: Some(GlobalInstructions {
                relative_path: PathBuf::from(".codex/instructions.md"),
                content: include_str!("templates/codex_instructions.md").to_string(),
            }),
            settings: None,
            hooks: vec![],
            commands: vec![],
        })
    }

    fn is_available(&self) -> bool {
        Command::new("codex").arg("--version").output().is_ok_and(|o| o.status.success())
    }

    fn install_instructions(&self) -> &str { "npm install -g @openai/codex" }
}
```

### CodexRuntime

Same structure as `ClaudeRuntime` but with different CLI flags.

```rust
#[derive(Debug, Clone, Copy)]
pub struct CodexRuntime;

impl AgentRuntime for CodexRuntime {
    fn agent_type(&self) -> AgentKind { AgentKind::Codex }

    fn invoke(&self, config: &InvocationConfig) -> anyhow::Result<InvocationResult> {
        let start = Instant::now();
        let mut cmd = Command::new("codex");
        cmd.args(["--quiet", "--approval-mode", "full-auto", &config.prompt])
            .current_dir(&config.working_dir);
        for (k, v) in &config.env { cmd.env(k, v); }

        let output = cmd.output()?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        Ok(InvocationResult {
            output: if stderr.is_empty() { stdout.into() } else { format!("{stdout}\n{stderr}") },
            exit_code: output.status.code().unwrap_or(-1),
            duration: start.elapsed(),
        })
    }

    fn parse_output(&self, result: &InvocationResult) -> ReviewOutput {
        super::parsing::extract_findings(&result.output)
    }

    fn is_available(&self) -> bool {
        Command::new("codex").arg("--version").output().is_ok_and(|o| o.status.success())
    }
}
```

CLI invocation: `codex --quiet --approval-mode full-auto "$PROMPT"`

---

## Review Prompt Template

Both agents use the same prompt template. The `{focus}` placeholder is substituted with the pass name (security, quality, etc.).

```markdown
You are a {focus}-focused code reviewer. Review this diff for {focus} issues only.

## Diff

{diff}

## Changed files

{changed_files}

## Prior findings from automated analysis

{prior_findings}

## Output format

Return findings as a JSON array. Each object:

- severity: "info", "warn", or "block"
- file: relative path
- line: line number (null if N/A)
- message: short description
- suggestion: fix suggestion (null if none)

Severity mapping:

- info: informational, no action needed
- warn: should fix, not blocking
- block: must fix before shipping

Return ONLY the JSON array. No surrounding text. Empty array if no issues: []
```

The prompt uses noslop's 3-level severity (info/warn/block) directly. Users can override templates per-pass by placing files in `.noslop/prompts/{focus}.md`.

---

## Output Parsing

Shared module that extracts `Vec<Finding>` from raw agent text output. Both adapters delegate to `parsing::extract_findings`.

### RawAgentFinding

Intermediate type mirroring what agents actually output:

```rust
// src/adapters/agents/parsing.rs

use serde::Deserialize;
use crate::core::models::*;
use crate::core::ports::agent::ReviewOutput;

#[derive(Debug, Deserialize)]
struct RawAgentFinding {
    severity: String,
    file: String,
    line: Option<u32>,
    message: String,
    suggestion: Option<String>,
}

impl RawAgentFinding {
    fn into_finding(self, source: FindingSource) -> Finding {
        Finding {
            id: String::new(),
            target: Target {
                path: self.file,
                span: self.line.map(|l| Span { start: l, end: l }),
                commit: None,
            },
            severity: parse_severity(&self.severity),
            message: self.message,
            source,
            status: FindingStatus::Open,
            suggestion: self.suggestion,
            created_at: String::new(),
        }
    }
}
```

### Severity Mapping

Accepts noslop's 3-level terms and common 4-level alternatives:

```rust
fn parse_severity(s: &str) -> Severity {
    match s.to_lowercase().as_str() {
        "info" | "informational" | "note" => Severity::Info,
        "warn" | "warning" | "suggestion" => Severity::Warn,
        "block" | "error" | "critical" | "high" => Severity::Block,
        _ => Severity::Warn,
    }
}
```

### JSON Extraction

```rust
/// Extract findings from raw agent output.
pub fn extract_findings(output: &str, source: FindingSource) -> ReviewOutput {
    let findings = extract_json_array(output)
        .and_then(|json| serde_json::from_str::<Vec<RawAgentFinding>>(&json).ok())
        .map(|raw| raw.into_iter().map(|r| r.into_finding(source.clone())).collect())
        .unwrap_or_default();

    ReviewOutput {
        findings,
        raw_output: output.to_string(),
        errors: extract_error_lines(output),
    }
}

/// Find the outermost JSON array in text, handling nested arrays and strings.
fn extract_json_array(output: &str) -> Option<String> {
    let start = output.find('[')?;
    let mut depth = 0i32;
    let mut in_string = false;
    let mut escape_next = false;

    for (i, &b) in output.as_bytes()[start..].iter().enumerate() {
        if escape_next { escape_next = false; continue; }
        match b {
            b'\\' if in_string => escape_next = true,
            b'"' => in_string = !in_string,
            b'[' if !in_string => depth += 1,
            b']' if !in_string => {
                depth -= 1;
                if depth == 0 { return Some(output[start..start + i + 1].to_string()); }
            }
            _ => {}
        }
    }
    None
}

fn extract_error_lines(output: &str) -> Vec<String> {
    output.lines().map(str::trim)
        .filter(|l| l.starts_with("Error:") || l.starts_with("error:")
            || l.starts_with("FAILED") || l.contains("panicked at"))
        .map(String::from).collect()
}
```

### Parsing Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn src() -> FindingSource { FindingSource::Agent("security".into()) }

    #[test]
    fn clean_json() {
        let out = r#"[{"severity":"warn","file":"src/auth.rs","line":42,"message":"Timing attack","suggestion":"Use constant-time comparison"}]"#;
        let r = extract_findings(out, src());
        assert_eq!(r.findings.len(), 1);
        assert_eq!(r.findings[0].severity, Severity::Warn);
        assert_eq!(r.findings[0].target.path, "src/auth.rs");
        assert_eq!(r.findings[0].target.span, Some(Span { start: 42, end: 42 }));
    }

    #[test]
    fn narrative_wrapper() {
        let out = "Here are findings:\n\n[{\"severity\":\"block\",\"file\":\"src/db.rs\",\"line\":15,\"message\":\"SQL injection\",\"suggestion\":\"Parameterized queries\"}]\n\nClean otherwise.";
        let r = extract_findings(out, src());
        assert_eq!(r.findings.len(), 1);
        assert_eq!(r.findings[0].severity, Severity::Block);
    }

    #[test]
    fn four_level_severity_mapping() {
        let cases = [("error", Severity::Block), ("critical", Severity::Block), ("warning", Severity::Warn)];
        for (sev, expected) in cases {
            let out = format!(r#"[{{"severity":"{sev}","file":"f.rs","line":1,"message":"x"}}]"#);
            assert_eq!(extract_findings(&out, src()).findings[0].severity, expected);
        }
    }

    #[test]
    fn no_json_returns_empty() {
        let r = extract_findings("No issues found.", src());
        assert!(r.findings.is_empty());
    }

    #[test]
    fn malformed_json_returns_empty() {
        assert!(extract_findings("[{bad json}]", src()).findings.is_empty());
    }

    #[test]
    fn error_line_extraction() {
        let r = extract_findings("ok\nError: timeout\nFAILED to connect\ndone", src());
        assert_eq!(r.errors.len(), 2);
    }
}
```

---

## Adapter Comparison

| Aspect              | Claude                                       | Codex                                               |
| ------------------- | -------------------------------------------- | --------------------------------------------------- |
| Config dir          | `~/.claude/`                                 | `~/.codex/`                                         |
| Settings file       | `settings.json`                              | None                                                |
| Global instructions | `CLAUDE.md`                                  | `instructions.md`                                   |
| Hooks               | branch-protection, auto-format               | None                                                |
| Slash commands      | /checkpoint, /worktree                       | None                                                |
| CLI invocation      | `claude -p "$PROMPT" --output-format text`   | `codex --quiet --approval-mode full-auto "$PROMPT"` |
| Permission bypass   | `--permission-mode bypassPermissions`        | `--approval-mode full-auto`                         |
| Special env vars    | `CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC=1` | None                                                |
| Install             | `npm install -g @anthropic-ai/claude-code`   | `npm install -g @openai/codex`                      |
| Output parsing      | Shared `parsing::extract_findings()`         | Same shared parser                                  |

---

## Integration Points

### With noslop init

```rust
fn init_agent(kind: AgentKind, project_root: &Path, home: &Path) -> anyhow::Result<()> {
    let config = get_agent_config(kind);
    config.validate()?;
    let bundle = config.generate_config(project_root)?;
    write_config_bundle(home, &bundle)?;
    Ok(())
}
```

### With noslop review (via AgentAnalyzer)

```rust
fn agent_review(
    runtime: &dyn AgentRuntime,
    diff: &str,
    prior_findings: &[Finding],
    focus: &str,
) -> anyhow::Result<Vec<Finding>> {
    let prompt = build_review_prompt(diff, prior_findings, focus);
    let config = InvocationConfig {
        prompt,
        timeout_secs: Some(120),
        bypass_permissions: true,
        ..Default::default()
    };

    let result = runtime.invoke(&config)?;
    if result.exit_code != 0 {
        anyhow::bail!("{:?} exited with code {}", runtime.agent_type(), result.exit_code);
    }

    let output = runtime.parse_output(&result);
    if !output.errors.is_empty() {
        log::warn!("Agent {:?} errors: {:?}", runtime.agent_type(), output.errors);
    }
    Ok(output.findings)
}
```

### With .noslop.toml

```toml
[agent]
type = "claude"

[review]
analyzers = ["conventions", "formatting", "co-change", "agent:security", "agent:quality"]

[review."agent:security"]
agent = "claude"
prompt_template = ".noslop/prompts/security.md"
```

---

## Extensibility

Adding a new agent (e.g., Cursor, Aider):

1. Add variant to `AgentKind` enum in `core::models`
2. Implement `AgentConfig` -- what files to generate
3. Implement `AgentRuntime` -- how to invoke the CLI
4. Register in factory functions
5. No changes to pipeline, parsers, or existing adapters

---

## Design Decisions

**Two traits, not one.** Config and Runtime have different lifecycles (init-time vs review-time), different dependencies (filesystem vs process execution), and are tested separately.

**Shared output parser.** Both agents output the same JSON format. The parser lives in a shared module. Individual adapters can override `parse_output` if needed.

**3-level severity in prompts.** Prompts ask agents to use noslop's native terms (info/warn/block). The parser also accepts 4-level terms (warning/error/critical) as fallback.

**`RawAgentFinding` as intermediate type.** Deserializes into the agent's flat schema, then converts to the domain `Finding` with `Target`, `FindingSource`, `FindingStatus`. Keeps the domain model clean.

**`AgentConfigBundle` as data, not side effects.** `generate_config` returns data. Writing to disk is the init command's job. Pure and testable.

**`include_str!` for templates.** Hook scripts, instructions, and command definitions are embedded at compile time. No runtime file resolution for defaults.
