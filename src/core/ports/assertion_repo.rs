//! Assertion repository port
//!
//! Defines the interface for loading and managing assertions.

use crate::models::{Assertion, Severity};

/// Repository for managing assertions
///
/// Implementations handle the persistence and retrieval of assertions
/// from various sources (TOML files, databases, etc.)
pub trait AssertionRepository: Send + Sync {
    /// Find all assertions that apply to the given files
    ///
    /// Returns a list of `(assertion, matched_file)` pairs.
    fn find_for_files(&self, files: &[String]) -> anyhow::Result<Vec<(Assertion, String)>>;

    /// Add a new assertion
    ///
    /// Returns the generated assertion ID.
    fn add(&self, target: &str, message: &str, severity: Severity) -> anyhow::Result<String>;

    /// Remove an assertion by ID
    fn remove(&self, id: &str) -> anyhow::Result<()>;

    /// List all assertions
    fn list(&self) -> anyhow::Result<Vec<Assertion>>;

    /// List all assertions, optionally filtered by target pattern
    fn list_filtered(&self, target_filter: Option<&str>) -> anyhow::Result<Vec<Assertion>> {
        let all = self.list()?;
        match target_filter {
            Some(filter) => Ok(all.into_iter().filter(|a| a.target.contains(filter)).collect()),
            None => Ok(all),
        }
    }
}
