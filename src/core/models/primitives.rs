//! Shared primitive types for the noslop domain.
//!
//! - [`Severity`] - How strictly a finding is enforced
//! - [`Span`] - A contiguous range of lines in a file
//! - [`Target`] - A reference to a location in the codebase
//! - [`FindingSource`] - Where a finding originated

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// How strictly a finding is enforced.
///
/// - `Info`: shown, never blocks push
/// - `Warn`: shown prominently, never blocks push
/// - `Block`: must be resolved or dismissed before push
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Informational - shown but never blocks
    Info,
    /// Warning - shown prominently, never blocks
    Warn,
    /// Blocking - must be resolved or dismissed before push
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

/// A contiguous range of lines in a file.
///
/// Both `start` and `end` are 1-indexed and inclusive.
/// A single-line span has `start == end`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Span {
    /// Start line (1-indexed, inclusive)
    pub start: u32,
    /// End line (1-indexed, inclusive)
    pub end: u32,
}

impl Span {
    /// A span covering a single line.
    #[must_use]
    pub const fn line(n: u32) -> Self {
        Self { start: n, end: n }
    }

    /// A span covering a range of lines (inclusive).
    #[must_use]
    pub const fn range(start: u32, end: u32) -> Self {
        Self { start, end }
    }

    /// Number of lines in this span.
    #[must_use]
    pub const fn len(&self) -> u32 {
        self.end - self.start + 1
    }

    /// Whether this span is empty (always false for valid spans).
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        // A valid span always has at least one line
        self.end < self.start
    }

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

/// A reference to a location in the codebase.
///
/// Used in three ways:
/// - **Check**: `Target::pattern("src/**/*.rs")` -- glob for matching changed files
/// - **Finding**: `Target::file("src/auth.rs").with_span(Span::line(42))` -- exact location
/// - **Diff**: `Target::file("src/auth.rs").with_commit("abc123")` -- revision-pinned
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Target {
    /// File path or glob pattern
    pub path: String,
    /// Optional line span
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span: Option<Span>,
    /// Optional commit SHA
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit: Option<String>,
}

impl Target {
    /// Target a specific file.
    #[must_use]
    pub fn file(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            span: None,
            commit: None,
        }
    }

    /// Target a glob pattern (for checks).
    #[must_use]
    pub fn pattern(glob: impl Into<String>) -> Self {
        Self {
            path: glob.into(),
            span: None,
            commit: None,
        }
    }

    /// Attach a line span.
    #[must_use]
    pub const fn with_span(mut self, span: Span) -> Self {
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
    ///
    /// For glob patterns, `*` matches within a single directory component
    /// and `**` matches across directory separators.
    #[must_use]
    pub fn matches(&self, path: &str) -> bool {
        if self.is_glob() {
            let opts = glob::MatchOptions {
                require_literal_separator: true,
                ..Default::default()
            };
            glob::Pattern::new(&self.path).is_ok_and(|p| p.matches_with(path, opts))
        } else {
            self.path == path
        }
    }
}

impl fmt::Display for Target {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.path)?;
        if let Some(span) = &self.span {
            write!(f, "#{span}")?;
        }
        if let Some(sha) = &self.commit {
            write!(f, "@{}", &sha[..7.min(sha.len())])?;
        }
        Ok(())
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

    // --- Severity tests ---

    #[test]
    fn severity_ordering() {
        assert!(Severity::Info < Severity::Warn);
        assert!(Severity::Warn < Severity::Block);
    }

    #[test]
    fn severity_default_is_block() {
        assert_eq!(Severity::default(), Severity::Block);
    }

    #[test]
    fn severity_is_blocking() {
        assert!(!Severity::Info.is_blocking());
        assert!(!Severity::Warn.is_blocking());
        assert!(Severity::Block.is_blocking());
    }

    #[test]
    fn severity_display() {
        assert_eq!(Severity::Info.to_string(), "info");
        assert_eq!(Severity::Warn.to_string(), "warn");
        assert_eq!(Severity::Block.to_string(), "block");
    }

    #[test]
    fn severity_from_str() {
        assert_eq!("info".parse::<Severity>().unwrap(), Severity::Info);
        assert_eq!("WARN".parse::<Severity>().unwrap(), Severity::Warn);
        assert_eq!("Block".parse::<Severity>().unwrap(), Severity::Block);
        assert!("invalid".parse::<Severity>().is_err());
    }

    #[test]
    fn severity_serde_roundtrip() {
        let json = serde_json::to_string(&Severity::Block).unwrap();
        assert_eq!(json, "\"block\"");
        let parsed: Severity = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, Severity::Block);
    }

    // --- Span tests ---

    #[test]
    fn span_single_line() {
        let s = Span::line(42);
        assert_eq!(s.start, 42);
        assert_eq!(s.end, 42);
        assert_eq!(s.len(), 1);
        assert!(s.contains(42));
        assert!(!s.contains(41));
    }

    #[test]
    fn span_range() {
        let s = Span::range(10, 20);
        assert_eq!(s.len(), 11);
        assert!(s.contains(10));
        assert!(s.contains(15));
        assert!(s.contains(20));
        assert!(!s.contains(9));
        assert!(!s.contains(21));
    }

    #[test]
    fn span_overlaps() {
        let a = Span::range(10, 20);
        let b = Span::range(15, 25);
        let c = Span::range(21, 30);
        assert!(a.overlaps(&b));
        assert!(!a.overlaps(&c));
    }

    #[test]
    fn span_display() {
        assert_eq!(Span::line(42).to_string(), "L42");
        assert_eq!(Span::range(10, 20).to_string(), "L10-L20");
    }

    // --- Target tests ---

    #[test]
    fn target_file() {
        let t = Target::file("src/auth.rs");
        assert_eq!(t.path, "src/auth.rs");
        assert!(!t.is_glob());
        assert!(t.matches("src/auth.rs"));
        assert!(!t.matches("src/main.rs"));
    }

    #[test]
    fn target_pattern_glob() {
        let t = Target::pattern("src/**/*.rs");
        assert!(t.is_glob());
        assert!(t.matches("src/core/models/review.rs"));
        assert!(!t.matches("tests/unit/check_test.rs"));
    }

    #[test]
    fn target_with_span() {
        let t = Target::file("src/auth.rs").with_span(Span::line(42));
        assert_eq!(t.to_string(), "src/auth.rs#L42");
    }

    #[test]
    fn target_with_commit() {
        let t = Target::file("src/auth.rs").with_commit("abc1234def");
        assert_eq!(t.to_string(), "src/auth.rs@abc1234");
    }

    #[test]
    fn target_display_full() {
        let t = Target::file("src/auth.rs")
            .with_span(Span::range(10, 20))
            .with_commit("abc1234def");
        assert_eq!(t.to_string(), "src/auth.rs#L10-L20@abc1234");
    }

    #[test]
    fn target_serde_roundtrip() {
        let t = Target::file("src/auth.rs").with_span(Span::line(42));
        let json = serde_json::to_string(&t).unwrap();
        let parsed: Target = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, t);
    }

    #[test]
    fn target_serde_skip_none() {
        let t = Target::file("src/auth.rs");
        let json = serde_json::to_string(&t).unwrap();
        assert!(!json.contains("span"));
        assert!(!json.contains("commit"));
    }

    // --- FindingSource tests ---

    #[test]
    fn finding_source_display() {
        assert_eq!(FindingSource::Check("NOS-1".into()).to_string(), "check:NOS-1");
        assert_eq!(FindingSource::Script("lint".into()).to_string(), "script:lint");
        assert_eq!(FindingSource::Agent("security".into()).to_string(), "agent:security");
        assert_eq!(FindingSource::Human.to_string(), "human");
    }

    #[test]
    fn finding_source_serde_tagged() {
        let src = FindingSource::Human;
        let json = serde_json::to_string(&src).unwrap();
        assert_eq!(json, r#"{"kind":"Human"}"#);

        let src = FindingSource::Check("NOS-1".into());
        let json = serde_json::to_string(&src).unwrap();
        assert!(json.contains("\"kind\":\"Check\""));
        assert!(json.contains("\"name\":\"NOS-1\""));
    }

    // --- Additional edge case tests ---

    #[test]
    fn severity_serde_all_variants() {
        for sev in [Severity::Info, Severity::Warn, Severity::Block] {
            let json = serde_json::to_string(&sev).unwrap();
            let parsed: Severity = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, sev);
        }
    }

    #[test]
    fn severity_parse_error_contains_input() {
        let err = "invalid".parse::<Severity>().unwrap_err();
        assert!(err.contains("invalid"));
    }

    #[test]
    fn span_is_empty_valid_span() {
        assert!(!Span::line(1).is_empty());
        assert!(!Span::range(5, 10).is_empty());
    }

    #[test]
    fn span_is_empty_inverted_range() {
        let s = Span { start: 10, end: 5 };
        assert!(s.is_empty());
    }

    #[test]
    fn span_overlaps_same_span() {
        let s = Span::range(10, 20);
        assert!(s.overlaps(&s));
    }

    #[test]
    fn span_overlaps_adjacent_not_overlapping() {
        let a = Span::range(10, 20);
        let b = Span::range(21, 30);
        assert!(!a.overlaps(&b));
    }

    #[test]
    fn span_overlaps_touching() {
        let a = Span::range(10, 20);
        let b = Span::range(20, 30);
        assert!(a.overlaps(&b));
    }

    #[test]
    fn target_is_glob_with_bracket() {
        let t = Target::pattern("src/[ab]*.rs");
        assert!(t.is_glob());
    }

    #[test]
    fn target_is_glob_with_question() {
        let t = Target::pattern("src/?.rs");
        assert!(t.is_glob());
    }

    #[test]
    fn target_display_short_commit() {
        // Commit shorter than 7 chars should display fully
        let t = Target::file("f.rs").with_commit("abc");
        assert_eq!(t.to_string(), "f.rs@abc");
    }

    #[test]
    fn target_display_no_extras() {
        let t = Target::file("src/main.rs");
        assert_eq!(t.to_string(), "src/main.rs");
    }

    #[test]
    fn target_matches_bracket_glob() {
        let t = Target::pattern("[abc].rs");
        assert!(t.matches("a.rs"));
        assert!(t.matches("b.rs"));
        assert!(!t.matches("d.rs"));
    }

    #[test]
    fn finding_source_serde_roundtrip_all_variants() {
        let variants = vec![
            FindingSource::Check("NOS-1".into()),
            FindingSource::Script("lint".into()),
            FindingSource::Agent("security".into()),
            FindingSource::Human,
        ];
        for src in variants {
            let json = serde_json::to_string(&src).unwrap();
            let parsed: FindingSource = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, src);
        }
    }

    #[test]
    fn finding_source_equality() {
        assert_eq!(FindingSource::Check("A".into()), FindingSource::Check("A".into()));
        assert_ne!(FindingSource::Check("A".into()), FindingSource::Check("B".into()));
        assert_ne!(FindingSource::Human, FindingSource::Agent("x".into()));
    }
}
