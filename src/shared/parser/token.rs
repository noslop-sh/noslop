//! Token types for parsed source code

use serde::{Deserialize, Serialize};

/// A span in source code (byte offsets)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Span {
    /// Start byte offset
    pub start: usize,
    /// End byte offset
    pub end: usize,
}

impl Span {
    /// Create a new span
    #[must_use]
    pub const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    /// Length of the span
    #[must_use]
    pub const fn len(&self) -> usize {
        self.end - self.start
    }

    /// Check if span is empty
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.start == self.end
    }

    /// Check if this span contains another
    #[must_use]
    pub const fn contains(&self, other: &Self) -> bool {
        self.start <= other.start && other.end <= self.end
    }
}

/// Kind of token
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TokenKind {
    /// Function or method
    Function,
    /// Class or struct
    Class,
    /// Variable or constant
    Variable,
    /// Module or namespace
    Module,
    /// Trait or interface
    Trait,
    /// Enum type
    Enum,
    /// Type alias
    Type,
    /// Macro
    Macro,
    /// Import/use statement
    Import,
    /// Comment (regular or doc)
    Comment,
    /// Unknown/other
    Other,
}

impl std::fmt::Display for TokenKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Function => write!(f, "function"),
            Self::Class => write!(f, "class"),
            Self::Variable => write!(f, "variable"),
            Self::Module => write!(f, "module"),
            Self::Trait => write!(f, "trait"),
            Self::Enum => write!(f, "enum"),
            Self::Type => write!(f, "type"),
            Self::Macro => write!(f, "macro"),
            Self::Import => write!(f, "import"),
            Self::Comment => write!(f, "comment"),
            Self::Other => write!(f, "other"),
        }
    }
}

/// A token in source code
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Token {
    /// Kind of token
    pub kind: TokenKind,
    /// Name of the token (e.g., function name)
    pub name: String,
    /// Span in source
    pub span: Span,
    /// Child tokens (e.g., methods in a class)
    #[serde(default)]
    pub children: Vec<Self>,
    /// Optional documentation
    #[serde(default)]
    pub doc: Option<String>,
}

impl Token {
    /// Create a new token
    #[must_use]
    pub const fn new(kind: TokenKind, name: String, span: Span) -> Self {
        Self {
            kind,
            name,
            span,
            children: Vec::new(),
            doc: None,
        }
    }

    /// Add a child token
    #[must_use]
    pub fn with_child(mut self, child: Self) -> Self {
        self.children.push(child);
        self
    }

    /// Add documentation
    #[must_use]
    pub fn with_doc(mut self, doc: String) -> Self {
        self.doc = Some(doc);
        self
    }

    /// Find a child token by name
    #[must_use]
    pub fn find_child(&self, name: &str) -> Option<&Self> {
        self.children.iter().find(|t| t.name == name)
    }

    /// Full path name (for nested tokens)
    #[must_use]
    pub fn full_name(&self, parent: Option<&str>) -> String {
        parent.map_or_else(|| self.name.clone(), |p| format!("{}::{}", p, self.name))
    }
}

/// A tree of tokens representing a source file
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenTree {
    /// Root-level tokens
    pub tokens: Vec<Token>,
}

impl TokenTree {
    /// Create an empty token tree
    #[must_use]
    pub const fn empty() -> Self {
        Self { tokens: Vec::new() }
    }

    /// Create a token tree with tokens
    #[must_use]
    pub const fn new(tokens: Vec<Token>) -> Self {
        Self { tokens }
    }

    /// Add a token
    pub fn push(&mut self, token: Token) {
        self.tokens.push(token);
    }

    /// Find a token by name (searches recursively)
    #[must_use]
    pub fn find_by_name(&self, name: &str) -> Option<Token> {
        for token in &self.tokens {
            if token.name == name {
                return Some(token.clone());
            }
            if let Some(child) = Self::find_in_children(token, name) {
                return Some(child);
            }
        }
        None
    }

    fn find_in_children(token: &Token, name: &str) -> Option<Token> {
        for child in &token.children {
            if child.name == name {
                return Some(child.clone());
            }
            if let Some(found) = Self::find_in_children(child, name) {
                return Some(found);
            }
        }
        None
    }

    /// Find all tokens of a specific kind
    #[must_use]
    pub fn find_by_kind(&self, kind: TokenKind) -> Vec<&Token> {
        let mut result = Vec::new();
        for token in &self.tokens {
            Self::collect_by_kind(token, kind, &mut result);
        }
        result
    }

    fn collect_by_kind<'a>(token: &'a Token, kind: TokenKind, result: &mut Vec<&'a Token>) {
        if token.kind == kind {
            result.push(token);
        }
        for child in &token.children {
            Self::collect_by_kind(child, kind, result);
        }
    }

    /// Get total number of tokens (recursive)
    #[must_use]
    pub fn len(&self) -> usize {
        self.tokens.iter().map(Self::count_token).sum()
    }

    /// Check if empty
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.tokens.is_empty()
    }

    fn count_token(token: &Token) -> usize {
        1 + token.children.iter().map(Self::count_token).sum::<usize>()
    }
}
