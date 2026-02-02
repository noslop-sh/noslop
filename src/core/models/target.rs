//! Target parsing and matching
//!
//! A Target represents a reference to code in the repository.
//! Targets support:
//! - Exact paths: `src/auth.rs`
//! - Glob patterns: `src/**/*.rs`, `*.rs`
//! - Fragment identifiers: `src/auth.rs#L42`, `src/auth.rs#Session`
//!
//! # Examples
//!
//! ```
//! use noslop::core::models::{Target, Fragment};
//!
//! // Exact file
//! let t = Target::parse("src/auth.rs").unwrap();
//! assert!(t.matches("src/auth.rs"));
//!
//! // Glob pattern
//! let t = Target::parse("src/**/*.rs").unwrap();
//! assert!(t.matches("src/auth/login.rs"));
//!
//! // With line fragment
//! let t = Target::parse("src/auth.rs#L42").unwrap();
//! assert!(t.matches("src/auth.rs"));
//! assert_eq!(t.fragment(), Some(&Fragment::Line(42)));
//! ```

use std::path::Path;

use regex::Regex;
use thiserror::Error;

/// Errors that can occur when parsing a target
#[derive(Debug, Error)]
pub enum ParseError {
    /// Target string was empty
    #[error("empty target")]
    Empty,

    /// Invalid line number in fragment
    #[error("invalid line number: {0}")]
    InvalidLineNumber(String),

    /// Invalid line range in fragment
    #[error("invalid line range: {0}")]
    InvalidLineRange(String),

    /// Invalid glob pattern syntax
    #[error("invalid glob pattern: {0}")]
    InvalidGlob(String),
}

/// A parsed target reference
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    /// The original target string
    raw: String,

    /// The path/pattern portion (before #)
    path_spec: PathSpec,

    /// Optional fragment (after #)
    fragment: Option<Fragment>,
}

/// The path specification portion of a target
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathSpec {
    /// Exact file path: `src/auth.rs`
    Exact(String),

    /// Glob pattern: `*.rs`, `src/**/*.rs`
    Glob(GlobPattern),
}

/// A compiled glob pattern
#[derive(Debug, Clone)]
pub struct GlobPattern {
    /// Original pattern string
    pattern: String,

    /// Compiled regex for matching
    regex: Regex,
}

impl PartialEq for GlobPattern {
    fn eq(&self, other: &Self) -> bool {
        self.pattern == other.pattern
    }
}

impl Eq for GlobPattern {}

/// Fragment identifier (portion after #)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fragment {
    /// Line number: `#L42`
    Line(u32),

    /// Line range: `#L42-L50` or `#L42-50`
    LineRange(u32, u32),

    /// Symbol reference: `#Session`, `#Session.validate`
    /// Note: Symbol resolution is deferred to the Resolver
    Symbol(String),
}

impl Target {
    /// Parse a target string into a Target
    ///
    /// # Format
    ///
    /// ```text
    /// <path_spec>[#<fragment>]
    ///
    /// path_spec:
    ///   - Exact path: src/auth.rs
    ///   - Glob: *.rs, src/**/*.rs, src/*.rs
    ///
    /// fragment:
    ///   - Line: #L42
    ///   - Line range: #L42-L50, #L42-50
    ///   - Symbol: #Session, #Session.validate
    /// ```
    pub fn parse(s: &str) -> Result<Self, ParseError> {
        let s = s.trim();
        if s.is_empty() {
            return Err(ParseError::Empty);
        }

        // Split on # for fragment
        let (path_part, fragment_part) =
            s.find('#').map_or((s, None), |pos| (&s[..pos], Some(&s[pos + 1..])));

        // Parse path spec
        let path_spec = PathSpec::parse(path_part)?;

        // Parse fragment if present
        let fragment = match fragment_part {
            Some(f) if !f.is_empty() => Some(Fragment::parse(f)?),
            _ => None,
        };

        Ok(Self {
            raw: s.to_string(),
            path_spec,
            fragment,
        })
    }

    /// Get the original target string
    #[must_use]
    pub fn raw(&self) -> &str {
        &self.raw
    }

    /// Get the path specification
    #[must_use]
    pub const fn path_spec(&self) -> &PathSpec {
        &self.path_spec
    }

    /// Get the fragment, if any
    #[must_use]
    pub const fn fragment(&self) -> Option<&Fragment> {
        self.fragment.as_ref()
    }

    /// Check if this target matches a given file path
    ///
    /// This only checks the path portion - fragments are handled by the Resolver
    pub fn matches(&self, path: impl AsRef<Path>) -> bool {
        self.path_spec.matches(path)
    }

    /// Check if this target has a fragment
    #[must_use]
    pub const fn has_fragment(&self) -> bool {
        self.fragment.is_some()
    }

    /// Check if this target is a glob pattern
    #[must_use]
    pub const fn is_glob(&self) -> bool {
        matches!(self.path_spec, PathSpec::Glob(_))
    }

    /// Get the path pattern as a string
    #[must_use]
    pub fn path_pattern(&self) -> &str {
        match &self.path_spec {
            PathSpec::Exact(p) => p,
            PathSpec::Glob(g) => &g.pattern,
        }
    }
}

impl PathSpec {
    /// Parse a path specification
    fn parse(s: &str) -> Result<Self, ParseError> {
        let s = s.trim();
        if s.is_empty() {
            return Err(ParseError::Empty);
        }

        // Check if it's a glob pattern
        if Self::is_glob_pattern(s) {
            let pattern = GlobPattern::new(s)?;
            Ok(Self::Glob(pattern))
        } else {
            // Normalize path separators
            let normalized = s.replace('\\', "/");
            Ok(Self::Exact(normalized))
        }
    }

    /// Check if a string contains glob metacharacters
    fn is_glob_pattern(s: &str) -> bool {
        s.contains('*') || s.contains('?') || s.contains('[')
    }

    /// Check if this path spec matches a given path
    pub fn matches(&self, path: impl AsRef<Path>) -> bool {
        let path = path.as_ref();
        let path_str = path.to_string_lossy();
        // Normalize to forward slashes for matching
        let normalized = path_str.replace('\\', "/");

        match self {
            Self::Exact(exact) => {
                // Exact match or prefix match for directories
                normalized == *exact
                    || normalized.starts_with(&format!("{exact}/"))
                    || exact.ends_with('/') && normalized.starts_with(exact.trim_end_matches('/'))
            },
            Self::Glob(pattern) => pattern.matches(&normalized),
        }
    }
}

impl GlobPattern {
    /// Create a new glob pattern
    fn new(pattern: &str) -> Result<Self, ParseError> {
        let regex = Self::glob_to_regex(pattern)?;
        Ok(Self {
            pattern: pattern.to_string(),
            regex,
        })
    }

    /// Convert a glob pattern to a regex
    fn glob_to_regex(glob: &str) -> Result<Regex, ParseError> {
        let mut regex = String::with_capacity(glob.len() * 2);
        regex.push('^');

        let chars: Vec<char> = glob.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            match chars[i] {
                '*' => {
                    // Check for **
                    if i + 1 < chars.len() && chars[i + 1] == '*' {
                        // ** matches zero or more path segments
                        if i + 2 < chars.len() && chars[i + 2] == '/' {
                            // **/ pattern - matches zero or more directories
                            regex.push_str("(?:[^/]+/)*");
                            i += 3;
                            continue;
                        } else if i == 0 || chars[i - 1] == '/' {
                            // Leading ** or /** at end - matches everything including subdirs
                            regex.push_str(".*");
                            i += 2;
                            continue;
                        }
                        // ** in middle without / - treat as .*
                        regex.push_str(".*");
                        i += 2;
                        continue;
                    }
                    // Single * matches anything except /
                    regex.push_str("[^/]*");
                },
                '?' => regex.push_str("[^/]"),
                '.' => regex.push_str("\\."),
                '/' => regex.push('/'),
                '[' => {
                    // Character class - find matching ]
                    let start = i;
                    i += 1;
                    while i < chars.len() && chars[i] != ']' {
                        i += 1;
                    }
                    if i >= chars.len() {
                        return Err(ParseError::InvalidGlob(format!(
                            "unclosed character class: {glob}"
                        )));
                    }
                    // Copy the character class as-is
                    let class: String = chars[start..=i].iter().collect();
                    regex.push_str(&class);
                },
                c => {
                    // Escape regex metacharacters
                    if "^$+{}|()\\".contains(c) {
                        regex.push('\\');
                    }
                    regex.push(c);
                },
            }
            i += 1;
        }

        regex.push('$');

        Regex::new(&regex).map_err(|e| ParseError::InvalidGlob(e.to_string()))
    }

    /// Check if a path matches this pattern
    fn matches(&self, path: &str) -> bool {
        self.regex.is_match(path)
    }
}

impl Fragment {
    /// Parse a fragment string (without the leading #)
    fn parse(s: &str) -> Result<Self, ParseError> {
        let s = s.trim();

        // Check for line number: L42
        if let Some(rest) = s.strip_prefix('L') {
            // Check for range: L42-L50 or L42-50
            if let Some(dash_pos) = rest.find('-') {
                let start_str = &rest[..dash_pos];
                let end_str = rest[dash_pos + 1..].trim_start_matches('L');

                let start: u32 = start_str
                    .parse()
                    .map_err(|_| ParseError::InvalidLineNumber(start_str.to_string()))?;
                let end: u32 = end_str
                    .parse()
                    .map_err(|_| ParseError::InvalidLineNumber(end_str.to_string()))?;

                if end < start {
                    return Err(ParseError::InvalidLineRange(format!(
                        "end ({end}) must be >= start ({start})"
                    )));
                }

                return Ok(Self::LineRange(start, end));
            }

            // Single line
            let line: u32 =
                rest.parse().map_err(|_| ParseError::InvalidLineNumber(rest.to_string()))?;
            return Ok(Self::Line(line));
        }

        // Otherwise it's a symbol reference
        Ok(Self::Symbol(s.to_string()))
    }

    /// Get line number if this is a Line fragment
    #[must_use]
    pub const fn line(&self) -> Option<u32> {
        match self {
            Self::Line(n) => Some(*n),
            _ => None,
        }
    }

    /// Get line range if this is a `LineRange` fragment
    #[must_use]
    pub const fn line_range(&self) -> Option<(u32, u32)> {
        match self {
            Self::LineRange(start, end) => Some((*start, *end)),
            _ => None,
        }
    }

    /// Get symbol name if this is a Symbol fragment
    #[must_use]
    pub fn symbol(&self) -> Option<&str> {
        match self {
            Self::Symbol(s) => Some(s),
            _ => None,
        }
    }
}

impl std::fmt::Display for Target {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.raw)
    }
}

impl std::fmt::Display for Fragment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Line(n) => write!(f, "L{n}"),
            Self::LineRange(start, end) => write!(f, "L{start}-L{end}"),
            Self::Symbol(s) => write!(f, "{s}"),
        }
    }
}
