//! Actor model - who is performing an action
//!
//! noslop gates agents, not people: blocking checks stop an agent's commit,
//! while a human committer sees the same guidance as an FYI and proceeds.

use std::fmt;

/// Who is performing the current commit or acknowledgment
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Actor {
    /// A person at a terminal
    Human,
    /// An automated actor (coding agent, CI), identified by name
    Agent(String),
}

impl Actor {
    /// The actor name as recorded in acknowledgment trailers
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Self::Human => "human",
            Self::Agent(name) => name,
        }
    }

    /// Whether blocking checks should stop this actor's commit
    #[must_use]
    pub const fn is_gated(&self) -> bool {
        matches!(self, Self::Agent(_))
    }
}

impl fmt::Display for Actor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn human_is_not_gated() {
        assert!(!Actor::Human.is_gated());
        assert_eq!(Actor::Human.name(), "human");
    }

    #[test]
    fn agent_is_gated() {
        let actor = Actor::Agent("claude-code".to_string());
        assert!(actor.is_gated());
        assert_eq!(actor.name(), "claude-code");
    }
}
