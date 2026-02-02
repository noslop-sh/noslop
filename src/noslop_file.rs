//! .noslop.toml file loading
//!
//! Scans for .noslop.toml files from a path up to the repo root,
//! collecting all assertions that apply.
//!
//! This module delegates to `noslop::adapters::toml` for the actual implementation.

use noslop::adapters::toml::add_assertion as adapter_add_assertion;
use noslop::adapters::toml::generate_prefix_from_repo as adapter_generate_prefix;
use noslop::core::models::{Assertion, Severity};
use noslop::core::services::matches_target;

// Re-export types for backwards compatibility (some may be unused but kept for external use)
#[allow(unused_imports)]
pub use noslop::adapters::toml::{
    AssertionEntry, NoslopFile, ProjectConfig, find_noslop_files, load_file,
};

/// Load all assertions applicable to a set of files
pub fn load_assertions_for_files(files: &[String]) -> anyhow::Result<Vec<(Assertion, String)>> {
    let mut result = Vec::new();
    let cwd = std::env::current_dir()?;

    for file in files {
        let file_path = cwd.join(file);
        let noslop_files = find_noslop_files(&file_path);

        for noslop_path in noslop_files {
            let noslop_file = load_file(&noslop_path)?;
            let noslop_dir = noslop_path.parent().unwrap_or(&cwd);

            for entry in &noslop_file.assertions {
                if matches_target(&entry.target, file, noslop_dir, &cwd) {
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

/// Create or update a .noslop.toml file with a new assertion
pub fn add_assertion(target: &str, message: &str, severity: &str) -> anyhow::Result<String> {
    adapter_add_assertion(target, message, severity)
}

/// Generate a 3-letter prefix from git repository name
///
/// Examples: "noslop" -> "NOS", "my-awesome-project" -> "MAP"
pub fn generate_prefix_from_repo() -> String {
    use crate::git;
    adapter_generate_prefix(&git::get_repo_name())
}
