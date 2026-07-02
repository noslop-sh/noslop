//! .noslop.toml file loading
//!
//! Scans for .noslop.toml files from a path up to the repo root,
//! collecting all checks that apply.
//!
//! This module delegates to `noslop::adapters::toml` for the actual implementation.

use noslop::adapters::toml::add_check as adapter_add_check;
use noslop::adapters::toml::generate_prefix_from_repo as adapter_generate_prefix;
use noslop::core::models::{Check, Severity};
use noslop::core::services::matches_target;

// Re-export types for backwards compatibility (some may be unused but kept for external use)
#[allow(unused_imports)]
pub use noslop::adapters::toml::{
    CheckEntry, NoslopFile, ProjectConfig, find_noslop_files, load_file,
};

/// Load all checks applicable to a set of files
pub fn load_checks_for_files(files: &[String]) -> anyhow::Result<Vec<(Check, String)>> {
    let mut result = Vec::new();
    let cwd = std::env::current_dir()?;

    for file in files {
        let file_path = cwd.join(file);
        let noslop_files = find_noslop_files(&file_path);

        for noslop_path in noslop_files {
            let noslop_file = load_file(&noslop_path)?;
            let noslop_dir = noslop_path.parent().unwrap_or(&cwd);

            for entry in &noslop_file.checks {
                if matches_target(&entry.target, file, noslop_dir, &cwd) {
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

/// Load every check defined in .noslop.toml files reachable from the cwd
pub fn load_all_checks() -> anyhow::Result<Vec<Check>> {
    let cwd = std::env::current_dir()?;
    let mut checks = Vec::new();

    for noslop_path in find_noslop_files(&cwd) {
        let noslop_file = load_file(&noslop_path)?;
        for entry in &noslop_file.checks {
            checks.push(Check::new(
                entry.id.clone(),
                entry.target.clone(),
                entry.message.clone(),
                entry.severity.parse().unwrap_or(Severity::Block),
            ));
        }
    }

    Ok(checks)
}

/// Find a check by exact ID
pub fn find_check_by_id(id: &str) -> anyhow::Result<Option<Check>> {
    Ok(load_all_checks()?.into_iter().find(|c| c.id == id))
}

/// Create or update a .noslop.toml file with a new check
pub fn add_check(target: &str, message: &str, severity: &str) -> anyhow::Result<String> {
    adapter_add_check(target, message, severity)
}

/// Generate a 3-letter prefix from git repository name
///
/// Examples: "noslop" -> "NOS", "my-awesome-project" -> "MAP"
pub fn generate_prefix_from_repo() -> String {
    use crate::git;
    adapter_generate_prefix(&git::get_repo_name())
}
