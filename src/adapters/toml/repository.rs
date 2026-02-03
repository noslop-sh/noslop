//! TOML-based check repository
//!
//! Implements the `CheckRepository` port trait using .noslop.toml files.

use std::fs;

use crate::core::models::{Check, Severity};
use crate::core::ports::CheckRepository;
use crate::core::services::matches_target;

use super::parser::{find_noslop_files, load_file};
use super::writer::add_check;

/// Check repository backed by .noslop.toml files
#[derive(Debug, Clone)]
pub struct TomlCheckRepository {
    /// Base directory for file operations (usually current directory)
    base_dir: std::path::PathBuf,
}

impl TomlCheckRepository {
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

impl CheckRepository for TomlCheckRepository {
    fn find_for_files(&self, files: &[String]) -> anyhow::Result<Vec<(Check, String)>> {
        let mut result = Vec::new();

        for file in files {
            let file_path = self.base_dir.join(file);
            let noslop_files = find_noslop_files(&file_path);

            for noslop_path in noslop_files {
                let noslop_file = load_file(&noslop_path)?;
                let noslop_dir = noslop_path.parent().unwrap_or(&self.base_dir);

                for entry in &noslop_file.checks {
                    if matches_target(&entry.target, file, noslop_dir, &self.base_dir) {
                        let check = Check::new(
                            entry.id.clone(),
                            entry.target.clone(),
                            entry.message.clone(),
                            entry.severity.parse().unwrap_or(Severity::Block),
                        );
                        result.push((check, file.clone()));
                    }
                }
            }
        }

        // Dedupe by check message + file (same check might match from multiple noslop files)
        result.sort_by(|a, b| (&a.0.message, &a.1).cmp(&(&b.0.message, &b.1)));
        result.dedup_by(|a, b| a.0.message == b.0.message && a.1 == b.1);

        Ok(result)
    }

    fn add(&self, target: &str, message: &str, severity: Severity) -> anyhow::Result<String> {
        add_check(target, message, &severity.to_string())
    }

    fn remove(&self, id: &str) -> anyhow::Result<()> {
        let path = self.base_dir.join(".noslop.toml");
        if !path.exists() {
            anyhow::bail!("No .noslop.toml file found");
        }

        let mut file = load_file(&path)?;
        let initial_len = file.checks.len();

        file.checks.retain(|c| c.id.as_deref() != Some(id));

        if file.checks.len() == initial_len {
            anyhow::bail!("Check not found: {id}");
        }

        let content = super::writer::format_noslop_file(&file);
        fs::write(&path, content)?;

        Ok(())
    }

    fn list(&self) -> anyhow::Result<Vec<Check>> {
        self.list_filtered(None)
    }

    fn list_filtered(&self, target_filter: Option<&str>) -> anyhow::Result<Vec<Check>> {
        let noslop_files = find_noslop_files(&self.base_dir);
        let mut checks = Vec::new();

        for noslop_path in noslop_files {
            let noslop_file = load_file(&noslop_path)?;

            for entry in noslop_file.checks {
                // Apply target filter if provided
                if let Some(filter) = target_filter
                    && !entry.target.contains(filter)
                {
                    continue;
                }

                let check = Check::new(
                    entry.id,
                    entry.target,
                    entry.message,
                    entry.severity.parse().unwrap_or(Severity::Block),
                );
                checks.push(check);
            }
        }

        Ok(checks)
    }
}
