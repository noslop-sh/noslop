# Core Domain Model

Foundation types for the noslop domain. Every other design document references these definitions. All types live in `src/core/models/` and have zero I/O dependencies.

## Design Philosophy

noslop is a local code review tool for agent-generated code. The domain has four user-facing concepts:

| Concept     | What it is                                            | GitHub parallel        |
| ----------- | ----------------------------------------------------- | ---------------------- |
| **Review**  | A review session on a branch diff                     | Pull Request           |
| **Finding** | A note on code from any source                        | PR comment             |
| **Check**   | A rule in `.noslop.toml` ("when X changes, verify Y") | Branch protection rule |
| **Agent**   | The AI reviewer (Claude, Codex)                       | Reviewer               |

The GitHub parallel drives our naming. A user "resolves" or "dismisses" a finding the same way they resolve a PR conversation. A review "blocks" push the same way required checks block merge.

### Why we unified

The original codebase had 12+ types for what is fundamentally one concept at different stages: `CheckItemResult`, `ReviewComment`, `SessionFinding`, `Acknowledgment`, `Resolution`, `CommentStatus`, `DiffPosition`, `DiffSide`, `Fragment`, `PathSpec`, `GlobPattern`. Each duplicated parts of the others.

The new model has 11 types across 5 files. The key insight: **a Finding is the universal note type**. Anything that says "there is something at this location in the code" is a Finding, regardless of whether a check rule, a script, an LLM, or a human produced it. The `FindingSource` tag tells you where it came from; the `FindingStatus` tells you where it is in its lifecycle.

## Module Layout

```text
src/core/models/
  mod.rs          # Re-exports all public types
  primitives.rs   # Severity, Span, Target, FindingSource
  finding.rs      # Finding, FindingStatus
  check.rs        # Check
  review.rs       # Review, ReviewStatus
  agent.rs        # AgentKind
```

`mod.rs` re-exports everything flat:

```rust
//! Domain models for noslop
//!
//! Pure data structures with no I/O dependencies.

mod agent;
mod check;
mod finding;
mod primitives;
mod review;

pub use agent::AgentKind;
pub use check::Check;
pub use finding::{Finding, FindingStatus};
pub use primitives::{FindingSource, Severity, Span, Target};
pub use review::{Review, ReviewStatus};
```

## Shared Primitives (`primitives.rs`)

### `Severity`

Three levels. `Block` is the default because the safe thing is to require attention. Ordering is `Info < Warn < Block` via derive order, so `findings.iter().max_by_key(|f| f.severity)` finds the worst.

```rust
/// How strictly a finding is enforced.
///
/// - `Info`: shown, never blocks push
/// - `Warn`: shown prominently, never blocks push
/// - `Block`: must be resolved or dismissed before push
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Info,
    Warn,
    #[default]
    Block,
}

impl Severity {
    /// Returns true if this severity prevents pushing.
    #[must_use]
    pub fn is_blocking(self) -> bool {
        self == Self::Block
    }
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Info => f.write_str("info"),
            Self::Warn => f.write_str("warn"),
            Self::Block => f.write_str("block"),
        }
    }
}

impl FromStr for Severity {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "info" => Ok(Self::Info),
            "warn" => Ok(Self::Warn),
            "block" => Ok(Self::Block),
            _ => Err(format!("invalid severity: {s}. expected: info, warn, block")),
        }
    }
}
```

### `Span`

A line range in a file. 1-indexed, inclusive on both ends (matches how editors and `git diff` report lines).

```rust
/// A contiguous range of lines in a file.
///
/// Both `start` and `end` are 1-indexed and inclusive.
/// A single-line span has `start == end`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Span {
    pub start: u32,
    pub end: u32,
}

impl Span {
    /// A span covering a single line.
    #[must_use]
    pub const fn line(n: u32) -> Self { Self { start: n, end: n } }

    /// A span covering a range of lines (inclusive).
    #[must_use]
    pub const fn range(start: u32, end: u32) -> Self { Self { start, end } }

    /// Number of lines in this span.
    #[must_use]
    pub const fn len(&self) -> u32 { self.end - self.start + 1 }

    /// Whether this span contains a given line number.
    #[must_use]
    pub const fn contains(&self, line: u32) -> bool {
        line >= self.start && line <= self.end
    }

    /// Whether two spans overlap.
    #[must_use]
    pub const fn overlaps(&self, other: &Self) -> bool {
        self.start <= other.end && other.start <= self.end
    }
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.start == self.end {
            write!(f, "L{}", self.start)
        } else {
            write!(f, "L{}-L{}", self.start, self.end)
        }
    }
}
```

### `Target`

A reference to code: file path, optional line span, optional commit SHA. This is THE shared primitive -- it appears in Check (as a glob pattern), Finding (as an exact location), and diff results (as a revision-pinned path).

```rust
/// A reference to a location in the codebase.
///
/// Used in three ways:
/// - **Check**: `Target::pattern("src/**/*.rs")` -- glob for matching changed files
/// - **Finding**: `Target::file("src/auth.rs").with_span(Span::line(42))` -- exact location
/// - **Diff**: `Target::file("src/auth.rs").with_commit("abc123")` -- revision-pinned
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Target {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span: Option<Span>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit: Option<String>,
}

impl Target {
    /// Target a specific file.
    #[must_use]
    pub fn file(path: impl Into<String>) -> Self {
        Self { path: path.into(), span: None, commit: None }
    }

    /// Target a glob pattern (for checks).
    #[must_use]
    pub fn pattern(glob: impl Into<String>) -> Self {
        Self { path: glob.into(), span: None, commit: None }
    }

    /// Attach a line span.
    #[must_use]
    pub fn with_span(mut self, span: Span) -> Self {
        self.span = Some(span);
        self
    }

    /// Pin to a specific commit.
    #[must_use]
    pub fn with_commit(mut self, sha: impl Into<String>) -> Self {
        self.commit = Some(sha.into());
        self
    }

    /// Whether this target's path contains glob metacharacters.
    #[must_use]
    pub fn is_glob(&self) -> bool {
        self.path.contains('*') || self.path.contains('?') || self.path.contains('[')
    }

    /// Test whether a concrete file path matches this target.
    pub fn matches(&self, path: &str) -> bool {
        if self.is_glob() {
            glob_match::glob_match(&self.path, path)
        } else {
            self.path == path
        }
    }
}

impl fmt::Display for Target {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.path)?;
        if let Some(span) = &self.span { write!(f, "#{span}")?; }
        if let Some(sha) = &self.commit { write!(f, "@{}", &sha[..7.min(sha.len())])?; }
        Ok(())
    }
}
```

**Target usage across the domain:**

```rust
// Check: glob pattern matches any Rust file under src/
let t = Target::pattern("src/**/*.rs");
assert!(t.matches("src/core/models/review.rs"));
assert!(!t.matches("tests/unit/check_test.rs"));

// Finding: exact file + line span
let t = Target::file("src/auth.rs").with_span(Span::line(42));
assert_eq!(t.to_string(), "src/auth.rs#L42");

// Diff result: file pinned to a commit
let t = Target::file("src/auth.rs").with_commit("abc1234def");
assert_eq!(t.to_string(), "src/auth.rs@abc1234");
```

### `FindingSource`

Tags a finding with its origin. The string payload in `Check`, `Script`, and `Agent` identifies which specific rule, tool, or agent pass produced it.

```rust
/// Where a finding originated.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", content = "name")]
pub enum FindingSource {
    /// From a `[[check]]` rule in `.noslop.toml`. Name is the check ID.
    Check(String),
    /// From an external script analyzer. Name is the script name.
    Script(String),
    /// From an LLM agent pass. Name is the pass identifier (e.g., "security").
    Agent(String),
    /// From the user directly (manual annotation on the diff).
    Human,
}

impl fmt::Display for FindingSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Check(id) => write!(f, "check:{id}"),
            Self::Script(name) => write!(f, "script:{name}"),
            Self::Agent(name) => write!(f, "agent:{name}"),
            Self::Human => f.write_str("human"),
        }
    }
}
```

## Finding (`finding.rs`)

The universal note type. Anything that says "there is something at this location" is a Finding.

```rust
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use super::primitives::{FindingSource, Severity, Span, Target};

/// A note on code from any source.
///
/// Findings are the primary output of the review pipeline. Every analyzer
/// produces `Vec<Finding>`. The user triages each finding: resolve it,
/// dismiss it, or leave it open. Open findings with `severity = Block`
/// prevent push.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    /// Unique identifier (e.g., "F-a1b2c3d4").
    pub id: String,
    /// Where in the codebase this finding points to.
    pub target: Target,
    /// How strictly this finding is enforced.
    pub severity: Severity,
    /// Human-readable description of the issue.
    pub message: String,
    /// What produced this finding.
    pub source: FindingSource,
    /// Lifecycle state: open, resolved, or dismissed.
    pub status: FindingStatus,
    /// Optional suggested fix or action.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
    /// ISO 8601 timestamp of creation.
    pub created_at: String,
}

/// Lifecycle state of a finding.
///
/// - `Open`: not yet addressed, may block push
/// - `Resolved`: addressed (code was fixed)
/// - `Dismissed`: intentionally ignored (with reason tracked elsewhere)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FindingStatus {
    #[default]
    Open,
    Resolved,
    Dismissed,
}

impl Finding {
    /// Create a new open finding.
    #[must_use]
    pub fn new(
        target: Target, severity: Severity,
        message: impl Into<String>, source: FindingSource,
    ) -> Self {
        Self {
            id: format!("F-{}", &Uuid::new_v4().to_string()[..8]),
            target, severity,
            message: message.into(), source,
            status: FindingStatus::Open,
            suggestion: None,
            created_at: Utc::now().to_rfc3339(),
        }
    }

    /// Attach a suggested fix.
    #[must_use]
    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }

    /// Whether this finding currently prevents pushing.
    #[must_use]
    pub fn is_blocking(&self) -> bool {
        self.severity == Severity::Block && self.status == FindingStatus::Open
    }

    /// Mark as resolved.
    pub fn resolve(&mut self) { self.status = FindingStatus::Resolved; }

    /// Mark as dismissed.
    pub fn dismiss(&mut self) { self.status = FindingStatus::Dismissed; }

    /// Whether this finding is still open.
    #[must_use]
    pub fn is_open(&self) -> bool { self.status == FindingStatus::Open }
}

impl fmt::Display for Finding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {} {} ({})", self.severity, self.id, self.message, self.source)
    }
}

impl fmt::Display for FindingStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Open => f.write_str("open"),
            Self::Resolved => f.write_str("resolved"),
            Self::Dismissed => f.write_str("dismissed"),
        }
    }
}
```

### Finding examples by source

```rust
// From a [[check]] rule
let f = Finding::new(
    Target::file("src/auth.rs").with_span(Span::range(40, 55)),
    Severity::Block,
    "Session validation changed -- verify token expiry",
    FindingSource::Check("NOS-1".into()),
);

// From an external script with a suggestion
let f = Finding::new(
    Target::file("migrations/003_add_users.sql"), Severity::Block,
    "NOT NULL column without default",
    FindingSource::Script("check-migrations".into()),
).with_suggestion("Add DEFAULT or split into two migrations");

// From an LLM agent pass
let f = Finding::new(
    Target::file("src/api/auth.rs").with_span(Span::range(12, 18)),
    Severity::Block,
    "JWT secret read without validation",
    FindingSource::Agent("security".into()),
).with_suggestion("Fail startup if JWT_SECRET is unset");

// From the user (manual annotation)
let f = Finding::new(
    Target::file("src/api/routes.rs").with_span(Span::line(88)),
    Severity::Warn,
    "Should require admin role",
    FindingSource::Human,
);
```

## Check (`check.rs`)

A rule declared in `.noslop.toml`. Checks are static: they exist before any review runs. The convention analyzer matches checks against changed files and calls `into_finding()` to bridge the two worlds.

````rust
use serde::{Deserialize, Serialize};
use super::finding::Finding;
use super::primitives::{FindingSource, Severity, Target};

/// A review rule from `.noslop.toml`.
///
/// Declares: "when files matching `target` change, surface this message."
///
/// ```toml
/// [[check]]
/// id = "NOS-1"
/// target = "src/adapters/git/hooks.rs"
/// message = "Verify hook installation still works"
/// severity = "block"
/// tags = ["hooks"]
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Check {
    /// Unique identifier (e.g., "NOS-1"). Set in TOML or auto-generated.
    pub id: String,
    /// File path or glob pattern to match against changed files.
    pub target: Target,
    /// What the reviewer should verify when matched files change.
    pub message: String,
    /// How strictly this check is enforced.
    pub severity: Severity,
    /// Optional tags for filtering and grouping.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

impl Check {
    /// Create a new check.
    #[must_use]
    pub fn new(
        id: impl Into<String>, target: Target,
        message: impl Into<String>, severity: Severity,
    ) -> Self {
        Self { id: id.into(), target, message: message.into(), severity, tags: Vec::new() }
    }

    /// Add tags.
    #[must_use]
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Whether this check's target matches a given file path.
    pub fn applies_to(&self, path: &str) -> bool {
        self.target.matches(path)
    }

    /// Convert this check into a Finding for a specific matched file.
    ///
    /// This is the bridge between the check world and the finding world.
    /// The convention analyzer calls this for each (check, changed_file) pair.
    #[must_use]
    pub fn into_finding(self, matched_path: &str) -> Finding {
        let source = FindingSource::Check(self.id.clone());
        Finding::new(Target::file(matched_path), self.severity, self.message, source)
    }
}
````

### Check matching example

```rust
let check = Check::new(
    "NOS-5", Target::pattern("src/adapters/**/*.rs"),
    "Adapter changed -- verify port trait contract", Severity::Block,
).with_tags(vec!["architecture".into()]);

let changed = vec!["src/adapters/git/hooks.rs", "src/cli/app.rs"];
let findings: Vec<Finding> = changed.iter()
    .filter(|f| check.applies_to(f))
    .map(|f| check.clone().into_finding(f))
    .collect();

assert_eq!(findings.len(), 1);
assert_eq!(findings[0].target.path, "src/adapters/git/hooks.rs");
```

## Review (`review.rs`)

A review session on a branch diff. Contains all findings from all analyzers across all pipeline runs.

```rust
use chrono::Utc;
use serde::{Deserialize, Serialize};
use super::finding::{Finding, FindingStatus};
use super::primitives::Severity;

/// A code review session on a branch diff.
///
/// Created by `noslop review`. Contains findings from every analyzer
/// that ran. The pre-push hook checks `is_blocked()` to gate push.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Review {
    /// Unique identifier (e.g., "REV-a1b2c3d4").
    pub id: String,
    /// Base commit SHA (typically merge-base with main).
    pub base: String,
    /// Head commit SHA (tip of the feature branch).
    pub head: String,
    /// Lifecycle state.
    pub status: ReviewStatus,
    /// All findings from all sources.
    pub findings: Vec<Finding>,
    /// ISO 8601 timestamp of creation.
    pub created_at: String,
    /// ISO 8601 timestamp when closed (if closed).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub closed_at: Option<String>,
}

/// Lifecycle state of a review.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReviewStatus {
    #[default]
    Open,
    Closed,
}

impl Review {
    /// Create a new open review for a commit range.
    #[must_use]
    pub fn new(base: impl Into<String>, head: impl Into<String>) -> Self {
        Self {
            id: generate_review_id(),
            base: base.into(), head: head.into(),
            status: ReviewStatus::Open,
            findings: Vec::new(),
            created_at: Utc::now().to_rfc3339(),
            closed_at: None,
        }
    }

    /// Add a finding to this review.
    pub fn add_finding(&mut self, finding: Finding) {
        self.findings.push(finding);
    }

    /// Add multiple findings from an analyzer pass.
    pub fn add_findings(&mut self, findings: impl IntoIterator<Item = Finding>) {
        self.findings.extend(findings);
    }

    /// All findings that currently prevent pushing
    /// (severity == Block and status == Open).
    #[must_use]
    pub fn blocking_findings(&self) -> Vec<&Finding> {
        self.findings.iter().filter(|f| f.is_blocking()).collect()
    }

    /// Whether this review has any blocking findings.
    /// Used by pre-push hook: `noslop review --check` exits 1 when true.
    #[must_use]
    pub fn is_blocked(&self) -> bool {
        self.findings.iter().any(|f| f.is_blocking())
    }

    /// All findings that target a specific file path.
    #[must_use]
    pub fn findings_for_file(&self, path: &str) -> Vec<&Finding> {
        self.findings.iter().filter(|f| f.target.path == path).collect()
    }

    /// All findings that are still open (any severity).
    #[must_use]
    pub fn open_findings(&self) -> Vec<&Finding> {
        self.findings.iter().filter(|f| f.is_open()).collect()
    }

    /// Close this review.
    pub fn close(&mut self) {
        self.status = ReviewStatus::Closed;
        self.closed_at = Some(Utc::now().to_rfc3339());
    }

    /// Whether this review is open.
    #[must_use]
    pub fn is_open(&self) -> bool { self.status == ReviewStatus::Open }
}

fn generate_review_id() -> String {
    use uuid::Uuid;
    format!("REV-{}", &Uuid::new_v4().to_string()[..8])
}
```

### Review lifecycle example

```rust
let mut review = Review::new("abc1234", "def5678");

// Convention analyzer adds a blocker
review.add_finding(Finding::new(
    Target::file("src/auth.rs").with_span(Span::line(42)),
    Severity::Block, "Session handling changed",
    FindingSource::Check("NOS-1".into()),
));

// Agent analyzer adds a warning
review.add_finding(Finding::new(
    Target::file("src/auth.rs").with_span(Span::range(10, 15)),
    Severity::Warn, "Consider rate limiting",
    FindingSource::Agent("security".into()),
));

assert!(review.is_blocked());                    // open Block finding
assert_eq!(review.blocking_findings().len(), 1);
assert_eq!(review.findings_for_file("src/auth.rs").len(), 2);

review.findings[0].resolve();
assert!(!review.is_blocked());                   // resolved, no longer blocking
```

## AgentKind (`agent.rs`)

Which AI reviewer to use. Agent-specific configuration lives in the adapter layer.

```rust
use std::fmt;
use std::str::FromStr;
use serde::{Deserialize, Serialize};

/// Supported AI review agents.
///
/// Each variant maps to a CLI tool that noslop invokes for
/// agent-based review analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentKind {
    /// Anthropic Claude Code (`claude` CLI).
    Claude,
    /// OpenAI Codex (`codex` CLI).
    Codex,
}

impl AgentKind {
    /// The CLI command to invoke this agent.
    #[must_use]
    pub fn command(self) -> &'static str {
        match self { Self::Claude => "claude", Self::Codex => "codex" }
    }

    /// Human-readable name for display.
    #[must_use]
    pub fn display_name(self) -> &'static str {
        match self { Self::Claude => "Claude", Self::Codex => "Codex" }
    }
}

impl fmt::Display for AgentKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.display_name())
    }
}

impl FromStr for AgentKind {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "claude" => Ok(Self::Claude),
            "codex" => Ok(Self::Codex),
            _ => Err(format!("unknown agent: {s}. supported: claude, codex")),
        }
    }
}
```

## Migration from Current Types

| Current type     | Current file        | New type        | New file        | Notes                                                                                                               |
| ---------------- | ------------------- | --------------- | --------------- | ------------------------------------------------------------------------------------------------------------------- |
| `Severity`       | `severity.rs`       | `Severity`      | `primitives.rs` | Add `PartialOrd, Ord, Hash`; add `is_blocking()`                                                                    |
| `Target`         | `target.rs`         | `Target`        | `primitives.rs` | Flatten to `{path, span, commit}`. Drop `PathSpec`, `GlobPattern`, `Fragment`, `ParseError`. Use `glob_match` crate |
| `PathSpec`       | `target.rs`         | --              | --              | Deleted. `Target.path` is a plain string; `is_glob()` checks for metacharacters                                     |
| `GlobPattern`    | `target.rs`         | --              | --              | Deleted. Use `glob_match::glob_match()` instead of compiled regex                                                   |
| `Fragment`       | `target.rs`         | `Span`          | `primitives.rs` | `Line(n)` and `LineRange(a,b)` collapse to `Span { start, end }`. `Symbol` removed (unused)                         |
| `ParseError`     | `target.rs`         | --              | --              | Deleted. `Target` is constructed via builders, not parsed                                                           |
| --               | --                  | `FindingSource` | `primitives.rs` | New. Tags findings with origin                                                                                      |
| `Check`          | `check.rs`          | `Check`         | `check.rs`      | `target`: `String` to `Target`. Drop `introduced_by`, `created_at`. Add `tags`, `into_finding()`                    |
| `Review`         | `review.rs`         | `Review`        | `review.rs`     | `comments` becomes `findings: Vec<Finding>`. `base_sha`/`head_sha` become `base`/`head`. Drop `description`         |
| `ReviewStatus`   | `review.rs`         | `ReviewStatus`  | `review.rs`     | Unchanged                                                                                                           |
| `ReviewComment`  | `review.rs`         | --              | --              | Deleted. Replaced by `Finding` with `Target` containing span                                                        |
| `DiffPosition`   | `review.rs`         | --              | --              | Deleted. `Target.span` covers this                                                                                  |
| `DiffSide`       | `review.rs`         | --              | --              | Deleted. Findings reference new-side lines                                                                          |
| `CommentStatus`  | `review.rs`         | `FindingStatus` | `finding.rs`    | Add `Dismissed` variant                                                                                             |
| `Acknowledgment` | `acknowledgment.rs` | --              | --              | Deleted. `FindingStatus::Resolved` on `Finding` replaces this                                                       |
| --               | --                  | `Finding`       | `finding.rs`    | New. Unifies `ReviewComment`, `CheckItemResult`, `SessionFinding`, `Acknowledgment`                                 |
| --               | --                  | `FindingStatus` | `finding.rs`    | New. `Open`, `Resolved`, `Dismissed`                                                                                |
| --               | --                  | `AgentKind`     | `agent.rs`      | New. Replaces `AgentType` from port layer                                                                           |

### Files removed

- `src/core/models/acknowledgment.rs`, `src/core/ports/acknowledgment_store.rs` -- replaced by `FindingStatus`

### Files changed

- `src/core/models/mod.rs`, `check.rs`, `review.rs` -- rewritten for new types
- `src/core/models/target.rs`, `severity.rs` -- deleted, merged into `primitives.rs`
- `src/core/ports/mod.rs` -- remove `acknowledgment_store`, add `analyzer`, `agent`
- `src/core/ports/review_store.rs` -- update to new `Review` shape

## Port Trait Signatures

Port traits reference the domain types above. Full designs in their respective documents.

**`ReviewAnalyzer`** (new -- `src/core/ports/analyzer.rs`). Each pipeline stage implements this. Full design in [06-run-loop-design.md](./06-run-loop-design.md).

```rust
pub trait ReviewAnalyzer: Send + Sync {
    fn analyze(&self, ctx: &ReviewContext, prior: &[Finding]) -> anyhow::Result<Vec<Finding>>;
    fn name(&self) -> &str;
}
```

**`ReviewStore`** (updated -- `src/core/ports/review_store.rs`). Persists reviews with `Vec<Finding>`.

```rust
pub trait ReviewStore: Send + Sync {
    fn save(&self, review: &Review) -> anyhow::Result<()>;
    fn load(&self, id: &str) -> anyhow::Result<Option<Review>>;
    fn list_all(&self) -> anyhow::Result<Vec<Review>>;
    fn list_open(&self) -> anyhow::Result<Vec<Review>>;
    fn delete(&self, id: &str) -> anyhow::Result<bool>;
    fn find_blocking_for_file(&self, file: &str) -> anyhow::Result<Vec<Review>>;
}
```

**`CheckRepository`** (updated -- `src/core/ports/check_repo.rs`). Returns `Check` with `Target`.

```rust
pub trait CheckRepository: Send + Sync {
    fn find_for_files(&self, files: &[String]) -> anyhow::Result<Vec<(Check, String)>>;
    fn add(&self, target: &str, message: &str, severity: Severity) -> anyhow::Result<String>;
    fn remove(&self, id: &str) -> anyhow::Result<()>;
    fn list(&self) -> anyhow::Result<Vec<Check>>;
}
```

**`AgentConfig` + `AgentRuntime`** (new -- `src/core/ports/agent.rs`). Full design in [03-agent-port-design.md](./03-agent-port-design.md).

```rust
pub trait AgentConfig: Send + Sync {
    fn agent_kind(&self) -> AgentKind;
    fn generate_config(&self, project_root: &Path) -> anyhow::Result<()>;
    fn is_available(&self) -> bool;
}

pub trait AgentRuntime: Send + Sync {
    fn agent_kind(&self) -> AgentKind;
    fn invoke(&self, prompt: &str, workdir: &Path) -> anyhow::Result<String>;
}
```

**`VersionControl`** (extended -- `src/core/ports/vcs.rs`). Existing methods plus new review pipeline methods. Full design in [05-checkpoint-worktree-design.md](./05-checkpoint-worktree-design.md).

```rust
pub trait VersionControl: Send + Sync {
    // ... existing: staged_files, repo_name, repo_root, install_hooks, current_branch
    fn diff_between(&self, base: &str, head: &str) -> anyhow::Result<String>;
    fn merge_base(&self, branch: &str) -> anyhow::Result<String>;
    fn changed_files(&self, base: &str, head: &str) -> anyhow::Result<Vec<String>>;
}
```
