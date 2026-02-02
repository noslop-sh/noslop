//! TOML-based assertion repository
//!
//! Implements the `AssertionRepository` port trait using .noslop.toml files.

use std::fs;

use crate::core::models::{Assertion, Severity};
use crate::core::ports::AssertionRepository;
use crate::core::services::matches_target;

use super::parser::{find_noslop_files, load_file};
use super::writer::add_assertion;

/// Assertion repository backed by .noslop.toml files
#[derive(Debug, Clone)]
pub struct TomlAssertionRepository {
    /// Base directory for file operations (usually current directory)
    base_dir: std::path::PathBuf,
}

impl TomlAssertionRepository {
    /// Create a new repository with the given base directory
    #[must_use]
    pub const fn new(base_dir: std::path::PathBuf) -> Self {
        Self { base_dir }
    }

    /// Create a repository for the current directory
    ///
    /// # Errors
    ///
    /// Returns an error if the current directory cannot be determined.
    pub fn current_dir() -> anyhow::Result<Self> {
        Ok(Self::new(std::env::current_dir()?))
    }
}

impl AssertionRepository for TomlAssertionRepository {
    fn find_for_files(&self, files: &[String]) -> anyhow::Result<Vec<(Assertion, String)>> {
        let mut result = Vec::new();

        for file in files {
            let file_path = self.base_dir.join(file);
            let noslop_files = find_noslop_files(&file_path);

            for noslop_path in noslop_files {
                let noslop_file = load_file(&noslop_path)?;
                let noslop_dir = noslop_path.parent().unwrap_or(&self.base_dir);

                for entry in &noslop_file.assertions {
                    if matches_target(&entry.target, file, noslop_dir, &self.base_dir) {
                        let assertion = Assertion::new(
                            entry.id.clone(),
                            entry.target.clone(),
                            entry.message.clone(),
                            entry.severity.parse().unwrap_or(Severity::Block),
                        );
                        result.push((assertion, file.clone()));
                    }
                }
            }
        }

        // Dedupe by assertion message + file (same assertion might match from multiple noslop files)
        result.sort_by(|a, b| (&a.0.message, &a.1).cmp(&(&b.0.message, &b.1)));
        result.dedup_by(|a, b| a.0.message == b.0.message && a.1 == b.1);

        Ok(result)
    }

    fn add(&self, target: &str, message: &str, severity: Severity) -> anyhow::Result<String> {
        add_assertion(target, message, &severity.to_string())
    }

    fn remove(&self, id: &str) -> anyhow::Result<()> {
        let path = self.base_dir.join(".noslop.toml");
        if !path.exists() {
            anyhow::bail!("No .noslop.toml file found");
        }

        let mut file = load_file(&path)?;
        let initial_len = file.assertions.len();

        file.assertions.retain(|a| a.id.as_deref() != Some(id));

        if file.assertions.len() == initial_len {
            anyhow::bail!("Assertion not found: {id}");
        }

        let content = super::writer::format_noslop_file(&file);
        fs::write(&path, content)?;

        Ok(())
    }

    fn list(&self) -> anyhow::Result<Vec<Assertion>> {
        self.list_filtered(None)
    }

    fn list_filtered(&self, target_filter: Option<&str>) -> anyhow::Result<Vec<Assertion>> {
        let noslop_files = find_noslop_files(&self.base_dir);
        let mut assertions = Vec::new();

        for noslop_path in noslop_files {
            let noslop_file = load_file(&noslop_path)?;

            for entry in noslop_file.assertions {
                // Apply target filter if provided
                if let Some(filter) = target_filter
                    && !entry.target.contains(filter)
                {
                    continue;
                }

                let assertion = Assertion::new(
                    entry.id,
                    entry.target,
                    entry.message,
                    entry.severity.parse().unwrap_or(Severity::Block),
                );
                assertions.push(assertion);
            }
        }

        Ok(assertions)
    }
}
