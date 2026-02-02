//! Parser abstraction for token-level assertions and pattern matching
//!
//! Provides pluggable parsers for different languages:
//! - Token extraction (for symbol-level assertions)
//! - Pattern matching (for code quality enforcement)
//!
//! Users can configure custom parsers for specific file extensions.

#![allow(dead_code)]

mod pattern;
mod token;

use std::path::Path;

// Re-export types for public API (used by integration tests)
#[allow(unused_imports)]
pub use pattern::{Match, Pattern, PatternKind};
#[allow(unused_imports)]
pub use token::{Span, Token, TokenKind, TokenTree};

/// A parser that can tokenize source code and match patterns
pub trait Parser: Send + Sync {
    /// Get the name of this parser (e.g., "rust", "python", "regex")
    fn name(&self) -> &str;

    /// Get file extensions this parser handles
    fn extensions(&self) -> &[&str];

    /// Extract tokens from source code
    fn tokenize(&self, content: &[u8], path: &Path) -> anyhow::Result<TokenTree>;

    /// Match a pattern against source code
    fn match_pattern(&self, content: &[u8], pattern: &Pattern) -> anyhow::Result<Vec<Match>>;

    /// Find a token by name (e.g., "`validate_session`")
    fn find_token(&self, content: &[u8], name: &str) -> anyhow::Result<Option<Token>> {
        let tree = self.tokenize(content, Path::new(""))?;
        Ok(tree.find_by_name(name))
    }
}

/// Registry of parsers by file extension
pub struct ParserRegistry {
    parsers: Vec<Box<dyn Parser>>,
}

impl std::fmt::Debug for ParserRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ParserRegistry")
            .field("parsers", &format!("{} parser(s)", self.parsers.len()))
            .finish()
    }
}

impl Default for ParserRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ParserRegistry {
    /// Create a new registry with built-in parsers
    #[must_use]
    pub fn new() -> Self {
        Self {
            parsers: vec![
                Box::new(RegexParser),
                // Add more built-in parsers here
            ],
        }
    }

    /// Register a custom parser
    pub fn register(&mut self, parser: Box<dyn Parser>) {
        self.parsers.push(parser);
    }

    /// Get a parser for a file path
    #[must_use]
    pub fn parser_for(&self, path: &Path) -> Option<&dyn Parser> {
        let ext = path.extension()?.to_str()?;
        self.parsers.iter().find(|p| p.extensions().contains(&ext)).map(AsRef::as_ref)
    }

    /// Get the regex parser (always available)
    ///
    /// # Panics
    ///
    /// Panics if the regex parser is not registered (should never happen)
    #[must_use]
    pub fn regex_parser(&self) -> &dyn Parser {
        self.parsers
            .iter()
            .find(|p| p.name() == "regex")
            .map(AsRef::as_ref)
            .expect("regex parser always available")
    }
}

/// Basic regex-based parser (works for all files)
#[derive(Debug, Clone, Copy)]
pub struct RegexParser;

impl Parser for RegexParser {
    fn name(&self) -> &'static str {
        "regex"
    }

    fn extensions(&self) -> &[&str] {
        // Matches all files as fallback
        &[]
    }

    fn tokenize(&self, _content: &[u8], _path: &Path) -> anyhow::Result<TokenTree> {
        // Regex parser doesn't do tokenization
        Ok(TokenTree::empty())
    }

    fn match_pattern(&self, content: &[u8], pattern: &Pattern) -> anyhow::Result<Vec<Match>> {
        use regex::Regex;

        let content_str = String::from_utf8_lossy(content);
        let re = Regex::new(&pattern.pattern)?;

        let matches: Vec<Match> = re
            .find_iter(&content_str)
            .map(|m| Match {
                span: Span {
                    start: m.start(),
                    end: m.end(),
                },
                text: m.as_str().to_string(),
                pattern_id: pattern.id.clone(),
            })
            .collect();

        Ok(matches)
    }
}

/// Get the global parser registry
#[must_use]
pub fn registry() -> ParserRegistry {
    ParserRegistry::new()
}
