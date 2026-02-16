# Agent Port Trait Design

Design specification for the `AgentConfig` and `AgentRuntime` port traits that enable agent-agnostic review inference in noslop.

## Overview

noslop is a local code review tool for agent-generated code. The agent abstraction provides a uniform interface for invoking LLM-based review analysis. An `AgentRuntime` wraps a CLI agent (Claude, Codex, etc.) and is used to send review prompts containing diffs, prior findings, and focus areas. The agent responds with structured findings.

The agent abstraction layer consists of two port traits:

1. **`AgentConfig`** -- Configuration generation for `noslop init <agent>`
2. **`AgentRuntime`** -- Agent invocation for review inference

The agent is one type of `ReviewAnalyzer`. The `AgentAnalyzer` adapter wraps an `AgentRuntime` and plugs it into the review pipeline alongside static and computed analyzers. Multiple agent passes are possible (security, quality, architecture) with different focused prompts.

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
pub use agent::{AgentConfig, AgentRuntime, AgentType, InvocationResult, ReviewOutput, Finding, Severity};
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

### Finding and Review Types

```rust
/// Severity level for a review finding
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Informational note, no action needed
    Info,
    /// Suggestion for improvement
    Warning,
    /// Likely bug or significant issue
    Error,
    /// Critical problem that must be fixed
    Critical,
}

/// A single finding from a review pass
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    /// Severity of the finding
    pub severity: Severity,
    /// File path relative to the repository root
    pub file: String,
    /// Line number (if applicable)
    pub line: Option<u32>,
    /// Short description of the finding
    pub message: String,
    /// Suggested fix or improvement (if any)
    pub suggestion: Option<String>,
    /// Which analyzer produced this finding (e.g., "agent:security", "clippy")
    pub source: String,
}

/// Context passed to a review analyzer
#[derive(Debug, Clone)]
pub struct ReviewContext {
    /// The unified diff being reviewed
    pub diff: String,
    /// List of changed files (paths relative to repo root)
    pub changed_files: Vec<String>,
    /// Repository root path
    pub repo_root: PathBuf,
    /// Optional commit range being reviewed (e.g., "main..HEAD")
    pub commit_range: Option<String>,
}

/// Configuration for a single agent invocation
#[derive(Debug, Clone)]
pub struct InvocationConfig {
    /// The prompt to send to the agent (review prompt with diff, focus, prior findings)
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
    /// Duration of the invocation
    pub duration: std::time::Duration,
}

/// Parsed output from an agent review pass
#[derive(Debug, Clone, Default)]
pub struct ReviewOutput {
    /// Structured findings extracted from the agent's output
    pub findings: Vec<Finding>,
    /// Raw output for logging/debugging
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
    /// project-specific content that can reference noslop review commands.
    fn generate_project_instructions(&self, project_root: &Path) -> anyhow::Result<Option<String>>;
}
```

### `AgentRuntime` Trait

Handles agent invocation for review inference.

```rust
//! Agent runtime port
//!
//! Defines the interface for invoking AI agents during review analysis.

use std::path::Path;

/// Agent runtime for review invocation
///
/// Implementations handle the actual invocation of an AI agent, including
/// command construction, execution, and output parsing. The agent receives
/// review prompts containing diffs and prior findings, and returns
/// structured review findings.
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

    /// Parse the raw output into structured review findings
    ///
    /// Extracts findings (severity, file, line, message, suggestion) from
    /// the agent's raw output. The agent is prompted to output findings in
    /// a structured format (JSON array or tagged text).
    fn parse_output(&self, result: &InvocationResult) -> ReviewOutput;

    /// Check if the agent CLI is available in PATH
    fn is_available(&self) -> bool;

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

## The ReviewAnalyzer Trait and AgentAnalyzer Bridge

The review pipeline consists of three tiers of analyzers:

```
Static analyzers (free) --> Computed analyzers (local) --> Agent analyzers (LLM)
```

Each analyzer implements the `ReviewAnalyzer` trait:

```rust
/// A review analyzer that produces findings from code changes.
///
/// Analyzers run in order: static (free, fast) -> computed (local) -> agent (LLM).
/// Each analyzer receives the findings from all prior analyzers, enabling
/// the LLM to focus on issues that automated tools cannot catch.
pub trait ReviewAnalyzer: Send + Sync {
    /// What context this analyzer needs (e.g., diff, file contents, AST)
    fn required_context(&self) -> Vec<ContextKind>;

    /// Run the analysis and return findings
    ///
    /// `prior_findings` contains all findings from analyzers that ran earlier
    /// in the pipeline. Agent analyzers use these to avoid duplicating work
    /// and to provide deeper analysis building on static results.
    fn analyze(&self, context: &ReviewContext, prior_findings: &[Finding]) -> Result<Vec<Finding>>;

    /// Human-readable name for this analyzer (e.g., "agent:security", "clippy")
    fn name(&self) -> &str;
}
```

The `AgentAnalyzer` wraps an `AgentRuntime` and implements `ReviewAnalyzer`:

```rust
/// An agent-based review analyzer.
///
/// Wraps an `AgentRuntime` to run LLM-powered review passes.
/// Each `AgentAnalyzer` instance is configured with a focus area
/// (security, quality, architecture, etc.) and a prompt template.
pub struct AgentAnalyzer {
    /// The underlying agent runtime (Claude, Codex, etc.)
    runtime: Box<dyn AgentRuntime>,
    /// Focus area for this review pass (e.g., "security", "quality", "architecture")
    focus: String,
    /// Prompt template with placeholders: {diff}, {prior_findings}, {focus}
    prompt_template: String,
}

impl AgentAnalyzer {
    pub fn new(
        runtime: Box<dyn AgentRuntime>,
        focus: impl Into<String>,
        prompt_template: impl Into<String>,
    ) -> Self {
        Self {
            runtime,
            focus: focus.into(),
            prompt_template: prompt_template.into(),
        }
    }

    /// Build the review prompt by substituting context into the template
    fn build_prompt(&self, ctx: &ReviewContext, prior: &[Finding]) -> String {
        let prior_summary = if prior.is_empty() {
            "No prior findings from static analysis.".to_string()
        } else {
            prior.iter()
                .map(|f| format!("- [{}] {}:{} {} (from {})",
                    format!("{:?}", f.severity).to_uppercase(),
                    f.file,
                    f.line.map_or("?".to_string(), |l| l.to_string()),
                    f.message,
                    f.source,
                ))
                .collect::<Vec<_>>()
                .join("\n")
        };

        self.prompt_template
            .replace("{diff}", &ctx.diff)
            .replace("{focus}", &self.focus)
            .replace("{prior_findings}", &prior_summary)
            .replace("{changed_files}", &ctx.changed_files.join("\n"))
    }
}

impl ReviewAnalyzer for AgentAnalyzer {
    fn required_context(&self) -> Vec<ContextKind> {
        vec![ContextKind::Diff, ContextKind::ChangedFiles]
    }

    fn analyze(&self, ctx: &ReviewContext, prior: &[Finding]) -> Result<Vec<Finding>> {
        let prompt = self.build_prompt(ctx, prior);

        let config = InvocationConfig {
            prompt,
            working_dir: ctx.repo_root.clone(),
            timeout_secs: Some(120),
            bypass_permissions: true,
            ..Default::default()
        };

        let result = self.runtime.invoke(&config)?;
        let output = self.runtime.parse_output(&result);

        if !output.errors.is_empty() {
            log::warn!(
                "Agent {} ({} focus) reported errors: {:?}",
                self.runtime.agent_type(),
                self.focus,
                output.errors
            );
        }

        Ok(output.findings)
    }

    fn name(&self) -> &str {
        // Returns e.g., "agent:security" or "agent:quality"
        // (stored as formatted string in practice, simplified here)
        Box::leak(format!("agent:{}", self.focus).into_boxed_str())
    }
}
```

### Multiple Agent Passes

The pipeline can run multiple `AgentAnalyzer` instances with different focus areas:

```rust
fn build_agent_analyzers(
    agent_type: AgentType,
    prompts_dir: &Path,
) -> Vec<Box<dyn ReviewAnalyzer>> {
    let runtime_factory = || get_agent_runtime(agent_type);

    let mut analyzers: Vec<Box<dyn ReviewAnalyzer>> = Vec::new();

    // Each focus area gets its own AgentAnalyzer with a dedicated prompt template
    let passes = ["security", "quality", "architecture"];

    for focus in &passes {
        let template_path = prompts_dir.join(format!("{focus}.md"));
        let template = if template_path.exists() {
            std::fs::read_to_string(&template_path)
                .unwrap_or_else(|_| default_prompt_template(focus))
        } else {
            default_prompt_template(focus)
        };

        analyzers.push(Box::new(AgentAnalyzer::new(
            runtime_factory(),
            *focus,
            template,
        )));
    }

    analyzers
}
```

## Design Rationale

### Separation of Config and Runtime

The two traits are separated because:

1. **Different lifecycles**: Config is used once during `noslop init`, runtime is used during `noslop review`
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

The `parse_output` method on `AgentRuntime` handles agent-specific output parsing. The agent is prompted to output findings in a structured JSON format:

```json
[
  {
    "severity": "warning",
    "file": "src/auth.rs",
    "line": 42,
    "message": "Password comparison is not constant-time",
    "suggestion": "Use `subtle::ConstantTimeEq` for password comparison to prevent timing attacks"
  }
]
```

The parser extracts this JSON array from the agent's output. If the agent includes narrative text around the JSON, the parser looks for the JSON array boundary (`[` ... `]`) and extracts it. Adapters can override parsing for agent-specific output quirks.

### AgentAnalyzer as Composition Layer

The `AgentAnalyzer` is the bridge between the agent abstraction and the review pipeline. This composition approach means:

- The `AgentRuntime` trait stays focused on "invoke a CLI agent and parse its output"
- The `ReviewAnalyzer` trait stays focused on "analyze code and produce findings"
- The `AgentAnalyzer` handles the translation: building review prompts, passing prior findings, invoking the runtime, and returning findings

This separation allows non-agent analyzers (clippy, eslint, custom scripts) to implement `ReviewAnalyzer` directly without touching the agent abstraction.

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

No changes to the review pipeline or existing adapters.

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

### With `noslop review`

The review command builds the analyzer pipeline and runs it:

```rust
// cli/commands/review.rs
pub fn run_review(
    agent_type: Option<AgentType>,
    diff: &str,
    changed_files: Vec<String>,
    repo_root: &Path,
) -> anyhow::Result<Vec<Finding>> {
    let ctx = ReviewContext {
        diff: diff.to_string(),
        changed_files,
        repo_root: repo_root.to_path_buf(),
        commit_range: None,
    };

    // Build the pipeline: static -> computed -> agent
    let mut all_findings: Vec<Finding> = Vec::new();

    // Tier 1: Static analyzers (free, fast)
    for analyzer in static_analyzers() {
        let findings = analyzer.analyze(&ctx, &all_findings)?;
        all_findings.extend(findings);
    }

    // Tier 2: Computed analyzers (local, moderate cost)
    for analyzer in computed_analyzers() {
        let findings = analyzer.analyze(&ctx, &all_findings)?;
        all_findings.extend(findings);
    }

    // Tier 3: Agent analyzers (LLM, highest cost, run last)
    if let Some(agent_type) = agent_type {
        let prompts_dir = repo_root.join(".noslop/prompts");
        let agent_analyzers = build_agent_analyzers(agent_type, &prompts_dir);
        for analyzer in &agent_analyzers {
            let findings = analyzer.analyze(&ctx, &all_findings)?;
            all_findings.extend(findings);
        }
    }

    Ok(all_findings)
}
```

### With `.noslop.toml`

The agent configuration is stored in the project config:

```toml
[project]
prefix = "NOS"

[agent]
type = "claude"

[review]
# Which agent focus passes to run
passes = ["security", "quality"]
# Timeout per agent pass in seconds
pass_timeout = 120
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
    fn test_review_with_mock_runtime() {
        let mut mock = MockAgentRuntime::new();

        mock.expect_agent_type()
            .return_const(AgentType::Claude);

        mock.expect_invoke()
            .with(predicate::always())
            .returning(|_| Ok(InvocationResult {
                output: r#"[{"severity":"warning","file":"src/auth.rs","line":42,"message":"Password comparison is not constant-time","suggestion":"Use constant-time comparison"}]"#.to_string(),
                exit_code: 0,
                duration: std::time::Duration::from_secs(5),
            }));

        mock.expect_parse_output()
            .returning(|result| {
                let findings: Vec<Finding> = serde_json::from_str(
                    &result.output
                ).unwrap_or_default();
                ReviewOutput {
                    findings,
                    raw_output: result.output.clone(),
                    errors: vec![],
                }
            });

        // Test review logic with mock
    }

    #[test]
    fn test_agent_analyzer_builds_prompt() {
        let mut mock = MockAgentRuntime::new();
        mock.expect_agent_type().return_const(AgentType::Claude);
        mock.expect_invoke().returning(|_| Ok(InvocationResult {
            output: "[]".to_string(),
            exit_code: 0,
            duration: std::time::Duration::from_secs(1),
        }));
        mock.expect_parse_output().returning(|_| ReviewOutput::default());

        let analyzer = AgentAnalyzer::new(
            Box::new(mock),
            "security",
            "Review this diff for {focus} issues:\n\n{diff}\n\nPrior findings:\n{prior_findings}",
        );

        let ctx = ReviewContext {
            diff: "+fn unsafe_thing() {}".to_string(),
            changed_files: vec!["src/lib.rs".to_string()],
            repo_root: PathBuf::from("/tmp/test"),
            commit_range: None,
        };

        let prior = vec![Finding {
            severity: Severity::Warning,
            file: "src/lib.rs".to_string(),
            line: Some(10),
            message: "unused import".to_string(),
            suggestion: None,
            source: "clippy".to_string(),
        }];

        let findings = analyzer.analyze(&ctx, &prior).unwrap();
        assert!(findings.is_empty()); // mock returns empty
    }
}
```

### Integration Tests with Real Agents

For integration tests, create fixture-based tests that exercise real agent CLI tools:

```rust
#[test]
#[ignore] // Run with: cargo test -- --ignored
fn test_claude_review_integration() {
    let runtime = ClaudeRuntime::new();

    if !runtime.is_available() {
        eprintln!("Skipping: claude CLI not found");
        return;
    }

    let config = InvocationConfig {
        prompt: "Review this diff and return findings as JSON:\n\n+fn main() { panic!(\"todo\"); }".to_string(),
        working_dir: temp_dir(),
        timeout_secs: Some(60),
        ..Default::default()
    };

    let result = runtime.invoke(&config).unwrap();
    assert_eq!(result.exit_code, 0);

    let output = runtime.parse_output(&result);
    // Agent should produce at least one finding about the panic
    assert!(!output.findings.is_empty() || !output.raw_output.is_empty());
}
```

## Complete Module Definition

```rust
//! Agent port traits
//!
//! Defines interfaces for AI agent configuration and runtime invocation
//! in the context of code review. Agents are invoked with review prompts
//! containing diffs and prior findings, and return structured review findings.
//!
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

/// Severity level for a review finding
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Info,
    Warning,
    Error,
    Critical,
}

/// A single finding from a review pass
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub severity: Severity,
    pub file: String,
    pub line: Option<u32>,
    pub message: String,
    pub suggestion: Option<String>,
    pub source: String,
}

/// Context passed to a review analyzer
#[derive(Debug, Clone)]
pub struct ReviewContext {
    pub diff: String,
    pub changed_files: Vec<String>,
    pub repo_root: PathBuf,
    pub commit_range: Option<String>,
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
    /// The review prompt to send to the agent
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
    /// Duration of the invocation
    pub duration: std::time::Duration,
}

/// Parsed output from an agent review pass
#[derive(Debug, Clone, Default)]
pub struct ReviewOutput {
    /// Structured findings extracted from the agent's output
    pub findings: Vec<Finding>,
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

/// Agent runtime for review invocation
///
/// Implementations handle the actual invocation of an AI agent for
/// code review analysis. The agent receives review prompts and returns
/// structured findings.
#[cfg_attr(test, mockall::automock)]
pub trait AgentRuntime: Send + Sync {
    /// Returns the agent type this runtime handles
    fn agent_type(&self) -> AgentType;

    /// Invoke the agent with the given configuration
    fn invoke(&self, config: &InvocationConfig) -> anyhow::Result<InvocationResult>;

    /// Parse the raw output into structured review findings
    fn parse_output(&self, result: &InvocationResult) -> ReviewOutput;

    /// Check if the agent CLI is available in PATH
    fn is_available(&self) -> bool;

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

1. **Clear abstraction**: `AgentConfig` for setup, `AgentRuntime` for review invocation
2. **Review-focused types**: `Finding`, `Severity`, `ReviewOutput`, `ReviewContext` for structured review data
3. **Pipeline composition**: `AgentAnalyzer` bridges `AgentRuntime` into the `ReviewAnalyzer` pipeline
4. **Multi-pass review**: Different focus areas (security, quality, architecture) via separate `AgentAnalyzer` instances
5. **Prior findings flow**: Agent analyzers receive findings from static/computed tiers to avoid duplication
6. **Extensibility**: Easy to add new agents by implementing two traits
7. **Testability**: Full mock support via `mockall`
8. **Consistency**: Follows noslop's existing port patterns exactly

Next steps:

- Task 4: Design Claude and Codex adapter implementations for review
- Task 7: Integrate with `noslop init <agent>` expansion
