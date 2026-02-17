//! Agent adapter implementations
//!
//! Concrete implementations of [`AgentConfig`] and [`AgentRuntime`]
//! for supported AI review agents.
//!
//! - [`claude`] -- Anthropic Claude Code
//! - [`codex`] -- `OpenAI` Codex
//! - [`parsing`] -- shared output parsing

mod claude;
mod codex;
/// Shared finding extraction from agent output.
pub mod parsing;

pub use claude::{ClaudeConfig, ClaudeRuntime};
pub use codex::{CodexConfig, CodexRuntime};

use crate::core::models::AgentKind;
use crate::core::ports::agent::{AgentConfig, AgentRuntime};

/// Get the configuration generator for a given agent kind.
#[must_use]
pub fn get_agent_config(kind: AgentKind) -> Box<dyn AgentConfig> {
    match kind {
        AgentKind::Claude => Box::new(ClaudeConfig),
        AgentKind::Codex => Box::new(CodexConfig),
    }
}

/// Get the runtime invoker for a given agent kind.
#[must_use]
pub fn get_agent_runtime(kind: AgentKind) -> Box<dyn AgentRuntime> {
    match kind {
        AgentKind::Claude => Box::new(ClaudeRuntime),
        AgentKind::Codex => Box::new(CodexRuntime),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn factory_returns_correct_config_type() {
        let claude = get_agent_config(AgentKind::Claude);
        assert_eq!(claude.agent_type(), AgentKind::Claude);

        let codex = get_agent_config(AgentKind::Codex);
        assert_eq!(codex.agent_type(), AgentKind::Codex);
    }

    #[test]
    fn factory_returns_correct_runtime_type() {
        let claude = get_agent_runtime(AgentKind::Claude);
        assert_eq!(claude.agent_type(), AgentKind::Claude);

        let codex = get_agent_runtime(AgentKind::Codex);
        assert_eq!(codex.agent_type(), AgentKind::Codex);
    }

    #[test]
    fn validate_fails_when_cli_not_available() {
        // Neither claude nor codex are likely installed in CI/test
        // validate() should return an error with install instructions
        let config = get_agent_config(AgentKind::Codex);
        if !config.is_available() {
            let err = config.validate().unwrap_err();
            let msg = err.to_string();
            assert!(msg.contains("not found in PATH"), "got: {msg}");
            assert!(msg.contains("npm install"), "got: {msg}");
        }
    }
}
