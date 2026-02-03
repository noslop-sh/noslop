//! Mock implementations of port traits for testing
//!
//! These mocks provide configurable behavior for unit testing
//! without real I/O operations.

use noslop::core::models::{Acknowledgment, Check, Severity};
use noslop::core::ports::{AcknowledgmentStore, CheckRepository, VersionControl};
use std::cell::RefCell;
use std::path::{Path, PathBuf};

/// Mock implementation of CheckRepository
pub struct MockCheckRepository {
    checks: RefCell<Vec<Check>>,
}

impl MockCheckRepository {
    pub fn new() -> Self {
        Self {
            checks: RefCell::new(Vec::new()),
        }
    }

    pub fn with_checks(checks: Vec<Check>) -> Self {
        Self {
            checks: RefCell::new(checks),
        }
    }
}

impl Default for MockCheckRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl CheckRepository for MockCheckRepository {
    fn find_for_files(&self, files: &[String]) -> anyhow::Result<Vec<(Check, String)>> {
        let checks = self.checks.borrow();
        let mut results = Vec::new();
        for check in checks.iter() {
            for file in files {
                // Simple matching: check if target matches file extension or exact match
                if file.ends_with(&check.target.trim_start_matches('*'))
                    || check.target == "*"
                    || file == &check.target
                {
                    results.push((check.clone(), file.clone()));
                }
            }
        }
        Ok(results)
    }

    fn add(&self, target: &str, message: &str, severity: Severity) -> anyhow::Result<String> {
        let id = format!("MOCK-{}", self.checks.borrow().len() + 1);
        let check = Check::new(
            Some(id.clone()),
            target.to_string(),
            message.to_string(),
            severity,
        );
        self.checks.borrow_mut().push(check);
        Ok(id)
    }

    fn remove(&self, id: &str) -> anyhow::Result<()> {
        self.checks.borrow_mut().retain(|c| c.id != id);
        Ok(())
    }

    fn list(&self) -> anyhow::Result<Vec<Check>> {
        Ok(self.checks.borrow().clone())
    }
}

/// Mock implementation of AcknowledgmentStore
pub struct MockAckStore {
    staged: RefCell<Vec<Acknowledgment>>,
}

impl MockAckStore {
    pub fn new() -> Self {
        Self {
            staged: RefCell::new(Vec::new()),
        }
    }

    pub fn with_staged(acks: Vec<Acknowledgment>) -> Self {
        Self {
            staged: RefCell::new(acks),
        }
    }
}

impl Default for MockAckStore {
    fn default() -> Self {
        Self::new()
    }
}

impl AcknowledgmentStore for MockAckStore {
    fn stage(&self, ack: &Acknowledgment) -> anyhow::Result<()> {
        self.staged.borrow_mut().push(ack.clone());
        Ok(())
    }

    fn staged(&self) -> anyhow::Result<Vec<Acknowledgment>> {
        Ok(self.staged.borrow().clone())
    }

    fn clear_staged(&self) -> anyhow::Result<()> {
        self.staged.borrow_mut().clear();
        Ok(())
    }

    fn format_trailers(&self, acks: &[Acknowledgment]) -> String {
        acks
            .iter()
            .map(|a| format!("Noslop-Ack: {} | {} | {}", a.check_id, a.message, a.acknowledged_by))
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn parse_from_commit(&self, _commit_sha: &str) -> anyhow::Result<Vec<Acknowledgment>> {
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
    fn test_mock_check_repo_add_and_list() {
        let repo = MockCheckRepository::new();
        let id = repo.add("*.rs", "Test message", Severity::Block).unwrap();
        assert!(id.starts_with("MOCK-"));

        let checks = repo.list().unwrap();
        assert_eq!(checks.len(), 1);
        assert_eq!(checks[0].message, "Test message");
    }

    #[test]
    fn test_mock_ack_store_stage_and_retrieve() {
        let store = MockAckStore::new();
        let ack = Acknowledgment::new(
            "CHK-1".to_string(),
            "Test acknowledgment".to_string(),
            "human".to_string(),
        );
        store.stage(&ack).unwrap();

        let staged = store.staged().unwrap();
        assert_eq!(staged.len(), 1);
        assert_eq!(staged[0].check_id, "CHK-1");
    }

    #[test]
    fn test_mock_version_control_staged_files() {
        let vcs = MockVersionControl::with_staged_files(vec!["src/main.rs".to_string()]);
        let files = vcs.staged_files().unwrap();
        assert_eq!(files, vec!["src/main.rs"]);
    }
}
