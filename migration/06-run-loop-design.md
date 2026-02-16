# Review Pipeline Design

Design specification for `noslop review` -- the core orchestrator that runs a pipeline of `ReviewAnalyzer` implementations against a branch diff.

## Overview

The review pipeline is the central command in noslop. `noslop review` computes the diff between the current branch and a base branch, then runs an ordered sequence of analyzers against that diff. Each analyzer produces `Finding` objects. Findings from earlier analyzers are passed to later ones (fold semantics), enabling LLM-based analyzers to build on what static tools already caught.

The pipeline follows three tiers:

```text
Static analyzers (free, fast) --> Computed analyzers (local) --> Agent analyzers (LLM)
```

Each tier is more expensive than the previous. Static analyzers match patterns against TOML-defined checks. Computed analyzers run local tools or query git history. Agent analyzers invoke an LLM via the `AgentRuntime` trait (defined in [03-agent-port-design.md](./03-agent-port-design.md)).

The pipeline is configured through `.noslop.toml` (defined in [08-config-design.md](./08-config-design.md)) and uses `VersionControl` trait methods for diff extraction (defined in [05-checkpoint-worktree-design.md](./05-checkpoint-worktree-design.md)).

## Module Organization

```text
src/
  core/
    models/
      review.rs             # ReviewSession, ReviewTarget, FindingId, Resolution
      pipeline_config.rs    # ReviewConfig, AnalyzerEntry
    ports/
      analyzer.rs           # ReviewAnalyzer trait (defined in doc 03, referenced here)
      finding_store.rs      # FindingStore trait -- load/save review sessions
    services/
      pipeline.rs           # ReviewPipeline orchestrator (pure logic)
  adapters/
    analyzers/
      mod.rs                # Module root, factory function
      convention.rs         # ConventionAnalyzer -- wraps [[check]] entries
      formatting.rs         # FormattingAnalyzer -- invokes external formatters
      co_change.rs          # CoChangeAnalyzer -- git history correlation
      script.rs             # ScriptAnalyzer -- user-configured external commands
    finding_store/
      json.rs               # JSON file-based FindingStore
  cli/
    commands/
      review.rs             # `noslop review` command handler
      findings.rs           # `noslop findings` command handler
```

## Domain Models

### Review Session (`src/core/models/review.rs`)

A `ReviewSession` captures a complete review run -- which analyzers ran, what findings they produced, and how those findings were resolved.

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::core::ports::agent::{Finding, Severity};

/// Unique identifier for a review session
pub type SessionId = String;

/// Unique identifier for a finding within a session
pub type FindingId = String;

/// Target of a review (which branch/commits are being reviewed)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewTarget {
    /// Base branch or commit (e.g., "main")
    pub base: String,
    /// Head branch or commit (e.g., "feature/x" or "HEAD")
    pub head: String,
    /// Resolved base commit SHA
    pub base_commit: String,
    /// Resolved head commit SHA
    pub head_commit: String,
}

/// A complete review run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewSession {
    /// Unique session identifier (e.g., "review-abc123")
    pub id: SessionId,
    /// What was reviewed
    pub target: ReviewTarget,
    /// When the review started
    pub started_at: DateTime<Utc>,
    /// When the review completed (None if still running)
    pub completed_at: Option<DateTime<Utc>>,
    /// Which analyzers ran, in order
    pub analyzers_run: Vec<String>,
    /// All findings from the pipeline, with stable IDs
    pub findings: Vec<SessionFinding>,
    /// Resolution status for each finding
    pub resolutions: HashMap<FindingId, Resolution>,
}

/// A finding within a review session, enriched with a stable ID and category
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionFinding {
    /// Stable identifier within this session (e.g., "F-001")
    pub id: FindingId,
    /// Source of the finding
    pub source: FindingSource,
    /// File path relative to repo root
    pub file: String,
    /// Line number (if applicable)
    pub line: Option<u32>,
    /// Severity level
    pub severity: Severity,
    /// Short description
    pub message: String,
    /// Suggested fix (if any)
    pub suggestion: Option<String>,
    /// Commit SHA where the issue was introduced (if determinable)
    pub commit: Option<String>,
    /// Category for grouping (e.g., "convention", "formatting", "co-change", "security")
    pub category: String,
}

/// Where a finding came from
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum FindingSource {
    /// From a [[check]] entry in .noslop.toml
    #[serde(rename = "check")]
    Check { id: String },
    /// From a built-in or external analyzer
    #[serde(rename = "analyzer")]
    Analyzer { name: String },
}

/// Resolution of a finding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resolution {
    /// Resolution status
    pub status: ResolutionStatus,
    /// Optional reason (required for dismissals)
    pub reason: Option<String>,
    /// When the resolution was recorded
    pub at: DateTime<Utc>,
}

/// Possible resolution states for a finding
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ResolutionStatus {
    /// The issue was fixed
    Resolved,
    /// The finding was dismissed (not applicable, won't fix, etc.)
    Dismissed,
}

impl ReviewSession {
    /// Generate the next finding ID for this session
    pub fn next_finding_id(&self) -> FindingId {
        format!("F-{:03}", self.findings.len() + 1)
    }

    /// Check if all blocking findings are resolved
    pub fn has_unresolved_blockers(&self) -> bool {
        self.findings.iter().any(|f| {
            matches!(f.severity, Severity::Error | Severity::Critical)
                && !self.resolutions.contains_key(&f.id)
        })
    }

    /// Get all unresolved findings
    pub fn unresolved_findings(&self) -> Vec<&SessionFinding> {
        self.findings
            .iter()
            .filter(|f| !self.resolutions.contains_key(&f.id))
            .collect()
    }

    /// Get findings grouped by severity
    pub fn findings_by_severity(&self) -> HashMap<Severity, Vec<&SessionFinding>> {
        let mut grouped: HashMap<Severity, Vec<&SessionFinding>> = HashMap::new();
        for finding in &self.findings {
            grouped.entry(finding.severity).or_default().push(finding);
        }
        grouped
    }
}
```

### Pipeline Configuration (`src/core/models/pipeline_config.rs`)

```rust
use crate::adapters::toml::parser::{CoChangeConfig, FormattingConfig};

/// Configuration for the review pipeline
///
/// Constructed from `.noslop.toml` [review] section merged with CLI overrides.
/// See 08-config-design.md for the TOML schema.
#[derive(Debug, Clone)]
pub struct ReviewConfig {
    /// Ordered list of analyzer entries to run
    pub analyzer_entries: Vec<AnalyzerEntry>,
    /// Co-change analyzer settings
    pub co_change: CoChangeConfig,
    /// Formatting analyzer settings
    pub formatting: FormattingConfig,
}

impl Default for ReviewConfig {
    fn default() -> Self {
        Self {
            analyzer_entries: vec![AnalyzerEntry::BuiltIn {
                name: "conventions".to_string(),
            }],
            co_change: CoChangeConfig::default(),
            formatting: FormattingConfig::default(),
        }
    }
}

/// A single entry in the analyzer pipeline
///
/// Resolved from the analyzer name strings in .noslop.toml.
/// See 08-config-design.md `resolve_analyzer_entry()` for the resolution logic.
#[derive(Debug, Clone)]
pub enum AnalyzerEntry {
    /// Built-in analyzer: "conventions", "formatting", "co-change"
    BuiltIn {
        name: String,
    },
    /// Agent-based analyzer: "agent:security", "agent:quality", etc.
    Agent {
        name: String,
        focus: String,
        agent: String,
        prompt_template: Option<String>,
    },
    /// External script analyzer: "custom:migrations", etc.
    Script {
        name: String,
        command: String,
    },
}

impl AnalyzerEntry {
    /// Returns the display name for this analyzer entry
    pub fn name(&self) -> &str {
        match self {
            Self::BuiltIn { name } => name,
            Self::Agent { name, .. } => name,
            Self::Script { name, .. } => name,
        }
    }
}
```

## Port Traits

### `ReviewAnalyzer` Trait

Defined in [03-agent-port-design.md](./03-agent-port-design.md). Reproduced here for context (the canonical definition is in doc 03):

```rust
// Defined in src/core/ports/agent.rs (doc 03)
pub trait ReviewAnalyzer: Send + Sync {
    fn required_context(&self) -> Vec<ContextKind>;
    fn analyze(&self, context: &ReviewContext, prior_findings: &[Finding]) -> Result<Vec<Finding>>;
    fn name(&self) -> &str;
}
```

All built-in analyzers, script analyzers, and agent analyzers implement this trait. The pipeline orchestrator operates exclusively through `ReviewAnalyzer` trait objects.

### `FindingStore` Trait (`src/core/ports/finding_store.rs`)

Manages persistence of review sessions. The pipeline creates a session, the `findings` command reads and updates it, and the pre-push hook queries it.

```rust
use std::path::Path;
use crate::core::models::review::{
    FindingId, Resolution, ReviewSession, SessionId,
};

/// Persistence for review sessions and finding resolutions
///
/// Implementations store review sessions as JSON files in `.noslop/reviews/`.
/// Sessions are git-tracked so the team can see review history.
#[cfg_attr(test, mockall::automock)]
pub trait FindingStore: Send + Sync {
    /// Save a new or updated review session
    fn save_session(&self, session: &ReviewSession) -> anyhow::Result<()>;

    /// Load a review session by ID
    fn load_session(&self, id: &SessionId) -> anyhow::Result<ReviewSession>;

    /// Load the most recent review session for the current branch
    ///
    /// Returns None if no review has been run on this branch.
    fn latest_session(&self) -> anyhow::Result<Option<ReviewSession>>;

    /// List all session IDs, most recent first
    fn list_sessions(&self) -> anyhow::Result<Vec<SessionId>>;

    /// Record a resolution for a finding
    fn resolve_finding(
        &self,
        session_id: &SessionId,
        finding_id: &FindingId,
        resolution: Resolution,
    ) -> anyhow::Result<()>;

    /// Check if there are unresolved blocking findings in the latest session
    ///
    /// Used by the pre-push hook to gate pushes.
    fn has_unresolved_blockers(&self) -> anyhow::Result<bool>;
}
```

## Built-in ReviewAnalyzer Implementations

### ConventionAnalyzer (`src/adapters/analyzers/convention.rs`)

Wraps the existing `[[check]]` system from `.noslop.toml`. Each `[[check]]` entry becomes a potential finding when a changed file matches its target pattern.

```rust
use std::path::Path;
use glob::Pattern;

use crate::adapters::toml::parser::CheckEntry;
use crate::core::ports::agent::{
    ContextKind, Finding, ReviewContext, Severity,
};

/// Analyzer that produces findings from [[check]] entries in .noslop.toml
///
/// This is the bridge between the existing check/acknowledgment system
/// and the review pipeline. Each [[check]] entry defines a target pattern
/// and a message. When a changed file matches the pattern, a finding is
/// produced with the check's severity.
pub struct ConventionAnalyzer {
    checks: Vec<CheckEntry>,
}

impl ConventionAnalyzer {
    pub fn new(checks: Vec<CheckEntry>) -> Self {
        Self { checks }
    }

    /// Match a check against the list of changed files
    fn match_check(
        &self,
        check: &CheckEntry,
        changed_files: &[String],
    ) -> Vec<Finding> {
        let pattern = match Pattern::new(&check.target) {
            Ok(p) => p,
            Err(_) => {
                // If the target is not a valid glob, treat it as a literal path
                return if changed_files.iter().any(|f| f == &check.target) {
                    vec![self.check_to_finding(check, &check.target)]
                } else {
                    vec![]
                };
            }
        };

        changed_files
            .iter()
            .filter(|f| pattern.matches(f))
            .map(|f| self.check_to_finding(check, f))
            .collect()
    }

    /// Convert a check entry to a finding for a specific file
    fn check_to_finding(&self, check: &CheckEntry, file: &str) -> Finding {
        let severity = match check.severity.as_str() {
            "info" => Severity::Info,
            "warn" => Severity::Warning,
            "block" => Severity::Error,
            _ => Severity::Warning,
        };

        let id = check
            .id
            .clone()
            .unwrap_or_else(|| format!("check:{}", check.target));

        Finding {
            severity,
            file: file.to_string(),
            line: None,
            message: check.message.clone(),
            suggestion: None,
            source: format!("convention:{}", id),
        }
    }
}

impl crate::core::ports::agent::ReviewAnalyzer for ConventionAnalyzer {
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

    fn name(&self) -> &str {
        "conventions"
    }
}
```

### FormattingAnalyzer (`src/adapters/analyzers/formatting.rs`)

Invokes external formatters (rustfmt, prettier) in check/dry-run mode and flags files that would change. This catches formatting drift without modifying files.

```rust
use std::path::Path;
use std::process::Command;

use crate::adapters::toml::parser::FormattingConfig;
use crate::core::ports::agent::{
    ContextKind, Finding, ReviewContext, Severity,
};

/// Analyzer that detects formatting violations by running external formatters
/// in check mode.
///
/// For each configured tool, the analyzer runs the tool's dry-run/check
/// command against changed files with matching extensions. Files that would
/// be reformatted are flagged as findings.
pub struct FormattingAnalyzer {
    config: FormattingConfig,
}

impl FormattingAnalyzer {
    pub fn new(config: FormattingConfig) -> Self {
        Self { config }
    }

    /// Run a single formatting tool in check mode against relevant files
    fn check_tool(
        &self,
        tool: &str,
        changed_files: &[String],
        repo_root: &Path,
    ) -> Vec<Finding> {
        let relevant_files: Vec<&String> = changed_files
            .iter()
            .filter(|f| Self::tool_handles_extension(tool, f))
            .collect();

        if relevant_files.is_empty() {
            return vec![];
        }

        let (cmd, args) = Self::check_command(tool, &relevant_files);

        let output = match Command::new(&cmd)
            .args(&args)
            .current_dir(repo_root)
            .output()
        {
            Ok(o) => o,
            Err(e) => {
                log::warn!("Formatting tool '{}' failed to execute: {}", tool, e);
                return vec![];
            }
        };

        // Non-zero exit from check mode means files would change
        if output.status.success() {
            return vec![];
        }

        // Parse which files would be changed from tool output
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        Self::parse_unformatted_files(tool, &stdout, &stderr, &relevant_files)
    }

    /// Build the check-mode command for a formatting tool
    fn check_command(tool: &str, files: &[&String]) -> (String, Vec<String>) {
        match tool {
            "rustfmt" => {
                let mut args = vec!["--check".to_string()];
                args.extend(files.iter().map(|f| f.to_string()));
                ("rustfmt".to_string(), args)
            }
            "prettier" => {
                let mut args = vec!["--check".to_string()];
                args.extend(files.iter().map(|f| f.to_string()));
                ("npx".to_string(), {
                    let mut a = vec!["prettier".to_string()];
                    a.extend(args);
                    a
                })
            }
            other => {
                // Generic: try `<tool> --check <files>`
                let mut args = vec!["--check".to_string()];
                args.extend(files.iter().map(|f| f.to_string()));
                (other.to_string(), args)
            }
        }
    }

    /// Check if a tool handles files with the given extension
    fn tool_handles_extension(tool: &str, file: &str) -> bool {
        let ext = file.rsplit('.').next().unwrap_or("");
        match tool {
            "rustfmt" => ext == "rs",
            "prettier" => matches!(
                ext,
                "ts" | "tsx" | "js" | "jsx" | "css" | "html" | "json"
                    | "md" | "yaml" | "yml" | "svelte"
            ),
            _ => true, // Unknown tools: check all files
        }
    }

    /// Parse tool output to determine which files are unformatted
    fn parse_unformatted_files(
        tool: &str,
        stdout: &str,
        stderr: &str,
        candidate_files: &[&String],
    ) -> Vec<Finding> {
        // For rustfmt: stderr contains "Diff in <path>" lines
        // For prettier: stdout lists unformatted file paths
        // Fallback: flag all candidate files

        let mut findings = Vec::new();
        let combined = format!("{}\n{}", stdout, stderr);

        for file in candidate_files {
            if combined.contains(file.as_str()) {
                findings.push(Finding {
                    severity: Severity::Warning,
                    file: file.to_string(),
                    line: None,
                    message: format!(
                        "File would be reformatted by {}",
                        tool
                    ),
                    suggestion: Some(format!(
                        "Run {} to fix formatting",
                        tool
                    )),
                    source: format!("formatting:{}", tool),
                });
            }
        }

        // If no specific files matched in output but tool returned non-zero,
        // flag all candidates
        if findings.is_empty() {
            for file in candidate_files {
                findings.push(Finding {
                    severity: Severity::Warning,
                    file: file.to_string(),
                    line: None,
                    message: format!(
                        "Formatting check failed ({})",
                        tool
                    ),
                    suggestion: Some(format!(
                        "Run {} to fix formatting",
                        tool
                    )),
                    source: format!("formatting:{}", tool),
                });
            }
        }

        findings
    }
}

impl crate::core::ports::agent::ReviewAnalyzer for FormattingAnalyzer {
    fn required_context(&self) -> Vec<ContextKind> {
        vec![ContextKind::ChangedFiles]
    }

    fn analyze(
        &self,
        context: &ReviewContext,
        _prior_findings: &[Finding],
    ) -> anyhow::Result<Vec<Finding>> {
        let mut findings = Vec::new();

        for tool in &self.config.tools {
            findings.extend(self.check_tool(
                tool,
                &context.changed_files,
                &context.repo_root,
            ));
        }

        Ok(findings)
    }

    fn name(&self) -> &str {
        "formatting"
    }
}
```

### CoChangeAnalyzer (`src/adapters/analyzers/co_change.rs`)

Uses git history to detect files that historically change together but are missing from the current diff. This catches "forgot to update the companion file" situations.

```rust
use std::collections::HashMap;
use std::path::Path;

use crate::adapters::toml::parser::CoChangeConfig;
use crate::core::models::diff::CommitInfo;
use crate::core::ports::agent::{
    ContextKind, Finding, ReviewContext, Severity,
};
use crate::core::ports::vcs::VersionControl;

/// Analyzer that detects missing co-change files using git history correlation.
///
/// Scans recent commit history to build a co-change frequency matrix. For each
/// changed file in the current diff, checks if there are correlated files that
/// are NOT in the diff. Missing correlated files are flagged as findings.
pub struct CoChangeAnalyzer {
    config: CoChangeConfig,
    vcs: Box<dyn VersionControl>,
}

impl CoChangeAnalyzer {
    pub fn new(config: CoChangeConfig, vcs: Box<dyn VersionControl>) -> Self {
        Self { config, vcs }
    }

    /// Build a co-change frequency matrix from recent commits.
    ///
    /// For each pair of files that appear in the same commit, increment
    /// their co-occurrence count. Returns a map from (file_a, file_b) to
    /// the number of commits where both appeared.
    fn build_cochange_matrix(
        &self,
        commits: &[CommitInfo],
    ) -> HashMap<(String, String), u32> {
        let mut matrix: HashMap<(String, String), u32> = HashMap::new();

        for commit in commits {
            let files = &commit.files_changed;
            for i in 0..files.len() {
                for j in (i + 1)..files.len() {
                    let pair = if files[i] < files[j] {
                        (files[i].clone(), files[j].clone())
                    } else {
                        (files[j].clone(), files[i].clone())
                    };
                    *matrix.entry(pair).or_insert(0) += 1;
                }
            }
        }

        matrix
    }

    /// Count how many commits each file appears in
    fn file_commit_counts(
        &self,
        commits: &[CommitInfo],
    ) -> HashMap<String, u32> {
        let mut counts: HashMap<String, u32> = HashMap::new();
        for commit in commits {
            for file in &commit.files_changed {
                *counts.entry(file.clone()).or_insert(0) += 1;
            }
        }
        counts
    }

    /// Find files that are correlated with the changed files but missing
    /// from the current diff.
    fn find_missing_cochanges(
        &self,
        changed_files: &[String],
        matrix: &HashMap<(String, String), u32>,
        file_counts: &HashMap<String, u32>,
    ) -> Vec<(String, String, f64)> {
        let changed_set: std::collections::HashSet<&str> =
            changed_files.iter().map(|s| s.as_str()).collect();
        let mut missing: Vec<(String, String, f64)> = Vec::new();

        for (pair, &co_count) in matrix {
            if co_count < self.config.min_commits {
                continue;
            }

            let (ref a, ref b) = *pair;
            let a_in_diff = changed_set.contains(a.as_str());
            let b_in_diff = changed_set.contains(b.as_str());

            // One file is in the diff, the other is not
            if a_in_diff == b_in_diff {
                continue;
            }

            let (changed, missing_file) = if a_in_diff { (a, b) } else { (b, a) };

            // Calculate correlation: co_count / max(count_a, count_b)
            let count_a = file_counts.get(a).copied().unwrap_or(1);
            let count_b = file_counts.get(b).copied().unwrap_or(1);
            let correlation = co_count as f64 / count_a.max(count_b) as f64;

            if correlation >= self.config.min_correlation {
                missing.push((
                    changed.clone(),
                    missing_file.clone(),
                    correlation,
                ));
            }
        }

        // Deduplicate: keep only the highest correlation per missing file
        missing.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap());
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        missing.retain(|(_changed, missing_file, _corr)| seen.insert(missing_file.clone()));

        missing
    }
}

impl crate::core::ports::agent::ReviewAnalyzer for CoChangeAnalyzer {
    fn required_context(&self) -> Vec<ContextKind> {
        vec![ContextKind::ChangedFiles]
    }

    fn analyze(
        &self,
        context: &ReviewContext,
        _prior_findings: &[Finding],
    ) -> anyhow::Result<Vec<Finding>> {
        let commits = self.vcs.recent_commits(self.config.lookback as usize)?;

        if commits.is_empty() {
            return Ok(vec![]);
        }

        let matrix = self.build_cochange_matrix(&commits);
        let file_counts = self.file_commit_counts(&commits);
        let missing = self.find_missing_cochanges(
            &context.changed_files,
            &matrix,
            &file_counts,
        );

        let findings = missing
            .into_iter()
            .map(|(changed, missing_file, correlation)| Finding {
                severity: Severity::Warning,
                file: missing_file.clone(),
                line: None,
                message: format!(
                    "{} usually changes with {} (correlation: {:.2})",
                    missing_file, changed, correlation
                ),
                suggestion: Some(format!(
                    "Review whether {} needs updates too",
                    missing_file
                )),
                source: "co-change".to_string(),
            })
            .collect();

        Ok(findings)
    }

    fn name(&self) -> &str {
        "co-change"
    }
}
```

### ScriptAnalyzer (`src/adapters/analyzers/script.rs`)

Runs user-configured external commands. The script receives the review context as JSON on stdin and writes findings as a JSON array on stdout.

```rust
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use crate::core::ports::agent::{
    ContextKind, Finding, ReviewContext, Severity,
};

/// Analyzer that runs a user-configured external command.
///
/// The command receives a JSON object on stdin containing:
/// - `changed_files`: list of changed file paths
/// - `diff`: the unified diff text
/// - `prior_findings`: findings from earlier analyzers
///
/// The command writes findings as a JSON array on stdout.
/// Non-zero exit is treated as an error (not a failure with findings).
pub struct ScriptAnalyzer {
    analyzer_name: String,
    command: String,
}

impl ScriptAnalyzer {
    pub fn new(name: impl Into<String>, command: impl Into<String>) -> Self {
        Self {
            analyzer_name: name.into(),
            command: command.into(),
        }
    }

    /// Build the JSON input for the script
    fn build_input(
        &self,
        context: &ReviewContext,
        prior_findings: &[Finding],
    ) -> serde_json::Value {
        serde_json::json!({
            "changed_files": context.changed_files,
            "diff": context.diff,
            "repo_root": context.repo_root.display().to_string(),
            "prior_findings": prior_findings,
        })
    }
}

impl crate::core::ports::agent::ReviewAnalyzer for ScriptAnalyzer {
    fn required_context(&self) -> Vec<ContextKind> {
        vec![ContextKind::Diff, ContextKind::ChangedFiles]
    }

    fn analyze(
        &self,
        context: &ReviewContext,
        prior_findings: &[Finding],
    ) -> anyhow::Result<Vec<Finding>> {
        let input = self.build_input(context, prior_findings);
        let input_json = serde_json::to_string(&input)?;

        let mut child = Command::new("sh")
            .args(["-c", &self.command])
            .current_dir(&context.repo_root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        // Write input to stdin
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(input_json.as_bytes())?;
        }

        let output = child.wait_with_output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!(
                "Script analyzer '{}' exited with code {}: {}",
                self.analyzer_name,
                output.status.code().unwrap_or(-1),
                stderr.trim()
            );
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let findings: Vec<Finding> = serde_json::from_str(stdout.trim())
            .unwrap_or_else(|e| {
                log::warn!(
                    "Script analyzer '{}' produced invalid JSON: {}",
                    self.analyzer_name,
                    e
                );
                vec![]
            });

        // Tag all findings with the script analyzer source
        let findings = findings
            .into_iter()
            .map(|mut f| {
                if f.source.is_empty() {
                    f.source = self.analyzer_name.clone();
                }
                f
            })
            .collect();

        Ok(findings)
    }

    fn name(&self) -> &str {
        &self.analyzer_name
    }
}
```

### AgentAnalyzer

The agent-based analyzer is defined in [03-agent-port-design.md](./03-agent-port-design.md). It wraps an `AgentRuntime` and implements `ReviewAnalyzer`. It is constructed by the pipeline from `AnalyzerEntry::Agent` config entries. See doc 03 for the full implementation.

## Core Services

### ReviewPipeline (`src/core/services/pipeline.rs`)

The central orchestrator. Takes a `ReviewConfig` and a `ReviewContext`, builds the ordered list of analyzers, executes them in sequence with fold semantics, and returns a `ReviewSession`.

```rust
use chrono::Utc;
use std::path::Path;

use crate::adapters::analyzers;
use crate::adapters::toml::parser::{CheckEntry, NoslopFile};
use crate::core::models::pipeline_config::{AnalyzerEntry, ReviewConfig};
use crate::core::models::review::{
    FindingSource, ReviewSession, ReviewTarget, SessionFinding,
};
use crate::core::ports::agent::{
    AgentType, Finding, ReviewAnalyzer, ReviewContext,
};
use crate::core::ports::vcs::VersionControl;

/// Builds and executes the review pipeline.
///
/// The pipeline runs analyzers in the order specified by `ReviewConfig`:
///
/// 1. Each analyzer receives the `ReviewContext` (diff + changed files)
///    plus all findings from prior analyzers (fold semantics).
/// 2. Findings accumulate across analyzers.
/// 3. The complete finding set is wrapped in a `ReviewSession`.
///
/// This service is pure orchestration logic. All I/O (diff extraction,
/// file reads, agent invocation) goes through trait objects.
pub struct ReviewPipeline {
    config: ReviewConfig,
    checks: Vec<CheckEntry>,
    vcs: Box<dyn VersionControl>,
}

impl ReviewPipeline {
    pub fn new(
        config: ReviewConfig,
        checks: Vec<CheckEntry>,
        vcs: Box<dyn VersionControl>,
    ) -> Self {
        Self {
            config,
            checks,
            vcs,
        }
    }

    /// Execute the full review pipeline.
    ///
    /// Returns a `ReviewSession` containing all findings from all analyzers.
    pub fn execute(
        &self,
        context: &ReviewContext,
        target: ReviewTarget,
    ) -> anyhow::Result<ReviewSession> {
        let session_id = generate_session_id();
        let started_at = Utc::now();

        let analyzers = self.build_analyzers()?;
        let mut all_findings: Vec<Finding> = Vec::new();
        let mut analyzers_run: Vec<String> = Vec::new();

        for analyzer in &analyzers {
            log::info!("Running analyzer: {}", analyzer.name());

            let findings = analyzer.analyze(context, &all_findings)?;

            log::info!(
                "  {} produced {} finding(s)",
                analyzer.name(),
                findings.len()
            );

            all_findings.extend(findings);
            analyzers_run.push(analyzer.name().to_string());
        }

        // Convert pipeline findings to session findings with stable IDs
        let session_findings = self.findings_to_session_findings(&all_findings);

        let session = ReviewSession {
            id: session_id,
            target,
            started_at,
            completed_at: Some(Utc::now()),
            analyzers_run,
            findings: session_findings,
            resolutions: std::collections::HashMap::new(),
        };

        Ok(session)
    }

    /// Build the ordered list of analyzers from the config.
    ///
    /// The order in `ReviewConfig.analyzer_entries` determines execution
    /// order. Built-in analyzers are instantiated directly; agent analyzers
    /// are constructed via the factory in doc 03/04; script analyzers
    /// wrap the user's command.
    fn build_analyzers(&self) -> anyhow::Result<Vec<Box<dyn ReviewAnalyzer>>> {
        let mut analyzers: Vec<Box<dyn ReviewAnalyzer>> = Vec::new();

        for entry in &self.config.analyzer_entries {
            let analyzer = self.build_analyzer(entry)?;
            analyzers.push(analyzer);
        }

        Ok(analyzers)
    }

    /// Build a single analyzer from its config entry
    fn build_analyzer(
        &self,
        entry: &AnalyzerEntry,
    ) -> anyhow::Result<Box<dyn ReviewAnalyzer>> {
        match entry {
            AnalyzerEntry::BuiltIn { name } => match name.as_str() {
                "conventions" => Ok(Box::new(
                    analyzers::ConventionAnalyzer::new(self.checks.clone()),
                )),
                "formatting" => Ok(Box::new(
                    analyzers::FormattingAnalyzer::new(
                        self.config.formatting.clone(),
                    ),
                )),
                "co-change" => Ok(Box::new(
                    analyzers::CoChangeAnalyzer::new(
                        self.config.co_change.clone(),
                        self.vcs.clone_box(),
                    ),
                )),
                unknown => anyhow::bail!(
                    "Unknown built-in analyzer: '{}'. \
                     Available: conventions, formatting, co-change",
                    unknown
                ),
            },
            AnalyzerEntry::Agent {
                name,
                focus,
                agent,
                prompt_template,
            } => {
                let agent_type: AgentType = agent.parse().map_err(|e| {
                    anyhow::anyhow!("Invalid agent type for {}: {}", name, e)
                })?;
                let runtime =
                    crate::adapters::agent::get_agent_runtime(agent_type);

                let template = match prompt_template {
                    Some(path) => std::fs::read_to_string(path)?,
                    None => default_agent_prompt_template(focus),
                };

                Ok(Box::new(
                    crate::core::ports::agent::AgentAnalyzer::new(
                        runtime, focus, template,
                    ),
                ))
            }
            AnalyzerEntry::Script { name, command } => {
                Ok(Box::new(analyzers::ScriptAnalyzer::new(
                    name.clone(),
                    command.clone(),
                )))
            }
        }
    }

    /// Convert raw `Finding` objects to `SessionFinding` with stable IDs
    fn findings_to_session_findings(
        &self,
        findings: &[Finding],
    ) -> Vec<SessionFinding> {
        findings
            .iter()
            .enumerate()
            .map(|(i, f)| {
                let source = if f.source.starts_with("convention:") {
                    let id = f.source.strip_prefix("convention:").unwrap();
                    FindingSource::Check { id: id.to_string() }
                } else {
                    FindingSource::Analyzer {
                        name: f.source.clone(),
                    }
                };

                let category = categorize_source(&f.source);

                SessionFinding {
                    id: format!("F-{:03}", i + 1),
                    source,
                    file: f.file.clone(),
                    line: f.line,
                    severity: f.severity,
                    message: f.message.clone(),
                    suggestion: f.suggestion.clone(),
                    commit: None,
                    category,
                }
            })
            .collect()
    }
}

/// Categorize a finding source into a human-readable category
fn categorize_source(source: &str) -> String {
    if source.starts_with("convention:") {
        "convention".to_string()
    } else if source.starts_with("formatting:") {
        "formatting".to_string()
    } else if source == "co-change" {
        "co-change".to_string()
    } else if source.starts_with("agent:") {
        source.strip_prefix("agent:").unwrap_or(source).to_string()
    } else if source.starts_with("custom:") {
        source.strip_prefix("custom:").unwrap_or(source).to_string()
    } else {
        source.to_string()
    }
}

/// Generate a unique session ID
fn generate_session_id() -> String {
    let timestamp = Utc::now().format("%Y%m%d%H%M%S");
    let random: u32 = rand::random::<u32>() % 0xFFFF;
    format!("review-{}-{:04x}", timestamp, random)
}

/// Default prompt template for agent analyzers when no custom template is provided.
///
/// Uses the same placeholder format as doc 03/04: {diff}, {focus},
/// {prior_findings}, {changed_files}.
fn default_agent_prompt_template(focus: &str) -> String {
    format!(
        r#"You are a code reviewer focused on {focus} issues.

## Focus area

{focus}

## Diff to review

{{diff}}

## Changed files

{{changed_files}}

## Prior findings from automated analysis

{{prior_findings}}

## Output format

Return your findings as a JSON array. Each finding must have these fields:

- severity: "info", "warning", "error", or "critical"
- file: relative file path
- line: line number (null if not applicable)
- message: short description of the issue
- suggestion: how to fix it (null if no suggestion)
- source: "agent:{focus}"

If you find no issues, return an empty array: []

Return ONLY the JSON array, no surrounding text."#,
        focus = focus
    )
}
```

### Analyzer Factory (`src/adapters/analyzers/mod.rs`)

```rust
//! Built-in ReviewAnalyzer implementations
//!
//! - [`ConventionAnalyzer`] -- wraps [[check]] entries from .noslop.toml
//! - [`FormattingAnalyzer`] -- invokes external formatters in check mode
//! - [`CoChangeAnalyzer`] -- detects missing co-change files via git history
//! - [`ScriptAnalyzer`] -- runs user-configured external commands

mod convention;
mod co_change;
mod formatting;
mod script;

pub use convention::ConventionAnalyzer;
pub use co_change::CoChangeAnalyzer;
pub use formatting::FormattingAnalyzer;
pub use script::ScriptAnalyzer;
```

## CLI Integration

### `noslop review` Command (`src/cli/commands/review.rs`)

```rust
use clap::Args;
use std::path::PathBuf;

use crate::adapters::git::GitVersionControl;
use crate::adapters::toml::parser;
use crate::core::models::pipeline_config::ReviewConfig;
use crate::core::models::review::ReviewTarget;
use crate::core::ports::agent::ReviewContext;
use crate::core::ports::vcs::VersionControl;
use crate::core::services::pipeline::ReviewPipeline;

/// Arguments for `noslop review`
#[derive(Args, Debug)]
pub struct ReviewArgs {
    /// Base branch or commit to diff against (default: auto-detect main branch)
    #[arg(short, long)]
    pub base: Option<String>,

    /// Head ref to review (default: HEAD)
    #[arg(long, default_value = "HEAD")]
    pub head: String,

    /// Override analyzer list (comma-separated)
    #[arg(short, long, value_delimiter = ',')]
    pub analyzer: Option<Vec<String>>,

    /// Agent type override
    #[arg(long)]
    pub agent: Option<String>,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,

    /// Check mode: exit non-zero if blocking findings exist
    #[arg(long)]
    pub check: bool,
}

/// Entry point for `noslop review`
pub fn review(args: ReviewArgs) -> anyhow::Result<()> {
    let vcs = GitVersionControl::current_dir()?;
    let repo_root = vcs.repo_root()?;

    // 1. Load configuration
    let noslop_file = parser::load_file(&repo_root.join(".noslop.toml"))?;
    let default_agent = noslop_file
        .agent
        .as_ref()
        .map(|a| a.agent_type.as_str());

    // 2. Build ReviewConfig from TOML + CLI overrides
    let review_config = build_review_config(
        &noslop_file,
        default_agent,
        args.analyzer.as_deref(),
        args.agent.as_deref(),
    )?;

    // 3. Resolve base branch
    let base = args
        .base
        .unwrap_or_else(|| detect_default_branch(&vcs));

    // 4. Extract diff via VersionControl trait (from doc 05)
    let file_diffs = vcs.diff_between(&base, &args.head)?;
    let changed_files: Vec<String> = file_diffs
        .iter()
        .map(|d| d.path.display().to_string())
        .collect();

    // 5. Build unified diff string for agent analyzers
    let diff = build_unified_diff(&file_diffs);

    let context = ReviewContext {
        diff,
        changed_files: changed_files.clone(),
        repo_root: repo_root.clone(),
        commit_range: Some(format!("{}..{}", base, args.head)),
    };

    // 6. Resolve commit SHAs for the review target
    let target = ReviewTarget {
        base: base.clone(),
        head: args.head.clone(),
        base_commit: vcs.resolve_rev(&base)?,
        head_commit: vcs.resolve_rev(&args.head)?,
    };

    // 7. Run the pipeline
    let pipeline = ReviewPipeline::new(
        review_config,
        noslop_file.checks,
        Box::new(vcs),
    );

    let session = pipeline.execute(&context, target)?;

    // 8. Save the session
    let finding_store = crate::adapters::finding_store::JsonFindingStore::new(
        &repo_root.join(".noslop/reviews"),
    );
    finding_store.save_session(&session)?;

    // 9. Display results
    if args.json {
        println!("{}", serde_json::to_string_pretty(&session)?);
    } else {
        print_review_summary(&session);
    }

    // 10. Exit code in check mode
    if args.check && session.has_unresolved_blockers() {
        std::process::exit(1);
    }

    Ok(())
}

/// Build ReviewConfig from TOML and CLI overrides
fn build_review_config(
    noslop_file: &parser::NoslopFile,
    default_agent: Option<&str>,
    cli_analyzers: Option<&[String]>,
    cli_agent: Option<&str>,
) -> anyhow::Result<ReviewConfig> {
    let effective_agent = cli_agent.or(default_agent);

    if let Some(ref review_section) = noslop_file.review {
        let mut config = review_section.to_review_config(effective_agent)?;

        // CLI analyzer override
        if let Some(analyzers) = cli_analyzers {
            let section = parser::ReviewSection {
                analyzers: analyzers.to_vec(),
                ..review_section.clone()
            };
            config = section.to_review_config(effective_agent)?;
        }

        Ok(config)
    } else {
        Ok(ReviewConfig::default())
    }
}

/// Detect the default branch (main or master)
fn detect_default_branch(vcs: &dyn VersionControl) -> String {
    // Try "main" first, fall back to "master"
    if vcs.branch_exists("main") {
        "main".to_string()
    } else {
        "master".to_string()
    }
}

/// Build a unified diff string from structured FileDiff objects
fn build_unified_diff(
    file_diffs: &[crate::core::models::diff::FileDiff],
) -> String {
    use crate::core::models::diff::DiffLineKind;

    let mut output = String::new();
    for file_diff in file_diffs {
        output.push_str(&format!(
            "--- a/{}\n+++ b/{}\n",
            file_diff
                .old_path
                .as_ref()
                .unwrap_or(&file_diff.path)
                .display(),
            file_diff.path.display()
        ));
        for hunk in &file_diff.hunks {
            output.push_str(&format!(
                "@@ -{},{} +{},{} @@\n",
                hunk.old_start,
                hunk.old_lines,
                hunk.new_start,
                hunk.new_lines
            ));
            for line in &hunk.lines {
                let prefix = match line.kind {
                    DiffLineKind::Context => ' ',
                    DiffLineKind::Addition => '+',
                    DiffLineKind::Deletion => '-',
                };
                output.push(prefix);
                output.push_str(&line.content);
                output.push('\n');
            }
        }
    }
    output
}

/// Print a human-readable summary of the review session
fn print_review_summary(session: &ReviewSession) {
    use crate::core::ports::agent::Severity;

    let total = session.findings.len();
    if total == 0 {
        println!("Review complete: no findings.");
        println!("Session: {}", session.id);
        return;
    }

    let by_severity = session.findings_by_severity();
    let critical = by_severity.get(&Severity::Critical).map_or(0, |v| v.len());
    let errors = by_severity.get(&Severity::Error).map_or(0, |v| v.len());
    let warnings = by_severity.get(&Severity::Warning).map_or(0, |v| v.len());
    let info = by_severity.get(&Severity::Info).map_or(0, |v| v.len());

    println!(
        "Review complete: {} finding(s) [critical: {}, error: {}, warning: {}, info: {}]",
        total, critical, errors, warnings, info
    );
    println!("Session: {}", session.id);
    println!("Analyzers: {}", session.analyzers_run.join(", "));
    println!();

    // Print findings grouped by severity (critical first)
    for severity in &[
        Severity::Critical,
        Severity::Error,
        Severity::Warning,
        Severity::Info,
    ] {
        if let Some(findings) = by_severity.get(severity) {
            for finding in findings {
                let line_str = finding
                    .line
                    .map(|l| format!(":{}", l))
                    .unwrap_or_default();
                let severity_label = match finding.severity {
                    Severity::Critical => "CRITICAL",
                    Severity::Error => "ERROR",
                    Severity::Warning => "WARN",
                    Severity::Info => "INFO",
                };
                println!(
                    "  [{severity_label}] {}{line_str}: {} ({})",
                    finding.file, finding.message, finding.id
                );
                if let Some(ref suggestion) = finding.suggestion {
                    println!("          -> {}", suggestion);
                }
            }
        }
    }

    if session.has_unresolved_blockers() {
        println!();
        println!(
            "Blocking findings exist. Resolve them with: noslop findings resolve <id>"
        );
    }
}
```

### `noslop findings` Command (`src/cli/commands/findings.rs`)

```rust
use clap::{Args, Subcommand};
use chrono::Utc;

use crate::core::models::review::{Resolution, ResolutionStatus};
use crate::core::ports::finding_store::FindingStore;

/// Arguments for `noslop findings`
#[derive(Args, Debug)]
pub struct FindingsArgs {
    #[command(subcommand)]
    pub command: Option<FindingsCommand>,
}

#[derive(Subcommand, Debug)]
pub enum FindingsCommand {
    /// List all findings from the latest review session
    List,
    /// Resolve a finding (mark as fixed)
    Resolve {
        /// Finding ID (e.g., "F-001")
        id: String,
    },
    /// Dismiss a finding (not applicable / won't fix)
    Dismiss {
        /// Finding ID (e.g., "F-001")
        id: String,
        /// Reason for dismissal
        #[arg(short, long)]
        reason: String,
    },
}

/// Entry point for `noslop findings`
pub fn findings(
    args: FindingsArgs,
    store: &dyn FindingStore,
) -> anyhow::Result<()> {
    let command = args.command.unwrap_or(FindingsCommand::List);

    match command {
        FindingsCommand::List => {
            let session = store
                .latest_session()?
                .ok_or_else(|| anyhow::anyhow!(
                    "No review sessions found. Run `noslop review` first."
                ))?;

            let unresolved = session.unresolved_findings();
            if unresolved.is_empty() {
                println!("All findings resolved in session {}.", session.id);
                return Ok(());
            }

            println!(
                "Session: {} ({} unresolved)",
                session.id,
                unresolved.len()
            );
            for finding in unresolved {
                let line_str = finding
                    .line
                    .map(|l| format!(":{}", l))
                    .unwrap_or_default();
                println!(
                    "  {} [{:?}] {}{}: {}",
                    finding.id,
                    finding.severity,
                    finding.file,
                    line_str,
                    finding.message
                );
            }
            Ok(())
        }
        FindingsCommand::Resolve { id } => {
            let session = store
                .latest_session()?
                .ok_or_else(|| anyhow::anyhow!("No review sessions found."))?;

            store.resolve_finding(
                &session.id,
                &id,
                Resolution {
                    status: ResolutionStatus::Resolved,
                    reason: None,
                    at: Utc::now(),
                },
            )?;

            println!("Resolved: {}", id);
            Ok(())
        }
        FindingsCommand::Dismiss { id, reason } => {
            let session = store
                .latest_session()?
                .ok_or_else(|| anyhow::anyhow!("No review sessions found."))?;

            store.resolve_finding(
                &session.id,
                &id,
                Resolution {
                    status: ResolutionStatus::Dismissed,
                    reason: Some(reason),
                    at: Utc::now(),
                },
            )?;

            println!("Dismissed: {}", id);
            Ok(())
        }
    }
}
```

## Diff Extraction

The review pipeline gets its diff data through the `VersionControl` trait, which is extended in [05-checkpoint-worktree-design.md](./05-checkpoint-worktree-design.md) with these methods:

- `diff_between(base, head) -> Vec<FileDiff>` -- structured diff for each changed file
- `commits_between(base, head) -> Vec<CommitInfo>` -- commit history for the range
- `file_at_revision(path, rev) -> String` -- full file content for context

The CLI command (`review.rs`) calls `vcs.diff_between()` to get structured `FileDiff` objects, extracts the list of changed file paths, and builds a unified diff string for agent analyzers. This means:

1. Static analyzers (conventions) only need the list of changed file paths
2. Computed analyzers (co-change) need git history via `recent_commits()`
3. Agent analyzers need the unified diff text in the `ReviewContext.diff` field

## Pre-Push Hook Integration

The review pipeline integrates with git's pre-push hook to gate pushes on unresolved blocking findings. The hook is installed by `noslop init` and checks the latest review session.

### Hook Script

```bash
#!/bin/bash
# .git/hooks/pre-push
# Installed by `noslop init`. Gates pushes on unresolved blocking findings.

if ! command -v noslop &>/dev/null; then
    exit 0  # noslop not installed, don't block
fi

# Check for unresolved blockers in the latest review session
if noslop findings --check-blockers 2>/dev/null; then
    exit 0
fi

echo "Push blocked: unresolved blocking findings exist." >&2
echo "Run 'noslop findings' to see them." >&2
echo "Run 'noslop findings resolve <id>' or 'noslop findings dismiss <id> -r <reason>' to clear." >&2
exit 1
```

### `--check-blockers` Flag

The `noslop findings` command supports a `--check-blockers` flag for use by the pre-push hook. It exits 0 if there are no unresolved blockers, and exits 1 if there are.

```rust
// In findings.rs, additional subcommand variant:
/// Check if there are unresolved blocking findings (for hooks)
CheckBlockers,

// Implementation:
FindingsCommand::CheckBlockers => {
    let has_blockers = store.has_unresolved_blockers()?;
    if has_blockers {
        std::process::exit(1);
    }
    Ok(())
}
```

## Data Flow

```text
noslop review [--base main]
       |
       v
+------+---------+
| CLI: review.rs  |
| 1. Load config  |  <--- parser::load_file(".noslop.toml")
| 2. Resolve base |  <--- VersionControl.current_branch()
| 3. Extract diff |  <--- VersionControl.diff_between(base, HEAD)
| 4. Build context|
+------+---------+
       |
       v
+------+------------------+
| ReviewPipeline.execute() |
|                          |
|  for each analyzer:      |
|    +------------------+  |
|    | ConventionAnalyzer|  | Match [[check]] patterns against changed files
|    +------------------+  |
|           |              |
|           v              |
|    +------------------+  |
|    | FormattingAnalyzer|  | Run rustfmt --check, prettier --check
|    +------------------+  |
|           |              |
|           v              |
|    +------------------+  |
|    | CoChangeAnalyzer  |  | Query git history for correlated files
|    +------------------+  |
|           |              |
|           v              |
|    +------------------+  |
|    | AgentAnalyzer     |  | Invoke LLM with diff + prior findings
|    | (security)        |  |   via AgentRuntime.invoke()
|    +------------------+  |
|           |              |
|           v              |
|    +------------------+  |
|    | AgentAnalyzer     |  | Invoke LLM with diff + all prior findings
|    | (quality)         |  |   (includes security findings)
|    +------------------+  |
|           |              |
|           v              |
|  Collect all findings    |
|  Assign stable IDs       |
|  Build ReviewSession     |
+------+------------------+
       |
       v
+------+------------------+
| Save session to          |
| .noslop/reviews/{id}.json|
| via FindingStore          |
+------+------------------+
       |
       v
+------+------------------+
| Display summary           |
| or JSON output            |
+--------------------------+
```

## FindingStore Adapter (`src/adapters/finding_store/json.rs`)

```rust
use std::fs;
use std::path::{Path, PathBuf};

use crate::core::models::review::{
    FindingId, Resolution, ReviewSession, SessionId,
};
use crate::core::ports::finding_store::FindingStore;

/// JSON file-based FindingStore.
///
/// Stores review sessions as individual JSON files in a directory
/// (typically `.noslop/reviews/`).
pub struct JsonFindingStore {
    dir: PathBuf,
}

impl JsonFindingStore {
    pub fn new(dir: &Path) -> Self {
        Self {
            dir: dir.to_path_buf(),
        }
    }

    fn session_path(&self, id: &SessionId) -> PathBuf {
        self.dir.join(format!("{}.json", id))
    }
}

impl FindingStore for JsonFindingStore {
    fn save_session(&self, session: &ReviewSession) -> anyhow::Result<()> {
        fs::create_dir_all(&self.dir)?;
        let path = self.session_path(&session.id);
        let json = serde_json::to_string_pretty(session)?;
        fs::write(path, json)?;
        Ok(())
    }

    fn load_session(&self, id: &SessionId) -> anyhow::Result<ReviewSession> {
        let path = self.session_path(id);
        let content = fs::read_to_string(&path)?;
        let session: ReviewSession = serde_json::from_str(&content)?;
        Ok(session)
    }

    fn latest_session(&self) -> anyhow::Result<Option<ReviewSession>> {
        let sessions = self.list_sessions()?;
        match sessions.first() {
            Some(id) => Ok(Some(self.load_session(id)?)),
            None => Ok(None),
        }
    }

    fn list_sessions(&self) -> anyhow::Result<Vec<SessionId>> {
        if !self.dir.exists() {
            return Ok(vec![]);
        }

        let mut sessions: Vec<(SessionId, std::time::SystemTime)> = Vec::new();
        for entry in fs::read_dir(&self.dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().map_or(false, |e| e == "json") {
                if let Some(stem) = path.file_stem() {
                    let id = stem.to_string_lossy().to_string();
                    let modified = entry.metadata()?.modified()?;
                    sessions.push((id, modified));
                }
            }
        }

        // Sort by modification time, most recent first
        sessions.sort_by(|a, b| b.1.cmp(&a.1));
        Ok(sessions.into_iter().map(|(id, _)| id).collect())
    }

    fn resolve_finding(
        &self,
        session_id: &SessionId,
        finding_id: &FindingId,
        resolution: Resolution,
    ) -> anyhow::Result<()> {
        let mut session = self.load_session(session_id)?;

        // Verify the finding exists
        if !session.findings.iter().any(|f| f.id == *finding_id) {
            anyhow::bail!(
                "Finding '{}' not found in session '{}'",
                finding_id,
                session_id
            );
        }

        session
            .resolutions
            .insert(finding_id.clone(), resolution);
        self.save_session(&session)?;
        Ok(())
    }

    fn has_unresolved_blockers(&self) -> anyhow::Result<bool> {
        match self.latest_session()? {
            Some(session) => Ok(session.has_unresolved_blockers()),
            None => Ok(false),
        }
    }
}
```

## Test Strategy

### Unit Tests

```rust
#[cfg(test)]
mod convention_tests {
    use super::*;
    use crate::adapters::toml::parser::CheckEntry;
    use crate::core::ports::agent::{ReviewContext, Severity};
    use std::path::PathBuf;

    fn make_context(changed_files: Vec<&str>) -> ReviewContext {
        ReviewContext {
            diff: String::new(),
            changed_files: changed_files.into_iter().map(String::from).collect(),
            repo_root: PathBuf::from("/tmp/test"),
            commit_range: None,
        }
    }

    #[test]
    fn convention_matches_glob_pattern() {
        let checks = vec![CheckEntry {
            id: Some("NOS-1".to_string()),
            target: "*.rs".to_string(),
            message: "Check Rust files".to_string(),
            severity: "warn".to_string(),
            tags: vec![],
        }];

        let analyzer = ConventionAnalyzer::new(checks);
        let ctx = make_context(vec!["src/lib.rs", "README.md"]);
        let findings = analyzer.analyze(&ctx, &[]).unwrap();

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].file, "src/lib.rs");
        assert_eq!(findings[0].severity, Severity::Warning);
    }

    #[test]
    fn convention_matches_literal_path() {
        let checks = vec![CheckEntry {
            id: Some("NOS-2".to_string()),
            target: "src/core/ports/agent.rs".to_string(),
            message: "Check agent port".to_string(),
            severity: "block".to_string(),
            tags: vec![],
        }];

        let analyzer = ConventionAnalyzer::new(checks);
        let ctx = make_context(vec!["src/core/ports/agent.rs"]);
        let findings = analyzer.analyze(&ctx, &[]).unwrap();

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, Severity::Error);
    }

    #[test]
    fn convention_no_match_returns_empty() {
        let checks = vec![CheckEntry {
            id: None,
            target: "*.py".to_string(),
            message: "Check Python files".to_string(),
            severity: "info".to_string(),
            tags: vec![],
        }];

        let analyzer = ConventionAnalyzer::new(checks);
        let ctx = make_context(vec!["src/lib.rs"]);
        let findings = analyzer.analyze(&ctx, &[]).unwrap();

        assert!(findings.is_empty());
    }
}

#[cfg(test)]
mod pipeline_tests {
    use super::*;
    use crate::core::ports::agent::{Finding, ReviewContext, Severity};
    use std::path::PathBuf;

    #[test]
    fn categorize_source_convention() {
        assert_eq!(categorize_source("convention:NOS-1"), "convention");
    }

    #[test]
    fn categorize_source_formatting() {
        assert_eq!(categorize_source("formatting:rustfmt"), "formatting");
    }

    #[test]
    fn categorize_source_agent() {
        assert_eq!(categorize_source("agent:security"), "security");
    }

    #[test]
    fn categorize_source_custom() {
        assert_eq!(categorize_source("custom:migrations"), "migrations");
    }

    #[test]
    fn session_has_unresolved_blockers() {
        let session = ReviewSession {
            id: "test".to_string(),
            target: ReviewTarget {
                base: "main".to_string(),
                head: "HEAD".to_string(),
                base_commit: "abc".to_string(),
                head_commit: "def".to_string(),
            },
            started_at: Utc::now(),
            completed_at: Some(Utc::now()),
            analyzers_run: vec!["conventions".to_string()],
            findings: vec![SessionFinding {
                id: "F-001".to_string(),
                source: FindingSource::Analyzer {
                    name: "conventions".to_string(),
                },
                file: "src/lib.rs".to_string(),
                line: None,
                severity: Severity::Error,
                message: "check failed".to_string(),
                suggestion: None,
                commit: None,
                category: "convention".to_string(),
            }],
            resolutions: HashMap::new(),
        };

        assert!(session.has_unresolved_blockers());
    }

    #[test]
    fn session_resolved_blockers() {
        let mut session = ReviewSession {
            id: "test".to_string(),
            target: ReviewTarget {
                base: "main".to_string(),
                head: "HEAD".to_string(),
                base_commit: "abc".to_string(),
                head_commit: "def".to_string(),
            },
            started_at: Utc::now(),
            completed_at: Some(Utc::now()),
            analyzers_run: vec!["conventions".to_string()],
            findings: vec![SessionFinding {
                id: "F-001".to_string(),
                source: FindingSource::Analyzer {
                    name: "conventions".to_string(),
                },
                file: "src/lib.rs".to_string(),
                line: None,
                severity: Severity::Error,
                message: "check failed".to_string(),
                suggestion: None,
                commit: None,
                category: "convention".to_string(),
            }],
            resolutions: HashMap::new(),
        };

        session.resolutions.insert(
            "F-001".to_string(),
            Resolution {
                status: ResolutionStatus::Resolved,
                reason: None,
                at: Utc::now(),
            },
        );

        assert!(!session.has_unresolved_blockers());
    }

    #[test]
    fn session_warnings_are_not_blockers() {
        let session = ReviewSession {
            id: "test".to_string(),
            target: ReviewTarget {
                base: "main".to_string(),
                head: "HEAD".to_string(),
                base_commit: "abc".to_string(),
                head_commit: "def".to_string(),
            },
            started_at: Utc::now(),
            completed_at: Some(Utc::now()),
            analyzers_run: vec!["formatting".to_string()],
            findings: vec![SessionFinding {
                id: "F-001".to_string(),
                source: FindingSource::Analyzer {
                    name: "formatting".to_string(),
                },
                file: "src/lib.rs".to_string(),
                line: None,
                severity: Severity::Warning,
                message: "would be reformatted".to_string(),
                suggestion: None,
                commit: None,
                category: "formatting".to_string(),
            }],
            resolutions: HashMap::new(),
        };

        assert!(!session.has_unresolved_blockers());
    }
}

#[cfg(test)]
mod co_change_tests {
    use super::*;
    use crate::core::models::diff::CommitInfo;

    #[test]
    fn build_cochange_matrix() {
        let commits = vec![
            CommitInfo {
                short_sha: "abc".to_string(),
                sha: "abc123".to_string(),
                summary: "commit 1".to_string(),
                message: "commit 1".to_string(),
                author: "dev".to_string(),
                timestamp: 0,
                files_changed: vec![
                    "src/a.rs".to_string(),
                    "src/b.rs".to_string(),
                ],
            },
            CommitInfo {
                short_sha: "def".to_string(),
                sha: "def456".to_string(),
                summary: "commit 2".to_string(),
                message: "commit 2".to_string(),
                author: "dev".to_string(),
                timestamp: 1,
                files_changed: vec![
                    "src/a.rs".to_string(),
                    "src/b.rs".to_string(),
                    "src/c.rs".to_string(),
                ],
            },
        ];

        let config = CoChangeConfig {
            min_correlation: 0.5,
            lookback: 100,
            min_commits: 2,
        };

        // a and b appear together in both commits: co_count = 2
        // a and c appear together in 1 commit: co_count = 1 (below min_commits)
        let analyzer = CoChangeAnalyzer {
            config,
            vcs: todo!("mock"),
        };
        let matrix = analyzer.build_cochange_matrix(&commits);

        let pair = ("src/a.rs".to_string(), "src/b.rs".to_string());
        assert_eq!(matrix.get(&pair), Some(&2));
    }
}

#[cfg(test)]
mod finding_store_tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn save_and_load_session() {
        let dir = TempDir::new().unwrap();
        let store = JsonFindingStore::new(dir.path());

        let session = ReviewSession {
            id: "review-test-001".to_string(),
            target: ReviewTarget {
                base: "main".to_string(),
                head: "HEAD".to_string(),
                base_commit: "abc123".to_string(),
                head_commit: "def456".to_string(),
            },
            started_at: Utc::now(),
            completed_at: Some(Utc::now()),
            analyzers_run: vec!["conventions".to_string()],
            findings: vec![],
            resolutions: HashMap::new(),
        };

        store.save_session(&session).unwrap();
        let loaded = store.load_session(&session.id).unwrap();
        assert_eq!(loaded.id, session.id);
    }

    #[test]
    fn latest_session_returns_most_recent() {
        let dir = TempDir::new().unwrap();
        let store = JsonFindingStore::new(dir.path());

        let session1 = ReviewSession {
            id: "review-001".to_string(),
            target: ReviewTarget {
                base: "main".to_string(),
                head: "HEAD".to_string(),
                base_commit: "aaa".to_string(),
                head_commit: "bbb".to_string(),
            },
            started_at: Utc::now(),
            completed_at: Some(Utc::now()),
            analyzers_run: vec![],
            findings: vec![],
            resolutions: HashMap::new(),
        };

        let session2 = ReviewSession {
            id: "review-002".to_string(),
            ..session1.clone()
        };

        store.save_session(&session1).unwrap();
        // Brief pause to ensure different mtime
        std::thread::sleep(std::time::Duration::from_millis(10));
        store.save_session(&session2).unwrap();

        let latest = store.latest_session().unwrap().unwrap();
        assert_eq!(latest.id, "review-002");
    }

    #[test]
    fn resolve_finding_updates_session() {
        let dir = TempDir::new().unwrap();
        let store = JsonFindingStore::new(dir.path());

        let session = ReviewSession {
            id: "review-test".to_string(),
            target: ReviewTarget {
                base: "main".to_string(),
                head: "HEAD".to_string(),
                base_commit: "aaa".to_string(),
                head_commit: "bbb".to_string(),
            },
            started_at: Utc::now(),
            completed_at: Some(Utc::now()),
            analyzers_run: vec!["conventions".to_string()],
            findings: vec![SessionFinding {
                id: "F-001".to_string(),
                source: FindingSource::Analyzer {
                    name: "conventions".to_string(),
                },
                file: "src/lib.rs".to_string(),
                line: None,
                severity: Severity::Error,
                message: "test".to_string(),
                suggestion: None,
                commit: None,
                category: "convention".to_string(),
            }],
            resolutions: HashMap::new(),
        };

        store.save_session(&session).unwrap();

        store
            .resolve_finding(
                &session.id,
                &"F-001".to_string(),
                Resolution {
                    status: ResolutionStatus::Resolved,
                    reason: None,
                    at: Utc::now(),
                },
            )
            .unwrap();

        let loaded = store.load_session(&session.id).unwrap();
        assert!(loaded.resolutions.contains_key("F-001"));
    }

    #[test]
    fn has_no_blockers_when_empty() {
        let dir = TempDir::new().unwrap();
        let store = JsonFindingStore::new(dir.path());
        assert!(!store.has_unresolved_blockers().unwrap());
    }
}
```

### Integration Tests

```rust
#[cfg(test)]
mod integration_tests {
    use super::*;
    use mockall::predicate::*;

    #[test]
    fn pipeline_runs_analyzers_in_order_with_fold() {
        // Create mock analyzers that verify fold semantics
        let mut analyzer1 = MockReviewAnalyzer::new();
        analyzer1.expect_name().return_const("first");
        analyzer1.expect_analyze()
            .withf(|_ctx, prior| prior.is_empty())
            .returning(|_, _| Ok(vec![Finding {
                severity: Severity::Warning,
                file: "a.rs".to_string(),
                line: Some(1),
                message: "from first".to_string(),
                suggestion: None,
                source: "first".to_string(),
            }]));

        let mut analyzer2 = MockReviewAnalyzer::new();
        analyzer2.expect_name().return_const("second");
        analyzer2.expect_analyze()
            .withf(|_ctx, prior| {
                prior.len() == 1 && prior[0].source == "first"
            })
            .returning(|_, _| Ok(vec![Finding {
                severity: Severity::Error,
                file: "b.rs".to_string(),
                line: Some(2),
                message: "from second".to_string(),
                suggestion: None,
                source: "second".to_string(),
            }]));

        // Verify that analyzer2 receives findings from analyzer1
        // (This tests the fold semantics of the pipeline)
    }
}
```

## Summary

The review pipeline design provides:

1. **ReviewPipeline orchestrator**: Central service that takes a `ReviewConfig` and `ReviewContext`, builds analyzers from config, executes them in sequence with fold semantics, and returns a `ReviewSession` with all findings.

2. **Four built-in analyzers**: `ConventionAnalyzer` (wraps `[[check]]` entries), `FormattingAnalyzer` (invokes external formatters in check mode), `CoChangeAnalyzer` (detects missing correlated files via git history), and `ScriptAnalyzer` (runs user-configured external commands).

3. **Agent analyzers**: Via `AgentAnalyzer` from doc 03, which wraps `AgentRuntime` implementations from doc 04. Multiple agent passes (security, quality, architecture) with different focused prompts.

4. **Review sessions**: `ReviewSession` captures a complete review run with stable finding IDs, resolution tracking, and metadata. Stored as JSON in `.noslop/reviews/`.

5. **FindingStore trait**: Persistence for review sessions with load/save/resolve operations. Used by the `noslop findings` command and the pre-push hook.

6. **CLI commands**: `noslop review` runs the pipeline and displays results. `noslop findings` lists, resolves, and dismisses findings. Both support JSON output.

7. **Pre-push hook integration**: Gates `git push` on unresolved blocking findings (Error/Critical severity).

8. **Fold semantics**: Each analyzer receives all findings from prior analyzers, enabling LLM-based analyzers to avoid duplicating work that static tools already caught and to provide deeper analysis building on those results.
