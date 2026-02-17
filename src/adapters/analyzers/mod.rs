//! Review analyzer implementations
//!
//! Concrete implementations of [`crate::core::ports::ReviewAnalyzer`].
//!
//! - [`convention`] - Matches `[[check]]` rules against changed files

/// Convention analyzer: matches `[[check]]` rules against changed files.
pub mod convention;

pub use convention::ConventionAnalyzer;
