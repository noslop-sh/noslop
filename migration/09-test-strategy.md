# Test Strategy

Comprehensive test plan for all new modules introduced by the review pipeline architecture. Every test follows noslop's existing patterns exactly: inline `#[cfg(test)]` unit tests in source modules, `mockall::automock`-annotated traits, `test-case` for parameterized tests, `proptest` for property-based tests, `TempGitRepo`/`tempfile::TempDir` for git operations, `assert_cmd` for CLI integration tests, and builder fixtures in `tests/common/`.

## Test Infrastructure

### Existing Patterns

noslop's test infrastructure is already established:

| Tool                                                      | Usage                                                  |
| --------------------------------------------------------- | ------------------------------------------------------ |
| `#[cfg(test)] mod tests`                                  | Inline unit tests in every source module               |
| `mockall::automock`                                       | Auto-generated mocks via trait annotation              |
| `test-case`                                               | Parameterized tests with `#[test_case(...)]`           |
| `proptest`                                                | Property-based testing (used for matcher tests)        |
| `TempGitRepo`                                             | Temporary git repos for adapter/integration tests      |
| `assert_cmd`                                              | CLI integration tests via `Command::cargo_bin`         |
| `tempfile::TempDir`                                       | Isolated temp directories for file I/O tests           |
| `CheckBuilder`/`AckBuilder`                               | Builder-pattern fixtures in `tests/common/fixtures.rs` |
| `MockCheckRepository`/`MockAckStore`/`MockVersionControl` | Manual mocks in `tests/common/mocks.rs`                |

### New Test Fixtures (`tests/common/fixtures.rs`)

Add builders alongside the existing `CheckBuilder` and `AckBuilder`:

```rust
/// Builder for creating test Finding instances
pub struct FindingBuilder {
    severity: Severity,
    file: String,
    line: Option<u32>,
    message: String,
    suggestion: Option<String>,
    source: String,
}

impl FindingBuilder {
    pub fn new() -> Self {
        Self {
            severity: Severity::Warning,
            file: "src/lib.rs".to_string(),
            line: Some(10),
            message: "Test finding".to_string(),
            suggestion: None,
            source: "test".to_string(),
        }
    }

    pub fn severity(mut self, severity: Severity) -> Self {
        self.severity = severity;
        self
    }

    pub fn file(mut self, file: &str) -> Self {
        self.file = file.to_string();
        self
    }

    pub fn line(mut self, line: u32) -> Self {
        self.line = Some(line);
        self
    }

    pub fn message(mut self, message: &str) -> Self {
        self.message = message.to_string();
        self
    }

    pub fn suggestion(mut self, suggestion: &str) -> Self {
        self.suggestion = Some(suggestion.to_string());
        self
    }

    pub fn source(mut self, source: &str) -> Self {
        self.source = source.to_string();
        self
    }

    pub fn build(self) -> Finding {
        Finding {
            severity: self.severity,
            file: self.file,
            line: self.line,
            message: self.message,
            suggestion: self.suggestion,
            source: self.source,
        }
    }
}

impl Default for FindingBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for creating test ReviewContext instances
pub struct ReviewContextBuilder {
    diff: String,
    changed_files: Vec<String>,
    repo_root: PathBuf,
    commit_range: Option<String>,
}

impl ReviewContextBuilder {
    pub fn new() -> Self {
        Self {
            diff: "+fn main() {}".to_string(),
            changed_files: vec!["src/main.rs".to_string()],
            repo_root: PathBuf::from("/tmp/test"),
            commit_range: None,
        }
    }

    pub fn diff(mut self, diff: &str) -> Self {
        self.diff = diff.to_string();
        self
    }

    pub fn changed_files(mut self, files: Vec<String>) -> Self {
        self.changed_files = files;
        self
    }

    pub fn repo_root(mut self, root: PathBuf) -> Self {
        self.repo_root = root;
        self
    }

    pub fn commit_range(mut self, range: &str) -> Self {
        self.commit_range = Some(range.to_string());
        self
    }

    pub fn build(self) -> ReviewContext {
        ReviewContext {
            diff: self.diff,
            changed_files: self.changed_files,
            repo_root: self.repo_root,
            commit_range: self.commit_range,
        }
    }
}

impl Default for ReviewContextBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for creating test InvocationResult instances
pub struct InvocationResultBuilder {
    output: String,
    exit_code: i32,
    duration: std::time::Duration,
}

impl InvocationResultBuilder {
    pub fn new() -> Self {
        Self {
            output: "[]".to_string(),
            exit_code: 0,
            duration: std::time::Duration::from_secs(1),
        }
    }

    pub fn output(mut self, output: &str) -> Self {
        self.output = output.to_string();
        self
    }

    pub fn exit_code(mut self, code: i32) -> Self {
        self.exit_code = code;
        self
    }

    pub fn duration_secs(mut self, secs: u64) -> Self {
        self.duration = std::time::Duration::from_secs(secs);
        self
    }

    pub fn build(self) -> InvocationResult {
        InvocationResult {
            output: self.output,
            exit_code: self.exit_code,
            duration: self.duration,
        }
    }
}

impl Default for InvocationResultBuilder {
    fn default() -> Self {
        Self::new()
    }
}
```

### Extended `TempGitRepo` (`tests/common/git_repo.rs`)

Add methods to support checkpoint and review pipeline testing:

```rust
impl TempGitRepo {
    // --- existing methods: new(), path(), write_file(), stage(), commit(), git() ---

    /// Stage all files and create initial commit (required before diff operations)
    pub fn init_with_commit(&self) {
        self.write_file("README.md", "# test");
        self.stage("README.md");
        self.commit("Initial commit");
    }

    /// Write a file and stage it in one call
    pub fn write_and_stage(&self, name: &str, content: &str) {
        self.write_file(name, content);
        self.stage(name);
    }

    /// Get the HEAD short SHA
    pub fn head_sha(&self) -> String {
        let output = self.git(&["rev-parse", "--short", "HEAD"]);
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    }

    /// Check if working tree is clean
    pub fn is_clean(&self) -> bool {
        let output = self.git(&["status", "--porcelain"]);
        String::from_utf8_lossy(&output.stdout).trim().is_empty()
    }

    /// Count commits on current branch
    pub fn commit_count(&self) -> usize {
        let output = self.git(&["rev-list", "--count", "HEAD"]);
        String::from_utf8_lossy(&output.stdout)
            .trim()
            .parse()
            .unwrap_or(0)
    }

    /// Create a branch from current HEAD
    pub fn create_branch(&self, name: &str) {
        self.git(&["branch", name]);
    }

    /// Checkout a branch
    pub fn checkout(&self, name: &str) {
        self.git(&["checkout", name]);
    }
}
```

### New Mock Implementations (`tests/common/mocks.rs`)

Extend the existing manual mocks with `VersionControl` checkpoint and review pipeline methods. The `MockVersionControl` struct gains fields for the new trait methods:

```rust
/// Extended mock VersionControl with checkpoint and review pipeline support
pub struct MockVersionControl {
    staged_files: Vec<String>,
    repo_name: String,
    repo_root: PathBuf,
    // New fields for review pipeline methods
    is_clean_result: bool,
    commit_all_result: Option<String>,
    diff_between_result: Vec<FileDiff>,
    commits_between_result: Vec<CommitInfo>,
}
```

Additionally, all new port traits are annotated with `#[cfg_attr(test, mockall::automock)]`, which auto-generates `MockAgentConfig`, `MockAgentRuntime`, and `MockReviewAnalyzer` for fine-grained expectation control in unit tests.

---

## Unit Tests (Pure Logic, No I/O)

### 1. Review Pipeline Orchestrator Tests

**Location**: `src/core/services/pipeline.rs` under `#[cfg(test)] mod tests`

The `ReviewPipeline` orchestrator runs analyzers in order, passing prior findings to each subsequent analyzer. Tests use `MockReviewAnalyzer` (auto-generated by `mockall`).

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use mockall::predicate::*;

    fn mock_analyzer(name: &str, findings: Vec<Finding>) -> MockReviewAnalyzer {
        let mut mock = MockReviewAnalyzer::new();
        let name_owned = name.to_string();
        mock.expect_name().return_const(name_owned);
        mock.expect_required_context().returning(|| vec![ContextKind::Diff]);
        mock.expect_analyze()
            .returning(move |_ctx, _prior| Ok(findings.clone()));
        mock
    }

    #[test]
    fn pipeline_runs_analyzers_in_order() {
        let a1 = mock_analyzer("conventions", vec![
            FindingBuilder::new().source("conventions").message("unused import").build(),
        ]);
        let a2 = mock_analyzer("agent:security", vec![
            FindingBuilder::new().source("agent:security").message("sql injection").build(),
        ]);

        let pipeline = ReviewPipeline::new(vec![Box::new(a1), Box::new(a2)]);
        let ctx = ReviewContextBuilder::new().build();
        let findings = pipeline.run(&ctx).unwrap();

        assert_eq!(findings.len(), 2);
        assert_eq!(findings[0].source, "conventions");
        assert_eq!(findings[1].source, "agent:security");
    }

    #[test]
    fn pipeline_passes_prior_findings_to_later_analyzers() {
        let a1 = mock_analyzer("conventions", vec![
            FindingBuilder::new().source("conventions").build(),
        ]);

        let mut a2 = MockReviewAnalyzer::new();
        a2.expect_name().return_const("agent:security".to_string());
        a2.expect_required_context().returning(|| vec![ContextKind::Diff]);
        a2.expect_analyze()
            .withf(|_ctx, prior| prior.len() == 1 && prior[0].source == "conventions")
            .returning(|_ctx, _prior| Ok(vec![]));

        let pipeline = ReviewPipeline::new(vec![Box::new(a1), Box::new(a2)]);
        let ctx = ReviewContextBuilder::new().build();
        pipeline.run(&ctx).unwrap();
    }

    #[test]
    fn pipeline_empty_analyzers_returns_empty_findings() {
        let pipeline = ReviewPipeline::new(vec![]);
        let ctx = ReviewContextBuilder::new().build();
        let findings = pipeline.run(&ctx).unwrap();
        assert!(findings.is_empty());
    }

    #[test]
    fn pipeline_continues_after_analyzer_error() {
        let mut a1 = MockReviewAnalyzer::new();
        a1.expect_name().return_const("failing".to_string());
        a1.expect_required_context().returning(|| vec![]);
        a1.expect_analyze()
            .returning(|_ctx, _prior| Err(anyhow::anyhow!("analyzer crashed")));

        let a2 = mock_analyzer("working", vec![
            FindingBuilder::new().source("working").build(),
        ]);

        let pipeline = ReviewPipeline::new(vec![Box::new(a1), Box::new(a2)]);
        let ctx = ReviewContextBuilder::new().build();
        let findings = pipeline.run(&ctx).unwrap();

        // Pipeline should continue past the failed analyzer
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].source, "working");
    }

    #[test]
    fn pipeline_accumulates_findings_across_analyzers() {
        let a1 = mock_analyzer("conventions", vec![
            FindingBuilder::new().source("conventions").message("a").build(),
            FindingBuilder::new().source("conventions").message("b").build(),
        ]);
        let a2 = mock_analyzer("formatting", vec![
            FindingBuilder::new().source("formatting").message("c").build(),
        ]);
        let a3 = mock_analyzer("agent:quality", vec![
            FindingBuilder::new().source("agent:quality").message("d").build(),
        ]);

        let pipeline = ReviewPipeline::new(vec![
            Box::new(a1), Box::new(a2), Box::new(a3),
        ]);
        let ctx = ReviewContextBuilder::new().build();
        let findings = pipeline.run(&ctx).unwrap();
        assert_eq!(findings.len(), 4);
    }
}
```

| Test                                                | What It Asserts                                                       |
| --------------------------------------------------- | --------------------------------------------------------------------- |
| `pipeline_runs_analyzers_in_order`                  | Findings from earlier analyzers appear first in the result            |
| `pipeline_passes_prior_findings_to_later_analyzers` | The second analyzer's `analyze()` receives findings from the first    |
| `pipeline_empty_analyzers_returns_empty_findings`   | Pipeline with no analyzers returns `Ok(vec![])`                       |
| `pipeline_continues_after_analyzer_error`           | A failing analyzer does not prevent subsequent analyzers from running |
| `pipeline_accumulates_findings_across_analyzers`    | Findings from all three tiers (static, computed, agent) are collected |

### 2. ConventionAnalyzer Tests

**Location**: `src/adapters/analyzer/convention.rs` under `#[cfg(test)] mod tests`

The `ConventionAnalyzer` wraps existing check logic from `CheckRepository` and runs it against diff context. Tests verify that convention checks produce the correct findings.

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn convention_analyzer_produces_finding_for_matching_check() {
        let checks = vec![
            CheckBuilder::new()
                .id("NOS-1")
                .target("src/auth.rs")
                .message("Verify auth logic")
                .severity(Severity::Block)
                .build(),
        ];

        let analyzer = ConventionAnalyzer::new(checks);
        let ctx = ReviewContextBuilder::new()
            .changed_files(vec!["src/auth.rs".to_string()])
            .build();

        let findings = analyzer.analyze(&ctx, &[]).unwrap();
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].message, "Verify auth logic");
        assert_eq!(findings[0].source, "conventions");
    }

    #[test]
    fn convention_analyzer_no_findings_for_unmatched_files() {
        let checks = vec![
            CheckBuilder::new()
                .target("src/auth.rs")
                .build(),
        ];

        let analyzer = ConventionAnalyzer::new(checks);
        let ctx = ReviewContextBuilder::new()
            .changed_files(vec!["src/main.rs".to_string()])
            .build();

        let findings = analyzer.analyze(&ctx, &[]).unwrap();
        assert!(findings.is_empty());
    }

    #[test]
    fn convention_analyzer_glob_pattern_matching() {
        let checks = vec![
            CheckBuilder::new()
                .target("*.rs")
                .message("Review Rust files")
                .build(),
        ];

        let analyzer = ConventionAnalyzer::new(checks);
        let ctx = ReviewContextBuilder::new()
            .changed_files(vec![
                "src/main.rs".to_string(),
                "src/lib.rs".to_string(),
                "README.md".to_string(),
            ])
            .build();

        let findings = analyzer.analyze(&ctx, &[]).unwrap();
        assert_eq!(findings.len(), 2); // Only .rs files match
    }

    #[test]
    fn convention_analyzer_maps_severity_correctly() {
        let checks = vec![
            CheckBuilder::new()
                .target("*.rs")
                .severity(Severity::Warn)
                .build(),
        ];

        let analyzer = ConventionAnalyzer::new(checks);
        let ctx = ReviewContextBuilder::new().build();
        let findings = analyzer.analyze(&ctx, &[]).unwrap();

        assert_eq!(findings[0].severity, Severity::Warning);
    }
}
```

### 3. CoChangeAnalyzer Tests

**Location**: `src/adapters/analyzer/co_change.rs` under `#[cfg(test)] mod tests`

The `CoChangeAnalyzer` detects files that historically change together but are missing from the current diff. Tests use fixture git histories built with `TempGitRepo`.

```rust
#[cfg(test)]
mod tests {
    use super::*;

    /// Build a co-change correlation map from fixture data
    fn fixture_correlations() -> HashMap<String, Vec<(String, f64)>> {
        let mut map = HashMap::new();
        map.insert(
            "src/core/ports/analyzer.rs".to_string(),
            vec![
                ("src/core/services/pipeline.rs".to_string(), 0.85),
                ("src/adapters/analyzer/convention.rs".to_string(), 0.72),
            ],
        );
        map.insert(
            "src/cli/app.rs".to_string(),
            vec![
                ("src/cli/commands/mod.rs".to_string(), 0.90),
            ],
        );
        map
    }

    #[test]
    fn co_change_flags_missing_correlated_file() {
        let analyzer = CoChangeAnalyzer::with_correlations(
            fixture_correlations(),
            CoChangeConfig { min_correlation: 0.7, lookback: 500, min_commits: 10 },
        );

        let ctx = ReviewContextBuilder::new()
            .changed_files(vec!["src/core/ports/analyzer.rs".to_string()])
            .build();

        let findings = analyzer.analyze(&ctx, &[]).unwrap();

        // Should flag pipeline.rs and convention.rs as missing
        assert_eq!(findings.len(), 2);
        assert!(findings.iter().any(|f| f.message.contains("pipeline.rs")));
        assert!(findings.iter().any(|f| f.message.contains("convention.rs")));
    }

    #[test]
    fn co_change_no_findings_when_all_correlated_files_present() {
        let analyzer = CoChangeAnalyzer::with_correlations(
            fixture_correlations(),
            CoChangeConfig::default(),
        );

        let ctx = ReviewContextBuilder::new()
            .changed_files(vec![
                "src/core/ports/analyzer.rs".to_string(),
                "src/core/services/pipeline.rs".to_string(),
                "src/adapters/analyzer/convention.rs".to_string(),
            ])
            .build();

        let findings = analyzer.analyze(&ctx, &[]).unwrap();
        assert!(findings.is_empty());
    }

    #[test]
    fn co_change_respects_min_correlation_threshold() {
        let analyzer = CoChangeAnalyzer::with_correlations(
            fixture_correlations(),
            CoChangeConfig { min_correlation: 0.80, lookback: 500, min_commits: 10 },
        );

        let ctx = ReviewContextBuilder::new()
            .changed_files(vec!["src/core/ports/analyzer.rs".to_string()])
            .build();

        let findings = analyzer.analyze(&ctx, &[]).unwrap();

        // Only pipeline.rs (0.85) should be flagged, not convention.rs (0.72)
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("pipeline.rs"));
    }

    #[test]
    fn co_change_no_findings_for_uncorrelated_file() {
        let analyzer = CoChangeAnalyzer::with_correlations(
            fixture_correlations(),
            CoChangeConfig::default(),
        );

        let ctx = ReviewContextBuilder::new()
            .changed_files(vec!["README.md".to_string()])
            .build();

        let findings = analyzer.analyze(&ctx, &[]).unwrap();
        assert!(findings.is_empty());
    }
}
```

### 4. FormattingAnalyzer Tests

**Location**: `src/adapters/analyzer/formatting.rs` under `#[cfg(test)] mod tests`

The `FormattingAnalyzer` delegates to external formatters and flags files that would change.

| Test                                              | What It Asserts                                                       |
| ------------------------------------------------- | --------------------------------------------------------------------- |
| `formatting_no_findings_when_no_tools_configured` | `FormattingAnalyzer` with empty `tools` list returns no findings      |
| `formatting_flags_unformatted_rust_file`          | With `rustfmt` configured, unformatted `.rs` file produces a finding  |
| `formatting_ignores_non_matching_extensions`      | `.py` file is not checked by `rustfmt` formatter                      |
| `formatting_multiple_tools_each_checked`          | With both `rustfmt` and `prettier`, both produce independent findings |

### 5. ScriptAnalyzer Tests

**Location**: `src/adapters/analyzer/script.rs` under `#[cfg(test)] mod tests`

The `ScriptAnalyzer` runs an external command and parses its stdout as JSON findings.

| Test                                               | What It Asserts                                                     |
| -------------------------------------------------- | ------------------------------------------------------------------- |
| `script_analyzer_parses_json_findings_from_stdout` | Command outputting valid JSON array produces correct `Vec<Finding>` |
| `script_analyzer_empty_output_returns_no_findings` | Command producing empty output returns empty findings               |
| `script_analyzer_nonzero_exit_returns_error`       | Command exiting with code 1 returns `Err`                           |
| `script_analyzer_passes_prior_findings_on_stdin`   | Prior findings are serialized to JSON and piped as stdin            |

### 6. AgentAnalyzer Tests

**Location**: `src/adapters/analyzer/agent.rs` under `#[cfg(test)] mod tests`

The `AgentAnalyzer` wraps an `AgentRuntime` and implements `ReviewAnalyzer`. Tests use `MockAgentRuntime`.

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use mockall::predicate::*;

    #[test]
    fn agent_analyzer_builds_prompt_with_diff_and_prior_findings() {
        let mut mock = MockAgentRuntime::new();
        mock.expect_agent_type().return_const(AgentType::Claude);
        mock.expect_invoke()
            .withf(|config: &InvocationConfig| {
                config.prompt.contains("+fn unsafe_thing()")
                    && config.prompt.contains("unused import")
                    && config.prompt.contains("security")
            })
            .returning(|_| Ok(InvocationResult {
                output: "[]".to_string(),
                exit_code: 0,
                duration: std::time::Duration::from_secs(1),
            }));
        mock.expect_parse_output().returning(|_| ReviewOutput::default());

        let analyzer = AgentAnalyzer::new(
            Box::new(mock),
            "security",
            "Review for {focus} issues:\n\n{diff}\n\nPrior:\n{prior_findings}",
        );

        let ctx = ReviewContextBuilder::new()
            .diff("+fn unsafe_thing() {}".to_string())
            .build();

        let prior = vec![
            FindingBuilder::new().source("clippy").message("unused import").build(),
        ];

        analyzer.analyze(&ctx, &prior).unwrap();
    }

    #[test]
    fn agent_analyzer_returns_parsed_findings() {
        let mut mock = MockAgentRuntime::new();
        mock.expect_agent_type().return_const(AgentType::Claude);
        mock.expect_invoke().returning(|_| Ok(InvocationResult {
            output: r#"[{"severity":"error","file":"src/db.rs","line":15,"message":"SQL injection","suggestion":"Use parameterized queries","source":"agent:security"}]"#.to_string(),
            exit_code: 0,
            duration: std::time::Duration::from_secs(5),
        }));
        mock.expect_parse_output().returning(|result| {
            let findings: Vec<Finding> = serde_json::from_str(&result.output).unwrap_or_default();
            ReviewOutput {
                findings,
                raw_output: result.output.clone(),
                errors: vec![],
            }
        });

        let analyzer = AgentAnalyzer::new(
            Box::new(mock),
            "security",
            "{diff}",
        );

        let ctx = ReviewContextBuilder::new().build();
        let findings = analyzer.analyze(&ctx, &[]).unwrap();

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, Severity::Error);
        assert_eq!(findings[0].file, "src/db.rs");
        assert_eq!(findings[0].message, "SQL injection");
    }

    #[test]
    fn agent_analyzer_empty_prior_findings_message() {
        let mut mock = MockAgentRuntime::new();
        mock.expect_agent_type().return_const(AgentType::Claude);
        mock.expect_invoke()
            .withf(|config: &InvocationConfig| {
                config.prompt.contains("No prior findings from static analysis")
            })
            .returning(|_| Ok(InvocationResult {
                output: "[]".to_string(),
                exit_code: 0,
                duration: std::time::Duration::from_secs(1),
            }));
        mock.expect_parse_output().returning(|_| ReviewOutput::default());

        let analyzer = AgentAnalyzer::new(
            Box::new(mock),
            "quality",
            "Prior: {prior_findings}",
        );

        let ctx = ReviewContextBuilder::new().build();
        analyzer.analyze(&ctx, &[]).unwrap();
    }

    #[test]
    fn agent_analyzer_name_includes_focus() {
        let mut mock = MockAgentRuntime::new();
        mock.expect_agent_type().return_const(AgentType::Claude);

        let analyzer = AgentAnalyzer::new(Box::new(mock), "security", "");
        assert_eq!(analyzer.name(), "agent:security");
    }
}
```

### 7. Finding Deduplication and Merging Tests

**Location**: `src/core/services/finding_merge.rs` under `#[cfg(test)] mod tests`

Findings from multiple analyzers may overlap. Deduplication ensures the same issue is not reported twice.

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dedup_removes_exact_duplicates() {
        let findings = vec![
            FindingBuilder::new().file("src/lib.rs").line(10).message("unused").source("clippy").build(),
            FindingBuilder::new().file("src/lib.rs").line(10).message("unused").source("agent:quality").build(),
        ];

        let deduped = deduplicate_findings(findings);
        assert_eq!(deduped.len(), 1);
    }

    #[test]
    fn dedup_keeps_distinct_findings() {
        let findings = vec![
            FindingBuilder::new().file("src/lib.rs").line(10).message("unused import").build(),
            FindingBuilder::new().file("src/lib.rs").line(20).message("missing error handling").build(),
        ];

        let deduped = deduplicate_findings(findings);
        assert_eq!(deduped.len(), 2);
    }

    #[test]
    fn dedup_prefers_higher_severity() {
        let findings = vec![
            FindingBuilder::new().file("src/auth.rs").line(42).message("timing attack").severity(Severity::Warning).build(),
            FindingBuilder::new().file("src/auth.rs").line(42).message("timing attack").severity(Severity::Error).build(),
        ];

        let deduped = deduplicate_findings(findings);
        assert_eq!(deduped.len(), 1);
        assert_eq!(deduped[0].severity, Severity::Error);
    }

    #[test]
    fn dedup_same_file_different_lines_kept() {
        let findings = vec![
            FindingBuilder::new().file("src/lib.rs").line(10).message("issue A").build(),
            FindingBuilder::new().file("src/lib.rs").line(50).message("issue B").build(),
        ];

        let deduped = deduplicate_findings(findings);
        assert_eq!(deduped.len(), 2);
    }

    #[test]
    fn dedup_empty_input_returns_empty() {
        let deduped = deduplicate_findings(vec![]);
        assert!(deduped.is_empty());
    }
}
```

### 8. TOML Parser Tests for New Sections

**Location**: `src/adapters/toml/parser.rs` under `#[cfg(test)] mod tests`

These tests are already sketched in [08-config-design.md](./08-config-design.md). The full set verifies that all new config sections parse correctly alongside existing sections.

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_minimal_config() {
        let toml_str = r#"
            [project]
            prefix = "NOS"
        "#;
        let file: NoslopFile = toml::from_str(toml_str).unwrap();
        assert_eq!(file.project.prefix, "NOS");
        assert!(file.agent.is_none());
        assert!(file.review.is_none());
        assert!(file.checks.is_empty());
    }

    #[test]
    fn parse_with_agent_section() {
        let toml_str = r#"
            [project]
            prefix = "NOS"

            [agent]
            type = "claude"
        "#;
        let file: NoslopFile = toml::from_str(toml_str).unwrap();
        assert_eq!(file.agent.unwrap().agent_type, "claude");
    }

    #[test]
    fn parse_with_review_pipeline() {
        let toml_str = r#"
            [project]
            prefix = "NOS"

            [review]
            analyzers = ["conventions", "formatting", "co-change"]
        "#;
        let file: NoslopFile = toml::from_str(toml_str).unwrap();
        let review = file.review.unwrap();
        assert_eq!(review.analyzers.len(), 3);
        assert_eq!(review.analyzers[0], "conventions");
    }

    #[test]
    fn parse_with_co_change_config() {
        let toml_str = r#"
            [project]
            prefix = "NOS"

            [review]
            analyzers = ["co-change"]

            [review.co-change]
            min_correlation = 0.8
            lookback = 1000
            min_commits = 5
        "#;
        let file: NoslopFile = toml::from_str(toml_str).unwrap();
        let cc = file.review.unwrap().co_change.unwrap();
        assert!((cc.min_correlation - 0.8).abs() < f64::EPSILON);
        assert_eq!(cc.lookback, 1000);
        assert_eq!(cc.min_commits, 5);
    }

    #[test]
    fn parse_with_agent_analyzer() {
        let toml_str = r#"
            [project]
            prefix = "NOS"

            [agent]
            type = "claude"

            [review]
            analyzers = ["agent:security"]

            [review."agent:security"]
            agent = "claude"
            prompt_template = ".noslop/prompts/security.md"
        "#;
        let file: NoslopFile = toml::from_str(toml_str).unwrap();
        let review = file.review.unwrap();
        assert!(review.analyzer_configs.contains_key("agent:security"));
    }

    #[test]
    fn parse_with_script_analyzer() {
        let toml_str = r#"
            [project]
            prefix = "NOS"

            [review]
            analyzers = ["custom:migrations"]

            [review."custom:migrations"]
            type = "script"
            command = "python ./scripts/check-migrations.py"
        "#;
        let file: NoslopFile = toml::from_str(toml_str).unwrap();
        let review = file.review.unwrap();
        assert!(review.analyzer_configs.contains_key("custom:migrations"));
    }

    #[test]
    fn parse_with_formatting_config() {
        let toml_str = r#"
            [project]
            prefix = "NOS"

            [review]
            analyzers = ["formatting"]

            [review.formatting]
            tools = ["rustfmt", "prettier"]
        "#;
        let file: NoslopFile = toml::from_str(toml_str).unwrap();
        let fmt = file.review.unwrap().formatting.unwrap();
        assert_eq!(fmt.tools, vec!["rustfmt", "prettier"]);
    }

    #[test]
    fn parse_full_review_config() {
        let toml_str = r#"
            [project]
            prefix = "NOS"

            [agent]
            type = "claude"

            [review]
            analyzers = ["conventions", "formatting", "co-change", "agent:security", "custom:lint"]

            [review.co-change]
            min_correlation = 0.8
            lookback = 1000
            min_commits = 5

            [review."agent:security"]
            agent = "claude"
            prompt_template = ".noslop/prompts/security.md"

            [review."custom:lint"]
            type = "script"
            command = "eslint --format json ."

            [review.formatting]
            tools = ["rustfmt", "prettier"]

            [[check]]
            target = "*.rs"
            message = "Check Rust files"
            severity = "warn"
        "#;
        let file: NoslopFile = toml::from_str(toml_str).unwrap();

        assert!(file.agent.is_some());
        let review = file.review.unwrap();
        assert_eq!(review.analyzers.len(), 5);
        assert!(review.co_change.is_some());
        assert!(review.formatting.is_some());
        assert!(review.analyzer_configs.contains_key("agent:security"));
        assert!(review.analyzer_configs.contains_key("custom:lint"));
        assert_eq!(file.checks.len(), 1);
    }

    #[test]
    fn existing_config_without_new_sections_parses() {
        let toml_str = r#"
            [project]
            prefix = "NOS"

            [[check]]
            id = "NOS-1"
            target = "src/adapters/git/hooks.rs"
            message = "Verify hook installation preserves permissions"
            severity = "block"
        "#;
        let file: NoslopFile = toml::from_str(toml_str).unwrap();
        assert_eq!(file.project.prefix, "NOS");
        assert!(file.agent.is_none());
        assert!(file.review.is_none());
        assert_eq!(file.checks.len(), 1);
    }

    #[test]
    fn agent_section_to_agent_type() {
        let section = AgentSection { agent_type: "claude".to_string() };
        assert_eq!(section.to_agent_type().unwrap().command_name(), "claude");
    }

    #[test]
    fn agent_section_unknown_type_errors() {
        let section = AgentSection { agent_type: "cursor".to_string() };
        assert!(section.to_agent_type().is_err());
    }

    #[test]
    fn co_change_config_defaults() {
        let config = CoChangeConfig::default();
        assert!((config.min_correlation - 0.7).abs() < f64::EPSILON);
        assert_eq!(config.lookback, 500);
        assert_eq!(config.min_commits, 10);
    }

    #[test]
    fn review_section_default_analyzers() {
        let section = ReviewSection::default();
        assert_eq!(section.analyzers, vec!["conventions"]);
    }
}
```

### 9. AgentType Parsing Tests

**Location**: `src/core/ports/agent.rs` under `#[cfg(test)] mod tests`

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_type_from_str_claude() {
        assert_eq!("claude".parse::<AgentType>().unwrap(), AgentType::Claude);
    }

    #[test]
    fn agent_type_from_str_claude_code() {
        assert_eq!("claude-code".parse::<AgentType>().unwrap(), AgentType::Claude);
    }

    #[test]
    fn agent_type_from_str_codex() {
        assert_eq!("codex".parse::<AgentType>().unwrap(), AgentType::Codex);
    }

    #[test]
    fn agent_type_from_str_openai_alias() {
        assert_eq!("openai".parse::<AgentType>().unwrap(), AgentType::Codex);
    }

    #[test]
    fn agent_type_from_str_case_insensitive() {
        assert_eq!("CLAUDE".parse::<AgentType>().unwrap(), AgentType::Claude);
    }

    #[test]
    fn agent_type_from_str_unknown_errors() {
        assert!("gpt".parse::<AgentType>().is_err());
    }

    #[test]
    fn agent_type_display() {
        assert_eq!(AgentType::Claude.to_string(), "claude");
        assert_eq!(AgentType::Codex.to_string(), "codex");
    }

    #[test]
    fn agent_type_command_name() {
        assert_eq!(AgentType::Claude.command_name(), "claude");
        assert_eq!(AgentType::Codex.command_name(), "codex");
    }

    #[test]
    fn agent_type_display_name() {
        assert_eq!(AgentType::Claude.display_name(), "Claude Code");
        assert_eq!(AgentType::Codex.display_name(), "Codex CLI");
    }

    #[test]
    fn invocation_config_default() {
        let config = InvocationConfig::default();
        assert!(config.prompt.is_empty());
        assert_eq!(config.working_dir, PathBuf::from("."));
        assert!(config.timeout_secs.is_none());
        assert!(config.bypass_permissions);
    }

    #[test]
    fn agent_config_bundle_default() {
        let bundle = AgentConfigBundle::default();
        assert!(bundle.global_instructions.is_none());
        assert!(bundle.settings.is_none());
        assert!(bundle.hooks.is_empty());
        assert!(bundle.commands.is_empty());
    }
}
```

### 10. Validation Tests

**Location**: `src/cli/commands/review.rs` or `src/adapters/toml/parser.rs`

```rust
#[test]
fn validate_known_analyzer_names() {
    let review = ReviewSection {
        analyzers: vec![
            "conventions".to_string(),
            "formatting".to_string(),
            "co-change".to_string(),
        ],
        ..Default::default()
    };
    assert!(validate_analyzers(&review).is_ok());
}

#[test]
fn validate_unknown_analyzer_name_errors() {
    let review = ReviewSection {
        analyzers: vec!["nonexistent".to_string()],
        ..Default::default()
    };
    assert!(validate_analyzers(&review).is_err());
}

#[test]
fn validate_agent_prefix_accepted() {
    let review = ReviewSection {
        analyzers: vec!["agent:security".to_string()],
        ..Default::default()
    };
    assert!(validate_analyzers(&review).is_ok());
}

#[test]
fn validate_custom_prefix_accepted() {
    let review = ReviewSection {
        analyzers: vec!["custom:lint".to_string()],
        ..Default::default()
    };
    assert!(validate_analyzers(&review).is_ok());
}
```

---

## Adapter Tests (Real I/O, Temp Dirs)

### 1. Checkpoint Adapter Tests

**Location**: `src/adapters/git/checkpoint.rs` under `#[cfg(test)] mod tests`

Each test creates a `TempDir` with `git2::Repository::init`.

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use git2::{Repository, Signature};
    use tempfile::TempDir;

    fn setup_repo() -> (TempDir, Repository) {
        let dir = TempDir::new().unwrap();
        let repo = Repository::init(dir.path()).unwrap();

        // Create initial commit so HEAD exists
        let mut index = repo.index().unwrap();
        let tree_oid = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_oid).unwrap();
        let sig = Signature::now("test", "test@test.com").unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[]).unwrap();

        (dir, repo)
    }

    #[test]
    fn is_clean_on_fresh_repo() {
        let (_dir, repo) = setup_repo();
        assert!(is_clean(&repo).unwrap());
    }

    #[test]
    fn is_clean_with_untracked_file() {
        let (dir, repo) = setup_repo();
        std::fs::write(dir.path().join("new.txt"), "content").unwrap();
        assert!(!is_clean(&repo).unwrap());
    }

    #[test]
    fn is_clean_with_staged_file() {
        let (dir, repo) = setup_repo();
        std::fs::write(dir.path().join("staged.txt"), "content").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("staged.txt")).unwrap();
        index.write().unwrap();
        assert!(!is_clean(&repo).unwrap());
    }

    #[test]
    fn commit_all_stages_and_commits() {
        let (dir, repo) = setup_repo();
        std::fs::write(dir.path().join("file.txt"), "content").unwrap();

        let result = commit_all(&repo, "test checkpoint").unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().len(), 7); // short SHA length
        assert!(is_clean(&repo).unwrap());
    }

    #[test]
    fn commit_all_returns_none_on_clean_tree() {
        let (_dir, repo) = setup_repo();
        let result = commit_all(&repo, "nothing here").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn commit_all_handles_deleted_files() {
        let (dir, repo) = setup_repo();

        // Create and commit a file
        std::fs::write(dir.path().join("to_delete.txt"), "content").unwrap();
        commit_all(&repo, "add file").unwrap();

        // Delete it
        std::fs::remove_file(dir.path().join("to_delete.txt")).unwrap();

        let result = commit_all(&repo, "delete file").unwrap();
        assert!(result.is_some());
        assert!(is_clean(&repo).unwrap());
    }

    #[test]
    fn commit_all_message_in_commit() {
        let (dir, repo) = setup_repo();
        std::fs::write(dir.path().join("file.txt"), "content").unwrap();

        commit_all(&repo, "my checkpoint message").unwrap();

        let head = repo.head().unwrap().peel_to_commit().unwrap();
        assert_eq!(head.message().unwrap(), "my checkpoint message");
    }
}
```

| Test                                    | What It Asserts                                         |
| --------------------------------------- | ------------------------------------------------------- |
| `is_clean_on_fresh_repo`                | Clean repo returns `true`                               |
| `is_clean_with_untracked_file`          | Untracked file means not clean                          |
| `is_clean_with_staged_file`             | Staged but uncommitted means not clean                  |
| `commit_all_stages_and_commits`         | Returns `Some(sha)` with 7-char SHA, tree becomes clean |
| `commit_all_returns_none_on_clean_tree` | Returns `None` when nothing to commit                   |
| `commit_all_handles_deleted_files`      | Deletion is committed correctly                         |
| `commit_all_message_in_commit`          | Commit message matches the argument                     |

### 2. Git Diff Extraction Tests

**Location**: `src/adapters/git/mod.rs` under `#[cfg(test)] mod review_vcs_tests`

Tests for the `VersionControl` trait extensions (`diff_between`, `commits_between`, `file_at_revision`) against real `git2` repositories.

```rust
#[cfg(test)]
mod review_vcs_tests {
    use super::*;
    use git2::{Repository, Signature};
    use tempfile::TempDir;

    fn setup_repo_with_branch() -> (TempDir, Repository) {
        let dir = TempDir::new().unwrap();
        let repo = Repository::init(dir.path()).unwrap();

        // Create initial commit on main
        std::fs::write(dir.path().join("README.md"), "# test").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("README.md")).unwrap();
        index.write().unwrap();
        let tree_oid = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_oid).unwrap();
        let sig = Signature::now("test", "test@test.com").unwrap();
        let main_commit = repo.commit(
            Some("HEAD"), &sig, &sig, "initial", &tree, &[]
        ).unwrap();

        // Create feature branch with an additional file
        let head = repo.find_commit(main_commit).unwrap();
        repo.branch("feature", &head, false).unwrap();
        repo.set_head("refs/heads/feature").unwrap();
        repo.checkout_head(Some(
            git2::build::CheckoutBuilder::new().force()
        )).unwrap();

        std::fs::write(dir.path().join("new_file.rs"), "fn main() {}").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("new_file.rs")).unwrap();
        index.write().unwrap();
        let tree_oid = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_oid).unwrap();
        repo.commit(
            Some("HEAD"), &sig, &sig, "add new file", &tree, &[&head]
        ).unwrap();

        (dir, repo)
    }

    #[test]
    fn diff_between_shows_added_file() {
        let (dir, _repo) = setup_repo_with_branch();
        let vcs = GitVersionControl::new(dir.path());

        let diffs = vcs.diff_between("refs/heads/master", "refs/heads/feature").unwrap();
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].path, PathBuf::from("new_file.rs"));
        assert_eq!(diffs[0].status, DiffStatus::Added);
    }

    #[test]
    fn commits_between_lists_feature_commits() {
        let (dir, _repo) = setup_repo_with_branch();
        let vcs = GitVersionControl::new(dir.path());

        let commits = vcs.commits_between("refs/heads/master", "refs/heads/feature").unwrap();
        assert_eq!(commits.len(), 1);
        assert_eq!(commits[0].summary, "add new file");
    }

    #[test]
    fn file_at_revision_reads_content() {
        let (dir, _repo) = setup_repo_with_branch();
        let vcs = GitVersionControl::new(dir.path());

        let content = vcs.file_at_revision(
            Path::new("new_file.rs"), "refs/heads/feature"
        ).unwrap();
        assert_eq!(content, "fn main() {}");
    }

    #[test]
    fn file_at_revision_returns_error_for_missing_file() {
        let (dir, _repo) = setup_repo_with_branch();
        let vcs = GitVersionControl::new(dir.path());

        let result = vcs.file_at_revision(
            Path::new("nonexistent.rs"), "refs/heads/feature"
        );
        assert!(result.is_err());
    }

    #[test]
    fn recent_commits_returns_correct_count() {
        let (dir, _repo) = setup_repo_with_branch();
        let vcs = GitVersionControl::new(dir.path());

        let commits = vcs.recent_commits(10).unwrap();
        assert_eq!(commits.len(), 2); // initial + feature commit
    }

    #[test]
    fn diff_stat_since_counts_changes() {
        let (dir, _repo) = setup_repo_with_branch();
        let vcs = GitVersionControl::new(dir.path());

        let stat = vcs.diff_stat_since("refs/heads/master").unwrap();
        assert_eq!(stat.files_changed, 1);
        assert!(stat.insertions > 0);
    }
}
```

### 3. Review Session File I/O Tests

**Location**: `tests/adapter/review_session_test.rs`

Tests for reading and writing review session JSON files in `.noslop/reviews/`.

```rust
use tempfile::TempDir;
use std::fs;

#[test]
fn write_and_read_review_session() {
    let temp = TempDir::new().unwrap();
    let reviews_dir = temp.path().join(".noslop/reviews");
    fs::create_dir_all(&reviews_dir).unwrap();

    let session = ReviewSession {
        id: "review-abc123".to_string(),
        target: ReviewTarget {
            base: "main".to_string(),
            head: "feature/x".to_string(),
            base_commit: "a1b2c3d".to_string(),
            head_commit: "e4f5g6h".to_string(),
        },
        started_at: "2026-02-16T10:00:00Z".to_string(),
        completed_at: Some("2026-02-16T10:02:30Z".to_string()),
        analyzers_run: vec!["conventions".to_string(), "agent:security".to_string()],
        findings: vec![
            FindingBuilder::new().source("conventions").message("unused import").build(),
        ],
        resolutions: HashMap::new(),
    };

    let path = reviews_dir.join("review-abc123.json");
    let json = serde_json::to_string_pretty(&session).unwrap();
    fs::write(&path, &json).unwrap();

    let loaded: ReviewSession = serde_json::from_str(
        &fs::read_to_string(&path).unwrap()
    ).unwrap();

    assert_eq!(loaded.id, "review-abc123");
    assert_eq!(loaded.findings.len(), 1);
    assert_eq!(loaded.analyzers_run.len(), 2);
}

#[test]
fn list_review_sessions_finds_all_json_files() {
    let temp = TempDir::new().unwrap();
    let reviews_dir = temp.path().join(".noslop/reviews");
    fs::create_dir_all(&reviews_dir).unwrap();

    fs::write(reviews_dir.join("review-001.json"), "{}").unwrap();
    fs::write(reviews_dir.join("review-002.json"), "{}").unwrap();
    fs::write(reviews_dir.join("memory.json"), "{}").unwrap(); // Not a session

    let sessions = list_review_sessions(&reviews_dir).unwrap();
    assert_eq!(sessions.len(), 2); // Only review-*.json
}
```

### 4. Agent Adapter Tests (Command Building, Output Parsing)

**Location**: `tests/adapter/claude_adapter_test.rs` and `tests/adapter/codex_adapter_test.rs`

These tests verify command construction and output parsing without invoking the real agent CLIs.

```rust
// Claude adapter tests
use noslop::adapters::agent::{ClaudeConfig, ClaudeRuntime};
use noslop::core::ports::agent::*;
use std::path::Path;

#[test]
fn claude_config_generates_all_files() {
    let config = ClaudeConfig::new();
    let bundle = config.generate_config(Path::new("/tmp/test")).unwrap();

    assert!(bundle.global_instructions.is_some());
    assert!(bundle.settings.is_some());
    assert_eq!(bundle.hooks.len(), 2);
    assert_eq!(bundle.commands.len(), 2);
}

#[test]
fn claude_settings_is_valid_json() {
    let config = ClaudeConfig::new();
    let bundle = config.generate_config(Path::new("/tmp/test")).unwrap();
    let settings = bundle.settings.unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&settings.content).unwrap();
    assert_eq!(parsed["model"], "opus");
    assert!(parsed["permissions"]["allow"].is_array());
}

#[test]
fn claude_settings_includes_noslop_permission() {
    let config = ClaudeConfig::new();
    let bundle = config.generate_config(Path::new("/tmp/test")).unwrap();
    let settings = bundle.settings.unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&settings.content).unwrap();
    let allow = parsed["permissions"]["allow"].as_array().unwrap();
    let allow_strings: Vec<&str> = allow.iter().filter_map(|v| v.as_str()).collect();

    assert!(allow_strings.contains(&"Bash(noslop *)"));
    assert!(allow_strings.contains(&"Bash(checkpoint *)"));
}

#[test]
fn claude_hooks_have_correct_events() {
    let config = ClaudeConfig::new();
    let bundle = config.generate_config(Path::new("/tmp/test")).unwrap();

    let events: Vec<&str> = bundle.hooks.iter().map(|h| h.event.as_str()).collect();
    assert!(events.contains(&"PreToolUse"));
    assert!(events.contains(&"PostToolUse"));
}

#[test]
fn claude_runtime_build_command_with_bypass() {
    let runtime = ClaudeRuntime::new();
    let config = InvocationConfig {
        prompt: "Review this diff for security issues".to_string(),
        bypass_permissions: true,
        ..Default::default()
    };

    let (cmd, args) = runtime.build_command(&config);
    assert_eq!(cmd, "claude");
    assert!(args.contains(&"-p".to_string()));
    assert!(args.contains(&"--output-format".to_string()));
    assert!(args.contains(&"text".to_string()));
    assert!(args.contains(&"--permission-mode".to_string()));
    assert!(args.contains(&"bypassPermissions".to_string()));
}

#[test]
fn claude_runtime_build_command_without_bypass() {
    let runtime = ClaudeRuntime::new();
    let config = InvocationConfig {
        prompt: "Review this diff".to_string(),
        bypass_permissions: false,
        ..Default::default()
    };

    let (_cmd, args) = runtime.build_command(&config);
    assert!(!args.contains(&"--permission-mode".to_string()));
}

#[test]
fn codex_config_has_no_hooks_or_commands() {
    let config = CodexConfig::new();
    let bundle = config.generate_config(Path::new("/tmp/test")).unwrap();

    assert!(bundle.global_instructions.is_some());
    assert!(bundle.settings.is_none());
    assert!(bundle.hooks.is_empty());
    assert!(bundle.commands.is_empty());
}

#[test]
fn codex_instructions_path_is_correct() {
    let config = CodexConfig::new();
    let bundle = config.generate_config(Path::new("/tmp/test")).unwrap();
    let instructions = bundle.global_instructions.unwrap();

    assert_eq!(instructions.relative_path, PathBuf::from(".codex/instructions.md"));
}

#[test]
fn codex_runtime_build_command() {
    let runtime = CodexRuntime::new();
    let config = InvocationConfig {
        prompt: "Review this diff for quality issues".to_string(),
        ..Default::default()
    };

    let (cmd, args) = runtime.build_command(&config);
    assert_eq!(cmd, "codex");
    assert!(args.contains(&"--quiet".to_string()));
    assert!(args.contains(&"--approval-mode".to_string()));
    assert!(args.contains(&"full-auto".to_string()));
}
```

### 5. Shared Output Parser Tests

**Location**: `src/adapters/agent/parsing.rs` under `#[cfg(test)] mod tests`

Already defined in [04-agent-adapters-design.md](./04-agent-adapters-design.md):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ports::agent::Severity;

    #[test]
    fn extracts_findings_from_clean_json() {
        let output = r#"[{"severity":"warning","file":"src/auth.rs","line":42,"message":"Timing attack","suggestion":"Use constant-time comparison","source":"agent:security"}]"#;
        let result = extract_findings(output);
        assert_eq!(result.findings.len(), 1);
        assert_eq!(result.findings[0].severity, Severity::Warning);
        assert_eq!(result.findings[0].file, "src/auth.rs");
    }

    #[test]
    fn extracts_findings_from_narrative_wrapper() {
        let output = r#"Here are my findings from the security review:

[{"severity":"error","file":"src/db.rs","line":15,"message":"SQL injection risk","suggestion":"Use parameterized queries","source":"agent:security"}]

I also noticed the code style is generally clean."#;

        let result = extract_findings(output);
        assert_eq!(result.findings.len(), 1);
        assert_eq!(result.findings[0].message, "SQL injection risk");
    }

    #[test]
    fn returns_empty_findings_when_no_json() {
        let output = "I reviewed the code and found no issues.";
        let result = extract_findings(output);
        assert!(result.findings.is_empty());
        assert_eq!(result.raw_output, output);
    }

    #[test]
    fn extracts_error_lines() {
        let output = "working...\nError: timeout exceeded\nFAILED to parse\n";
        let result = extract_findings(output);
        assert_eq!(result.errors.len(), 2);
    }

    #[test]
    fn handles_empty_json_array() {
        let output = "[]";
        let result = extract_findings(output);
        assert!(result.findings.is_empty());
        assert!(result.errors.is_empty());
    }

    #[test]
    fn handles_malformed_json() {
        let output = "[{invalid json}]";
        let result = extract_findings(output);
        assert!(result.findings.is_empty());
        assert!(!result.raw_output.is_empty());
    }

    #[test]
    fn handles_nested_json_arrays() {
        let output = r#"[{"severity":"info","file":"a.rs","line":1,"message":"note","suggestion":null,"source":"test"}]"#;
        let result = extract_findings(output);
        assert_eq!(result.findings.len(), 1);
    }
}
```

### 6. Config Bundle Writing Tests

**Location**: `tests/adapter/config_writer_test.rs`

```rust
use tempfile::TempDir;

#[test]
fn write_config_bundle_to_tempdir() {
    let home = TempDir::new().unwrap();
    let config = ClaudeConfig::new();
    let bundle = config.generate_config(Path::new("/tmp/test")).unwrap();

    let written = write_config_bundle(home.path(), &bundle, false).unwrap();

    assert!(home.path().join(".claude/CLAUDE.md").exists());
    assert!(home.path().join(".claude/settings.json").exists());
    assert!(home.path().join(".claude/commands/checkpoint.md").exists());
    assert!(home.path().join(".claude/commands/worktree.md").exists());
    assert!(written.len() >= 4);
}

#[test]
fn write_config_bundle_creates_backup() {
    let home = TempDir::new().unwrap();
    let claude_dir = home.path().join(".claude");
    std::fs::create_dir_all(&claude_dir).unwrap();
    std::fs::write(claude_dir.join("CLAUDE.md"), "old content").unwrap();

    let config = ClaudeConfig::new();
    let bundle = config.generate_config(Path::new("/tmp/test")).unwrap();

    write_config_bundle(home.path(), &bundle, false).unwrap();

    // Original should be backed up
    assert!(claude_dir.join("CLAUDE.bak").exists());
    let backup = std::fs::read_to_string(claude_dir.join("CLAUDE.bak")).unwrap();
    assert_eq!(backup, "old content");
}
```

---

## Integration Tests (assert_cmd, Real CLI)

### 1. `noslop review` End-to-End Tests

**Location**: `tests/integration/review_test.rs`

```rust
use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn cli_review_with_no_changes() {
    let repo = TempGitRepo::new();
    repo.init_with_commit();

    Command::cargo_bin("noslop")
        .unwrap()
        .current_dir(repo.path())
        .args(["init"])
        .assert()
        .success();

    Command::cargo_bin("noslop")
        .unwrap()
        .current_dir(repo.path())
        .args(["review"])
        .assert()
        .success()
        .stdout(predicates::str::contains("No changes to review"));
}

#[test]
fn cli_review_with_changes_runs_conventions() {
    let repo = TempGitRepo::new();
    repo.init_with_commit();

    // Initialize noslop with a check
    Command::cargo_bin("noslop")
        .unwrap()
        .current_dir(repo.path())
        .args(["init"])
        .assert()
        .success();

    Command::cargo_bin("noslop")
        .unwrap()
        .current_dir(repo.path())
        .args(["check", "add", "*.rs", "-m", "Review Rust files"])
        .assert()
        .success();

    // Create a branch with changes
    repo.create_branch("feature");
    repo.checkout("feature");
    repo.write_file("src/new.rs", "fn main() {}");
    repo.write_and_stage("src/new.rs", "fn main() {}");
    repo.commit("add new file");

    Command::cargo_bin("noslop")
        .unwrap()
        .current_dir(repo.path())
        .args(["review", "--base", "master"])
        .assert()
        .success();
}

#[test]
fn cli_review_check_mode_exits_nonzero_on_blocking_findings() {
    let repo = TempGitRepo::new();
    repo.init_with_commit();

    Command::cargo_bin("noslop")
        .unwrap()
        .current_dir(repo.path())
        .args(["init"])
        .assert()
        .success();

    // Add a blocking check that will trigger
    Command::cargo_bin("noslop")
        .unwrap()
        .current_dir(repo.path())
        .args(["check", "add", "*.rs", "-m", "Must review", "-s", "block"])
        .assert()
        .success();

    // Create changes matching the check
    repo.create_branch("feature");
    repo.checkout("feature");
    repo.write_and_stage("src/new.rs", "fn main() {}");
    repo.commit("add file");

    Command::cargo_bin("noslop")
        .unwrap()
        .current_dir(repo.path())
        .args(["review", "--check", "--base", "master"])
        .assert()
        .failure();
}

#[test]
fn cli_review_json_output() {
    let repo = TempGitRepo::new();
    repo.init_with_commit();

    Command::cargo_bin("noslop")
        .unwrap()
        .current_dir(repo.path())
        .args(["init"])
        .assert()
        .success();

    Command::cargo_bin("noslop")
        .unwrap()
        .current_dir(repo.path())
        .args(["review", "--json"])
        .assert()
        .success()
        .stdout(predicates::str::starts_with("{"));
}
```

### 2. `noslop findings` Subcommand Tests

**Location**: `tests/integration/findings_test.rs`

| Test                                      | What It Asserts                                                           |
| ----------------------------------------- | ------------------------------------------------------------------------- |
| `cli_findings_list_empty_when_no_reviews` | `noslop findings` with no review sessions outputs "No findings"           |
| `cli_findings_list_shows_findings`        | After a review produces findings, `noslop findings` lists them            |
| `cli_findings_resolve_marks_resolved`     | `noslop findings resolve F-001` marks the finding as resolved             |
| `cli_findings_dismiss_marks_dismissed`    | `noslop findings dismiss F-001 -m "not applicable"` dismisses with reason |

### 3. `noslop checkpoint` CLI Tests

**Location**: `tests/integration/checkpoint_test.rs`

```rust
use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn cli_checkpoint_with_changes() {
    let repo = TempGitRepo::new();
    repo.init_with_commit();
    repo.write_file("test.txt", "content");

    Command::cargo_bin("noslop")
        .unwrap()
        .current_dir(repo.path())
        .args(["checkpoint", "test message"])
        .assert()
        .success()
        .stdout(predicates::str::contains("Checkpoint:"));
}

#[test]
fn cli_checkpoint_clean_tree() {
    let repo = TempGitRepo::new();
    repo.init_with_commit();

    Command::cargo_bin("noslop")
        .unwrap()
        .current_dir(repo.path())
        .args(["checkpoint"])
        .assert()
        .success()
        .stdout(predicates::str::contains("Nothing to checkpoint"));
}

#[test]
fn cli_checkpoint_json_output() {
    let repo = TempGitRepo::new();
    repo.init_with_commit();
    repo.write_file("test.txt", "content");

    Command::cargo_bin("noslop")
        .unwrap()
        .current_dir(repo.path())
        .args(["checkpoint", "--json", "test"])
        .assert()
        .success()
        .stdout(predicates::str::contains("\"success\": true"));
}

#[test]
fn cli_checkpoint_multiple_files_committed() {
    let repo = TempGitRepo::new();
    repo.init_with_commit();
    repo.write_file("a.txt", "a");
    repo.write_file("b.txt", "b");
    repo.write_file("c.txt", "c");

    Command::cargo_bin("noslop")
        .unwrap()
        .current_dir(repo.path())
        .args(["checkpoint", "batch"])
        .assert()
        .success();

    assert!(repo.is_clean());
}
```

### 4. `noslop init claude` CLI Tests

**Location**: `tests/integration/init_test.rs`

```rust
#[test]
fn init_bare_creates_noslop_toml_without_agent() {
    let repo = TempGitRepo::new();

    Command::cargo_bin("noslop")
        .unwrap()
        .current_dir(repo.path())
        .arg("init")
        .assert()
        .success();

    let toml_content = std::fs::read_to_string(
        repo.path().join(".noslop.toml")
    ).unwrap();
    assert!(toml_content.contains("[project]"));
    assert!(!toml_content.contains("[agent]"));
    assert!(!toml_content.contains("[review]"));

    // Pre-push hook should NOT be installed without agent
    assert!(!repo.path().join(".git/hooks/pre-push").exists());
}

#[test]
fn init_claude_creates_agent_and_review_sections() {
    let repo = TempGitRepo::new();

    // Note: this test succeeds for .noslop.toml generation even
    // if claude CLI is not installed (agent validation is Phase 2)
    Command::cargo_bin("noslop")
        .unwrap()
        .current_dir(repo.path())
        .args(["init", "claude"])
        .assert();

    let toml_content = std::fs::read_to_string(
        repo.path().join(".noslop.toml")
    ).unwrap();
    assert!(toml_content.contains("[agent]"));
    assert!(toml_content.contains("type = \"claude\""));
    assert!(toml_content.contains("[review]"));

    // Pre-push hook should be installed
    assert!(repo.path().join(".git/hooks/pre-push").exists());
}

#[test]
fn init_unknown_agent_fails() {
    let repo = TempGitRepo::new();

    Command::cargo_bin("noslop")
        .unwrap()
        .current_dir(repo.path())
        .args(["init", "cursor"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("Unknown agent type"));
}

#[test]
fn init_force_overwrites_existing() {
    let repo = TempGitRepo::new();

    // First init
    Command::cargo_bin("noslop")
        .unwrap()
        .current_dir(repo.path())
        .arg("init")
        .assert()
        .success();

    // Second init without --force
    Command::cargo_bin("noslop")
        .unwrap()
        .current_dir(repo.path())
        .arg("init")
        .assert()
        .success()
        .stdout(predicates::str::contains("Already initialized"));

    // Second init with --force
    Command::cargo_bin("noslop")
        .unwrap()
        .current_dir(repo.path())
        .args(["init", "--force"])
        .assert()
        .success()
        .stdout(predicates::str::contains("Initializing"));
}
```

### 5. Pre-Push Hook Integration Tests

**Location**: `tests/integration/hook_test.rs`

```rust
#[test]
fn pre_push_hook_blocks_on_unresolved_findings() {
    let repo = TempGitRepo::new();
    repo.init_with_commit();

    // Initialize with agent to get pre-push hook
    Command::cargo_bin("noslop")
        .unwrap()
        .current_dir(repo.path())
        .args(["init", "claude"])
        .assert();

    // Verify pre-push hook exists and is executable
    let hook_path = repo.path().join(".git/hooks/pre-push");
    assert!(hook_path.exists());

    let content = std::fs::read_to_string(&hook_path).unwrap();
    assert!(content.contains("noslop review --check"));
}
```

---

## Agent Integration Tests (#[ignore], Require Real CLI)

These tests invoke real agent CLIs and are `#[ignore]`-tagged. Run with `cargo test -- --ignored`.

**Location**: `tests/integration/agent_integration_test.rs`

### Claude Review Invocation

```rust
#[test]
#[ignore] // Run with: cargo test -- --ignored
fn claude_review_produces_findings() {
    let runtime = ClaudeRuntime::new();

    if !runtime.is_available() {
        eprintln!("Skipping: claude CLI not found");
        return;
    }

    let config = InvocationConfig {
        prompt: r#"Review this diff and return findings as a JSON array.

+fn handle_login(username: &str, password: &str) -> bool {
+    let stored = db::get_password(username);
+    stored == password  // direct comparison
+}

Return ONLY a JSON array of findings with: severity, file, line, message, suggestion, source."#
            .to_string(),
        working_dir: std::env::temp_dir(),
        timeout_secs: Some(60),
        bypass_permissions: true,
        ..Default::default()
    };

    let result = runtime.invoke(&config).unwrap();
    assert_eq!(result.exit_code, 0);

    let output = runtime.parse_output(&result);
    // Agent should identify the timing-attack-vulnerable comparison
    assert!(!output.findings.is_empty() || !output.raw_output.is_empty());
}

#[test]
#[ignore]
fn claude_is_available_check() {
    let config = ClaudeConfig::new();
    if config.is_available() {
        assert!(config.validate().is_ok());
    }
}
```

### Codex Review Invocation

```rust
#[test]
#[ignore]
fn codex_review_produces_findings() {
    let runtime = CodexRuntime::new();

    if !runtime.is_available() {
        eprintln!("Skipping: codex CLI not found");
        return;
    }

    let config = InvocationConfig {
        prompt: r#"Review this diff for security issues. Return findings as JSON array.

+fn query_user(name: &str) -> String {
+    format!("SELECT * FROM users WHERE name = '{name}'")
+}

Return ONLY a JSON array."#.to_string(),
        working_dir: std::env::temp_dir(),
        timeout_secs: Some(60),
        ..Default::default()
    };

    let result = runtime.invoke(&config).unwrap();
    assert_eq!(result.exit_code, 0);

    let output = runtime.parse_output(&result);
    assert!(!output.findings.is_empty() || !output.raw_output.is_empty());
}

#[test]
#[ignore]
fn codex_is_available_check() {
    let config = CodexConfig::new();
    if config.is_available() {
        assert!(config.validate().is_ok());
    }
}
```

---

## Property-Based Tests (proptest)

### Finding Deduplication Invariants

**Location**: `tests/unit/proptest_findings.rs`

```rust
use noslop::core::ports::agent::{Finding, Severity};
use proptest::prelude::*;

fn arb_severity() -> impl Strategy<Value = Severity> {
    prop_oneof![
        Just(Severity::Info),
        Just(Severity::Warning),
        Just(Severity::Error),
        Just(Severity::Critical),
    ]
}

fn arb_finding() -> impl Strategy<Value = Finding> {
    (
        arb_severity(),
        "[a-z]{3,10}\\.rs",
        proptest::option::of(1u32..200),
        "[A-Za-z ]{5,30}",
        proptest::option::of("[A-Za-z ]{5,30}"),
        "[a-z:]{3,15}",
    )
        .prop_map(|(severity, file, line, message, suggestion, source)| Finding {
            severity,
            file,
            line,
            message,
            suggestion,
            source,
        })
}

proptest! {
    /// Deduplication never increases the number of findings
    #[test]
    fn dedup_never_adds_findings(
        findings in proptest::collection::vec(arb_finding(), 0..20)
    ) {
        let count_before = findings.len();
        let deduped = deduplicate_findings(findings);
        prop_assert!(deduped.len() <= count_before);
    }

    /// Deduplication is idempotent
    #[test]
    fn dedup_is_idempotent(
        findings in proptest::collection::vec(arb_finding(), 0..20)
    ) {
        let once = deduplicate_findings(findings);
        let twice = deduplicate_findings(once.clone());
        prop_assert_eq!(once.len(), twice.len());
    }

    /// All output findings were present in the input
    #[test]
    fn dedup_preserves_messages(
        findings in proptest::collection::vec(arb_finding(), 1..10)
    ) {
        let input_messages: Vec<String> = findings.iter()
            .map(|f| f.message.clone())
            .collect();
        let deduped = deduplicate_findings(findings);
        for f in &deduped {
            prop_assert!(input_messages.contains(&f.message));
        }
    }
}
```

### TOML Round-Trip Parsing

**Location**: `tests/unit/proptest_toml.rs`

```rust
use proptest::prelude::*;

proptest! {
    /// Any valid prefix string round-trips through TOML
    #[test]
    fn toml_prefix_round_trip(prefix in "[A-Z]{3}") {
        let toml_str = format!(
            "[project]\nprefix = \"{prefix}\"\n"
        );
        let file: NoslopFile = toml::from_str(&toml_str).unwrap();
        prop_assert_eq!(file.project.prefix, prefix);
    }

    /// Analyzer list round-trips through TOML
    #[test]
    fn toml_analyzer_list_round_trip(
        analyzers in proptest::collection::vec("[a-z:-]{3,20}", 1..5)
    ) {
        let list = analyzers.iter()
            .map(|a| format!("\"{a}\""))
            .collect::<Vec<_>>()
            .join(", ");
        let toml_str = format!(
            "[project]\nprefix = \"TST\"\n\n[review]\nanalyzers = [{list}]\n"
        );
        let file: NoslopFile = toml::from_str(&toml_str).unwrap();
        let review = file.review.unwrap();
        prop_assert_eq!(review.analyzers.len(), analyzers.len());
    }
}
```

### AgentType Parsing

**Location**: `tests/unit/proptest_agent_type.rs`

```rust
use noslop::core::ports::agent::AgentType;
use proptest::prelude::*;
use test_case::test_case;

// Parameterized tests for known aliases
#[test_case("claude", AgentType::Claude ; "lowercase claude")]
#[test_case("Claude", AgentType::Claude ; "titlecase claude")]
#[test_case("CLAUDE", AgentType::Claude ; "uppercase claude")]
#[test_case("claude-code", AgentType::Claude ; "hyphenated claude")]
#[test_case("codex", AgentType::Codex ; "lowercase codex")]
#[test_case("Codex", AgentType::Codex ; "titlecase codex")]
#[test_case("openai", AgentType::Codex ; "openai alias")]
fn agent_type_parsing(input: &str, expected: AgentType) {
    let parsed: AgentType = input.parse().unwrap();
    assert_eq!(parsed, expected);
}

#[test_case("gpt" ; "gpt is unknown")]
#[test_case("cursor" ; "cursor is unknown")]
#[test_case("" ; "empty string")]
#[test_case("aider" ; "aider is unknown")]
fn agent_type_parsing_errors(input: &str) {
    let result: Result<AgentType, _> = input.parse();
    assert!(result.is_err());
}

proptest! {
    /// Random strings that are not known agent names always fail to parse
    #[test]
    fn random_strings_are_not_agent_types(
        s in "[a-z]{8,20}"
    ) {
        // Filter out the known valid inputs
        prop_assume!(
            !["claude", "claude-code", "codex", "openai"]
                .contains(&s.to_lowercase().as_str())
        );
        let result: Result<AgentType, _> = s.parse();
        prop_assert!(result.is_err());
    }
}
```

---

## Test File Layout

### Where Each Test File Lives

```
src/
├── core/
│   ├── ports/
│   │   └── agent.rs              # Inline: AgentType parsing, defaults
│   └── services/
│       ├── pipeline.rs           # Inline: ReviewPipeline orchestrator tests
│       └── finding_merge.rs      # Inline: deduplication tests
├── adapters/
│   ├── git/
│   │   ├── checkpoint.rs         # Inline: git2 checkpoint tests
│   │   └── mod.rs                # Inline: review VCS method tests
│   ├── toml/
│   │   └── parser.rs             # Inline: TOML parser tests (new sections)
│   └── agent/
│       ├── parsing.rs            # Inline: finding extraction tests
│       ├── claude.rs             # Inline: config + runtime tests
│       └── codex.rs              # Inline: config + runtime tests
│   └── analyzer/
│       ├── convention.rs         # Inline: ConventionAnalyzer tests
│       ├── co_change.rs          # Inline: CoChangeAnalyzer tests
│       ├── formatting.rs         # Inline: FormattingAnalyzer tests
│       ├── script.rs             # Inline: ScriptAnalyzer tests
│       └── agent.rs              # Inline: AgentAnalyzer tests

tests/
├── common/
│   ├── mod.rs                    # Extended: re-exports new builders
│   ├── fixtures.rs               # Extended: FindingBuilder, ReviewContextBuilder,
│   │                             #   InvocationResultBuilder
│   ├── git_repo.rs               # Extended: init_with_commit, is_clean,
│   │                             #   head_sha, commit_count, create_branch
│   └── mocks.rs                  # Extended: MockVersionControl gains review
│                                 #   pipeline methods
├── unit/
│   ├── proptest_findings.rs      # NEW: finding deduplication properties
│   ├── proptest_toml.rs          # NEW: TOML round-trip properties
│   └── proptest_agent_type.rs    # NEW: agent type parsing properties
├── adapter/
│   ├── toml_test.rs              # Existing + extended for new sections
│   ├── claude_adapter_test.rs    # NEW: ClaudeConfig/ClaudeRuntime tests
│   ├── codex_adapter_test.rs     # NEW: CodexConfig/CodexRuntime tests
│   ├── config_writer_test.rs     # NEW: write_config_bundle tests
│   ├── review_session_test.rs    # NEW: review session file I/O tests
│   └── checkpoint_test.rs        # NEW: GitVersionControl checkpoint tests
└── integration/
    ├── review_test.rs            # NEW: noslop review CLI tests
    ├── findings_test.rs          # NEW: noslop findings CLI tests
    ├── checkpoint_test.rs        # NEW: noslop checkpoint CLI tests
    ├── init_test.rs              # NEW: noslop init <agent> CLI tests
    ├── hook_test.rs              # NEW: pre-push hook tests
    └── agent_integration_test.rs # NEW: real agent CLI tests (#[ignore])
```

### Module Coverage Map

Every new module maps to its test coverage:

| Module                           | Inline Tests               | External Tests                          | Property Tests          |
| -------------------------------- | -------------------------- | --------------------------------------- | ----------------------- |
| `core::ports::agent` (types)     | AgentType, defaults        | `proptest_agent_type.rs`                | Random string rejection |
| `core::services::pipeline`       | Pipeline orchestration     | --                                      | --                      |
| `core::services::finding_merge`  | Dedup logic                | --                                      | `proptest_findings.rs`  |
| `adapters::git::checkpoint`      | git2 operations            | `adapter/checkpoint_test.rs`            | --                      |
| `adapters::git` (review VCS)     | diff, commits, file_at_rev | --                                      | --                      |
| `adapters::toml::parser` (new)   | All new sections           | `adapter/toml_test.rs`                  | `proptest_toml.rs`      |
| `adapters::agent::parsing`       | JSON extraction            | --                                      | --                      |
| `adapters::agent::claude`        | Config + runtime           | `adapter/claude_adapter_test.rs`        | --                      |
| `adapters::agent::codex`         | Config + runtime           | `adapter/codex_adapter_test.rs`         | --                      |
| `adapters::analyzer::convention` | Check matching             | --                                      | --                      |
| `adapters::analyzer::co_change`  | Correlation logic          | --                                      | --                      |
| `adapters::analyzer::formatting` | Tool delegation            | --                                      | --                      |
| `adapters::analyzer::script`     | Command + parsing          | --                                      | --                      |
| `adapters::analyzer::agent`      | Prompt building            | --                                      | --                      |
| CLI: `noslop review`             | --                         | `integration/review_test.rs`            | --                      |
| CLI: `noslop findings`           | --                         | `integration/findings_test.rs`          | --                      |
| CLI: `noslop checkpoint`         | --                         | `integration/checkpoint_test.rs`        | --                      |
| CLI: `noslop init <agent>`       | --                         | `integration/init_test.rs`              | --                      |
| Real agents                      | --                         | `agent_integration_test.rs` (#[ignore]) | --                      |

## Dev Dependencies

No new dev dependencies required. All test functionality is covered by the existing `Cargo.toml`:

| Crate                     | Used For                                                                                   |
| ------------------------- | ------------------------------------------------------------------------------------------ |
| `mockall` (0.13)          | Auto-generated mocks for `AgentConfig`, `AgentRuntime`, `ReviewAnalyzer`, `VersionControl` |
| `tempfile` (3.23)         | `TempDir` for git repo tests, TOML file tests                                              |
| `assert_cmd` (2.1)        | CLI integration tests for `noslop review`, `noslop checkpoint`, `noslop init`              |
| `predicates` (3.1)        | Stdout/stderr assertions in integration tests                                              |
| `pretty_assertions` (1.4) | Readable diff output for finding comparison                                                |
| `test-case` (3.3)         | Parameterized tests for agent type parsing, analyzer resolution                            |
| `proptest` (1.5)          | Property-based tests for dedup invariants, TOML round-trips                                |

## Test Execution

```bash
# All unit tests (fast, no I/O beyond tempfile)
cargo test --lib

# All unit + adapter tests (uses tempfile and git2, no external deps)
cargo test

# Integration tests that require agent CLIs
cargo test -- --ignored

# Specific module tests
cargo test pipeline
cargo test convention_analyzer
cargo test checkpoint
cargo test finding_merge
cargo test extract_findings

# Property-based tests with more cases
PROPTEST_CASES=1000 cargo test proptest
```
