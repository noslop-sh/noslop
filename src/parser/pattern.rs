//! Pattern matching for code quality enforcement

use serde::{Deserialize, Serialize};

use super::Span;

/// Kind of pattern matching
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PatternKind {
    /// Regular expression
    #[default]
    Regex,
    /// Glob pattern (file paths)
    Glob,
    /// AST-based pattern (future)
    Ast,
    /// Semantic pattern (future)
    Semantic,
}

impl std::fmt::Display for PatternKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Regex => write!(f, "regex"),
            Self::Glob => write!(f, "glob"),
            Self::Ast => write!(f, "ast"),
            Self::Semantic => write!(f, "semantic"),
        }
    }
}

/// A pattern to match against source code
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pattern {
    /// Unique identifier
    pub id: String,
    /// The pattern string (regex, glob, etc.)
    pub pattern: String,
    /// Kind of pattern
    #[serde(default)]
    pub kind: PatternKind,
    /// Human-readable message
    pub message: String,
    /// Severity: info, warn, block
    #[serde(default = "default_severity")]
    pub severity: String,
    /// Languages this pattern applies to
    #[serde(default)]
    pub languages: Vec<String>,
    /// File patterns this applies to (glob)
    #[serde(default)]
    pub files: Vec<String>,
}

fn default_severity() -> String {
    "warn".to_string()
}

impl Pattern {
    /// Create a new regex pattern
    #[must_use]
    pub fn regex(id: &str, pattern: &str, message: &str) -> Self {
        Self {
            id: id.to_string(),
            pattern: pattern.to_string(),
            kind: PatternKind::Regex,
            message: message.to_string(),
            severity: "warn".to_string(),
            languages: Vec::new(),
            files: Vec::new(),
        }
    }

    /// Set severity
    #[must_use]
    pub fn with_severity(mut self, severity: &str) -> Self {
        self.severity = severity.to_string();
        self
    }

    /// Set languages
    #[must_use]
    pub fn with_languages(mut self, languages: Vec<String>) -> Self {
        self.languages = languages;
        self
    }

    /// Check if this pattern applies to a file
    #[must_use]
    pub fn applies_to(&self, path: &str) -> bool {
        // Check language by extension
        if !self.languages.is_empty() {
            let ext = std::path::Path::new(path).extension().and_then(|e| e.to_str()).unwrap_or("");

            let lang_match = self.languages.iter().any(|lang| {
                matches!(
                    (lang.as_str(), ext),
                    ("rust", "rs")
                        | ("python", "py")
                        | ("javascript", "js")
                        | ("typescript", "ts")
                        | ("go", "go")
                        | ("java", "java")
                        | ("c", "c")
                        | ("cpp", "cpp" | "cc" | "cxx")
                        | ("ruby", "rb")
                )
            });

            if !lang_match {
                return false;
            }
        }

        // Check file patterns
        if !self.files.is_empty() {
            let file_match = self.files.iter().any(|pattern| {
                // Simple glob matching
                pattern.strip_prefix('*').map_or_else(
                    || {
                        pattern.strip_suffix('*').map_or_else(
                            || path.contains(pattern),
                            |prefix| path.starts_with(prefix),
                        )
                    },
                    |suffix| path.ends_with(suffix),
                )
            });

            if !file_match {
                return false;
            }
        }

        true
    }
}

/// A match result from pattern matching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Match {
    /// Span in source where the match occurred
    pub span: Span,
    /// The matched text
    pub text: String,
    /// ID of the pattern that matched
    pub pattern_id: String,
}

impl Match {
    /// Create a new match
    #[must_use]
    pub const fn new(span: Span, text: String, pattern_id: String) -> Self {
        Self {
            span,
            text,
            pattern_id,
        }
    }

    /// Get line number (1-indexed) from content
    #[must_use]
    pub fn line_number(&self, content: &str) -> usize {
        content[..self.span.start].lines().count() + 1
    }
}
