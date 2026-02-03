//! Check repository port
//!
//! Defines the interface for loading and managing checks.

use super::super::models::{Check, Severity};

/// Repository for managing checks
///
/// Implementations handle the persistence and retrieval of checks
/// from various sources (TOML files, databases, etc.)
pub trait CheckRepository: Send + Sync {
    /// Find all checks that apply to the given files
    ///
    /// Returns a list of `(check, matched_file)` pairs.
    fn find_for_files(&self, files: &[String]) -> anyhow::Result<Vec<(Check, String)>>;

    /// Add a new check
    ///
    /// Returns the generated check ID.
    fn add(&self, target: &str, message: &str, severity: Severity) -> anyhow::Result<String>;

    /// Remove a check by ID
    fn remove(&self, id: &str) -> anyhow::Result<()>;

    /// List all checks
    fn list(&self) -> anyhow::Result<Vec<Check>>;

    /// List all checks, optionally filtered by target pattern
    fn list_filtered(&self, target_filter: Option<&str>) -> anyhow::Result<Vec<Check>> {
        let all = self.list()?;
        match target_filter {
            Some(filter) => Ok(all.into_iter().filter(|c| c.target.contains(filter)).collect()),
            None => Ok(all),
        }
    }
}
