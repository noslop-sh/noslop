//! Resolver - resolves Targets to actual file content
//!
//! The Resolver takes a `Target` and returns matching files from the repository,
//! optionally extracting specific content (lines, symbols).
//!
//! # Examples
//!
//! ```no_run
//! use noslop::target::Target;
//! use noslop::resolver::Resolver;
//!
//! let resolver = Resolver::new(".").unwrap();
//! let target = Target::parse("src/**/*.rs").unwrap();
//! let files = resolver.resolve(&target).unwrap();
//! ```

use std::fs;
use std::path::{Path, PathBuf};

use thiserror::Error;
use walkdir::WalkDir;

use crate::target::{Fragment, Target};

/// Errors that can occur during resolution
#[derive(Debug, Error)]
pub enum ResolveError {
    /// Root path does not exist
    #[error("root path does not exist: {0}")]
    RootNotFound(PathBuf),

    /// Path is not a directory
    #[error("not a directory: {0}")]
    NotADirectory(PathBuf),

    /// IO error during file operations
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// Error walking directory tree
    #[error("walkdir error: {0}")]
    WalkDir(#[from] walkdir::Error),

    /// Line number exceeds file length
    #[error("line {0} out of range (file has {1} lines)")]
    LineOutOfRange(u32, usize),

    /// Symbol resolution is not yet implemented
    #[error("symbol resolution not yet implemented: {0}")]
    SymbolNotImplemented(String),
}

/// A resolved file with optional content extraction
#[derive(Debug, Clone)]
pub struct ResolvedFile {
    /// The file path (relative to resolver root)
    pub path: PathBuf,

    /// The absolute file path
    pub absolute_path: PathBuf,

    /// Extracted content, if fragment was specified
    pub content: Option<ResolvedContent>,
}

/// Extracted content from a file
#[derive(Debug, Clone)]
pub enum ResolvedContent {
    /// Full file content
    Full(String),

    /// Specific lines (1-indexed, inclusive range)
    Lines {
        /// Start line (1-indexed)
        start: u32,
        /// End line (1-indexed, inclusive)
        end: u32,
        /// The extracted content
        content: String,
    },

    /// Symbol reference (deferred - just stores the symbol name for now)
    Symbol {
        /// The symbol name (e.g., "Session", "Session.validate")
        name: String,
        // Future: actual resolved content, span, etc.
    },
}

/// Resolver for finding and extracting file content
#[derive(Debug)]
pub struct Resolver {
    /// Root directory to resolve from
    root: PathBuf,

    /// Whether to respect .gitignore (future)
    #[allow(dead_code)]
    respect_gitignore: bool,
}

impl Resolver {
    /// Create a new resolver rooted at the given path
    pub fn new(root: impl AsRef<Path>) -> Result<Self, ResolveError> {
        let root = root.as_ref().to_path_buf();

        if !root.exists() {
            return Err(ResolveError::RootNotFound(root));
        }

        let root = if root.is_file() {
            root.parent()
                .ok_or_else(|| ResolveError::NotADirectory(root.clone()))?
                .to_path_buf()
        } else {
            root
        };

        Ok(Self {
            root,
            respect_gitignore: true,
        })
    }

    /// Create a resolver at the current working directory
    pub fn current_dir() -> Result<Self, ResolveError> {
        let cwd = std::env::current_dir()?;
        Self::new(cwd)
    }

    /// Get the root path
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Find all files matching a target (without content extraction)
    pub fn find_files(&self, target: &Target) -> Result<Vec<PathBuf>, ResolveError> {
        let mut matches = Vec::new();
        let root = &self.root;

        for entry in WalkDir::new(root).follow_links(true).into_iter().filter_entry(|e| {
            // Don't filter the root directory itself
            if e.path() == root {
                return true;
            }
            !Self::is_hidden(e)
        }) {
            let entry = entry?;
            if !entry.file_type().is_file() {
                continue;
            }

            let path = entry.path();
            let relative = path.strip_prefix(root).unwrap_or(path).to_path_buf();

            if target.matches(&relative) {
                matches.push(relative);
            }
        }

        // Sort for deterministic output
        matches.sort();
        Ok(matches)
    }

    /// Resolve a target to files with content extraction
    pub fn resolve(&self, target: &Target) -> Result<Vec<ResolvedFile>, ResolveError> {
        let files = self.find_files(target)?;
        let mut resolved = Vec::with_capacity(files.len());

        for path in files {
            let absolute_path = self.root.join(&path);
            let content = match target.fragment() {
                Some(fragment) => Some(self.extract_content(&absolute_path, fragment)?),
                None => None,
            };

            resolved.push(ResolvedFile {
                path,
                absolute_path,
                content,
            });
        }

        Ok(resolved)
    }

    /// Resolve a target and return just file paths (convenience method)
    pub fn resolve_paths(&self, target: &Target) -> Result<Vec<PathBuf>, ResolveError> {
        self.find_files(target)
    }

    /// Check if a single path matches a target
    pub fn matches(&self, target: &Target, path: impl AsRef<Path>) -> bool {
        target.matches(path)
    }

    /// Extract content based on fragment
    #[allow(clippy::unused_self)] // Will use self for symbol resolution in future
    fn extract_content(
        &self,
        path: &Path,
        fragment: &Fragment,
    ) -> Result<ResolvedContent, ResolveError> {
        match fragment {
            Fragment::Line(line) => {
                let content = fs::read_to_string(path)?;
                let lines: Vec<&str> = content.lines().collect();
                let line_idx = (*line as usize).saturating_sub(1);

                if line_idx >= lines.len() {
                    return Err(ResolveError::LineOutOfRange(*line, lines.len()));
                }

                Ok(ResolvedContent::Lines {
                    start: *line,
                    end: *line,
                    content: lines[line_idx].to_string(),
                })
            },
            Fragment::LineRange(start, end) => {
                let content = fs::read_to_string(path)?;
                let lines: Vec<&str> = content.lines().collect();
                let start_idx = (*start as usize).saturating_sub(1);
                let end_idx = (*end as usize).saturating_sub(1);

                if end_idx >= lines.len() {
                    return Err(ResolveError::LineOutOfRange(*end, lines.len()));
                }

                let extracted: String = lines[start_idx..=end_idx].join("\n");
                Ok(ResolvedContent::Lines {
                    start: *start,
                    end: *end,
                    content: extracted,
                })
            },
            Fragment::Symbol(name) => {
                // Defer symbol resolution to future implementation
                // For now, just record the symbol name
                Ok(ResolvedContent::Symbol { name: name.clone() })
            },
        }
    }

    /// Check if an entry is hidden (starts with .)
    fn is_hidden(entry: &walkdir::DirEntry) -> bool {
        entry.file_name().to_str().is_some_and(|s| s.starts_with('.'))
    }

    /// Read full content of a file
    pub fn read_file(&self, path: impl AsRef<Path>) -> Result<String, ResolveError> {
        let absolute = if path.as_ref().is_absolute() {
            path.as_ref().to_path_buf()
        } else {
            self.root.join(path)
        };
        Ok(fs::read_to_string(absolute)?)
    }

    /// Read specific lines from a file (1-indexed, inclusive)
    pub fn read_lines(
        &self,
        path: impl AsRef<Path>,
        start: u32,
        end: u32,
    ) -> Result<String, ResolveError> {
        let content = self.read_file(path)?;
        let lines: Vec<&str> = content.lines().collect();
        let start_idx = (start as usize).saturating_sub(1);
        let end_idx = (end as usize).saturating_sub(1);

        if end_idx >= lines.len() {
            return Err(ResolveError::LineOutOfRange(end, lines.len()));
        }

        Ok(lines[start_idx..=end_idx].join("\n"))
    }
}

impl ResolvedFile {
    /// Get the content as a string, if available
    #[must_use]
    pub fn content_str(&self) -> Option<&str> {
        match &self.content {
            Some(ResolvedContent::Full(s)) => Some(s),
            Some(ResolvedContent::Lines { content, .. }) => Some(content),
            Some(ResolvedContent::Symbol { .. }) | None => None,
        }
    }

    /// Check if this resolved file has content
    #[must_use]
    pub const fn has_content(&self) -> bool {
        self.content.is_some()
    }
}
