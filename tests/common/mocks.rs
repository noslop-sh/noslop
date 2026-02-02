//! Mock implementations of port traits for testing
//!
//! These mocks provide configurable behavior for unit testing
//! without real I/O operations.

use noslop::core::models::{Assertion, Attestation, Severity};
use noslop::core::ports::{AssertionRepository, AttestationStore, VersionControl};
use std::cell::RefCell;
use std::path::{Path, PathBuf};

/// Mock implementation of AssertionRepository
pub struct MockAssertionRepository {
    assertions: RefCell<Vec<Assertion>>,
}

impl MockAssertionRepository {
    pub fn new() -> Self {
        Self {
            assertions: RefCell::new(Vec::new()),
        }
    }

    pub fn with_assertions(assertions: Vec<Assertion>) -> Self {
        Self {
            assertions: RefCell::new(assertions),
        }
    }
}

impl Default for MockAssertionRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl AssertionRepository for MockAssertionRepository {
    fn find_for_files(&self, files: &[String]) -> anyhow::Result<Vec<(Assertion, String)>> {
        let assertions = self.assertions.borrow();
        let mut results = Vec::new();
        for assertion in assertions.iter() {
            for file in files {
                // Simple matching: check if target matches file extension or exact match
                if file.ends_with(&assertion.target.trim_start_matches('*'))
                    || assertion.target == "*"
                    || file == &assertion.target
                {
                    results.push((assertion.clone(), file.clone()));
                }
            }
        }
        Ok(results)
    }

    fn add(&self, target: &str, message: &str, severity: Severity) -> anyhow::Result<String> {
        let id = format!("MOCK-{}", self.assertions.borrow().len() + 1);
        let assertion = Assertion::new(
            Some(id.clone()),
            target.to_string(),
            message.to_string(),
            severity,
        );
        self.assertions.borrow_mut().push(assertion);
        Ok(id)
    }

    fn remove(&self, id: &str) -> anyhow::Result<()> {
        self.assertions.borrow_mut().retain(|a| a.id != id);
        Ok(())
    }

    fn list(&self) -> anyhow::Result<Vec<Assertion>> {
        Ok(self.assertions.borrow().clone())
    }
}

/// Mock implementation of AttestationStore
pub struct MockAttestationStore {
    staged: RefCell<Vec<Attestation>>,
}

impl MockAttestationStore {
    pub fn new() -> Self {
        Self {
            staged: RefCell::new(Vec::new()),
        }
    }

    pub fn with_staged(attestations: Vec<Attestation>) -> Self {
        Self {
            staged: RefCell::new(attestations),
        }
    }
}

impl Default for MockAttestationStore {
    fn default() -> Self {
        Self::new()
    }
}

impl AttestationStore for MockAttestationStore {
    fn stage(&self, attestation: &Attestation) -> anyhow::Result<()> {
        self.staged.borrow_mut().push(attestation.clone());
        Ok(())
    }

    fn staged(&self) -> anyhow::Result<Vec<Attestation>> {
        Ok(self.staged.borrow().clone())
    }

    fn clear_staged(&self) -> anyhow::Result<()> {
        self.staged.borrow_mut().clear();
        Ok(())
    }

    fn format_trailers(&self, attestations: &[Attestation]) -> String {
        attestations
            .iter()
            .map(|a| format!("Noslop-Attest: {} | {} | {}", a.assertion_id, a.message, a.attested_by))
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn parse_from_commit(&self, _commit_sha: &str) -> anyhow::Result<Vec<Attestation>> {
        // Mock: return empty for simplicity
        Ok(Vec::new())
    }
}

/// Mock implementation of VersionControl
pub struct MockVersionControl {
    staged_files: Vec<String>,
    repo_name: String,
    repo_root: PathBuf,
}

impl MockVersionControl {
    pub fn new() -> Self {
        Self {
            staged_files: Vec::new(),
            repo_name: "mock-repo".to_string(),
            repo_root: PathBuf::from("/mock/repo"),
        }
    }

    pub fn with_staged_files(files: Vec<String>) -> Self {
        Self {
            staged_files: files,
            repo_name: "mock-repo".to_string(),
            repo_root: PathBuf::from("/mock/repo"),
        }
    }
}

impl Default for MockVersionControl {
    fn default() -> Self {
        Self::new()
    }
}

impl VersionControl for MockVersionControl {
    fn staged_files(&self) -> anyhow::Result<Vec<String>> {
        Ok(self.staged_files.clone())
    }

    fn repo_name(&self) -> String {
        self.repo_name.clone()
    }

    fn repo_root(&self) -> anyhow::Result<PathBuf> {
        Ok(self.repo_root.clone())
    }

    fn install_hooks(&self, _force: bool) -> anyhow::Result<()> {
        Ok(())
    }

    fn is_inside_repo(&self, path: &Path) -> bool {
        path.starts_with(&self.repo_root)
    }

    fn current_branch(&self) -> anyhow::Result<Option<String>> {
        Ok(Some("main".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_assertion_repo_add_and_list() {
        let repo = MockAssertionRepository::new();
        let id = repo.add("*.rs", "Test message", Severity::Block).unwrap();
        assert!(id.starts_with("MOCK-"));

        let assertions = repo.list().unwrap();
        assert_eq!(assertions.len(), 1);
        assert_eq!(assertions[0].message, "Test message");
    }

    #[test]
    fn test_mock_attestation_store_stage_and_retrieve() {
        let store = MockAttestationStore::new();
        let attestation = Attestation::new(
            "AST-1".to_string(),
            "Test attestation".to_string(),
            "human".to_string(),
        );
        store.stage(&attestation).unwrap();

        let staged = store.staged().unwrap();
        assert_eq!(staged.len(), 1);
        assert_eq!(staged[0].assertion_id, "AST-1");
    }

    #[test]
    fn test_mock_version_control_staged_files() {
        let vcs = MockVersionControl::with_staged_files(vec!["src/main.rs".to_string()]);
        let files = vcs.staged_files().unwrap();
        assert_eq!(files, vec!["src/main.rs"]);
    }
}
