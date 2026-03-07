//! Codex adapter
//!
//! [`CodexConfig`] generates configuration for `noslop init codex`.
//! [`CodexRuntime`] invokes the `codex` CLI for `noslop review`.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use crate::core::models::AgentKind;
use crate::core::ports::agent::{
    AgentConfig, AgentConfigBundle, AgentRuntime, GlobalInstructions, InvocationConfig,
    InvocationResult, ReviewOutput,
};

/// Codex configuration generator.
///
/// Produces only `instructions.md` under `~/.codex/`. Codex CLI has
/// no settings file, hooks, or slash commands.
#[derive(Debug, Clone, Copy)]
pub struct CodexConfig;

impl AgentConfig for CodexConfig {
    fn agent_type(&self) -> AgentKind {
        AgentKind::Codex
    }

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
        Command::new("codex")
            .arg("--version")
            .output()
            .is_ok_and(|o| o.status.success())
    }

    fn install_instructions(&self) -> &'static str {
        "npm install -g @openai/codex"
    }
}

/// Codex runtime for review invocation.
///
/// Invokes `codex --quiet --approval-mode full-auto "$PROMPT"`.
#[derive(Debug, Clone, Copy)]
pub struct CodexRuntime;

impl AgentRuntime for CodexRuntime {
    fn agent_type(&self) -> AgentKind {
        AgentKind::Codex
    }

    fn invoke(&self, config: &InvocationConfig) -> anyhow::Result<InvocationResult> {
        let start = Instant::now();
        let mut cmd = Command::new("codex");
        cmd.args(["--quiet", "--approval-mode", "full-auto", &config.prompt])
            .current_dir(&config.working_dir);
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
            &crate::core::models::FeedbackSource::Agent("codex".into()),
        )
    }

    fn is_available(&self) -> bool {
        Command::new("codex")
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
    fn codex_config_agent_type() {
        assert_eq!(CodexConfig.agent_type(), AgentKind::Codex);
    }

    #[test]
    fn codex_config_generates_bundle() {
        let bundle = CodexConfig.generate_config(Path::new("/tmp/project")).unwrap();

        // Global instructions only
        let gi = bundle.global_instructions.as_ref().unwrap();
        assert_eq!(gi.relative_path, PathBuf::from(".codex/instructions.md"));
        assert!(!gi.content.is_empty());

        // No settings, hooks, or commands
        assert!(bundle.settings.is_none());
        assert!(bundle.hooks.is_empty());
        assert!(bundle.commands.is_empty());
    }

    #[test]
    fn codex_config_install_instructions() {
        assert!(CodexConfig.install_instructions().contains("@openai/codex"));
    }

    #[test]
    fn codex_runtime_agent_type() {
        assert_eq!(CodexRuntime.agent_type(), AgentKind::Codex);
    }

    #[test]
    fn codex_runtime_parse_output() {
        let result = InvocationResult {
            output:
                r#"[{"severity":"block","file":"src/db.rs","line":15,"message":"SQL injection"}]"#
                    .to_string(),
            exit_code: 0,
            duration: std::time::Duration::from_secs(1),
        };
        let review = CodexRuntime.parse_output(&result);
        assert_eq!(review.feedbacks.len(), 1);
        assert_eq!(review.feedbacks[0].target.path, "src/db.rs");
    }

    #[test]
    fn codex_is_available_returns_false_when_not_installed() {
        // codex is almost certainly not installed in CI/test environments
        // This test documents the expected behavior: returns false when CLI not found
        let available = CodexConfig.is_available();
        // We can't assert true or false reliably, but the call must not panic
        let _ = available;
    }
}
