# Agent Adapter Implementations

Design specification for the Claude and Codex adapter implementations of the `AgentConfig` and `AgentRuntime` port traits defined in [03-agent-port-design.md](./03-agent-port-design.md).

## Overview

Each agent adapter lives in `src/adapters/agent/` and implements both port traits from `src/core/ports/agent.rs`:

- `AgentConfig` — generates configuration files during `noslop init <agent>`
- `AgentRuntime` — invokes the agent during `noslop run`

```
src/adapters/agent/
├── mod.rs           # Module docs, re-exports, factory functions
├── claude.rs        # ClaudeConfig + ClaudeRuntime
├── codex.rs         # CodexConfig + CodexRuntime
└── templates/       # Embedded template content (compile-time via include_str!)
    ├── claude_global_instructions.md
    ├── claude_branch_protection.sh
    ├── claude_auto_format.sh
    ├── claude_cmd_checkpoint.md
    ├── claude_cmd_worktree.md
    └── codex_instructions.md
```

Registration in `src/adapters/mod.rs`:

```rust
pub mod agent;

pub use agent::{ClaudeConfig, ClaudeRuntime, CodexConfig, CodexRuntime};
```

## Module Root (`mod.rs`)

```rust
//! Agent adapter implementations
//!
//! Provides concrete implementations of the [`AgentConfig`] and [`AgentRuntime`]
//! port traits for supported AI agents.
//!
//! - [`claude`] - Anthropic Claude Code adapter
//! - [`codex`] - OpenAI Codex CLI adapter

pub mod claude;
pub mod codex;

pub use claude::{ClaudeConfig, ClaudeRuntime};
pub use codex::{CodexConfig, CodexRuntime};

use crate::core::ports::agent::{AgentConfig, AgentRuntime, AgentType};

/// Create an `AgentConfig` implementation for the given agent type
#[must_use]
pub fn get_agent_config(agent_type: AgentType) -> Box<dyn AgentConfig> {
    match agent_type {
        AgentType::Claude => Box::new(ClaudeConfig::new()),
        AgentType::Codex => Box::new(CodexConfig::new()),
    }
}

/// Create an `AgentRuntime` implementation for the given agent type
#[must_use]
pub fn get_agent_runtime(agent_type: AgentType) -> Box<dyn AgentRuntime> {
    match agent_type {
        AgentType::Claude => Box::new(ClaudeRuntime::new()),
        AgentType::Codex => Box::new(CodexRuntime::new()),
    }
}
```

---

## Claude Adapter

### File Layout

`noslop init claude` generates these files:

| File                   | Location                               | Purpose                                            |
| ---------------------- | -------------------------------------- | -------------------------------------------------- |
| `settings.json`        | `~/.claude/settings.json`              | Permissions, hooks, sandbox, model settings        |
| `CLAUDE.md`            | `~/.claude/CLAUDE.md`                  | Global agent instructions                          |
| `branch-protection.sh` | `~/.claude/hooks/branch-protection.sh` | PreToolUse hook: block edits on protected branches |
| `auto-format.sh`       | `~/.claude/hooks/auto-format.sh`       | PostToolUse hook: auto-format after edits          |
| `checkpoint.md`        | `~/.claude/commands/checkpoint.md`     | `/checkpoint` slash command definition             |
| `worktree.md`          | `~/.claude/commands/worktree.md`       | `/worktree` slash command definition               |

All paths are relative to `$HOME`. The adapter resolves `~` to the actual home directory at generation time.

### Generated Content: `settings.json`

Extracted from `setup/settings.json`. The adapter builds this programmatically via `serde_json` so individual sections can be customized per-project.

```json
{
  "$schema": "https://json.schemastore.org/claude-code-settings.json",
  "cleanupPeriodDays": 30,
  "env": {
    "CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS": "1"
  },
  "attribution": {
    "commit": "Co-Authored-By: Claude <noreply@anthropic.com>",
    "pr": "Generated with [Claude Code](https://claude.ai/claude-code)"
  },
  "permissions": {
    "allow": [
      "Read",
      "Edit",
      "Write",
      "Glob",
      "Grep",
      "WebFetch",
      "WebSearch",
      "Bash(git status *)",
      "Bash(git diff *)",
      "Bash(git log *)",
      "Bash(git branch *)",
      "Bash(git show *)",
      "Bash(git remote *)",
      "Bash(git rev-parse *)",
      "Bash(git add *)",
      "Bash(git commit *)",
      "Bash(git checkout *)",
      "Bash(git switch *)",
      "Bash(git stash *)",
      "Bash(git merge *)",
      "Bash(git rebase *)",
      "Bash(git fetch *)",
      "Bash(git cherry-pick *)",
      "Bash(git tag *)",
      "Bash(npm run *)",
      "Bash(npm install *)",
      "Bash(npm test *)",
      "Bash(npm ci *)",
      "Bash(npm exec *)",
      "Bash(npx *)",
      "Bash(node *)",
      "Bash(poetry run *)",
      "Bash(poetry install *)",
      "Bash(poetry add *)",
      "Bash(poetry remove *)",
      "Bash(poetry lock *)",
      "Bash(poetry show *)",
      "Bash(python -m pytest *)",
      "Bash(python -m *)",
      "Bash(pip install *)",
      "Bash(pip list *)",
      "Bash(docker compose *)",
      "Bash(docker ps *)",
      "Bash(docker logs *)",
      "Bash(docker images *)",
      "Bash(docker inspect *)",
      "Bash(docker exec *)",
      "Bash(pre-commit *)",
      "Bash(gh *)",
      "Bash(ls *)",
      "Bash(pwd)",
      "Bash(which *)",
      "Bash(cat *)",
      "Bash(head *)",
      "Bash(tail *)",
      "Bash(wc *)",
      "Bash(find *)",
      "Bash(tree *)",
      "Bash(mkdir *)",
      "Bash(cp *)",
      "Bash(mv *)",
      "Bash(touch *)",
      "Bash(sort *)",
      "Bash(uniq *)",
      "Bash(grep *)",
      "Bash(rg *)",
      "Bash(sed *)",
      "Bash(awk *)",
      "Bash(jq *)",
      "Bash(curl *)",
      "Bash(echo *)",
      "Bash(printf *)",
      "Bash(date *)",
      "Bash(env *)",
      "Bash(alembic *)",
      "Bash(ruff *)",
      "Bash(mypy *)",
      "Bash(eslint *)",
      "Bash(prettier *)",
      "Bash(tsc *)",
      "Bash(noslop *)",
      "Bash(checkpoint *)",
      "Bash(checkpoint)",
      "Bash(worktree *)"
    ],
    "deny": [
      "Read(./.env)",
      "Read(./.env.*)",
      "Read(./secrets/*.json)",
      "Read(~/.ssh/**)",
      "Read(./ejson-keys/**)",
      "Bash(rm -rf /)",
      "Bash(git push --force *)",
      "Bash(git reset --hard *)",
      "Bash(git clean -f *)",
      "Bash(chmod 777 *)",
      "Bash(> /dev/sda*)",
      "Bash(dd if=*)",
      "Bash(mkfs *)"
    ],
    "ask": [
      "Bash(git push *)",
      "Bash(docker run *)",
      "Bash(docker build *)",
      "Bash(rm -rf *)",
      "Bash(rm -r *)"
    ]
  },
  "model": "opus",
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Edit|Write",
        "hooks": [
          {
            "type": "command",
            "command": "~/.claude/hooks/branch-protection.sh",
            "statusMessage": "Checking branch protection..."
          }
        ]
      }
    ],
    "PostToolUse": [
      {
        "matcher": "Edit|Write",
        "hooks": [
          {
            "type": "command",
            "command": "~/.claude/hooks/auto-format.sh",
            "statusMessage": "Auto-formatting..."
          }
        ]
      },
      {
        "matcher": "Bash",
        "hooks": [
          {
            "type": "command",
            "command": "jq -r '.tool_input.command' >> ~/.claude/command-log.txt",
            "statusMessage": "Logging command..."
          }
        ]
      }
    ],
    "Stop": [
      {
        "hooks": [
          {
            "type": "prompt",
            "prompt": "Review the conversation transcript and determine if Claude has completed all tasks the user requested. Check: 1) Were all requested changes made? 2) Were there any errors that went unresolved? 3) Did Claude skip anything the user asked for? If everything is complete, respond with {\"ok\": true}. If something is incomplete, respond with {\"ok\": false, \"reason\": \"Describe what still needs to be done\"}.",
            "timeout": 30
          }
        ]
      }
    ],
    "Notification": [
      {
        "matcher": "",
        "hooks": [
          {
            "type": "command",
            "command": "notify-send 'Claude Code' 'Claude Code needs your attention' 2>/dev/null || true"
          }
        ]
      }
    ]
  },
  "sandbox": {
    "enabled": true,
    "autoAllowBashIfSandboxed": true,
    "allowUnsandboxedCommands": true,
    "network": {
      "allowedDomains": [
        "github.com",
        "*.github.com",
        "*.githubusercontent.com",
        "*.npmjs.org",
        "registry.npmjs.org",
        "registry.yarnpkg.com",
        "*.pypi.org",
        "pypi.org",
        "files.pythonhosted.org",
        "*.docker.io",
        "*.docker.com",
        "production.cloudflare.docker.com",
        "stackoverflow.com",
        "*.stackoverflow.com",
        "*.stackexchange.com",
        "developer.mozilla.org",
        "docs.python.org",
        "nodejs.org",
        "*.nodejs.org",
        "svelte.dev",
        "kit.svelte.dev",
        "tailwindcss.com",
        "fastapi.tiangolo.com",
        "docs.pydantic.dev",
        "sqlmodel.tiangolo.com",
        "redis.io",
        "www.postgresql.org",
        "docs.celeryq.dev",
        "jinja.palletsprojects.com",
        "docs.anthropic.com",
        "api.anthropic.com",
        "code.claude.com",
        "caddyserver.com",
        "eslint.org",
        "prettier.io",
        "vitejs.dev",
        "typescriptlang.org",
        "*.typescriptlang.org",
        "zod.dev",
        "json-schema.org",
        "*.google.com",
        "*.googleapis.com"
      ],
      "allowAllUnixSockets": false,
      "allowLocalBinding": false
    },
    "excludedCommands": ["docker", "docker-compose"]
  },
  "spinnerTipsEnabled": false,
  "alwaysThinkingEnabled": true,
  "autoUpdatesChannel": "stable",
  "teammateMode": "auto",
  "showTurnDuration": true
}
```

**Key differences from raw setup**: The `"Bash(ralph *)"` permission is replaced with `"Bash(noslop *)"` since the iteration loop is now built into noslop itself.

### Generated Content: `CLAUDE.md`

Extracted from `setup/CLAUDE.global.md`, adapted to reference noslop commands.

```markdown
# Global Instructions

## Available Tools

The following commands are available in PATH and auto-allowed in permissions.
Use them proactively when appropriate.

### `noslop`

Code review checks and autonomous iteration. See `noslop --help` for all commands.

### `checkpoint`

Safety commit of all current changes. Use before risky operations, major
refactors, or between significant steps.

    checkpoint                    # Auto-timestamp message
    checkpoint "before refactor"  # Custom message

### `worktree`

Git worktree management for isolated work. Auto-detects single-repo vs
multi-repo workspace.

    worktree create <branch> [base]        # Create isolated worktree
    worktree list                          # List active worktrees
    worktree remove <branch>               # Remove worktree
    worktree clean                         # Remove all worktrees

## Conventions

- Use git flow: work on feature branches, never commit directly to
  main/master/development/production
- Run `checkpoint` before destructive or risky operations
- Prefer `worktree create` over branching in-place for large parallel tasks
- Never use "enhanced", "extended", "improved", or similar generic prefixes
  in names
- Type hints required for all Python functions
- TypeScript strict mode for all frontend code
```

### Generated Content: `branch-protection.sh`

Extracted from `setup/hooks/branch-protection.sh`.

```bash
#!/bin/bash
# Blocks file edits when on protected branches (git flow).
# Used as a PreToolUse hook for Edit|Write tools.

BRANCH=$(git rev-parse --abbrev-ref HEAD 2>/dev/null)

PROTECTED_BRANCHES=("main" "master" "development" "production")

for protected in "${PROTECTED_BRANCHES[@]}"; do
  if [[ "$BRANCH" == "$protected" ]]; then
    echo "Blocked: you are on the '$BRANCH' branch. Create a feature branch first." >&2
    exit 2
  fi
done

exit 0
```

### Generated Content: `auto-format.sh`

Extracted from `setup/hooks/auto-format.sh`.

```bash
#!/bin/bash
# Auto-formats files after Claude edits them.
# Used as a PostToolUse hook for Edit|Write tools.
# Routes to the right formatter based on file extension.

INPUT=$(cat)
FILE_PATH=$(echo "$INPUT" | jq -r '.tool_input.file_path // empty')

if [ -z "$FILE_PATH" ] || [ ! -f "$FILE_PATH" ]; then
  exit 0
fi

EXT="${FILE_PATH##*.}"

case "$EXT" in
  py)
    if command -v ruff &>/dev/null; then
      ruff format "$FILE_PATH" 2>/dev/null
      ruff check --fix "$FILE_PATH" 2>/dev/null
    fi
    ;;
  ts|tsx|js|jsx|svelte|css|html|json|md|yaml|yml)
    if command -v npx &>/dev/null; then
      npx prettier --write "$FILE_PATH" 2>/dev/null
    fi
    ;;
  rs)
    if command -v rustfmt &>/dev/null; then
      rustfmt "$FILE_PATH" 2>/dev/null
    fi
    ;;
esac

exit 0
```

### Generated Content: Slash Commands

**`checkpoint.md`** — extracted from `setup/commands/checkpoint.md`:

```markdown
Create a safety checkpoint commit of all current changes.

Run: `checkpoint` (auto-generates timestamp message)
Or: `checkpoint "your message here"` (custom message)

This stages all changes (tracked + untracked) and commits with `--no-verify`.

## When to use checkpoint proactively

Run `checkpoint` BEFORE any of these situations:

- Deleting or moving files
- Large refactors that touch many files
- Changing configuration files
- Reverting or resetting git state
- Running unfamiliar commands that might modify files
- Switching approaches after a failed attempt

Run `checkpoint` BETWEEN:

- Major implementation steps in a multi-step task
- Completing one feature and starting the next
- Making a working change and attempting an optimization

You do NOT need to checkpoint after:

- Read-only operations (grep, find, git log)
- Minor single-file edits that are easy to redo
- Changes already committed via normal git commit
```

**`worktree.md`** — extracted from `setup/commands/worktree.md`:

```markdown
Manage git worktrees for isolated work. Worktrees let you work on a branch
in a separate directory without touching the main checkout.

## Commands

- `worktree create <branch>` -- Create worktree from HEAD
- `worktree create <branch> <base>` -- Create from a specific base branch
- `worktree list` -- List all active worktrees
- `worktree remove <branch>` -- Remove worktree (keeps branch if unmerged)
- `worktree clean` -- Remove all worktrees

## When to suggest worktrees

- User is about to start a large, risky, or experimental change
- User wants to work on a feature in parallel with their current work
- User needs to review or test a branch without losing their current state
```

### `ClaudeConfig` Struct and Implementation

```rust
//! Claude Code adapter
//!
//! Implements [`AgentConfig`] and [`AgentRuntime`] for Anthropic's Claude Code CLI.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use crate::core::ports::agent::{
    AgentConfig, AgentConfigBundle, AgentRuntime, AgentType, GlobalInstructions,
    HookDefinition, InvocationConfig, InvocationResult, IterationOutput,
    SettingsFile, SlashCommand,
};

/// Claude Code configuration generator
///
/// Generates `settings.json`, `CLAUDE.md`, hooks, and slash commands
/// for the `~/.claude/` directory.
#[derive(Debug, Clone, Copy)]
pub struct ClaudeConfig;

impl ClaudeConfig {
    /// Create a new Claude configuration generator
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Build the settings.json content
    fn build_settings(&self) -> anyhow::Result<String> {
        let settings = serde_json::json!({
            "$schema": "https://json.schemastore.org/claude-code-settings.json",
            "cleanupPeriodDays": 30,
            "env": {
                "CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS": "1"
            },
            "attribution": {
                "commit": "Co-Authored-By: Claude <noreply@anthropic.com>",
                "pr": "Generated with [Claude Code](https://claude.ai/claude-code)"
            },
            "permissions": self.build_permissions(),
            "model": "opus",
            "hooks": self.build_hooks(),
            "sandbox": self.build_sandbox(),
            "spinnerTipsEnabled": false,
            "alwaysThinkingEnabled": true,
            "autoUpdatesChannel": "stable",
            "teammateMode": "auto",
            "showTurnDuration": true
        });

        serde_json::to_string_pretty(&settings).map_err(Into::into)
    }

    /// Build the permissions section
    fn build_permissions(&self) -> serde_json::Value {
        serde_json::json!({
            "allow": Self::allow_rules(),
            "deny": Self::deny_rules(),
            "ask": Self::ask_rules(),
        })
    }

    /// Permission allow rules
    ///
    /// Core tools, git operations, language toolchains, system utilities,
    /// and noslop-specific commands.
    fn allow_rules() -> Vec<&'static str> {
        vec![
            // Core Claude Code tools
            "Read", "Edit", "Write", "Glob", "Grep", "WebFetch", "WebSearch",
            // Git operations
            "Bash(git status *)", "Bash(git diff *)", "Bash(git log *)",
            "Bash(git branch *)", "Bash(git show *)", "Bash(git remote *)",
            "Bash(git rev-parse *)", "Bash(git add *)", "Bash(git commit *)",
            "Bash(git checkout *)", "Bash(git switch *)", "Bash(git stash *)",
            "Bash(git merge *)", "Bash(git rebase *)", "Bash(git fetch *)",
            "Bash(git cherry-pick *)", "Bash(git tag *)",
            // Node.js
            "Bash(npm run *)", "Bash(npm install *)", "Bash(npm test *)",
            "Bash(npm ci *)", "Bash(npm exec *)", "Bash(npx *)", "Bash(node *)",
            // Python
            "Bash(poetry run *)", "Bash(poetry install *)", "Bash(poetry add *)",
            "Bash(poetry remove *)", "Bash(poetry lock *)", "Bash(poetry show *)",
            "Bash(python -m pytest *)", "Bash(python -m *)",
            "Bash(pip install *)", "Bash(pip list *)",
            // Docker (read-only)
            "Bash(docker compose *)", "Bash(docker ps *)", "Bash(docker logs *)",
            "Bash(docker images *)", "Bash(docker inspect *)", "Bash(docker exec *)",
            // Development tools
            "Bash(pre-commit *)", "Bash(gh *)",
            "Bash(alembic *)", "Bash(ruff *)", "Bash(mypy *)",
            "Bash(eslint *)", "Bash(prettier *)", "Bash(tsc *)",
            // System utilities
            "Bash(ls *)", "Bash(pwd)", "Bash(which *)", "Bash(cat *)",
            "Bash(head *)", "Bash(tail *)", "Bash(wc *)", "Bash(find *)",
            "Bash(tree *)", "Bash(mkdir *)", "Bash(cp *)", "Bash(mv *)",
            "Bash(touch *)", "Bash(sort *)", "Bash(uniq *)", "Bash(grep *)",
            "Bash(rg *)", "Bash(sed *)", "Bash(awk *)", "Bash(jq *)",
            "Bash(curl *)", "Bash(echo *)", "Bash(printf *)",
            "Bash(date *)", "Bash(env *)",
            // Noslop tools
            "Bash(noslop *)", "Bash(checkpoint *)", "Bash(checkpoint)",
            "Bash(worktree *)",
        ]
    }

    /// Permission deny rules
    ///
    /// Blocks reading secrets and running destructive commands.
    fn deny_rules() -> Vec<&'static str> {
        vec![
            "Read(./.env)", "Read(./.env.*)", "Read(./secrets/*.json)",
            "Read(~/.ssh/**)", "Read(./ejson-keys/**)",
            "Bash(rm -rf /)", "Bash(git push --force *)",
            "Bash(git reset --hard *)", "Bash(git clean -f *)",
            "Bash(chmod 777 *)", "Bash(> /dev/sda*)",
            "Bash(dd if=*)", "Bash(mkfs *)",
        ]
    }

    /// Permission ask rules
    ///
    /// Prompts the user before potentially destructive operations.
    fn ask_rules() -> Vec<&'static str> {
        vec![
            "Bash(git push *)", "Bash(docker run *)",
            "Bash(docker build *)", "Bash(rm -rf *)", "Bash(rm -r *)",
        ]
    }

    /// Build the hooks configuration section
    fn build_hooks(&self) -> serde_json::Value {
        serde_json::json!({
            "PreToolUse": [{
                "matcher": "Edit|Write",
                "hooks": [{
                    "type": "command",
                    "command": "~/.claude/hooks/branch-protection.sh",
                    "statusMessage": "Checking branch protection..."
                }]
            }],
            "PostToolUse": [
                {
                    "matcher": "Edit|Write",
                    "hooks": [{
                        "type": "command",
                        "command": "~/.claude/hooks/auto-format.sh",
                        "statusMessage": "Auto-formatting..."
                    }]
                },
                {
                    "matcher": "Bash",
                    "hooks": [{
                        "type": "command",
                        "command": "jq -r '.tool_input.command' >> ~/.claude/command-log.txt",
                        "statusMessage": "Logging command..."
                    }]
                }
            ],
            "Stop": [{
                "hooks": [{
                    "type": "prompt",
                    "prompt": "Review the conversation transcript and determine if Claude has completed all tasks the user requested. Check: 1) Were all requested changes made? 2) Were there any errors that went unresolved? 3) Did Claude skip anything the user asked for? If everything is complete, respond with {\"ok\": true}. If something is incomplete, respond with {\"ok\": false, \"reason\": \"Describe what still needs to be done\"}.",
                    "timeout": 30
                }]
            }],
            "Notification": [{
                "matcher": "",
                "hooks": [{
                    "type": "command",
                    "command": "notify-send 'Claude Code' 'Claude Code needs your attention' 2>/dev/null || true"
                }]
            }]
        })
    }

    /// Build the sandbox configuration section
    fn build_sandbox(&self) -> serde_json::Value {
        serde_json::json!({
            "enabled": true,
            "autoAllowBashIfSandboxed": true,
            "allowUnsandboxedCommands": true,
            "network": {
                "allowedDomains": [
                    "github.com", "*.github.com", "*.githubusercontent.com",
                    "*.npmjs.org", "registry.npmjs.org", "registry.yarnpkg.com",
                    "*.pypi.org", "pypi.org", "files.pythonhosted.org",
                    "*.docker.io", "*.docker.com", "production.cloudflare.docker.com",
                    "stackoverflow.com", "*.stackoverflow.com", "*.stackexchange.com",
                    "developer.mozilla.org", "docs.python.org",
                    "nodejs.org", "*.nodejs.org",
                    "svelte.dev", "kit.svelte.dev", "tailwindcss.com",
                    "fastapi.tiangolo.com", "docs.pydantic.dev", "sqlmodel.tiangolo.com",
                    "redis.io", "www.postgresql.org", "docs.celeryq.dev",
                    "jinja.palletsprojects.com", "docs.anthropic.com", "api.anthropic.com",
                    "code.claude.com", "caddyserver.com", "eslint.org", "prettier.io",
                    "vitejs.dev", "typescriptlang.org", "*.typescriptlang.org",
                    "zod.dev", "json-schema.org", "*.google.com", "*.googleapis.com"
                ],
                "allowAllUnixSockets": false,
                "allowLocalBinding": false
            },
            "excludedCommands": ["docker", "docker-compose"]
        })
    }
}

impl Default for ClaudeConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentConfig for ClaudeConfig {
    fn agent_type(&self) -> AgentType {
        AgentType::Claude
    }

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
                    event: "PreToolUse".to_string(),
                    matcher: "Edit|Write".to_string(),
                    command: include_str!("templates/claude_branch_protection.sh").to_string(),
                },
                HookDefinition {
                    event: "PostToolUse".to_string(),
                    matcher: "Edit|Write".to_string(),
                    command: include_str!("templates/claude_auto_format.sh").to_string(),
                },
            ],
            commands: vec![
                SlashCommand {
                    name: "checkpoint".to_string(),
                    relative_path: PathBuf::from("commands/checkpoint.md"),
                    content: include_str!("templates/claude_cmd_checkpoint.md").to_string(),
                },
                SlashCommand {
                    name: "worktree".to_string(),
                    relative_path: PathBuf::from("commands/worktree.md"),
                    content: include_str!("templates/claude_cmd_worktree.md").to_string(),
                },
            ],
        })
    }

    fn is_available(&self) -> bool {
        Command::new("claude")
            .arg("--version")
            .output()
            .is_ok_and(|o| o.status.success())
    }

    fn install_instructions(&self) -> &str {
        "npm install -g @anthropic-ai/claude-code"
    }

    fn generate_project_instructions(
        &self,
        project_root: &Path,
    ) -> anyhow::Result<Option<String>> {
        // Check if a CLAUDE.md already exists at the project root
        let claude_md = project_root.join("CLAUDE.md");
        if claude_md.exists() {
            return Ok(None); // Don't overwrite existing project instructions
        }

        Ok(Some(format!(
            "# Project Instructions\n\
             \n\
             This project uses [noslop](https://github.com/noslop/noslop) for code review checks.\n\
             \n\
             ## Before committing\n\
             \n\
             Run `noslop check` to verify all checks are acknowledged.\n\
             Use `noslop ack <ID> -m \"reason\"` to acknowledge checks.\n\
             \n\
             ## Iteration loop\n\
             \n\
             Use `noslop run <plan.md>` for autonomous task iteration.\n\
             Use `checkpoint` before risky operations.\n"
        )))
    }
}
```

### `ClaudeRuntime` Struct and Implementation

```rust
/// Claude Code runtime for iteration loop invocation
///
/// Invokes `claude -p` with appropriate flags and parses output for
/// completion signals and status lines.
#[derive(Debug, Clone, Copy)]
pub struct ClaudeRuntime;

impl ClaudeRuntime {
    /// Create a new Claude runtime
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl Default for ClaudeRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentRuntime for ClaudeRuntime {
    fn agent_type(&self) -> AgentType {
        AgentType::Claude
    }

    fn invoke(&self, config: &InvocationConfig) -> anyhow::Result<InvocationResult> {
        let (cmd, args) = self.build_command(config);
        let start = Instant::now();

        let mut command = Command::new(&cmd);
        command
            .args(&args)
            .current_dir(&config.working_dir)
            .env("CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC", "1");

        // Set additional environment variables
        for (key, value) in &config.env {
            command.env(key, value);
        }

        let output = command.output()?;
        let duration = start.elapsed();

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let combined = if stderr.is_empty() {
            stdout
        } else {
            format!("{stdout}\n{stderr}")
        };

        let exit_code = output.status.code().unwrap_or(-1);
        let completed = self.is_plan_complete(&combined);
        let status_line = self.extract_status_line(&combined);

        Ok(InvocationResult {
            output: combined,
            exit_code,
            completed,
            status_line,
            duration,
        })
    }

    fn parse_output(&self, result: &InvocationResult) -> IterationOutput {
        let plan_complete = self.is_plan_complete(&result.output);

        // Extract task name and iteration from status line
        let (completed_task, iteration) = result
            .status_line
            .as_ref()
            .and_then(|line| {
                let re = regex::Regex::new(
                    r#"\[RALPH:DONE\s+task="([^"]+)"\s+iteration=(\d+)\]"#
                ).ok()?;
                let caps = re.captures(line)?;
                let task = caps.get(1).map(|m| m.as_str().to_string());
                let iter = caps.get(2).and_then(|m| m.as_str().parse().ok());
                Some((task, iter))
            })
            .unwrap_or((None, None));

        // Extract error patterns from output
        let errors = Self::extract_errors(&result.output);

        IterationOutput {
            plan_complete,
            completed_task,
            iteration,
            raw_output: result.output.clone(),
            errors,
        }
    }

    fn build_command(&self, config: &InvocationConfig) -> (String, Vec<String>) {
        let mut args = vec![
            "-p".to_string(),
            config.prompt.clone(),
            "--output-format".to_string(),
            "text".to_string(),
        ];

        if config.bypass_permissions {
            args.push("--permission-mode".to_string());
            args.push("bypassPermissions".to_string());
        }

        ("claude".to_string(), args)
    }

    fn invocation_env(&self) -> HashMap<String, String> {
        let mut env = HashMap::new();
        env.insert(
            "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC".to_string(),
            "1".to_string(),
        );
        env
    }
}

impl ClaudeRuntime {
    /// Extract error patterns from Claude's output
    ///
    /// Looks for common error indicators in the raw output text.
    fn extract_errors(output: &str) -> Vec<String> {
        let mut errors = Vec::new();

        for line in output.lines() {
            let trimmed = line.trim();
            // Capture lines that look like error messages
            if trimmed.starts_with("Error:")
                || trimmed.starts_with("error:")
                || trimmed.starts_with("FAILED")
                || trimmed.contains("panicked at")
                || trimmed.contains("compilation error")
            {
                errors.push(trimmed.to_string());
            }
        }

        errors
    }
}
```

### Claude Invocation Flow

```
noslop run plan.md
    │
    ▼
┌─────────────────────────────────────────────┐
│ ClaudeRuntime::build_command()              │
│                                             │
│ claude -p "$PROMPT"                         │
│        --output-format text                 │
│        --permission-mode bypassPermissions  │
│                                             │
│ Environment:                                │
│   CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC=1│
│   (working_dir set via .current_dir())      │
└─────────────────┬───────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────┐
│ ClaudeRuntime::invoke()                     │
│                                             │
│ 1. Build Command with args + env            │
│ 2. Execute via std::process::Command        │
│ 3. Capture stdout + stderr                  │
│ 4. Record duration                          │
│ 5. Check exit code                          │
│ 6. Return InvocationResult                  │
└─────────────────┬───────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────┐
│ ClaudeRuntime::parse_output()               │
│                                             │
│ 1. Check for <promise>COMPLETE</promise>    │
│ 2. Extract [RALPH:DONE task="X" iteration=N]│
│ 3. Scan for error patterns                  │
│ 4. Return IterationOutput                   │
└─────────────────────────────────────────────┘
```

### Output Parsing Details

Claude's raw text output is scanned for two structured signals:

1. **Plan completion**: The literal string `<promise>COMPLETE</promise>` anywhere in the output signals that all tasks in the plan are done. This is a convention from the ralph iteration loop prompt template, not a Claude API feature.

2. **Task status line**: The pattern `[RALPH:DONE task="<description>" iteration=<N>]` is emitted by the agent when it finishes a single task. Parsed with:

   ```rust
   r#"\[RALPH:DONE\s+task="([^"]+)"\s+iteration=(\d+)\]"#
   ```

3. **Error extraction**: Lines matching error prefixes (`Error:`, `FAILED`, `panicked at`, `compilation error`) are collected into `IterationOutput.errors` for stuck detection and logging.

### Claude Agent SDK Invocation (Future)

The current design uses `claude -p` (headless CLI mode). A future version will use the Claude Agent SDK for structured communication:

```rust
// Future: SDK-based invocation (not implemented in v1)
// let session = claude_sdk::Session::new()?;
// let result = session.query(&prompt)
//     .max_turns(50)
//     .max_budget_usd(5.0)
//     .execute()
//     .await?;
```

The `AgentRuntime` trait is designed to accommodate this transition: `invoke()` returns the same `InvocationResult` regardless of whether the underlying implementation uses CLI or SDK.

---

## Codex Adapter

### File Layout

`noslop init codex` generates these files:

| File              | Location                   | Purpose                   |
| ----------------- | -------------------------- | ------------------------- |
| `instructions.md` | `~/.codex/instructions.md` | Global agent instructions |

Codex CLI has a simpler configuration model than Claude Code. It reads instructions from `~/.codex/instructions.md` and does not have a settings.json equivalent, a hook system, or slash commands.

### Generated Content: `instructions.md`

```markdown
# Global Instructions

## Available Tools

### `noslop`

Code review checks and autonomous iteration. See `noslop --help` for all commands.

### `checkpoint`

Safety commit of all current changes. Use before risky operations.

    checkpoint                    # Auto-timestamp message
    checkpoint "before refactor"  # Custom message

### `worktree`

Git worktree management for isolated work.

    worktree create <branch> [base]
    worktree list
    worktree remove <branch>
    worktree clean

## Conventions

- Use git flow: work on feature branches, never commit directly to
  main/master/development/production
- Run `checkpoint` before destructive or risky operations
- Prefer `worktree create` over branching in-place for large parallel tasks
- Never use "enhanced", "extended", "improved", or similar generic prefixes
  in names
- Type hints required for all Python functions
- TypeScript strict mode for all frontend code
```

### `CodexConfig` Struct and Implementation

```rust
//! Codex CLI adapter
//!
//! Implements [`AgentConfig`] and [`AgentRuntime`] for OpenAI's Codex CLI.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use crate::core::ports::agent::{
    AgentConfig, AgentConfigBundle, AgentRuntime, AgentType, GlobalInstructions,
    InvocationConfig, InvocationResult, IterationOutput,
};

/// Codex CLI configuration generator
///
/// Generates `instructions.md` for the `~/.codex/` directory.
/// Codex has a simpler config model than Claude: no settings.json, hooks,
/// or slash commands.
#[derive(Debug, Clone, Copy)]
pub struct CodexConfig;

impl CodexConfig {
    /// Create a new Codex configuration generator
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl Default for CodexConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentConfig for CodexConfig {
    fn agent_type(&self) -> AgentType {
        AgentType::Codex
    }

    fn generate_config(&self, _project_root: &Path) -> anyhow::Result<AgentConfigBundle> {
        Ok(AgentConfigBundle {
            global_instructions: Some(GlobalInstructions {
                relative_path: PathBuf::from(".codex/instructions.md"),
                content: include_str!("templates/codex_instructions.md").to_string(),
            }),
            settings: None,  // Codex has no settings.json equivalent
            hooks: vec![],   // Codex has no hook system
            commands: vec![], // Codex has no slash commands
        })
    }

    fn is_available(&self) -> bool {
        Command::new("codex")
            .arg("--version")
            .output()
            .is_ok_and(|o| o.status.success())
    }

    fn install_instructions(&self) -> &str {
        "npm install -g @openai/codex"
    }

    fn generate_project_instructions(
        &self,
        project_root: &Path,
    ) -> anyhow::Result<Option<String>> {
        // Check if a AGENTS.md already exists (Codex convention)
        let agents_md = project_root.join("AGENTS.md");
        if agents_md.exists() {
            return Ok(None);
        }

        Ok(Some(format!(
            "# Agent Instructions\n\
             \n\
             This project uses [noslop](https://github.com/noslop/noslop) for code review checks.\n\
             \n\
             ## Before committing\n\
             \n\
             Run `noslop check` to verify all checks are acknowledged.\n\
             Use `noslop ack <ID> -m \"reason\"` to acknowledge checks.\n\
             \n\
             ## Iteration loop\n\
             \n\
             Use `noslop run <plan.md>` for autonomous task iteration.\n\
             Use `checkpoint` before risky operations.\n"
        )))
    }
}
```

### `CodexRuntime` Struct and Implementation

```rust
/// Codex CLI runtime for iteration loop invocation
///
/// Invokes `codex` with quiet/full-auto mode and parses output
/// for completion signals and status lines.
#[derive(Debug, Clone, Copy)]
pub struct CodexRuntime;

impl CodexRuntime {
    /// Create a new Codex runtime
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl Default for CodexRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentRuntime for CodexRuntime {
    fn agent_type(&self) -> AgentType {
        AgentType::Codex
    }

    fn invoke(&self, config: &InvocationConfig) -> anyhow::Result<InvocationResult> {
        let (cmd, args) = self.build_command(config);
        let start = Instant::now();

        let mut command = Command::new(&cmd);
        command
            .args(&args)
            .current_dir(&config.working_dir);

        for (key, value) in &config.env {
            command.env(key, value);
        }

        let output = command.output()?;
        let duration = start.elapsed();

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let combined = if stderr.is_empty() {
            stdout
        } else {
            format!("{stdout}\n{stderr}")
        };

        let exit_code = output.status.code().unwrap_or(-1);
        let completed = self.is_plan_complete(&combined);
        let status_line = self.extract_status_line(&combined);

        Ok(InvocationResult {
            output: combined,
            exit_code,
            completed,
            status_line,
            duration,
        })
    }

    fn parse_output(&self, result: &InvocationResult) -> IterationOutput {
        let plan_complete = self.is_plan_complete(&result.output);

        let (completed_task, iteration) = result
            .status_line
            .as_ref()
            .and_then(|line| {
                let re = regex::Regex::new(
                    r#"\[RALPH:DONE\s+task="([^"]+)"\s+iteration=(\d+)\]"#
                ).ok()?;
                let caps = re.captures(line)?;
                let task = caps.get(1).map(|m| m.as_str().to_string());
                let iter = caps.get(2).and_then(|m| m.as_str().parse().ok());
                Some((task, iter))
            })
            .unwrap_or((None, None));

        let errors = Self::extract_errors(&result.output);

        IterationOutput {
            plan_complete,
            completed_task,
            iteration,
            raw_output: result.output.clone(),
            errors,
        }
    }

    fn build_command(&self, config: &InvocationConfig) -> (String, Vec<String>) {
        let mut args = vec![
            "--quiet".to_string(),
            "--approval-mode".to_string(),
            "full-auto".to_string(),
            config.prompt.clone(),
        ];

        // Codex does not have a separate bypass-permissions flag;
        // full-auto mode handles this.

        args
    }

    fn invocation_env(&self) -> HashMap<String, String> {
        HashMap::new()
    }
}

impl CodexRuntime {
    /// Extract error patterns from Codex output
    fn extract_errors(output: &str) -> Vec<String> {
        let mut errors = Vec::new();

        for line in output.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("Error:")
                || trimmed.starts_with("error:")
                || trimmed.starts_with("FAILED")
                || trimmed.contains("panicked at")
                || trimmed.contains("compilation error")
            {
                errors.push(trimmed.to_string());
            }
        }

        errors
    }
}
```

### Codex Invocation Flow

```
noslop run plan.md --agent codex
    │
    ▼
┌──────────────────────────────────────────┐
│ CodexRuntime::build_command()            │
│                                          │
│ codex --quiet                            │
│       --approval-mode full-auto          │
│       "$PROMPT"                          │
│                                          │
│ (no special environment variables)       │
└──────────────────┬───────────────────────┘
                   │
                   ▼
┌──────────────────────────────────────────┐
│ CodexRuntime::invoke()                   │
│                                          │
│ 1. Build Command with args               │
│ 2. Execute via std::process::Command     │
│ 3. Capture stdout + stderr               │
│ 4. Record duration                       │
│ 5. Check exit code                       │
│ 6. Return InvocationResult               │
└──────────────────┬───────────────────────┘
                   │
                   ▼
┌──────────────────────────────────────────┐
│ CodexRuntime::parse_output()             │
│                                          │
│ Same signals as Claude:                  │
│ 1. <promise>COMPLETE</promise>           │
│ 2. [RALPH:DONE task="X" iteration=N]    │
│ 3. Error pattern extraction              │
│                                          │
│ Return IterationOutput                   │
└──────────────────────────────────────────┘
```

### Codex Output Parsing

Codex uses the same completion signals as Claude because these are prompt-level conventions injected by the noslop iteration loop, not agent-specific API features:

- `<promise>COMPLETE</promise>` for plan completion
- `[RALPH:DONE task="..." iteration=N]` for per-task status

The prompt template (owned by `noslop run`, not the adapter) instructs whatever agent is running to emit these markers. The adapter just parses them.

---

## Adapter Comparison

| Aspect                   | Claude                                                                         | Codex                                               |
| ------------------------ | ------------------------------------------------------------------------------ | --------------------------------------------------- |
| **Config directory**     | `~/.claude/`                                                                   | `~/.codex/`                                         |
| **Settings file**        | `settings.json` (permissions, hooks, sandbox, model)                           | None                                                |
| **Global instructions**  | `CLAUDE.md`                                                                    | `instructions.md`                                   |
| **Hook scripts**         | `branch-protection.sh`, `auto-format.sh`                                       | None (no hook system)                               |
| **Slash commands**       | `checkpoint.md`, `worktree.md`                                                 | None (no command system)                            |
| **Project instructions** | `CLAUDE.md` at repo root                                                       | `AGENTS.md` at repo root                            |
| **CLI invocation**       | `claude -p "$PROMPT" --output-format text --permission-mode bypassPermissions` | `codex --quiet --approval-mode full-auto "$PROMPT"` |
| **Permission bypass**    | `--permission-mode bypassPermissions`                                          | `--approval-mode full-auto`                         |
| **Output format**        | `--output-format text`                                                         | Default text output with `--quiet`                  |
| **Special env vars**     | `CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC=1`                                   | None                                                |
| **Completion signal**    | `<promise>COMPLETE</promise>` (prompt convention)                              | Same (prompt convention)                            |
| **Status line**          | `[RALPH:DONE ...]` (prompt convention)                                         | Same (prompt convention)                            |
| **Install command**      | `npm install -g @anthropic-ai/claude-code`                                     | `npm install -g @openai/codex`                      |

---

## Config Writing: Shared Logic

The `AgentConfigBundle` returned by `generate_config()` is written to disk by shared logic in the init command, not by the adapter itself. This ensures consistent behavior across agents (conflict detection, backup, permissions).

```rust
// In cli/commands/init.rs or a shared config_writer module

use std::fs;
use std::path::Path;

use crate::core::ports::agent::AgentConfigBundle;

/// Write an agent configuration bundle to disk
///
/// Creates all files in the bundle under the user's home directory.
/// Backs up existing files before overwriting.
pub fn write_config_bundle(
    home: &Path,
    bundle: &AgentConfigBundle,
    force: bool,
) -> anyhow::Result<Vec<std::path::PathBuf>> {
    let mut written = Vec::new();

    // Write global instructions
    if let Some(ref instructions) = bundle.global_instructions {
        let path = home.join(&instructions.relative_path);
        write_with_backup(&path, &instructions.content, force)?;
        written.push(path);
    }

    // Write settings file
    if let Some(ref settings) = bundle.settings {
        let path = home.join(&settings.relative_path);
        write_with_backup(&path, &settings.content, force)?;
        written.push(path);
    }

    // Write hook scripts
    for hook in &bundle.hooks {
        // Hook commands that are inline scripts don't need separate files.
        // Hook commands that are file paths get written as scripts.
        if hook.command.contains('\n') {
            // Multi-line command: this is a script to write
            let hook_dir = home.join(".claude/hooks"); // or agent-specific path
            fs::create_dir_all(&hook_dir)?;

            let filename = format!(
                "{}-{}.sh",
                hook.event.to_lowercase(),
                hook.matcher.replace('|', "-").to_lowercase()
            );
            let path = hook_dir.join(&filename);
            write_with_backup(&path, &hook.command, force)?;
            make_executable(&path)?;
            written.push(path);
        }
    }

    // Write slash commands
    for cmd in &bundle.commands {
        let path = home.join(".claude").join(&cmd.relative_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        write_with_backup(&path, &cmd.content, force)?;
        written.push(path);
    }

    Ok(written)
}

/// Write content to a file, creating a .bak backup if the file exists
fn write_with_backup(path: &Path, content: &str, force: bool) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    if path.exists() && !force {
        // Create backup
        let backup = path.with_extension("bak");
        fs::copy(path, &backup)?;
        log::info!("Backed up {} to {}", path.display(), backup.display());
    }

    fs::write(path, content)?;
    log::info!("Wrote {}", path.display());
    Ok(())
}

/// Make a file executable on Unix
#[cfg(unix)]
fn make_executable(path: &Path) -> anyhow::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = fs::metadata(path)?.permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms)?;
    Ok(())
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) -> anyhow::Result<()> {
    Ok(()) // No-op on non-Unix
}
```

---

## Error Handling

Both adapters follow noslop's established error patterns:

### Invocation Errors

```rust
// Agent CLI not found
if !runtime.is_available() {
    anyhow::bail!(
        "{} CLI not found. Install: {}",
        runtime.agent_type().display_name(),
        config.install_instructions()
    );
}

// Non-zero exit code (agent crashed or timed out)
if result.exit_code != 0 {
    anyhow::bail!(
        "{} exited with code {} after {:.1}s",
        runtime.agent_type().display_name(),
        result.exit_code,
        result.duration.as_secs_f64()
    );
}

// Empty output (agent produced nothing)
if result.output.trim().is_empty() {
    anyhow::bail!(
        "{} produced no output (exit code {})",
        runtime.agent_type().display_name(),
        result.exit_code
    );
}
```

### Config Generation Errors

```rust
// Home directory not found
let home = dirs::home_dir()
    .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?;

// Settings serialization failure
let content = self.build_settings()
    .context("Failed to serialize Claude settings.json")?;

// Template embedding failure is a compile-time error (include_str! panics)
```

---

## Test Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ports::agent::AgentConfig;

    #[test]
    fn claude_config_generates_all_files() {
        let config = ClaudeConfig::new();
        let bundle = config.generate_config(Path::new("/tmp/test")).unwrap();

        assert!(bundle.global_instructions.is_some());
        assert!(bundle.settings.is_some());
        assert_eq!(bundle.hooks.len(), 2);
        assert_eq!(bundle.commands.len(), 2);
    }

    #[test]
    fn claude_settings_is_valid_json() {
        let config = ClaudeConfig::new();
        let bundle = config.generate_config(Path::new("/tmp/test")).unwrap();
        let settings = bundle.settings.unwrap();

        let parsed: serde_json::Value = serde_json::from_str(&settings.content).unwrap();
        assert_eq!(parsed["model"], "opus");
        assert!(parsed["permissions"]["allow"].is_array());
    }

    #[test]
    fn claude_settings_includes_noslop_permission() {
        let config = ClaudeConfig::new();
        let bundle = config.generate_config(Path::new("/tmp/test")).unwrap();
        let settings = bundle.settings.unwrap();

        let parsed: serde_json::Value = serde_json::from_str(&settings.content).unwrap();
        let allow = parsed["permissions"]["allow"].as_array().unwrap();
        let allow_strings: Vec<&str> = allow.iter().filter_map(|v| v.as_str()).collect();

        assert!(allow_strings.contains(&"Bash(noslop *)"));
        assert!(allow_strings.contains(&"Bash(checkpoint *)"));
    }

    #[test]
    fn claude_hooks_have_correct_events() {
        let config = ClaudeConfig::new();
        let bundle = config.generate_config(Path::new("/tmp/test")).unwrap();

        let events: Vec<&str> = bundle.hooks.iter().map(|h| h.event.as_str()).collect();
        assert!(events.contains(&"PreToolUse"));
        assert!(events.contains(&"PostToolUse"));
    }

    #[test]
    fn codex_config_has_no_hooks_or_commands() {
        let config = CodexConfig::new();
        let bundle = config.generate_config(Path::new("/tmp/test")).unwrap();

        assert!(bundle.global_instructions.is_some());
        assert!(bundle.settings.is_none());
        assert!(bundle.hooks.is_empty());
        assert!(bundle.commands.is_empty());
    }

    #[test]
    fn codex_instructions_path_is_correct() {
        let config = CodexConfig::new();
        let bundle = config.generate_config(Path::new("/tmp/test")).unwrap();
        let instructions = bundle.global_instructions.unwrap();

        assert_eq!(instructions.relative_path, PathBuf::from(".codex/instructions.md"));
    }

    #[test]
    fn claude_runtime_build_command_with_bypass() {
        let runtime = ClaudeRuntime::new();
        let config = InvocationConfig {
            prompt: "do the thing".to_string(),
            bypass_permissions: true,
            ..Default::default()
        };

        let (cmd, args) = runtime.build_command(&config);
        assert_eq!(cmd, "claude");
        assert!(args.contains(&"-p".to_string()));
        assert!(args.contains(&"--output-format".to_string()));
        assert!(args.contains(&"text".to_string()));
        assert!(args.contains(&"--permission-mode".to_string()));
        assert!(args.contains(&"bypassPermissions".to_string()));
    }

    #[test]
    fn claude_runtime_build_command_without_bypass() {
        let runtime = ClaudeRuntime::new();
        let config = InvocationConfig {
            prompt: "do the thing".to_string(),
            bypass_permissions: false,
            ..Default::default()
        };

        let (cmd, args) = runtime.build_command(&config);
        assert_eq!(cmd, "claude");
        assert!(!args.contains(&"--permission-mode".to_string()));
    }

    #[test]
    fn codex_runtime_build_command() {
        let runtime = CodexRuntime::new();
        let config = InvocationConfig {
            prompt: "do the thing".to_string(),
            ..Default::default()
        };

        let (cmd, args) = runtime.build_command(&config);
        assert_eq!(cmd, "codex");
        assert!(args.contains(&"--quiet".to_string()));
        assert!(args.contains(&"--approval-mode".to_string()));
        assert!(args.contains(&"full-auto".to_string()));
    }

    #[test]
    fn parse_output_detects_completion() {
        let runtime = ClaudeRuntime::new();
        let result = InvocationResult {
            output: "did some work\n<promise>COMPLETE</promise>\n".to_string(),
            exit_code: 0,
            completed: true,
            status_line: None,
            duration: std::time::Duration::from_secs(10),
        };

        let output = runtime.parse_output(&result);
        assert!(output.plan_complete);
    }

    #[test]
    fn parse_output_extracts_status_line() {
        let runtime = ClaudeRuntime::new();
        let status = "[RALPH:DONE task=\"implement auth\" iteration=3]".to_string();
        let result = InvocationResult {
            output: format!("some output\n{status}\nmore output"),
            exit_code: 0,
            completed: false,
            status_line: Some(status),
            duration: std::time::Duration::from_secs(10),
        };

        let output = runtime.parse_output(&result);
        assert_eq!(output.completed_task.as_deref(), Some("implement auth"));
        assert_eq!(output.iteration, Some(3));
    }

    #[test]
    fn parse_output_extracts_errors() {
        let runtime = ClaudeRuntime::new();
        let result = InvocationResult {
            output: "working...\nError: file not found\nFAILED test_auth\n".to_string(),
            exit_code: 1,
            completed: false,
            status_line: None,
            duration: std::time::Duration::from_secs(5),
        };

        let output = runtime.parse_output(&result);
        assert_eq!(output.errors.len(), 2);
        assert!(output.errors[0].contains("file not found"));
        assert!(output.errors[1].contains("test_auth"));
    }
}
```

### Integration Tests

```rust
#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    #[ignore] // Run with: cargo test -- --ignored
    fn claude_is_available() {
        let config = ClaudeConfig::new();
        // This test only passes when claude CLI is installed
        if config.is_available() {
            assert!(config.validate().is_ok());
        }
    }

    #[test]
    #[ignore]
    fn codex_is_available() {
        let config = CodexConfig::new();
        if config.is_available() {
            assert!(config.validate().is_ok());
        }
    }

    #[test]
    fn write_config_bundle_to_tempdir() {
        let home = tempfile::tempdir().unwrap();
        let config = ClaudeConfig::new();
        let bundle = config.generate_config(Path::new("/tmp/test")).unwrap();

        let written = write_config_bundle(home.path(), &bundle, false).unwrap();

        // Verify all expected files were created
        assert!(home.path().join(".claude/CLAUDE.md").exists());
        assert!(home.path().join(".claude/settings.json").exists());
        assert!(home.path().join(".claude/commands/checkpoint.md").exists());
        assert!(home.path().join(".claude/commands/worktree.md").exists());
        assert!(written.len() >= 4); // instructions + settings + 2 commands
    }
}
```

---

## Summary

The adapter design provides:

1. **Claude adapter**: Full configuration generation covering settings.json (97 allow rules, deny rules, hooks, sandbox), global instructions, hook scripts (branch protection, auto-format), and slash commands (checkpoint, worktree). Runtime invokes `claude -p` with `--output-format text --permission-mode bypassPermissions`.

2. **Codex adapter**: Minimal configuration (instructions.md only). Runtime invokes `codex --quiet --approval-mode full-auto`. No hooks or slash commands because Codex CLI does not support them.

3. **Shared patterns**: Both adapters parse the same completion signals (`<promise>COMPLETE</promise>`, `[RALPH:DONE ...]`) because these are prompt conventions, not agent-specific features. Config writing uses shared logic for backups, directory creation, and executable permissions.

4. **Extensibility**: Adding a new agent requires implementing `AgentConfig` and `AgentRuntime` with whatever config files and CLI flags that agent needs. No changes to the iteration loop or existing adapters.

Next steps:

- Task 5: Iteration loop design (`noslop run` command)
- Task 6: Checkpoint and worktree Rust ports
- Task 7: Extended `noslop init <agent>` command design
