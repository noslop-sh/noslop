//! Git-like refs storage for tasks
//!
//! Stores tasks as individual files, like git refs:
//! - .noslop/HEAD - current active task
//! - .noslop/refs/tasks/TSK-1 - task data
//!
//! Simple, atomic, no complex parsing.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

const NOSLOP_DIR: &str = ".noslop";
const HEAD_FILE: &str = "HEAD";
const REFS_DIR: &str = "refs/tasks";

/// Task data stored in a ref file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRef {
    /// Task title
    pub title: String,
    /// Current status: pending, `in_progress`, done
    pub status: String,
    /// Priority: p0, p1, p2, p3
    #[serde(default = "default_priority")]
    pub priority: String,
    /// When created (RFC3339)
    pub created_at: String,
    /// Optional notes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    /// Pending trailer action (set by pre-commit, cleared by post-commit)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pending_trailer: Option<String>,
}

fn default_priority() -> String {
    "p1".to_string()
}

impl TaskRef {
    /// Create a new task
    #[must_use]
    pub fn new(title: String) -> Self {
        Self {
            title,
            status: "pending".to_string(),
            priority: default_priority(),
            created_at: chrono::Utc::now().to_rfc3339(),
            notes: None,
            pending_trailer: None,
        }
    }
}

/// Refs-based task storage
#[derive(Debug, Clone, Copy)]
pub struct TaskRefs;

impl TaskRefs {
    /// Get path to .noslop directory
    fn noslop_dir() -> PathBuf {
        PathBuf::from(NOSLOP_DIR)
    }

    /// Get path to HEAD file
    fn head_path() -> PathBuf {
        Self::noslop_dir().join(HEAD_FILE)
    }

    /// Get path to refs/tasks directory
    fn refs_dir() -> PathBuf {
        Self::noslop_dir().join(REFS_DIR)
    }

    /// Get path to a task ref file
    fn task_path(id: &str) -> PathBuf {
        Self::refs_dir().join(id)
    }

    /// Ensure refs directory exists
    fn ensure_refs_dir() -> anyhow::Result<()> {
        fs::create_dir_all(Self::refs_dir())?;
        Ok(())
    }

    // === HEAD operations ===

    /// Get current active task ID (from HEAD)
    pub fn current() -> anyhow::Result<Option<String>> {
        let path = Self::head_path();
        if !path.exists() {
            return Ok(None);
        }
        let content = fs::read_to_string(&path)?;
        let id = content.trim();
        if id.is_empty() {
            Ok(None)
        } else {
            Ok(Some(id.to_string()))
        }
    }

    /// Set current active task (write to HEAD)
    pub fn set_current(id: &str) -> anyhow::Result<()> {
        fs::create_dir_all(Self::noslop_dir())?;
        fs::write(Self::head_path(), format!("{id}\n"))?;
        Ok(())
    }

    /// Clear current active task (remove HEAD)
    pub fn clear_current() -> anyhow::Result<()> {
        let path = Self::head_path();
        if path.exists() {
            fs::remove_file(path)?;
        }
        Ok(())
    }

    // === Task ref operations ===

    /// Create a new task, returns the ID
    pub fn create(title: &str, priority: Option<&str>) -> anyhow::Result<String> {
        Self::ensure_refs_dir()?;

        // Generate next ID
        let id = Self::next_id()?;

        let mut task = TaskRef::new(title.to_string());
        if let Some(p) = priority {
            task.priority = p.to_string();
        }

        Self::write_task(&id, &task)?;
        Ok(id)
    }

    /// Get a task by ID
    pub fn get(id: &str) -> anyhow::Result<Option<TaskRef>> {
        let path = Self::task_path(id);
        if !path.exists() {
            return Ok(None);
        }
        let content = fs::read_to_string(&path)?;
        let task: TaskRef = serde_json::from_str(&content)?;
        Ok(Some(task))
    }

    /// Write a task to its ref file
    pub fn write_task(id: &str, task: &TaskRef) -> anyhow::Result<()> {
        Self::ensure_refs_dir()?;
        let path = Self::task_path(id);
        let json = serde_json::to_string_pretty(task)?;
        fs::write(path, json)?;
        Ok(())
    }

    /// Update a task's status
    pub fn set_status(id: &str, status: &str) -> anyhow::Result<bool> {
        if let Some(mut task) = Self::get(id)? {
            task.status = status.to_string();
            Self::write_task(id, &task)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Set pending trailer action (for commit-msg hook)
    pub fn set_pending_trailer(id: &str, action: &str) -> anyhow::Result<bool> {
        if let Some(mut task) = Self::get(id)? {
            task.pending_trailer = Some(action.to_string());
            Self::write_task(id, &task)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Clear pending trailer (after commit)
    pub fn clear_pending_trailer(id: &str) -> anyhow::Result<bool> {
        if let Some(mut task) = Self::get(id)? {
            task.pending_trailer = None;
            Self::write_task(id, &task)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Delete a task
    pub fn delete(id: &str) -> anyhow::Result<bool> {
        let path = Self::task_path(id);
        if path.exists() {
            fs::remove_file(path)?;
            // If this was the current task, clear HEAD
            if Self::current()?.as_deref() == Some(id) {
                Self::clear_current()?;
            }
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// List all task IDs
    pub fn list_ids() -> anyhow::Result<Vec<String>> {
        let refs_dir = Self::refs_dir();
        if !refs_dir.exists() {
            return Ok(Vec::new());
        }

        let mut ids = Vec::new();
        for entry in fs::read_dir(refs_dir)? {
            let entry = entry?;
            if let Some(name) = entry.file_name().to_str() {
                ids.push(name.to_string());
            }
        }
        ids.sort();
        Ok(ids)
    }

    /// List all tasks
    pub fn list() -> anyhow::Result<Vec<(String, TaskRef)>> {
        let ids = Self::list_ids()?;
        let mut tasks = Vec::new();
        for id in ids {
            if let Some(task) = Self::get(&id)? {
                tasks.push((id, task));
            }
        }
        Ok(tasks)
    }

    /// List tasks with pending trailers
    pub fn list_pending_trailers() -> anyhow::Result<Vec<(String, TaskRef)>> {
        Ok(Self::list()?.into_iter().filter(|(_, t)| t.pending_trailer.is_some()).collect())
    }

    /// Clear all pending trailers
    pub fn clear_all_pending_trailers() -> anyhow::Result<()> {
        for (id, _) in Self::list_pending_trailers()? {
            Self::clear_pending_trailer(&id)?;
        }
        Ok(())
    }

    /// Generate next task ID
    fn next_id() -> anyhow::Result<String> {
        let prefix = Self::get_prefix()?;
        let ids = Self::list_ids()?;

        // Find max existing number
        let max_num = ids
            .iter()
            .filter_map(|id| {
                id.strip_prefix(&prefix)
                    .and_then(|s| s.strip_prefix('-'))
                    .and_then(|n| n.parse::<u32>().ok())
            })
            .max()
            .unwrap_or(0);

        Ok(format!("{}-{}", prefix, max_num + 1))
    }

    /// Get project prefix from .noslop.toml
    fn get_prefix() -> anyhow::Result<String> {
        let path = Path::new(".noslop.toml");
        if path.exists() {
            let content = fs::read_to_string(path)?;
            // Simple extraction - look for prefix = "..."
            for line in content.lines() {
                let line = line.trim();
                if line.starts_with("prefix")
                    && let Some(value) = line.split('=').nth(1)
                {
                    let value = value.trim().trim_matches('"');
                    if !value.is_empty() && value != "NOS" {
                        return Ok(value.to_string());
                    }
                }
            }
        }
        Ok("TSK".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use tempfile::TempDir;

    fn setup() -> TempDir {
        let temp = TempDir::new().unwrap();
        std::env::set_current_dir(temp.path()).unwrap();
        temp
    }

    #[test]
    #[serial]
    fn test_create_and_get_task() {
        let _temp = setup();

        let id = TaskRefs::create("Test task", None).unwrap();
        assert!(id.starts_with("TSK-"));

        let task = TaskRefs::get(&id).unwrap().unwrap();
        assert_eq!(task.title, "Test task");
        assert_eq!(task.status, "pending");
    }

    #[test]
    #[serial]
    fn test_head_operations() {
        let _temp = setup();

        // No current task initially
        assert!(TaskRefs::current().unwrap().is_none());

        // Set current
        TaskRefs::set_current("TSK-1").unwrap();
        assert_eq!(TaskRefs::current().unwrap(), Some("TSK-1".to_string()));

        // Clear current
        TaskRefs::clear_current().unwrap();
        assert!(TaskRefs::current().unwrap().is_none());
    }

    #[test]
    #[serial]
    fn test_pending_trailer() {
        let _temp = setup();

        let id = TaskRefs::create("Test", None).unwrap();

        // Set pending trailer
        TaskRefs::set_pending_trailer(&id, "done").unwrap();
        let task = TaskRefs::get(&id).unwrap().unwrap();
        assert_eq!(task.pending_trailer, Some("done".to_string()));

        // Clear it
        TaskRefs::clear_pending_trailer(&id).unwrap();
        let task = TaskRefs::get(&id).unwrap().unwrap();
        assert!(task.pending_trailer.is_none());
    }
}
