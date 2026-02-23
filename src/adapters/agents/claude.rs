//! Claude adapter
//!
//! [`ClaudeConfig`] generates configuration for `noslop init claude`.
//! [`ClaudeRuntime`] invokes the `claude` CLI for `noslop review`.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use crate::core::models::AgentKind;
use crate::core::ports::agent::{
    AgentConfig, AgentConfigBundle, AgentRuntime, GlobalInstructions, HookDefinition,
    InvocationConfig, InvocationResult, ReviewOutput, SettingsFile, SlashCommand,
};

/// Claude configuration generator.
///
/// Produces `settings.json`, `CLAUDE.md`, hooks, and slash commands
/// under `~/.claude/`.
#[derive(Debug, Clone, Copy)]
pub struct ClaudeConfig;

impl AgentConfig for ClaudeConfig {
    fn agent_type(&self) -> AgentKind {
        AgentKind::Claude
    }

    fn generate_config(&self, _project_root: &Path) -> anyhow::Result<AgentConfigBundle> {
        Ok(AgentConfigBundle {
            global_instructions: Some(GlobalInstructions {
                relative_path: PathBuf::from(".claude/CLAUDE.md"),
                content: include_str!("templates/claude_global_instructions.md").to_string(),
            }),
            settings: Some(SettingsFile {
                relative_path: PathBuf::from(".claude/settings.json"),
                content: Self::build_settings()?,
            }),
            hooks: vec![
                HookDefinition {
                    event: "PreToolUse".into(),
                    matcher: "Edit|Write".into(),
                    command: Self::branch_protection_script().to_string(),
                },
                HookDefinition {
                    event: "PostToolUse".into(),
                    matcher: "Edit|Write".into(),
                    command: Self::auto_format_script().to_string(),
                },
            ],
            commands: vec![
                SlashCommand {
                    name: "checkpoint".into(),
                    relative_path: PathBuf::from("commands/checkpoint.md"),
                    content: Self::checkpoint_command().to_string(),
                },
                SlashCommand {
                    name: "worktree".into(),
                    relative_path: PathBuf::from("commands/worktree.md"),
                    content: Self::worktree_command().to_string(),
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

    fn install_instructions(&self) -> &'static str {
        "npm install -g @anthropic-ai/claude-code"
    }
}

impl ClaudeConfig {
    /// Build `settings.json` content programmatically.
    fn build_settings() -> anyhow::Result<String> {
        let settings = serde_json::json!({
            "$schema": "https://json.schemastore.org/claude-code-settings.json",
            "permissions": {
                "allow": [
                    "Read", "Edit", "Write", "Glob", "Grep", "WebFetch", "WebSearch",
                    "Bash(git status *)", "Bash(git diff *)", "Bash(git log *)",
                    "Bash(git add *)", "Bash(git commit *)", "Bash(git checkout *)",
                    "Bash(npm run *)", "Bash(npx *)", "Bash(python -m *)",
                    "Bash(ruff *)", "Bash(eslint *)", "Bash(prettier *)",
                    "Bash(ls *)", "Bash(find *)", "Bash(jq *)",
                    "Bash(noslop *)", "Bash(checkpoint *)", "Bash(checkpoint)",
                    "Bash(worktree *)"
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

    /// Branch protection hook script content.
    const fn branch_protection_script() -> &'static str {
        concat!(
            "#!/bin/bash\n",
            "BRANCH=$(git rev-parse --abbrev-ref HEAD 2>/dev/null)\n",
            "PROTECTED=(\"main\" \"master\" \"development\" \"production\")\n",
            "for p in \"${PROTECTED[@]}\"; do\n",
            "  [[ \"$BRANCH\" == \"$p\" ]] && echo \"Blocked: on '$BRANCH'. ",
            "Create a feature branch.\" >&2 && exit 2\n",
            "done\n",
            "exit 0\n",
        )
    }

    /// Auto-format hook script content.
    const fn auto_format_script() -> &'static str {
        concat!(
            "#!/bin/bash\n",
            "FILE=\"$1\"\n",
            "case \"$FILE\" in\n",
            "  *.py) ruff format --quiet \"$FILE\" 2>/dev/null ;;\n",
            "  *.rs) rustfmt --quiet \"$FILE\" 2>/dev/null ;;\n",
            "  *.ts|*.tsx|*.js|*.jsx|*.json|*.md) prettier --write \"$FILE\" 2>/dev/null ;;\n",
            "esac\n",
            "exit 0\n",
        )
    }

    /// Checkpoint slash command content.
    const fn checkpoint_command() -> &'static str {
        concat!(
            "Run `checkpoint` to create a safety commit of all current changes.\n",
            "\n",
            "Usage:\n",
            "- `checkpoint` -- auto-timestamp message\n",
            "- `checkpoint \"before refactor\"` -- custom message\n",
        )
    }

    /// Worktree slash command content.
    const fn worktree_command() -> &'static str {
        concat!(
            "Run `worktree` for git worktree management.\n",
            "\n",
            "Usage:\n",
            "- `worktree create <branch> [base]` -- create isolated worktree\n",
            "- `worktree list` -- list active worktrees\n",
            "- `worktree remove <branch>` -- remove worktree\n",
            "- `worktree clean` -- remove all worktrees\n",
        )
    }
}

/// Claude runtime for review invocation.
///
/// Invokes `claude -p "$PROMPT" --output-format text --permission-mode bypassPermissions`.
#[derive(Debug, Clone, Copy)]
pub struct ClaudeRuntime;

impl AgentRuntime for ClaudeRuntime {
    fn agent_type(&self) -> AgentKind {
        AgentKind::Claude
    }

    fn invoke(&self, config: &InvocationConfig) -> anyhow::Result<InvocationResult> {
        let start = Instant::now();
        let mut cmd = Command::new("claude");
        cmd.args(["-p", &config.prompt, "--output-format", "text"])
            .current_dir(&config.working_dir)
            .env("CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC", "1");

        if config.bypass_permissions {
            cmd.args(["--permission-mode", "bypassPermissions"]);
        }
        for (k, v) in &config.env {
            cmd.env(k, v);
        }

        let output = cmd.output()?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        Ok(InvocationResult {
            output: if stderr.is_empty() {
                stdout.into_owned()
            } else {
                format!("{stdout}\n{stderr}")
            },
            exit_code: output.status.code().unwrap_or(-1),
            duration: start.elapsed(),
        })
    }

    fn parse_output(&self, result: &InvocationResult) -> ReviewOutput {
        super::parsing::extract_feedbacks(
            &result.output,
            &crate::core::models::FeedbackSource::Agent("claude".into()),
        )
    }

    fn is_available(&self) -> bool {
        Command::new("claude")
            .arg("--version")
            .output()
            .is_ok_and(|o| o.status.success())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ports::agent::AgentConfig;

    #[test]
    fn claude_config_agent_type() {
        assert_eq!(ClaudeConfig.agent_type(), AgentKind::Claude);
    }

    #[test]
    fn claude_config_generates_bundle() {
        let bundle = ClaudeConfig.generate_config(Path::new("/tmp/project")).unwrap();

        // Global instructions
        let gi = bundle.global_instructions.as_ref().unwrap();
        assert_eq!(gi.relative_path, PathBuf::from(".claude/CLAUDE.md"));
        assert!(!gi.content.is_empty());

        // Settings
        let sf = bundle.settings.as_ref().unwrap();
        assert_eq!(sf.relative_path, PathBuf::from(".claude/settings.json"));
        let parsed: serde_json::Value = serde_json::from_str(&sf.content).unwrap();
        assert!(parsed.get("permissions").is_some());
        assert!(parsed.get("hooks").is_some());

        // Hooks
        assert_eq!(bundle.hooks.len(), 2);
        assert_eq!(bundle.hooks[0].event, "PreToolUse");
        assert_eq!(bundle.hooks[1].event, "PostToolUse");

        // Commands
        assert_eq!(bundle.commands.len(), 2);
        assert_eq!(bundle.commands[0].name, "checkpoint");
        assert_eq!(bundle.commands[1].name, "worktree");
    }

    #[test]
    fn claude_config_settings_is_valid_json() {
        let bundle = ClaudeConfig.generate_config(Path::new("/tmp/project")).unwrap();
        let sf = bundle.settings.unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&sf.content).unwrap();
        // Verify specific fields
        assert_eq!(parsed["model"], "opus");
        assert!(parsed["sandbox"]["enabled"].as_bool().unwrap());
    }

    #[test]
    fn claude_config_install_instructions() {
        assert!(ClaudeConfig.install_instructions().contains("@anthropic-ai/claude-code"));
    }

    #[test]
    fn claude_runtime_agent_type() {
        assert_eq!(ClaudeRuntime.agent_type(), AgentKind::Claude);
    }

    #[test]
    fn claude_runtime_parse_output() {
        let result = InvocationResult {
            output:
                r#"[{"severity":"warn","file":"src/auth.rs","line":42,"message":"Timing attack"}]"#
                    .to_string(),
            exit_code: 0,
            duration: std::time::Duration::from_secs(1),
        };
        let review = ClaudeRuntime.parse_output(&result);
        assert_eq!(review.feedbacks.len(), 1);
        assert_eq!(review.feedbacks[0].target.path, "src/auth.rs");
    }

    #[test]
    fn claude_hooks_contain_expected_content() {
        assert!(ClaudeConfig::branch_protection_script().contains("PROTECTED"));
        assert!(ClaudeConfig::branch_protection_script().contains("exit 2"));
        assert!(ClaudeConfig::auto_format_script().contains("rustfmt"));
        assert!(ClaudeConfig::auto_format_script().contains("prettier"));
    }
}
