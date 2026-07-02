//! Actor detection from the process environment
//!
//! Detection order:
//! 1. `NOSLOP_ACTOR` - explicit override (`human` or an agent name)
//! 2. Known agent-harness environment fingerprints
//! 3. CI environments (treated as agents: they must verify, never rubber-stamp)
//! 4. TTY heuristic - an interactive terminal on stdin means a person

use std::io::IsTerminal;

use crate::core::models::Actor;

/// Environment fingerprints for known agent harnesses, checked in order
const AGENT_FINGERPRINTS: &[(&str, &str)] = &[
    ("CLAUDECODE", "claude-code"),
    ("CLAUDE_CODE_ENTRYPOINT", "claude-code"),
    ("CURSOR_AGENT", "cursor"),
    ("CODEX_SANDBOX", "codex"),
    ("AIDER_MODEL", "aider"),
    ("GITHUB_ACTIONS", "github-actions"),
    ("CI", "ci"),
];

/// Detect who is running the current command
#[must_use]
pub fn detect_actor() -> Actor {
    detect_from(|k| std::env::var(k).ok(), std::io::stdin().is_terminal())
}

/// Detection logic, injectable for tests
fn detect_from(env: impl Fn(&str) -> Option<String>, stdin_is_tty: bool) -> Actor {
    if let Some(explicit) = env("NOSLOP_ACTOR") {
        let explicit = explicit.trim().to_string();
        if !explicit.is_empty() {
            return if explicit.eq_ignore_ascii_case("human") {
                Actor::Human
            } else {
                Actor::Agent(explicit)
            };
        }
    }

    for (var, name) in AGENT_FINGERPRINTS {
        if let Some(value) = env(var)
            && !value.is_empty()
            && value != "0"
            && !value.eq_ignore_ascii_case("false")
        {
            return Actor::Agent((*name).to_string());
        }
    }

    if stdin_is_tty {
        Actor::Human
    } else {
        // Non-interactive with no known fingerprint: assume automation
        Actor::Agent("unknown-agent".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn env_of(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let map: HashMap<String, String> =
            pairs.iter().map(|(k, v)| ((*k).to_string(), (*v).to_string())).collect();
        move |k| map.get(k).cloned()
    }

    #[test]
    fn explicit_override_wins() {
        let actor = detect_from(env_of(&[("NOSLOP_ACTOR", "my-bot"), ("CLAUDECODE", "1")]), true);
        assert_eq!(actor, Actor::Agent("my-bot".to_string()));
    }

    #[test]
    fn explicit_human_override() {
        let actor = detect_from(env_of(&[("NOSLOP_ACTOR", "human"), ("CLAUDECODE", "1")]), false);
        assert_eq!(actor, Actor::Human);
    }

    #[test]
    fn claude_code_fingerprint() {
        let actor = detect_from(env_of(&[("CLAUDECODE", "1")]), true);
        assert_eq!(actor, Actor::Agent("claude-code".to_string()));
    }

    #[test]
    fn falsey_fingerprint_ignored() {
        let actor = detect_from(env_of(&[("CLAUDECODE", "0")]), true);
        assert_eq!(actor, Actor::Human);
    }

    #[test]
    fn tty_means_human() {
        let actor = detect_from(env_of(&[]), true);
        assert_eq!(actor, Actor::Human);
    }

    #[test]
    fn no_tty_no_fingerprint_means_unknown_agent() {
        let actor = detect_from(env_of(&[]), false);
        assert_eq!(actor, Actor::Agent("unknown-agent".to_string()));
    }
}
