# Test Strategy

Test plan for the review pipeline, organized by phase to match `10-implementation-plan.md`. Follows existing patterns: inline `#[cfg(test)]` unit tests, `mockall::automock` traits, `test-case` parameterized tests, `proptest` property tests, `TempGitRepo`/`tempfile` for git, `assert_cmd` for CLI.

## Testing Principles

**Hexagonal architecture enables testing without I/O.** Business logic in `src/core/services/` depends only on port traits, never on adapters. Every service test runs with mocks.

**Mock ports via mockall.** All new traits annotated with `#[cfg_attr(test, mockall::automock)]` auto-generate `MockReviewAnalyzer`, `MockAgentRuntime`, `MockAgentConfig`, `MockFindingStore`. Existing manual mocks in `tests/common/mocks.rs` are extended with new trait methods.

**Property-based testing.** `proptest` for `Target::parse()` edge cases, `Severity` ordering, dedup idempotency. Follows `tests/unit/proptest_matcher.rs`.

**Integration tests with real git.** `TempGitRepo` + `tempfile::TempDir` + `git2::Repository` for adapter tests. Fast (in-process), isolated (temp dirs).

**Existing patterns to follow:**

| Pattern               | Example                                      | Continue For                             |
| --------------------- | -------------------------------------------- | ---------------------------------------- |
| Builder fixtures      | `CheckBuilder` in `tests/common/fixtures.rs` | `FindingBuilder`, `ReviewContextBuilder` |
| Inline `#[cfg(test)]` | Every source module                          | All new modules                          |
| `TempGitRepo`         | `tests/common/git_repo.rs`                   | VCS, checkpoint tests                    |
| `assert_cmd`          | `tests/integration/lifecycle_test.rs`        | Review, findings CLI                     |
| `proptest!`           | `tests/unit/proptest_matcher.rs`             | Target, Severity, dedup                  |
| Nested `mod` grouping | `tests/unit/check_test.rs`                   | Finding, Review tests                    |

## Test Fixtures and Builders

Add to `tests/common/fixtures.rs` alongside existing `CheckBuilder`/`AckBuilder`:

```rust
pub struct FindingBuilder {
    id: String, target: Target, severity: Severity, message: String,
    source: FindingSource, status: FindingStatus, suggestion: Option<String>,
}

impl FindingBuilder {
    pub fn new() -> Self {
        Self {
            id: "F-001".into(), target: Target::file("src/lib.rs").with_span(Span::line(10)),
            severity: Severity::Block, message: "Test finding".into(),
            source: FindingSource::Check("TEST-1".into()),
            status: FindingStatus::Open, suggestion: None,
        }
    }
    pub fn severity(mut self, s: Severity) -> Self { self.severity = s; self }
    pub fn file(mut self, p: &str) -> Self { self.target = Target::file(p); self }
    pub fn message(mut self, m: &str) -> Self { self.message = m.into(); self }
    pub fn source(mut self, s: FindingSource) -> Self { self.source = s; self }
    pub fn status(mut self, s: FindingStatus) -> Self { self.status = s; self }
    pub fn build(self) -> Finding { /* construct Finding from fields */ }
}

pub struct ReviewContextBuilder { /* diff, changed_files, repo_root, commit_range */ }
impl ReviewContextBuilder {
    pub fn new() -> Self { /* sensible defaults: "+fn main() {}", ["src/main.rs"] */ }
    pub fn diff(mut self, d: &str) -> Self { /* ... */ }
    pub fn changed_files(mut self, f: Vec<String>) -> Self { /* ... */ }
    pub fn build(self) -> ReviewContext { /* ... */ }
}

pub struct InvocationResultBuilder { /* output, exit_code, duration */ }
impl InvocationResultBuilder {
    pub fn new() -> Self { /* defaults: "[]", 0, 1s */ }
    pub fn output(mut self, o: &str) -> Self { /* ... */ }
    pub fn build(self) -> InvocationResult { /* ... */ }
}
```

Extend `TempGitRepo` in `tests/common/git_repo.rs`:

```rust
impl TempGitRepo {
    pub fn init_with_commit(&self) { /* write README, stage, commit */ }
    pub fn write_and_stage(&self, name: &str, content: &str) { /* write + stage */ }
    pub fn head_sha(&self) -> String { /* rev-parse --short HEAD */ }
    pub fn create_branch(&self, name: &str) { /* checkout -b */ }
    pub fn is_clean(&self) -> bool { /* status --porcelain is empty */ }
}
```

## Phase 1 Tests: Core Types

All inline `#[cfg(test)]` in model files.

### Target, Span, Severity

```rust
// Target: file(), pattern(), is_glob(), matches(), with_span(), with_commit(), Display
#[test]
fn is_glob_detects_wildcards() {
    assert!(Target::pattern("*.rs").is_glob());
    assert!(!Target::file("src/auth.rs").is_glob());
}

#[test]
fn matches_glob() {
    let t = Target::pattern("src/**/*.rs");
    assert!(t.matches("src/auth/login.rs"));
    assert!(!t.matches("tests/auth.rs"));
}

// Span: line(), range(), Display
#[test]
fn line_creates_single_line_span() {
    let s = Span::line(42);
    assert_eq!(s.start, 42); assert_eq!(s.end, 42);
}

// Severity: ordering, Default, FromStr, Display
#[test]
fn ordering_info_lt_warn_lt_block() {
    assert!(Severity::Info < Severity::Warn);
    assert!(Severity::Warn < Severity::Block);
}

#[test_case("info", Severity::Info ; "info")]
#[test_case("warn", Severity::Warn ; "warn")]
#[test_case("block", Severity::Block ; "block")]
fn from_str_valid(input: &str, expected: Severity) {
    assert_eq!(input.parse::<Severity>().unwrap(), expected);
}
```

**Property tests** (`tests/unit/proptest_target.rs`):

```rust
proptest! {
    #[test]
    fn parse_does_not_panic(s in "\\PC{0,100}") { let _ = Target::parse(&s); }

    #[test]
    fn file_path_round_trip(dir in "[a-z]{1,5}", file in "[a-z]{1,10}\\.[a-z]{1,5}") {
        let path = format!("{dir}/{file}");
        let t = Target::parse(&path).unwrap();
        prop_assert_eq!(t.path, path);
    }
}
```

### Finding

```rust
#[test]
fn is_blocking_true_for_open_block() {
    assert!(FindingBuilder::new().severity(Severity::Block).build().is_blocking());
}

#[test]
fn is_blocking_false_for_resolved_block() {
    assert!(!FindingBuilder::new().severity(Severity::Block)
        .status(FindingStatus::Resolved).build().is_blocking());
}

#[test]
fn is_blocking_false_for_open_warn() {
    assert!(!FindingBuilder::new().severity(Severity::Warn).build().is_blocking());
}

#[test]
fn resolve_and_dismiss_set_status() {
    let mut f = FindingBuilder::new().build();
    f.resolve(); assert_eq!(f.status, FindingStatus::Resolved);
    let mut g = FindingBuilder::new().build();
    g.dismiss(); assert_eq!(g.status, FindingStatus::Dismissed);
}
```

### Check and Review

```rust
// Check::into_finding()
#[test]
fn into_finding_maps_fields() {
    let check = CheckBuilder::new().id("NOS-1").target("*.rs")
        .message("Review Rust").severity(Severity::Block).build();
    let f = check.into_finding("src/main.rs");
    assert_eq!(f.target.path, "src/main.rs");
    assert!(matches!(f.source, FindingSource::Check(ref id) if id == "NOS-1"));
}

// Review: blocking_findings(), is_blocked(), findings_for_file(), close()
#[test]
fn blocking_findings_returns_only_open_blockers() {
    let r = review_with_findings(vec![
        FindingBuilder::new().severity(Severity::Block).build(),
        FindingBuilder::new().severity(Severity::Warn).build(),
        FindingBuilder::new().severity(Severity::Block)
            .status(FindingStatus::Resolved).build(),
    ]);
    assert_eq!(r.blocking_findings().len(), 1);
}

#[test]
fn findings_for_file_filters_by_path() {
    let r = review_with_findings(vec![
        FindingBuilder::new().file("src/auth.rs").build(),
        FindingBuilder::new().file("src/main.rs").build(),
    ]);
    assert_eq!(r.findings_for_file("src/auth.rs").len(), 1);
}
```

## Phase 2 Tests: VCS

All use `tempfile::TempDir` + `git2::Repository::init()`.

### Checkpoint (`src/adapters/git/checkpoint.rs`)

```rust
fn setup_repo() -> (TempDir, Repository) {
    let dir = TempDir::new().unwrap();
    let repo = Repository::init(dir.path()).unwrap();
    // Create initial commit so HEAD exists
    // ...
    (dir, repo)
}

#[test]
fn is_clean_on_fresh_repo() { assert!(is_clean(&setup_repo().1).unwrap()); }

#[test]
fn commit_all_stages_and_commits() {
    let (dir, repo) = setup_repo();
    std::fs::write(dir.path().join("f.txt"), "x").unwrap();
    let result = commit_all(&repo, "checkpoint").unwrap();
    assert!(result.is_some());
    assert!(is_clean(&repo).unwrap());
}

#[test]
fn commit_all_returns_none_on_clean() { assert!(commit_all(&setup_repo().1, "x").unwrap().is_none()); }
```

### diff_between / commits_between (`src/adapters/git/mod.rs`)

| Test                                    | Assertion                                                   |
| --------------------------------------- | ----------------------------------------------------------- |
| `diff_between_shows_added_file`         | New file on feature branch appears with `DiffStatus::Added` |
| `diff_between_empty_when_identical`     | Same ref returns empty vec                                  |
| `diff_between_binary_file`              | Binary file: `Modified` status, no hunks                    |
| `diff_between_renamed_file`             | `DiffStatus::Renamed`, `old_path` set                       |
| `diff_between_deleted_file`             | `DiffStatus::Deleted`                                       |
| `commits_between_lists_feature_commits` | Returns correct count and summaries                         |
| `commits_between_empty_range`           | Same base/head returns empty vec                            |

## Phase 3 Tests: Agent Adapters

### Output Parsing (`src/adapters/agent/parsing.rs`)

```rust
#[test]
fn extracts_findings_from_clean_json() {
    let result = extract_findings(r#"[{"severity":"warn","file":"a.rs","line":42,"message":"issue"}]"#);
    assert_eq!(result.findings.len(), 1);
}

#[test]
fn extracts_json_from_narrative_wrapper() {
    let result = extract_findings("Here:\n[{\"severity\":\"block\",\"file\":\"a.rs\",\"line\":1,\"message\":\"x\"}]\nDone.");
    assert_eq!(result.findings.len(), 1);
}

#[test]
fn handles_malformed_and_empty() {
    assert!(extract_findings("[{invalid}]").findings.is_empty());
    assert!(extract_findings("No issues.").findings.is_empty());
    assert!(extract_findings("[]").findings.is_empty());
}

#[test]
fn maps_four_level_severity_to_three_level() {
    // "info"->Info, "warning"->Warn, "error"->Block, "critical"->Block
    let output = r#"[
        {"severity":"info","file":"a.rs","line":1,"message":"a"},
        {"severity":"warning","file":"b.rs","line":2,"message":"b"},
        {"severity":"error","file":"c.rs","line":3,"message":"c"},
        {"severity":"critical","file":"d.rs","line":4,"message":"d"}
    ]"#;
    let r = extract_findings(output);
    assert_eq!(r.findings[0].severity, Severity::Info);
    assert_eq!(r.findings[1].severity, Severity::Warn);
    assert_eq!(r.findings[2].severity, Severity::Block);
    assert_eq!(r.findings[3].severity, Severity::Block);
}
```

### AgentAnalyzer with MockAgentRuntime

```rust
#[test]
fn builds_prompt_with_placeholders() {
    let mut mock = MockAgentRuntime::new();
    mock.expect_invoke()
        .withf(|c: &InvocationConfig| c.prompt.contains("security") && c.prompt.contains("+fn x()"))
        .returning(|_| Ok(InvocationResultBuilder::new().build()));
    mock.expect_parse_output().returning(|_| ReviewOutput::default());

    let analyzer = AgentAnalyzer::new(Box::new(mock), "security", "Review for {focus}:\n{diff}");
    analyzer.analyze(&ReviewContextBuilder::new().diff("+fn x()").build(), &[]).unwrap();
}

#[test]
fn returns_parsed_findings() {
    let mut mock = MockAgentRuntime::new();
    mock.expect_invoke().returning(|_| Ok(InvocationResultBuilder::new()
        .output(r#"[{"severity":"block","file":"db.rs","line":15,"message":"SQL injection"}]"#)
        .build()));
    mock.expect_parse_output().returning(|r| extract_findings(&r.output));

    let findings = AgentAnalyzer::new(Box::new(mock), "security", "{diff}")
        .analyze(&ReviewContextBuilder::new().build(), &[]).unwrap();
    assert_eq!(findings.len(), 1);
}
```

### Claude/Codex Config and Runtime

| Test                                  | Module      | Assertion                                          |
| ------------------------------------- | ----------- | -------------------------------------------------- |
| `generate_config_produces_settings`   | `claude.rs` | Bundle has settings with `permissions.allow` array |
| `build_command_includes_bypass_flag`  | `claude.rs` | `--permission-mode bypassPermissions` in args      |
| `settings_includes_noslop_permission` | `claude.rs` | `Bash(noslop *)` in allow list                     |
| `generate_config_instructions_only`   | `codex.rs`  | Bundle has instructions, no settings/hooks         |
| `build_command_uses_full_auto`        | `codex.rs`  | `--approval-mode full-auto` in args                |
| `is_available_checks_binary`          | both        | Runs `<agent> --version`, returns bool             |

## Phase 5 Tests: Pipeline

### ReviewPipeline Orchestrator (`src/core/services/pipeline.rs`)

```rust
fn mock_analyzer(name: &str, findings: Vec<Finding>) -> MockReviewAnalyzer {
    let mut m = MockReviewAnalyzer::new();
    m.expect_name().return_const(name.to_string());
    m.expect_analyze().returning(move |_, _| Ok(findings.clone()));
    m
}

#[test]
fn empty_pipeline_returns_empty() {
    let findings = ReviewPipeline::new(vec![]).run(&ReviewContextBuilder::new().build()).unwrap();
    assert!(findings.is_empty());
}

#[test]
fn fold_semantics_prior_findings_accumulate() {
    let a1 = mock_analyzer("conventions", vec![FindingBuilder::new().build()]);
    let mut a2 = MockReviewAnalyzer::new();
    a2.expect_name().return_const("agent".to_string());
    a2.expect_analyze()
        .withf(|_, prior| prior.len() == 1)  // a2 sees a1's finding
        .returning(|_, _| Ok(vec![]));
    ReviewPipeline::new(vec![Box::new(a1), Box::new(a2)])
        .run(&ReviewContextBuilder::new().build()).unwrap();
}

#[test]
fn continues_after_analyzer_error() {
    let mut failing = MockReviewAnalyzer::new();
    failing.expect_name().return_const("failing".to_string());
    failing.expect_analyze().returning(|_, _| Err(anyhow::anyhow!("boom")));
    let working = mock_analyzer("working", vec![FindingBuilder::new().build()]);
    let findings = ReviewPipeline::new(vec![Box::new(failing), Box::new(working)])
        .run(&ReviewContextBuilder::new().build()).unwrap();
    assert_eq!(findings.len(), 1);
}
```

### ConventionAnalyzer, CoChangeAnalyzer, ScriptAnalyzer

| Test                           | Analyzer   | Assertion                                                               |
| ------------------------------ | ---------- | ----------------------------------------------------------------------- |
| `finding_for_matching_check`   | Convention | Check with `target="src/auth.rs"` produces finding when file changed    |
| `no_findings_for_unmatched`    | Convention | Different file, no findings                                             |
| `glob_matches_multiple`        | Convention | `*.rs` matches 2 of 3 changed files                                     |
| `flags_missing_correlated`     | CoChange   | Missing pipeline.rs flagged when analyzer.rs changed (0.85 correlation) |
| `no_findings_when_all_present` | CoChange   | All correlated files in diff, no findings                               |
| `respects_min_threshold`       | CoChange   | 0.80 threshold skips 0.72 correlation                                   |
| `parses_json_stdout`           | Script     | Echo JSON, get findings                                                 |
| `empty_output_returns_empty`   | Script     | No output, no findings                                                  |
| `nonzero_exit_returns_error`   | Script     | Exit 1 is `Err`                                                         |
| `no_tools_no_findings`         | Formatting | Empty tools list, empty findings                                        |

## Phase 6 Tests: CLI Integration

All use `assert_cmd::Command::cargo_bin("noslop")` + `TempGitRepo`.

```rust
#[test]
fn review_no_changes_reports_clean() {
    let repo = TempGitRepo::new(); repo.init_with_commit();
    noslop(&repo, &["init"]).assert().success();
    noslop(&repo, &["review"]).assert().success()
        .stdout(predicate::str::contains("No changes"));
}

#[test]
fn review_check_mode_exits_nonzero_on_blockers() {
    let repo = TempGitRepo::new(); repo.init_with_commit();
    noslop(&repo, &["init"]).assert().success();
    noslop(&repo, &["check", "add", "*.rs", "-m", "Must review", "-s", "block"]).assert().success();
    repo.create_branch("feature");
    repo.write_and_stage("new.rs", "fn main() {}"); repo.commit("add");
    noslop(&repo, &["review", "--check", "--base", "master"]).assert().failure();
}

#[test]
fn review_json_is_valid() {
    // noslop review --json produces parseable JSON
}

#[test]
fn findings_list_empty_when_no_reviews() {
    // noslop findings shows "No findings" before any review
}

#[test]
fn findings_resolve_and_dismiss() {
    // After review: noslop findings resolve F-001; noslop findings dismiss F-002 --reason "x"
}

#[test]
fn pre_push_hook_installed() {
    // After noslop init claude: .git/hooks/pre-push contains "noslop review --check"
}
```

## Phase 7 Tests: Full Integration

### Complete Workflow

```rust
#[test]
fn full_workflow_init_to_push() {
    let repo = TempGitRepo::new(); repo.init_with_commit();
    noslop(&repo, &["init"]).assert().success();
    noslop(&repo, &["check", "add", "*.rs", "-m", "Review Rust", "-s", "block"]).assert().success();
    repo.create_branch("feature");
    repo.write_and_stage("src/lib.rs", "pub fn hello() {}"); repo.commit("add lib");

    // Review produces findings
    let out = noslop(&repo, &["review", "--base", "master", "--json"]).output().unwrap();
    let review: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(!review["findings"].as_array().unwrap().is_empty());

    // Resolve finding, then check mode passes
    let id = review["findings"][0]["id"].as_str().unwrap();
    noslop(&repo, &["findings", "resolve", id]).assert().success();
    noslop(&repo, &["review", "--check", "--base", "master"]).assert().success();
}
```

### Session Persistence and Multiple Reviews

| Test                                        | Assertion                                                       |
| ------------------------------------------- | --------------------------------------------------------------- |
| `session_persists_and_reloads`              | Review creates `.noslop/reviews/*.json`; findings reads it back |
| `multiple_reviews_create_separate_sessions` | Two reviews produce two session files                           |

## Property-Based Tests

### Finding Deduplication (`tests/unit/proptest_findings.rs`)

```rust
fn arb_finding() -> impl Strategy<Value = Finding> {
    (arb_severity(), "[a-z]{3,10}\\.rs", "[A-Za-z ]{5,30}")
        .prop_map(|(sev, file, msg)| FindingBuilder::new().severity(sev).file(&file).message(&msg).build())
}

proptest! {
    #[test]
    fn dedup_never_adds(findings in proptest::collection::vec(arb_finding(), 0..20)) {
        prop_assert!(deduplicate_findings(findings.clone()).len() <= findings.len());
    }
    #[test]
    fn dedup_is_idempotent(findings in proptest::collection::vec(arb_finding(), 0..20)) {
        let once = deduplicate_findings(findings);
        prop_assert_eq!(once.len(), deduplicate_findings(once.clone()).len());
    }
}
```

### AgentKind Parsing (`tests/unit/proptest_agent.rs`)

```rust
#[test_case("claude", AgentKind::Claude ; "claude")]
#[test_case("codex", AgentKind::Codex ; "codex")]
#[test_case("CLAUDE", AgentKind::Claude ; "uppercase")]
fn agent_kind_from_str(input: &str, expected: AgentKind) {
    assert_eq!(input.parse::<AgentKind>().unwrap(), expected);
}

proptest! {
    #[test]
    fn random_strings_rejected(s in "[a-z]{8,20}") {
        prop_assume!(!["claude", "codex"].contains(&s.as_str()));
        prop_assert!(s.parse::<AgentKind>().is_err());
    }
}
```

## Test File Layout

```
tests/
  common/
    fixtures.rs        # Extended: FindingBuilder, ReviewContextBuilder, InvocationResultBuilder
    git_repo.rs        # Extended: init_with_commit, write_and_stage, create_branch, is_clean
    mocks.rs           # Extended: MockVersionControl with review pipeline methods
  unit/
    proptest_target.rs    # NEW
    proptest_findings.rs  # NEW
    proptest_agent.rs     # NEW
  adapter/
    checkpoint_test.rs    # NEW
    agent_adapter_test.rs # NEW
  integration/
    review_test.rs        # NEW
    findings_test.rs      # NEW
    workflow_test.rs      # NEW
    agent_integration_test.rs  # NEW (#[ignore])
```

## Test Execution

```bash
cargo test --lib                        # Unit tests only
cargo test                              # Unit + adapter + integration
cargo test pipeline                     # Pipeline tests
cargo test --test integration           # CLI integration only
PROPTEST_CASES=1000 cargo test proptest # More property test cases
cargo test -- --ignored                 # Real agent CLI tests
```
