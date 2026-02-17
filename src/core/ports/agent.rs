//! Agent port traits
//!
//! Defines the boundary between core domain logic and agent implementations.
//!
//! - [`AgentConfig`] -- generates configuration files during `noslop init <agent>`
//! - [`AgentRuntime`] -- invokes an agent CLI during `noslop review`

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::core::models::{AgentKind, Finding};

// ---------------------------------------------------------------------------
// AgentConfig types
// ---------------------------------------------------------------------------

/// Global instructions file (e.g., `~/.claude/CLAUDE.md`).
#[derive(Debug, Clone)]
pub struct GlobalInstructions {
    /// Path relative to `$HOME`.
    pub relative_path: PathBuf,
    /// File content.
    pub content: String,
}

/// Agent settings file (e.g., `~/.claude/settings.json`).
#[derive(Debug, Clone)]
pub struct SettingsFile {
    /// Path relative to `$HOME`.
    pub relative_path: PathBuf,
    /// File content.
    pub content: String,
}

/// Hook definition for agent configuration.
#[derive(Debug, Clone)]
pub struct HookDefinition {
    /// Event name (e.g., `"PreToolUse"`).
    pub event: String,
    /// Tool matcher (e.g., `"Edit|Write"`).
    pub matcher: String,
    /// Script content or path.
    pub command: String,
}

/// Slash command definition.
#[derive(Debug, Clone)]
pub struct SlashCommand {
    /// Command name without leading slash.
    pub name: String,
    /// Path relative to agent config dir.
    pub relative_path: PathBuf,
    /// Markdown content.
    pub content: String,
}

/// Complete configuration bundle returned by [`AgentConfig::generate_config`].
#[derive(Debug, Clone, Default)]
pub struct AgentConfigBundle {
    /// Global instructions file.
    pub global_instructions: Option<GlobalInstructions>,
    /// Settings file.
    pub settings: Option<SettingsFile>,
    /// Hook definitions.
    pub hooks: Vec<HookDefinition>,
    /// Slash command definitions.
    pub commands: Vec<SlashCommand>,
}

/// Agent configuration generator.
///
/// Implementations produce all files needed for an agent to work with noslop.
/// Called once during `noslop init <agent>`.
#[cfg_attr(test, mockall::automock)]
pub trait AgentConfig: Send + Sync {
    /// Which agent this config targets.
    fn agent_type(&self) -> AgentKind;

    /// Generate all configuration files for the agent.
    fn generate_config(&self, project_root: &Path) -> anyhow::Result<AgentConfigBundle>;

    /// Whether the agent CLI is installed and available.
    fn is_available(&self) -> bool;

    /// Human-readable install instructions.
    fn install_instructions(&self) -> &'static str;

    /// Validate that the agent is usable. Default checks CLI availability.
    fn validate(&self) -> anyhow::Result<()> {
        if !self.is_available() {
            anyhow::bail!(
                "{} CLI not found in PATH. Install: {}",
                self.agent_type(),
                self.install_instructions()
            );
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// AgentRuntime types
// ---------------------------------------------------------------------------

/// Configuration for a single agent invocation.
#[derive(Debug, Clone)]
pub struct InvocationConfig {
    /// The review prompt to send.
    pub prompt: String,
    /// Working directory for the CLI process.
    pub working_dir: PathBuf,
    /// Optional timeout in seconds.
    pub timeout_secs: Option<u64>,
    /// Whether to bypass agent permission prompts.
    pub bypass_permissions: bool,
    /// Extra environment variables.
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

/// Raw result from an agent invocation.
#[derive(Debug, Clone)]
pub struct InvocationResult {
    /// Combined stdout (and stderr if non-empty) output.
    pub output: String,
    /// Process exit code.
    pub exit_code: i32,
    /// Wall-clock duration of the invocation.
    pub duration: std::time::Duration,
}

/// Parsed output from an agent review pass.
#[derive(Debug, Clone, Default)]
pub struct ReviewOutput {
    /// Structured findings extracted from agent output.
    pub findings: Vec<Finding>,
    /// The full raw output text.
    pub raw_output: String,
    /// Any error lines detected in the output.
    pub errors: Vec<String>,
}

/// Agent runtime for review invocation.
///
/// Wraps a CLI tool (`claude`, `codex`), sends it a review prompt,
/// and parses structured findings from the output.
#[cfg_attr(test, mockall::automock)]
pub trait AgentRuntime: Send + Sync {
    /// Which agent this runtime wraps.
    fn agent_type(&self) -> AgentKind;

    /// Invoke the agent CLI with the given configuration.
    fn invoke(&self, config: &InvocationConfig) -> anyhow::Result<InvocationResult>;

    /// Parse structured findings from raw invocation output.
    fn parse_output(&self, result: &InvocationResult) -> ReviewOutput;

    /// Whether the agent CLI is installed and available.
    fn is_available(&self) -> bool;
}
