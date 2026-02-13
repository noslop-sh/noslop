# Agent Port Trait Design

Design specification for the `AgentConfig` and `AgentRuntime` port traits that will enable agent-agnostic iteration loops in noslop.

## Overview

The agent abstraction layer consists of two port traits:

1. **`AgentConfig`** — Configuration generation for `noslop init <agent>`
2. **`AgentRuntime`** — Agent invocation for `noslop run`

These traits follow noslop's existing port patterns:
- Traits are `Send + Sync` for thread safety
- Methods return `anyhow::Result<T>` for error handling
- Traits are annotated with `#[cfg_attr(test, mockall::automock)]` for testing
- Implementations live in `src/adapters/agent/`

## File Location

```
src/core/ports/agent.rs
```

Re-exported via `src/core/ports/mod.rs`:
```rust
mod agent;
pub use agent::{AgentConfig, AgentRuntime, AgentType, InvocationResult, IterationOutput};
```

## Type Definitions

### `AgentType` Enum

Enumerates supported AI agents. Only Claude and Codex are implemented initially; the enum is extensible for future agents.

```rust
/// Supported AI agent types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentType {
    /// Anthropic's Claude Code agent
    Claude,
    /// OpenAI's Codex CLI agent
    Codex,
}

impl AgentType {
    /// Returns the CLI command name for this agent
    #[must_use]
    pub fn command_name(&self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
        }
    }

    /// Returns a human-readable display name
    #[must_use]
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Claude => "Claude Code",
            Self::Codex => "Codex CLI",
        }
    }
}

impl std::str::FromStr for AgentType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "claude" | "claude-code" => Ok(Self::Claude),
            "codex" | "openai" => Ok(Self::Codex),
            _ => Err(format!(
                "Unknown agent type: {s}. Supported: claude, codex"
            )),
        }
    }
}

impl std::fmt::Display for AgentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Claude => write!(f, "claude"),
            Self::Codex => write!(f, "codex"),
        }
    }
}
```

### Configuration Types

```rust
/// Global instructions file to generate
#[derive(Debug, Clone)]
pub struct GlobalInstructions {
    /// Target path relative to home (e.g., ".claude/CLAUDE.md" or ".codex/instructions.md")
    pub relative_path: PathBuf,
    /// Content to write
    pub content: String,
}

/// Hook definition for agent configuration
#[derive(Debug, Clone)]
pub struct HookDefinition {
    /// Hook event type (e.g., "PreToolUse", "PostToolUse")
    pub event: String,
    /// Matcher pattern (e.g., "Edit|Write")
    pub matcher: String,
    /// Hook script path or inline command
    pub command: String,
}

/// Agent settings file to generate
#[derive(Debug, Clone)]
pub struct SettingsFile {
    /// Target path relative to home (e.g., ".claude/settings.json")
    pub relative_path: PathBuf,
    /// Content (typically JSON)
    pub content: String,
}

/// Slash command definition
#[derive(Debug, Clone)]
pub struct SlashCommand {
    /// Command name without leading slash (e.g., "checkpoint")
    pub name: String,
    /// Command file path relative to agent config dir
    pub relative_path: PathBuf,
    /// Command definition content (markdown)
    pub content: String,
}

/// Complete agent configuration bundle
#[derive(Debug, Clone, Default)]
pub struct AgentConfigBundle {
    /// Global instructions file
    pub global_instructions: Option<GlobalInstructions>,
    /// Settings file
    pub settings: Option<SettingsFile>,
    /// Hook definitions
    pub hooks: Vec<HookDefinition>,
    /// Slash commands
    pub commands: Vec<SlashCommand>,
}
```

### Invocation Types

```rust
/// Configuration for a single agent invocation
#[derive(Debug, Clone)]
pub struct InvocationConfig {
    /// The prompt to send to the agent
    pub prompt: String,
    /// Working directory for the agent
    pub working_dir: PathBuf,
    /// Optional timeout in seconds (None = no timeout)
    pub timeout_secs: Option<u64>,
    /// Whether to bypass permission prompts
    pub bypass_permissions: bool,
    /// Additional environment variables
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

/// Result of an agent invocation
#[derive(Debug, Clone)]
pub struct InvocationResult {
    /// Raw output text from the agent
    pub output: String,
    /// Exit code (0 = success)
    pub exit_code: i32,
    /// Whether the agent reported completion
    pub completed: bool,
    /// Parsed status line if present (e.g., "[RALPH:DONE task=\"...\"]")
    pub status_line: Option<String>,
    /// Duration of the invocation
    pub duration: std::time::Duration,
}

/// Parsed output from an agent iteration
#[derive(Debug, Clone, Default)]
pub struct IterationOutput {
    /// Whether the agent signaled full plan completion
    pub plan_complete: bool,
    /// Task description from status line (if present)
    pub completed_task: Option<String>,
    /// Iteration number from status line (if present)
    pub iteration: Option<u32>,
    /// Raw output for logging
    pub raw_output: String,
    /// Any error messages extracted from output
    pub errors: Vec<String>,
}
```

## Port Traits

### `AgentConfig` Trait

Handles configuration generation for `noslop init <agent>`.

```rust
//! Agent configuration port
//!
//! Defines the interface for generating agent-specific configuration files.

use std::path::Path;

/// Agent configuration generator
///
/// Implementations generate the configuration files needed for a specific
/// AI agent to work with noslop (global instructions, settings, hooks, etc.)
#[cfg_attr(test, mockall::automock)]
pub trait AgentConfig: Send + Sync {
    /// Returns the agent type this config handles
    fn agent_type(&self) -> AgentType;

    /// Generate the full configuration bundle for this agent
    ///
    /// The bundle contains all files that should be created during `noslop init <agent>`.
    fn generate_config(&self, project_root: &Path) -> anyhow::Result<AgentConfigBundle>;

    /// Check if this agent's CLI tool is available in PATH
    fn is_available(&self) -> bool;

    /// Get the minimum required version of the agent CLI (if any)
    fn min_version(&self) -> Option<&str> {
        None
    }

    /// Validate that the agent meets requirements
    ///
    /// Returns Ok(()) if the agent is available and meets version requirements.
    /// Returns an error with a helpful message if not.
    fn validate(&self) -> anyhow::Result<()> {
        if !self.is_available() {
            anyhow::bail!(
                "{} CLI not found in PATH. Install it first: {}",
                self.agent_type().display_name(),
                self.install_instructions()
            );
        }
        Ok(())
    }

    /// Get installation instructions for this agent
    fn install_instructions(&self) -> &str;

    /// Generate project-local agent instructions (goes in .noslop/ or repo root)
    ///
    /// Unlike global_instructions which go in ~/.agent/, this generates
    /// project-specific content that can reference noslop checks and conventions.
    fn generate_project_instructions(&self, project_root: &Path) -> anyhow::Result<Option<String>>;
}
```

### `AgentRuntime` Trait

Handles agent invocation for `noslop run`.

```rust
//! Agent runtime port
//!
//! Defines the interface for invoking AI agents during iteration loops.

use std::path::Path;

/// Agent runtime for iteration loop invocation
///
/// Implementations handle the actual invocation of an AI agent, including
/// command construction, execution, and output parsing.
#[cfg_attr(test, mockall::automock)]
pub trait AgentRuntime: Send + Sync {
    /// Returns the agent type this runtime handles
    fn agent_type(&self) -> AgentType;

    /// Invoke the agent with the given configuration
    ///
    /// This is the core method that runs the agent and captures its output.
    /// Implementations should:
    /// 1. Build the appropriate CLI command
    /// 2. Execute it with the given working directory
    /// 3. Capture stdout/stderr
    /// 4. Return structured results
    fn invoke(&self, config: &InvocationConfig) -> anyhow::Result<InvocationResult>;

    /// Parse the raw output into structured iteration output
    ///
    /// Extracts completion signals, status lines, and error messages from
    /// the agent's raw output.
    fn parse_output(&self, result: &InvocationResult) -> IterationOutput;

    /// Check if the agent signaled completion of the entire plan
    ///
    /// Default implementation checks for `<promise>COMPLETE</promise>` tag.
    /// Agents may override for different completion signals.
    fn is_plan_complete(&self, output: &str) -> bool {
        output.contains("<promise>COMPLETE</promise>")
    }

    /// Extract the status line from agent output
    ///
    /// Default implementation looks for `[RALPH:DONE task="..." iteration=N]`.
    /// Returns None if no status line found.
    fn extract_status_line(&self, output: &str) -> Option<String> {
        // Regex pattern: [RALPH:DONE task="..." iteration=N]
        let re = regex::Regex::new(r#"\[RALPH:DONE\s+task="([^"]+)"\s+iteration=(\d+)\]"#).ok()?;
        re.captures(output).map(|c| c[0].to_string())
    }

    /// Build the CLI command arguments for this agent
    ///
    /// Returns the command and arguments that will be executed.
    fn build_command(&self, config: &InvocationConfig) -> (String, Vec<String>);

    /// Get environment variables to set for agent invocation
    fn invocation_env(&self) -> HashMap<String, String> {
        HashMap::new()
    }
}
```

## Design Rationale

### Separation of Config and Runtime

The two traits are separated because:

1. **Different lifecycles**: Config is used once during `noslop init`, runtime is used repeatedly during `noslop run`
2. **Different dependencies**: Config may need filesystem access for templates, runtime needs process execution
3. **Easier testing**: Can mock config generation separately from invocation
4. **Single responsibility**: Each trait has a clear, focused purpose

### Trait Object Compatibility

Both traits are designed to work with trait objects (`Box<dyn AgentConfig>`, `Box<dyn AgentRuntime>`):
- `Send + Sync` bounds for thread safety
- No generic methods (all methods use concrete types)
- No `Self` constraints beyond `Sized`

This enables runtime dispatch based on `AgentType`:

```rust
pub fn get_agent_config(agent_type: AgentType) -> Box<dyn AgentConfig> {
    match agent_type {
        AgentType::Claude => Box::new(ClaudeConfig::new()),
        AgentType::Codex => Box::new(CodexConfig::new()),
    }
}

pub fn get_agent_runtime(agent_type: AgentType) -> Box<dyn AgentRuntime> {
    match agent_type {
        AgentType::Claude => Box::new(ClaudeRuntime::new()),
        AgentType::Codex => Box::new(CodexRuntime::new()),
    }
}
```

### Output Parsing Strategy

The `parse_output` method on `AgentRuntime` handles agent-specific output parsing:

- **Claude**: Looks for `<promise>COMPLETE</promise>` and `[RALPH:DONE ...]` patterns
- **Codex**: May use different completion signals (TBD based on Codex CLI behavior)

The default implementations in the trait provide the current Ralph conventions, but adapters can override for agent-specific behavior.

### Configuration Bundle Design

The `AgentConfigBundle` struct groups all configuration artifacts:

```rust
AgentConfigBundle {
    global_instructions: Some(GlobalInstructions {
        relative_path: PathBuf::from(".claude/CLAUDE.md"),
        content: "# Global Instructions\n...".to_string(),
    }),
    settings: Some(SettingsFile {
        relative_path: PathBuf::from(".claude/settings.json"),
        content: serde_json::to_string_pretty(&settings)?,
    }),
    hooks: vec![
        HookDefinition {
            event: "PreToolUse".to_string(),
            matcher: "Edit|Write".to_string(),
            command: "~/.claude/hooks/branch-protection.sh".to_string(),
        },
    ],
    commands: vec![
        SlashCommand {
            name: "checkpoint".to_string(),
            relative_path: PathBuf::from("commands/checkpoint.md"),
            content: include_str!("templates/checkpoint.md").to_string(),
        },
    ],
}
```

This allows the init command to:
1. Call `config.generate_config(project_root)?`
2. Iterate over the bundle and write each file
3. Handle conflicts/overwrites consistently

### Extensibility for Future Agents

Adding a new agent (e.g., Cursor, Aider) requires:

1. Add variant to `AgentType` enum
2. Implement `AgentConfig` for config generation
3. Implement `AgentRuntime` for invocation
4. Register in factory functions

No changes to core iteration loop logic or existing adapters.

## Integration Points

### With `noslop init`

The init command gains an optional agent argument:

```rust
// cli/commands/init.rs
pub fn run_init(agent: Option<AgentType>, force: bool) -> anyhow::Result<()> {
    // ... existing noslop init logic (hooks, .noslop.toml) ...

    if let Some(agent_type) = agent {
        let config = get_agent_config(agent_type);
        config.validate()?;

        let bundle = config.generate_config(&project_root)?;
        write_config_bundle(&bundle)?;

        if let Some(project_instructions) = config.generate_project_instructions(&project_root)? {
            write_agent_instructions(&project_root, agent_type, &project_instructions)?;
        }
    }

    Ok(())
}
```

### With `noslop run`

The run command uses `AgentRuntime`:

```rust
// cli/commands/run.rs
pub fn run_iteration(
    runtime: &dyn AgentRuntime,
    plan_file: &Path,
    iteration: u32,
    context: &IterationContext,
) -> anyhow::Result<IterationOutput> {
    let prompt = build_iteration_prompt(plan_file, iteration, context)?;

    let config = InvocationConfig {
        prompt,
        working_dir: plan_file.parent().unwrap().to_path_buf(),
        bypass_permissions: true,
        ..Default::default()
    };

    let result = runtime.invoke(&config)?;
    let output = runtime.parse_output(&result);

    Ok(output)
}
```

### With `.noslop.toml`

The agent configuration is stored in the project config:

```toml
[project]
prefix = "NOS"

[agent]
type = "claude"

[loop]
max_iterations = 20
stuck_threshold = 3
fail_threshold = 5
```

## Test Strategy

### Unit Tests with Mocks

Both traits use `mockall` for auto-generated mocks:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use mockall::predicate::*;

    #[test]
    fn test_iteration_with_mock_runtime() {
        let mut mock = MockAgentRuntime::new();

        mock.expect_agent_type()
            .return_const(AgentType::Claude);

        mock.expect_invoke()
            .with(predicate::always())
            .returning(|_| Ok(InvocationResult {
                output: "[RALPH:DONE task=\"Test task\" iteration=1]\n<promise>COMPLETE</promise>".to_string(),
                exit_code: 0,
                completed: true,
                status_line: Some("[RALPH:DONE task=\"Test task\" iteration=1]".to_string()),
                duration: std::time::Duration::from_secs(5),
            }));

        mock.expect_parse_output()
            .returning(|result| IterationOutput {
                plan_complete: result.output.contains("<promise>COMPLETE</promise>"),
                completed_task: Some("Test task".to_string()),
                iteration: Some(1),
                raw_output: result.output.clone(),
                errors: vec![],
            });

        // Test iteration logic with mock
    }
}
```

### Integration Tests with Real Agents

For integration tests, create fixture-based tests that exercise real agent CLI tools:

```rust
#[test]
#[ignore] // Run with: cargo test -- --ignored
fn test_claude_invocation_integration() {
    let runtime = ClaudeRuntime::new();

    if !runtime.is_available() {
        eprintln!("Skipping: claude CLI not found");
        return;
    }

    let config = InvocationConfig {
        prompt: "echo 'Hello from test'".to_string(),
        working_dir: temp_dir(),
        timeout_secs: Some(30),
        ..Default::default()
    };

    let result = runtime.invoke(&config).unwrap();
    assert_eq!(result.exit_code, 0);
}
```

## Complete Module Definition

```rust
//! Agent port traits
//!
//! Defines interfaces for AI agent configuration and runtime invocation.
//! Implementations live in `adapters::agent`.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

// === Type Definitions ===

/// Supported AI agent types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentType {
    /// Anthropic's Claude Code agent
    Claude,
    /// OpenAI's Codex CLI agent
    Codex,
}

impl AgentType {
    /// Returns the CLI command name for this agent
    #[must_use]
    pub fn command_name(&self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
        }
    }

    /// Returns a human-readable display name
    #[must_use]
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Claude => "Claude Code",
            Self::Codex => "Codex CLI",
        }
    }
}

impl std::str::FromStr for AgentType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "claude" | "claude-code" => Ok(Self::Claude),
            "codex" | "openai" => Ok(Self::Codex),
            _ => Err(format!(
                "Unknown agent type: {s}. Supported: claude, codex"
            )),
        }
    }
}

impl std::fmt::Display for AgentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Claude => write!(f, "claude"),
            Self::Codex => write!(f, "codex"),
        }
    }
}

/// Global instructions file to generate
#[derive(Debug, Clone)]
pub struct GlobalInstructions {
    /// Target path relative to home (e.g., ".claude/CLAUDE.md")
    pub relative_path: PathBuf,
    /// Content to write
    pub content: String,
}

/// Hook definition for agent configuration
#[derive(Debug, Clone)]
pub struct HookDefinition {
    /// Hook event type (e.g., "PreToolUse", "PostToolUse")
    pub event: String,
    /// Matcher pattern (e.g., "Edit|Write")
    pub matcher: String,
    /// Hook script path or inline command
    pub command: String,
}

/// Agent settings file to generate
#[derive(Debug, Clone)]
pub struct SettingsFile {
    /// Target path relative to home (e.g., ".claude/settings.json")
    pub relative_path: PathBuf,
    /// Content (typically JSON)
    pub content: String,
}

/// Slash command definition
#[derive(Debug, Clone)]
pub struct SlashCommand {
    /// Command name without leading slash
    pub name: String,
    /// Command file path relative to agent config dir
    pub relative_path: PathBuf,
    /// Command definition content
    pub content: String,
}

/// Complete agent configuration bundle
#[derive(Debug, Clone, Default)]
pub struct AgentConfigBundle {
    /// Global instructions file
    pub global_instructions: Option<GlobalInstructions>,
    /// Settings file
    pub settings: Option<SettingsFile>,
    /// Hook definitions
    pub hooks: Vec<HookDefinition>,
    /// Slash commands
    pub commands: Vec<SlashCommand>,
}

/// Configuration for a single agent invocation
#[derive(Debug, Clone)]
pub struct InvocationConfig {
    /// The prompt to send to the agent
    pub prompt: String,
    /// Working directory for the agent
    pub working_dir: PathBuf,
    /// Optional timeout in seconds
    pub timeout_secs: Option<u64>,
    /// Whether to bypass permission prompts
    pub bypass_permissions: bool,
    /// Additional environment variables
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

/// Result of an agent invocation
#[derive(Debug, Clone)]
pub struct InvocationResult {
    /// Raw output text from the agent
    pub output: String,
    /// Exit code (0 = success)
    pub exit_code: i32,
    /// Whether the agent reported completion
    pub completed: bool,
    /// Parsed status line if present
    pub status_line: Option<String>,
    /// Duration of the invocation
    pub duration: std::time::Duration,
}

/// Parsed output from an agent iteration
#[derive(Debug, Clone, Default)]
pub struct IterationOutput {
    /// Whether the agent signaled full plan completion
    pub plan_complete: bool,
    /// Task description from status line
    pub completed_task: Option<String>,
    /// Iteration number from status line
    pub iteration: Option<u32>,
    /// Raw output for logging
    pub raw_output: String,
    /// Any error messages extracted
    pub errors: Vec<String>,
}

// === Port Traits ===

/// Agent configuration generator
///
/// Implementations generate the configuration files needed for a specific
/// AI agent to work with noslop.
#[cfg_attr(test, mockall::automock)]
pub trait AgentConfig: Send + Sync {
    /// Returns the agent type this config handles
    fn agent_type(&self) -> AgentType;

    /// Generate the full configuration bundle
    fn generate_config(&self, project_root: &Path) -> anyhow::Result<AgentConfigBundle>;

    /// Check if this agent's CLI tool is available
    fn is_available(&self) -> bool;

    /// Get the minimum required version (if any)
    fn min_version(&self) -> Option<&str> {
        None
    }

    /// Validate that the agent meets requirements
    fn validate(&self) -> anyhow::Result<()> {
        if !self.is_available() {
            anyhow::bail!(
                "{} CLI not found in PATH. Install it first: {}",
                self.agent_type().display_name(),
                self.install_instructions()
            );
        }
        Ok(())
    }

    /// Get installation instructions
    fn install_instructions(&self) -> &str;

    /// Generate project-local agent instructions
    fn generate_project_instructions(&self, project_root: &Path) -> anyhow::Result<Option<String>>;
}

/// Agent runtime for iteration loop invocation
///
/// Implementations handle the actual invocation of an AI agent.
#[cfg_attr(test, mockall::automock)]
pub trait AgentRuntime: Send + Sync {
    /// Returns the agent type this runtime handles
    fn agent_type(&self) -> AgentType;

    /// Invoke the agent with the given configuration
    fn invoke(&self, config: &InvocationConfig) -> anyhow::Result<InvocationResult>;

    /// Parse the raw output into structured iteration output
    fn parse_output(&self, result: &InvocationResult) -> IterationOutput;

    /// Check if the agent signaled completion of the entire plan
    fn is_plan_complete(&self, output: &str) -> bool {
        output.contains("<promise>COMPLETE</promise>")
    }

    /// Extract the status line from agent output
    fn extract_status_line(&self, output: &str) -> Option<String> {
        let re = regex::Regex::new(r#"\[RALPH:DONE\s+task="([^"]+)"\s+iteration=(\d+)\]"#).ok()?;
        re.captures(output).map(|c| c[0].to_string())
    }

    /// Build the CLI command arguments
    fn build_command(&self, config: &InvocationConfig) -> (String, Vec<String>);

    /// Get environment variables for invocation
    fn invocation_env(&self) -> HashMap<String, String> {
        HashMap::new()
    }
}
```

## Summary

The agent port design provides:

1. **Clear abstraction**: `AgentConfig` for setup, `AgentRuntime` for execution
2. **Type safety**: Strong types for configuration bundles and invocation results
3. **Extensibility**: Easy to add new agents by implementing two traits
4. **Testability**: Full mock support via `mockall`
5. **Consistency**: Follows noslop's existing port patterns exactly

Next steps:
- Task 4: Design Claude and Codex adapter implementations
- Task 7: Integrate with `noslop init <agent>` expansion
