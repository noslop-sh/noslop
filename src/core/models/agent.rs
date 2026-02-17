//! Agent model
//!
//! Which AI reviewer to use. Agent-specific configuration lives in
//! the adapter layer.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// Supported AI review agents.
///
/// Each variant maps to a CLI tool that noslop invokes for
/// agent-based review analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentKind {
    /// Anthropic Claude Code (`claude` CLI).
    Claude,
    /// `OpenAI` Codex (`codex` CLI).
    Codex,
}

impl AgentKind {
    /// The CLI command to invoke this agent.
    #[must_use]
    pub const fn command(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
        }
    }

    /// Human-readable name for display.
    #[must_use]
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Claude => "Claude",
            Self::Codex => "Codex",
        }
    }
}

impl fmt::Display for AgentKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.display_name())
    }
}

impl FromStr for AgentKind {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "claude" => Ok(Self::Claude),
            "codex" => Ok(Self::Codex),
            _ => Err(format!("unknown agent: {s}. supported: claude, codex")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_kind_command() {
        assert_eq!(AgentKind::Claude.command(), "claude");
        assert_eq!(AgentKind::Codex.command(), "codex");
    }

    #[test]
    fn agent_kind_display_name() {
        assert_eq!(AgentKind::Claude.display_name(), "Claude");
        assert_eq!(AgentKind::Codex.display_name(), "Codex");
    }

    #[test]
    fn agent_kind_display() {
        assert_eq!(AgentKind::Claude.to_string(), "Claude");
        assert_eq!(AgentKind::Codex.to_string(), "Codex");
    }

    #[test]
    fn agent_kind_from_str() {
        assert_eq!("claude".parse::<AgentKind>().unwrap(), AgentKind::Claude);
        assert_eq!("CODEX".parse::<AgentKind>().unwrap(), AgentKind::Codex);
        assert!("gpt4".parse::<AgentKind>().is_err());
    }

    #[test]
    fn agent_kind_serde_roundtrip() {
        let json = serde_json::to_string(&AgentKind::Claude).unwrap();
        assert_eq!(json, "\"claude\"");
        let parsed: AgentKind = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, AgentKind::Claude);
    }
}
