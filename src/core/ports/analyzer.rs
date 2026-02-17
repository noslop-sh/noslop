//! Review analyzer port
//!
//! Defines the trait that all review analyzers implement. The pipeline
//! orchestrator operates exclusively through `dyn ReviewAnalyzer` trait objects.

use std::path::PathBuf;

use crate::core::models::Finding;

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

/// Execution tier for ordering analyzers in the pipeline.
///
/// Analyzers run in tier order: `Static` first, then `Computed`, then `Agent`.
/// Within a tier, analyzers run in the order they appear in configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AnalyzerTier {
    /// Free, fast checks (e.g., convention matching)
    Static,
    /// Local tool invocations (e.g., formatters, git history)
    Computed,
    /// LLM-based analysis (most expensive)
    Agent,
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
/// - `ConventionAnalyzer` -- matches `[[check]]` rules against changed files
/// - `FormattingAnalyzer` -- invokes external formatters in check mode
/// - `CoChangeAnalyzer` -- flags missing correlated files via git history
/// - `ScriptAnalyzer` -- runs user-configured external commands
/// - `AgentAnalyzer` -- wraps `AgentRuntime` for LLM-based review
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

    /// Which tier this analyzer belongs to, for pipeline ordering.
    fn tier(&self) -> AnalyzerTier;
}
