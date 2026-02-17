# Review Pipeline Design

Design specification for `noslop review` -- the core orchestrator that runs an ordered pipeline of `ReviewAnalyzer` implementations against a branch diff.

## 1. Overview

`noslop review` computes the diff between the current branch and a base branch, then runs an ordered sequence of analyzers against that diff. Each analyzer produces `Finding` objects. Findings from earlier analyzers are passed to later ones (fold semantics), so an LLM-based analyzer sees what static tools already caught and can focus on deeper issues.

The pipeline runs three tiers, each more expensive than the last:

```text
Static (free, fast)       --> Computed (local tools)       --> Agent (LLM)
ConventionAnalyzer            FormattingAnalyzer               AgentAnalyzer
                              CoChangeAnalyzer
                              ScriptAnalyzer
```

Static analyzers match `[[check]]` patterns against changed files. Computed analyzers run local tools or query git history. Agent analyzers invoke an LLM via the `AgentRuntime` trait. All three tiers implement the same `ReviewAnalyzer` trait, and the pipeline treats them uniformly.

Configured in `.noslop.toml`:

```toml
[review]
analyzers = ["conventions", "formatting", "co-change", "agent:security", "agent:quality"]
```

Three analyzer kinds:

- `BuiltIn { name }` -- conventions, formatting, co-change
- `Agent { name, focus, agent, prompt_template }` -- agent:security, agent:quality
- `Script { name, command }` -- custom:migrations (user's own tools)

All types below reference the unified domain model from `00-core-domain-model.md`. Domain types (`Finding`, `FindingStatus`, `Severity`, `Target`, `Span`, `FindingSource`, `Review`, `ReviewStatus`, `Check`, `AgentKind`) are imported with `use crate::core::models::*` and are NOT redefined here.

## 2. ReviewAnalyzer Trait

The core port trait. Every analyzer -- built-in, script, or agent -- implements this interface.

### File: `src/core/ports/analyzer.rs`

```rust
//! Review analyzer port
//!
//! Defines the trait that all review analyzers implement. The pipeline
//! orchestrator operates exclusively through `dyn ReviewAnalyzer` trait objects.

use std::path::PathBuf;
use crate::core::models::*;

/// What kind of context an analyzer needs from the pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ContextKind {
    /// List of changed file paths
    ChangedFiles,
    /// Unified diff text
    Diff,
    /// Full file contents at HEAD
    FileContents,
}

/// Context passed to every analyzer during a review run.
///
/// Built once by the pipeline from VCS data, then shared immutably.
#[derive(Debug, Clone)]
pub struct ReviewContext {
    /// Unified diff text (base..head)
    pub diff: String,
    /// Changed file paths relative to repo root
    pub changed_files: Vec<String>,
    /// Absolute path to the repository root
    pub repo_root: PathBuf,
    /// Commit range string, e.g. "main..HEAD"
    pub commit_range: Option<String>,
    /// Base ref (branch name or SHA)
    pub base: String,
    /// Head ref (branch name or SHA)
    pub head: String,
}

/// A review analyzer that produces findings from code changes.
///
/// Analyzers run in pipeline order: static -> computed -> agent.
/// Each receives findings from all prior analyzers (fold semantics).
///
/// # Implementors
///
/// - `ConventionAnalyzer` -- matches [[check]] rules against changed files
/// - `FormattingAnalyzer` -- invokes external formatters in check mode
/// - `CoChangeAnalyzer` -- flags missing correlated files via git history
/// - `ScriptAnalyzer` -- runs user-configured external commands
/// - `AgentAnalyzer` -- wraps AgentRuntime for LLM-based review
#[cfg_attr(test, mockall::automock)]
pub trait ReviewAnalyzer: Send + Sync {
    /// What context this analyzer needs.
    fn required_context(&self) -> Vec<ContextKind>;

    /// Run the analysis and return findings.
    ///
    /// `prior_findings` contains all findings from earlier analyzers.
    fn analyze(
        &self,
        context: &ReviewContext,
        prior_findings: &[Finding],
    ) -> anyhow::Result<Vec<Finding>>;

    /// Human-readable name (e.g., "conventions", "agent:security").
    fn name(&self) -> &str;
}
```

Re-exported via `src/core/ports/mod.rs`:

```rust
mod analyzer;
pub use analyzer::{ReviewAnalyzer, ReviewContext, ContextKind};
```

## 3. Built-in Analyzers

All live in `src/adapters/analyzers/`.

### 3.1 ConventionAnalyzer

Wraps `[[check]]` entries from `.noslop.toml`. When a changed file matches a check's target pattern, a `Finding` is produced via `check.into_finding()`.

#### File: `src/adapters/analyzers/convention.rs`

```rust
use glob::Pattern;
use crate::core::models::*;
use crate::core::ports::analyzer::{ContextKind, ReviewAnalyzer, ReviewContext};

/// Produces findings from [[check]] entries in .noslop.toml.
pub struct ConventionAnalyzer {
    checks: Vec<Check>,
}

impl ConventionAnalyzer {
    pub fn new(checks: Vec<Check>) -> Self {
        Self { checks }
    }

    fn match_check(&self, check: &Check, changed_files: &[String]) -> Vec<Finding> {
        if check.target.is_glob() {
            let pattern = match Pattern::new(&check.target.path) {
                Ok(p) => p,
                Err(_) => return vec![],
            };
            changed_files
                .iter()
                .filter(|f| pattern.matches(f))
                .map(|f| check.into_finding(f))
                .collect()
        } else {
            changed_files
                .iter()
                .filter(|f| *f == &check.target.path)
                .map(|f| check.into_finding(f))
                .collect()
        }
    }
}

impl ReviewAnalyzer for ConventionAnalyzer {
    fn required_context(&self) -> Vec<ContextKind> {
        vec![ContextKind::ChangedFiles]
    }

    fn analyze(
        &self,
        context: &ReviewContext,
        _prior_findings: &[Finding],
    ) -> anyhow::Result<Vec<Finding>> {
        let mut findings = Vec::new();
        for check in &self.checks {
            findings.extend(self.match_check(check, &context.changed_files));
        }
        Ok(findings)
    }

    fn name(&self) -> &str { "conventions" }
}
```

The `check.into_finding(matched_file)` method lives on `Check` in `core::models` (from `00-core-domain-model.md`). It creates a `Finding` with `source: FindingSource::Check(self.id)` and `severity: self.severity`.

### 3.2 FormattingAnalyzer

Invokes external formatters (rustfmt, prettier) in check/dry-run mode. Files that would be reformatted are flagged as findings. Does not modify files.

#### File: `src/adapters/analyzers/formatting.rs`

```rust
use std::path::Path;
use std::process::Command;
use crate::core::models::*;
use crate::core::ports::analyzer::{ContextKind, ReviewAnalyzer, ReviewContext};

/// Configuration for the formatting analyzer.
#[derive(Debug, Clone, Default)]
pub struct FormattingConfig {
    pub tools: Vec<String>,
}

/// Detects formatting violations by running formatters in check mode.
pub struct FormattingAnalyzer {
    config: FormattingConfig,
}

impl FormattingAnalyzer {
    pub fn new(config: FormattingConfig) -> Self {
        Self { config }
    }

    /// Run a single tool in check mode against files it handles.
    fn check_tool(&self, tool: &str, changed_files: &[String], repo_root: &Path) -> Vec<Finding> {
        let relevant: Vec<&String> = changed_files
            .iter()
            .filter(|f| Self::tool_handles(tool, f))
            .collect();
        if relevant.is_empty() { return vec![]; }

        let (cmd, args) = Self::check_command(tool, &relevant);
        let output = match Command::new(&cmd).args(&args).current_dir(repo_root).output() {
            Ok(o) => o,
            Err(_) => return vec![],
        };

        if output.status.success() { return vec![]; }

        // Parse which files would change from stdout/stderr, flag each as Warn
        Self::parse_dirty_files(tool, &output, &relevant)
    }

    fn tool_handles(tool: &str, file: &str) -> bool { /* extension matching */ }
    fn check_command(tool: &str, files: &[&String]) -> (String, Vec<String>) { /* --check args */ }
    fn parse_dirty_files(tool: &str, output: &std::process::Output, candidates: &[&String]) -> Vec<Finding> {
        // Each dirty file becomes a Finding with:
        //   severity: Warn, source: FindingSource::Script("formatting:{tool}")
        //   suggestion: "Run {tool} to fix formatting"
    }
}

impl ReviewAnalyzer for FormattingAnalyzer {
    fn required_context(&self) -> Vec<ContextKind> { vec![ContextKind::ChangedFiles] }

    fn analyze(&self, context: &ReviewContext, _prior: &[Finding]) -> anyhow::Result<Vec<Finding>> {
        let mut findings = Vec::new();
        for tool in &self.config.tools {
            findings.extend(self.check_tool(tool, &context.changed_files, &context.repo_root));
        }
        Ok(findings)
    }

    fn name(&self) -> &str { "formatting" }
}
```

### 3.3 CoChangeAnalyzer

Queries git history to build a co-change frequency matrix. Flags files that historically change together with files in the current diff but are missing from it.

#### File: `src/adapters/analyzers/co_change.rs`

```rust
use std::collections::{HashMap, HashSet};
use crate::core::models::*;
use crate::core::ports::analyzer::{ContextKind, ReviewAnalyzer, ReviewContext};
use crate::core::ports::vcs::VersionControl;

/// Configuration for the co-change analyzer.
#[derive(Debug, Clone)]
pub struct CoChangeConfig {
    /// Minimum correlation ratio to flag (0.0 - 1.0)
    pub min_correlation: f64,
    /// How many commits to look back
    pub lookback: u32,
    /// Minimum co-occurrences before considering a pair
    pub min_commits: u32,
}

impl Default for CoChangeConfig {
    fn default() -> Self {
        Self { min_correlation: 0.7, lookback: 500, min_commits: 3 }
    }
}

/// Detects missing co-change files using git history correlation.
pub struct CoChangeAnalyzer {
    config: CoChangeConfig,
    vcs: Box<dyn VersionControl>,
}

impl CoChangeAnalyzer {
    pub fn new(config: CoChangeConfig, vcs: Box<dyn VersionControl>) -> Self {
        Self { config, vcs }
    }

    /// Build co-change frequency matrix: (file_a, file_b) -> co-occurrence count.
    fn build_matrix(&self, commits: &[CommitFiles]) -> HashMap<(String, String), u32> {
        // For each commit, for each pair of changed files, increment co-occurrence.
        // Pairs are canonicalized (a < b) for dedup.
    }

    /// For each changed file, find correlated files missing from the diff.
    /// Returns (changed_file, missing_file, correlation_ratio).
    fn find_missing(
        &self,
        changed_files: &[String],
        matrix: &HashMap<(String, String), u32>,
        file_counts: &HashMap<String, u32>,
    ) -> Vec<(String, String, f64)> {
        // correlation = co_count / max(count_a, count_b)
        // Filter by min_correlation and min_commits.
        // Deduplicate: keep highest correlation per missing file.
    }
}

impl ReviewAnalyzer for CoChangeAnalyzer {
    fn required_context(&self) -> Vec<ContextKind> { vec![ContextKind::ChangedFiles] }

    fn analyze(&self, context: &ReviewContext, _prior: &[Finding]) -> anyhow::Result<Vec<Finding>> {
        let commits = self.vcs.recent_commits(self.config.lookback as usize)?;
        if commits.is_empty() { return Ok(vec![]); }

        let matrix = self.build_matrix(&commits);
        let file_counts = self.file_counts(&commits);
        let missing = self.find_missing(&context.changed_files, &matrix, &file_counts);

        Ok(missing.into_iter().map(|(changed, absent, corr)| Finding {
            id: format!("cochange-{}", absent.replace('/', "-")),
            target: Target::file(&absent),
            severity: Severity::Warn,
            message: format!("{} usually changes with {} ({:.0}%)", absent, changed, corr * 100.0),
            source: FindingSource::Script("co-change".into()),
            status: FindingStatus::default(),
            suggestion: Some(format!("Review whether {} needs updates too", absent)),
            created_at: now_iso8601(),
        }).collect())
    }

    fn name(&self) -> &str { "co-change" }
}
```

### 3.4 ScriptAnalyzer

Runs user-configured external commands. The script receives review context as JSON on stdin and writes findings as a JSON array on stdout.

#### File: `src/adapters/analyzers/script.rs`

```rust
use std::io::Write;
use std::process::{Command, Stdio};
use crate::core::models::*;
use crate::core::ports::analyzer::{ContextKind, ReviewAnalyzer, ReviewContext};

/// JSON protocol: what scripts receive on stdin.
#[derive(serde::Serialize)]
struct ScriptInput<'a> {
    changed_files: &'a [String],
    diff: &'a str,
    repo_root: String,
    prior_findings: &'a [Finding],
}

/// JSON protocol: what scripts write to stdout.
#[derive(serde::Deserialize)]
struct ScriptFinding {
    severity: String,            // "info" | "warn" | "block"
    file: String,
    #[serde(default)]
    line: Option<u32>,
    message: String,
    #[serde(default)]
    suggestion: Option<String>,
}

/// Runs a user-configured external command as an analyzer.
pub struct ScriptAnalyzer {
    analyzer_name: String,
    command: String,
}

impl ScriptAnalyzer {
    pub fn new(name: impl Into<String>, command: impl Into<String>) -> Self {
        Self { analyzer_name: name.into(), command: command.into() }
    }
}

impl ReviewAnalyzer for ScriptAnalyzer {
    fn required_context(&self) -> Vec<ContextKind> {
        vec![ContextKind::Diff, ContextKind::ChangedFiles]
    }

    fn analyze(&self, context: &ReviewContext, prior: &[Finding]) -> anyhow::Result<Vec<Finding>> {
        let input = ScriptInput {
            changed_files: &context.changed_files,
            diff: &context.diff,
            repo_root: context.repo_root.display().to_string(),
            prior_findings: prior,
        };

        let mut child = Command::new("sh")
            .args(["-c", &self.command])
            .current_dir(&context.repo_root)
            .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped())
            .spawn()?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(serde_json::to_string(&input)?.as_bytes())?;
        }

        let output = child.wait_with_output()?;
        if !output.status.success() {
            anyhow::bail!("Script '{}' exited {}", self.analyzer_name, output.status);
        }

        // Parse stdout JSON array of ScriptFinding, map each to Finding
        let raw: Vec<ScriptFinding> = serde_json::from_str(
            String::from_utf8_lossy(&output.stdout).trim()
        ).unwrap_or_default();

        Ok(raw.into_iter().enumerate().map(|(i, sf)| {
            let mut target = Target::file(&sf.file);
            if let Some(line) = sf.line {
                target = target.with_span(Span::line(line));
            }
            Finding {
                id: format!("{}-{:03}", self.analyzer_name, i + 1),
                target,
                severity: match sf.severity.as_str() {
                    "info" => Severity::Info, "warn" => Severity::Warn, _ => Severity::Block,
                },
                message: sf.message,
                source: FindingSource::Script(self.analyzer_name.clone()),
                status: FindingStatus::default(),
                suggestion: sf.suggestion,
                created_at: now_iso8601(),
            }
        }).collect())
    }

    fn name(&self) -> &str { &self.analyzer_name }
}
```

#### Script JSON Protocol

Scripts receive on stdin: `{ "changed_files": [...], "diff": "...", "repo_root": "/path", "prior_findings": [...] }`

Scripts write to stdout: `[{ "severity": "block", "file": "src/auth.rs", "line": 42, "message": "...", "suggestion": "..." }]`

## 4. AgentAnalyzer Bridge

The `AgentAnalyzer` wraps an `AgentRuntime` (from `core::ports::agent`) to implement `ReviewAnalyzer`. It builds a prompt from the diff, changed files, focus area, and prior findings, sends it to the agent, then parses output into `Vec<Finding>`.

Multiple instances can run in a single pipeline with different focus areas (security, quality, architecture), each seeing all prior findings including those from earlier agent passes.

### File: `src/adapters/analyzers/agent_bridge.rs`

```rust
use crate::core::models::*;
use crate::core::ports::agent::{AgentRuntime, InvocationConfig};
use crate::core::ports::analyzer::{ContextKind, ReviewAnalyzer, ReviewContext};

/// An agent-based review analyzer.
///
/// Wraps an `AgentRuntime` to run LLM-powered review passes.
/// Configured with a focus area and a prompt template.
pub struct AgentAnalyzer {
    runtime: Box<dyn AgentRuntime>,
    focus: String,
    display_name: String,
    prompt_template: String,
}

impl AgentAnalyzer {
    pub fn new(
        runtime: Box<dyn AgentRuntime>,
        focus: impl Into<String>,
        prompt_template: impl Into<String>,
    ) -> Self {
        let focus = focus.into();
        let display_name = format!("agent:{}", focus);
        Self { runtime, focus, display_name, prompt_template: prompt_template.into() }
    }

    /// Build the prompt by substituting {diff}, {focus}, {prior_findings}, {changed_files}.
    fn build_prompt(&self, ctx: &ReviewContext, prior: &[Finding]) -> String {
        let prior_summary = if prior.is_empty() {
            "No prior findings from static analysis.".to_string()
        } else {
            prior.iter()
                .map(|f| format!("- [{:?}] {}: {} (from {:?})", f.severity, f.target.path, f.message, f.source))
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

    fn analyze(&self, ctx: &ReviewContext, prior: &[Finding]) -> anyhow::Result<Vec<Finding>> {
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

        // Tag all findings with agent source
        Ok(output.findings.into_iter().map(|mut f| {
            f.source = FindingSource::Agent(self.display_name.clone());
            f
        }).collect())
    }

    fn name(&self) -> &str { &self.display_name }
}
```

## 5. ReviewPipeline Service

The central orchestrator. Builds analyzers from config, runs them in sequence with fold semantics, wraps findings into a `Review`, saves via `ReviewStore`.

### File: `src/core/services/pipeline.rs`

```rust
use crate::adapters::analyzers::*;
use crate::core::models::*;
use crate::core::ports::analyzer::{ReviewAnalyzer, ReviewContext};
use crate::core::ports::review_store::ReviewStore;
use crate::core::ports::vcs::VersionControl;

/// A single entry in the analyzer pipeline, resolved from config.
#[derive(Debug, Clone)]
pub enum AnalyzerEntry {
    BuiltIn { name: String },
    Agent { name: String, focus: String, agent: String, prompt_template: Option<String> },
    Script { name: String, command: String },
}

/// Pipeline configuration resolved from .noslop.toml + CLI overrides.
#[derive(Debug, Clone)]
pub struct PipelineConfig {
    pub entries: Vec<AnalyzerEntry>,
    pub co_change: CoChangeConfig,
    pub formatting: FormattingConfig,
}

/// Builds and executes the review pipeline.
pub struct ReviewPipeline<'a> {
    config: PipelineConfig,
    checks: Vec<Check>,
    vcs: &'a dyn VersionControl,
    store: &'a dyn ReviewStore,
}

impl<'a> ReviewPipeline<'a> {
    pub fn new(
        config: PipelineConfig,
        checks: Vec<Check>,
        vcs: &'a dyn VersionControl,
        store: &'a dyn ReviewStore,
    ) -> Self {
        Self { config, checks, vcs, store }
    }

    /// Execute the full review pipeline.
    ///
    /// 1. Build analyzers from config
    /// 2. Run each in order, passing all prior findings (fold semantics)
    /// 3. Wrap findings in a Review
    /// 4. Save via ReviewStore
    pub fn execute(&self, context: &ReviewContext) -> anyhow::Result<Review> {
        let analyzers = self.build_analyzers()?;
        let mut all_findings: Vec<Finding> = Vec::new();

        for analyzer in &analyzers {
            log::info!("Running analyzer: {}", analyzer.name());
            let findings = analyzer.analyze(context, &all_findings)?;
            log::info!("  {} produced {} finding(s)", analyzer.name(), findings.len());
            all_findings.extend(findings);
        }

        let review = Review {
            id: generate_review_id(),
            base: context.base.clone(),
            head: context.head.clone(),
            status: ReviewStatus::Open,
            findings: all_findings,
            created_at: now_iso8601(),
            closed_at: None,
        };

        self.store.save(&review)?;
        Ok(review)
    }

    /// Build a single analyzer from its config entry.
    fn build_one(&self, entry: &AnalyzerEntry) -> anyhow::Result<Box<dyn ReviewAnalyzer>> {
        match entry {
            AnalyzerEntry::BuiltIn { name } => match name.as_str() {
                "conventions" => Ok(Box::new(ConventionAnalyzer::new(self.checks.clone()))),
                "formatting" => Ok(Box::new(FormattingAnalyzer::new(self.config.formatting.clone()))),
                "co-change" => Ok(Box::new(CoChangeAnalyzer::new(
                    self.config.co_change.clone(), self.vcs.clone_box(),
                ))),
                unknown => anyhow::bail!("Unknown built-in analyzer: '{}'", unknown),
            },
            AnalyzerEntry::Agent { focus, agent, prompt_template, .. } => {
                let kind: AgentKind = agent.parse()?;
                let runtime = crate::adapters::agents::get_runtime(kind);
                let template = match prompt_template {
                    Some(path) => std::fs::read_to_string(path)?,
                    None => default_agent_prompt(focus),
                };
                Ok(Box::new(AgentAnalyzer::new(runtime, focus, template)))
            }
            AnalyzerEntry::Script { name, command } => {
                Ok(Box::new(ScriptAnalyzer::new(name.clone(), command.clone())))
            }
        }
    }

    fn build_analyzers(&self) -> anyhow::Result<Vec<Box<dyn ReviewAnalyzer>>> {
        self.config.entries.iter().map(|e| self.build_one(e)).collect()
    }
}

fn generate_review_id() -> String {
    let ts = chrono::Utc::now().format("%Y%m%d%H%M%S");
    let rand: u32 = rand::random::<u32>() % 0xFFFF;
    format!("REV-{}-{:04x}", ts, rand)
}

fn default_agent_prompt(focus: &str) -> String {
    // Template with placeholders: {diff}, {changed_files}, {prior_findings}
    // Instructs agent to return JSON array of findings with severity/file/line/message/suggestion
    format!(r#"You are a code reviewer focused on {focus} issues.
## Diff to review
{{diff}}
## Changed files
{{changed_files}}
## Prior findings
{{prior_findings}}
## Instructions
Return findings as a JSON array: [{{ "severity": "info|warn|block", "file": "...", "line": N, "message": "...", "suggestion": "..." }}]
Return ONLY the JSON array. If no issues: []"#, focus = focus)
}
```

## 6. ReviewStore Port

The `ReviewStore` trait provides persistence for `Review` objects. The pipeline writes reviews after execution. The `noslop findings` command reads and updates them. The pre-push hook queries for unresolved blockers.

### File: `src/core/ports/review_store.rs`

```rust
//! Review storage port

use crate::core::models::Review;

/// Persistence for reviews.
///
/// Implementations store reviews as JSON files in `.noslop/reviews/`.
#[cfg_attr(test, mockall::automock)]
pub trait ReviewStore: Send + Sync {
    /// Save a review (create or update).
    fn save(&self, review: &Review) -> anyhow::Result<()>;

    /// Load a review by ID.
    fn load(&self, id: &str) -> anyhow::Result<Option<Review>>;

    /// Load the most recent review for the current branch.
    fn latest(&self) -> anyhow::Result<Option<Review>>;

    /// List all review IDs, most recent first.
    fn list(&self) -> anyhow::Result<Vec<String>>;
}
```

### JsonReviewStore Adapter

#### File: `src/adapters/review_store/json.rs`

```rust
use std::fs;
use std::path::{Path, PathBuf};
use crate::core::models::Review;
use crate::core::ports::review_store::ReviewStore;

/// JSON file-based ReviewStore.
///
/// Each review is a separate file in `.noslop/reviews/{id}.json`.
pub struct JsonReviewStore {
    dir: PathBuf,
}

impl JsonReviewStore {
    pub fn new(dir: &Path) -> Self {
        Self { dir: dir.to_path_buf() }
    }

    fn review_path(&self, id: &str) -> PathBuf {
        self.dir.join(format!("{}.json", id))
    }
}

impl ReviewStore for JsonReviewStore {
    fn save(&self, review: &Review) -> anyhow::Result<()> {
        fs::create_dir_all(&self.dir)?;
        let json = serde_json::to_string_pretty(review)?;
        fs::write(self.review_path(&review.id), json)?;
        Ok(())
    }

    fn load(&self, id: &str) -> anyhow::Result<Option<Review>> {
        let path = self.review_path(id);
        if !path.exists() { return Ok(None); }
        Ok(Some(serde_json::from_str(&fs::read_to_string(path)?)?))
    }

    fn latest(&self) -> anyhow::Result<Option<Review>> {
        match self.list()?.first() {
            Some(id) => self.load(id),
            None => Ok(None),
        }
    }

    fn list(&self) -> anyhow::Result<Vec<String>> {
        if !self.dir.exists() { return Ok(vec![]); }

        let mut entries: Vec<(String, std::time::SystemTime)> = Vec::new();
        for entry in fs::read_dir(&self.dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "json") {
                if let Some(stem) = path.file_stem() {
                    entries.push((
                        stem.to_string_lossy().to_string(),
                        entry.metadata()?.modified()?,
                    ));
                }
            }
        }

        entries.sort_by(|a, b| b.1.cmp(&a.1)); // most recent first
        Ok(entries.into_iter().map(|(id, _)| id).collect())
    }
}
```

## 7. Pre-push Hook Integration

The pre-push hook gates `git push` on unresolved blocking findings. Installed by `noslop init`.

### Hook Behavior

`noslop review --check` loads the latest review and exits non-zero if any findings have `severity == Block` and `status == Open`. This is a read-only check -- it does not re-run the pipeline.

### Hook Script

```bash
#!/bin/bash
# .git/hooks/pre-push -- installed by `noslop init`

if ! command -v noslop &>/dev/null; then exit 0; fi

if noslop review --check 2>/dev/null; then exit 0; fi

echo "Push blocked: unresolved blocking findings exist." >&2
echo "Run 'noslop findings' to see them, then resolve or dismiss." >&2
exit 1
```

### --check Implementation

```rust
// In the noslop review --check path (CLI layer)
pub fn review_check(store: &dyn ReviewStore) -> anyhow::Result<()> {
    let review = store
        .latest()?
        .ok_or_else(|| anyhow::anyhow!("No reviews found. Run `noslop review` first."))?;

    if review.is_blocked() {
        let blockers = review.blocking_findings();
        eprintln!("{} unresolved blocking finding(s):", blockers.len());
        for f in &blockers {
            eprintln!("  [{}] {}: {}", f.id, f.target.path, f.message);
        }
        std::process::exit(1);
    }
    Ok(())
}
```

The `review.is_blocked()` and `review.blocking_findings()` methods live on `Review` in `core::models` (from `00-core-domain-model.md`):

```rust
impl Review {
    pub fn is_blocked(&self) -> bool {
        self.findings.iter().any(|f| {
            f.severity == Severity::Block && f.status == FindingStatus::Open
        })
    }

    pub fn blocking_findings(&self) -> Vec<&Finding> {
        self.findings.iter()
            .filter(|f| f.severity == Severity::Block && f.status == FindingStatus::Open)
            .collect()
    }
}
```

## Data Flow Summary

```text
noslop review [--base main]
       |
       v
  CLI: load .noslop.toml, resolve base, extract diff via VersionControl
       |
       v
  Build ReviewContext { diff, changed_files, repo_root, base, head }
       |
       v
  ReviewPipeline.execute(&context)
       |
       +-- ConventionAnalyzer    (static: match [[check]] patterns)
       |     findings: [F1, F2]
       |
       +-- FormattingAnalyzer    (computed: rustfmt --check, prettier --check)
       |     prior: [F1, F2], findings: [F3]
       |
       +-- CoChangeAnalyzer      (computed: git history correlation)
       |     prior: [F1, F2, F3], findings: [F4]
       |
       +-- AgentAnalyzer:security (agent: LLM with security focus)
       |     prior: [F1..F4], findings: [F5, F6]
       |
       +-- AgentAnalyzer:quality  (agent: LLM with quality focus)
       |     prior: [F1..F6], findings: [F7]
       |
       v
  Review { findings: [F1..F7], status: Open }
       |
       v
  ReviewStore.save() --> .noslop/reviews/REV-{ts}-{rand}.json
```

## Module Organization

```text
src/
  core/
    models/             # All from 00-core-domain-model.md (NOT redefined here)
    ports/
      analyzer.rs       # ReviewAnalyzer trait, ReviewContext, ContextKind
      review_store.rs   # ReviewStore trait
      agent.rs          # AgentConfig, AgentRuntime (from 03)
      vcs.rs            # VersionControl trait
    services/
      pipeline.rs       # ReviewPipeline, AnalyzerEntry, PipelineConfig
  adapters/
    analyzers/
      mod.rs            # Re-exports
      convention.rs     # ConventionAnalyzer
      formatting.rs     # FormattingAnalyzer, FormattingConfig
      co_change.rs      # CoChangeAnalyzer, CoChangeConfig
      script.rs         # ScriptAnalyzer
      agent_bridge.rs   # AgentAnalyzer (wraps AgentRuntime)
    review_store/
      json.rs           # JsonReviewStore
```
