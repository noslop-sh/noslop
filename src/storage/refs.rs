//! Git-like refs storage for tasks
//!
//! Stores tasks as individual files, like git refs:
//! - .noslop/HEAD - current active task
//! - .noslop/refs/tasks/TSK-1 - task data
//!
//! Simple, atomic, no complex parsing.
//!
//! WORKTREE SUPPORT:
//! When running from a git worktree, .noslop/ is always resolved
//! relative to the MAIN worktree, not the linked one. This ensures
//! all worktrees share the same task state.

use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::git;

const NOSLOP_DIR: &str = ".noslop";
const HEAD_FILE: &str = "HEAD";
const REFS_DIR: &str = "refs/tasks";

/// Task data stored in a ref file
///
/// Note: Uses custom deserialization to support migration from old `concept: Option<String>`
/// to new `concepts: Vec<String>` format. When reading old files with `concept`, it will
/// be converted to `concepts` array.
#[derive(Debug, Clone, Serialize)]
pub struct TaskRef {
    /// Task title
    pub title: String,
    /// Task description (detailed context for LLMs)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Current status: backlog, pending, `in_progress`, done
    pub status: String,
    /// Priority: p0, p1, p2, p3
    pub priority: String,
    /// When created (RFC3339)
    pub created_at: String,
    /// Optional notes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    /// IDs of tasks that block this one
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub blocked_by: Vec<String>,
    /// Pending trailer action (set by pre-commit, cleared by post-commit)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pending_trailer: Option<String>,
    /// Optional associated git branch
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    /// When work started (RFC3339)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<String>,
    /// When completed (RFC3339)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<String>,
    /// Concepts this task belongs to (supports multiple)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub concepts: Vec<String>,
    /// Scope patterns (files this task touches)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scope: Vec<String>,
}

/// Helper struct for deserializing with backwards compatibility
#[derive(Deserialize)]
struct TaskRefHelper {
    title: String,
    #[serde(default)]
    description: Option<String>,
    status: String,
    #[serde(default = "default_priority")]
    priority: String,
    created_at: String,
    #[serde(default)]
    notes: Option<String>,
    #[serde(default)]
    blocked_by: Vec<String>,
    #[serde(default)]
    pending_trailer: Option<String>,
    #[serde(default)]
    branch: Option<String>,
    #[serde(default)]
    started_at: Option<String>,
    #[serde(default)]
    completed_at: Option<String>,
    /// New format: array of concepts
    #[serde(default)]
    concepts: Vec<String>,
    /// Old format: single concept (for backwards compatibility)
    #[serde(default)]
    concept: Option<String>,
    /// Scope patterns (files this task touches)
    #[serde(default)]
    scope: Vec<String>,
}

impl<'de> Deserialize<'de> for TaskRef {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let helper = TaskRefHelper::deserialize(deserializer)?;

        // Merge old `concept` into `concepts` if present
        let concepts = if helper.concepts.is_empty() {
            // If concepts is empty, check for old single concept field
            helper.concept.into_iter().collect()
        } else {
            helper.concepts
        };

        Ok(Self {
            title: helper.title,
            description: helper.description,
            status: helper.status,
            priority: helper.priority,
            created_at: helper.created_at,
            notes: helper.notes,
            blocked_by: helper.blocked_by,
            pending_trailer: helper.pending_trailer,
            branch: helper.branch,
            started_at: helper.started_at,
            completed_at: helper.completed_at,
            concepts,
            scope: helper.scope,
        })
    }
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
            description: None,
            status: "backlog".to_string(),
            priority: default_priority(),
            created_at: chrono::Utc::now().to_rfc3339(),
            notes: None,
            blocked_by: Vec::new(),
            pending_trailer: None,
            branch: None,
            started_at: None,
            completed_at: None,
            concepts: Vec::new(),
            scope: Vec::new(),
        }
    }

    /// Create a new task with concepts
    #[must_use]
    pub fn with_concepts(title: String, concepts: Vec<String>) -> Self {
        Self {
            title,
            description: None,
            status: "backlog".to_string(),
            priority: default_priority(),
            created_at: chrono::Utc::now().to_rfc3339(),
            notes: None,
            blocked_by: Vec::new(),
            pending_trailer: None,
            branch: None,
            started_at: None,
            completed_at: None,
            concepts,
            scope: Vec::new(),
        }
    }

    /// Create a new task with a single concept (convenience method)
    #[must_use]
    pub fn with_concept(title: String, concept: Option<String>) -> Self {
        Self::with_concepts(title, concept.into_iter().collect())
    }

    /// Check if this task is blocked by unfinished tasks
    #[must_use]
    pub fn is_blocked(&self, all_tasks: &[(String, Self)]) -> bool {
        self.blocked_by.iter().any(|blocker_id| {
            all_tasks
                .iter()
                .find(|(id, _)| id == blocker_id)
                .is_some_and(|(_, task)| task.status != "done")
        })
    }

    /// Check if this task belongs to a specific concept
    #[must_use]
    pub fn has_concept(&self, concept_id: &str) -> bool {
        self.concepts.contains(&concept_id.to_string())
    }

    /// Get the primary concept (first one, for backwards compatibility)
    #[must_use]
    pub fn primary_concept(&self) -> Option<&str> {
        self.concepts.first().map(String::as_str)
    }
}

/// Refs-based task storage
#[derive(Debug, Clone, Copy)]
pub struct TaskRefs;

impl TaskRefs {
    /// Get path to .noslop directory
    ///
    /// Returns the .noslop directory in the main worktree.
    /// This ensures all worktrees share the same task state.
    fn noslop_dir() -> PathBuf {
        git::get_main_worktree()
            .map_or_else(|| PathBuf::from(NOSLOP_DIR), |root| root.join(NOSLOP_DIR))
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
        Self::create_with_concepts(title, priority, &[])
    }

    /// Create a new task with multiple concepts, returns the ID
    pub fn create_with_concepts(
        title: &str,
        priority: Option<&str>,
        concepts: &[String],
    ) -> anyhow::Result<String> {
        Self::ensure_refs_dir()?;

        // Generate next ID
        let id = Self::next_id()?;

        let mut task = TaskRef::with_concepts(title.to_string(), concepts.to_vec());
        if let Some(p) = priority {
            task.priority = p.to_string();
        }

        Self::write_task(&id, &task)?;
        Ok(id)
    }

    /// Create a new task with an optional concept, returns the ID
    #[deprecated(note = "Use create_with_concepts instead")]
    pub fn create_with_concept(
        title: &str,
        priority: Option<&str>,
        concept: Option<&str>,
    ) -> anyhow::Result<String> {
        let concepts: Vec<String> = concept.into_iter().map(String::from).collect();
        Self::create_with_concepts(title, priority, &concepts)
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

    /// Update a task's status (also sets timestamps)
    pub fn set_status(id: &str, status: &str) -> anyhow::Result<bool> {
        if let Some(mut task) = Self::get(id)? {
            let now = chrono::Utc::now().to_rfc3339();
            task.status = status.to_string();

            // Set timestamps based on status transition
            if status == "in_progress" && task.started_at.is_none() {
                task.started_at = Some(now);
            } else if status == "done" && task.completed_at.is_none() {
                task.completed_at = Some(now);
            }

            Self::write_task(id, &task)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Start a task: set to `in_progress`, make current, auto-link to branch if unlinked
    /// This is the canonical way to start a task - use this instead of `set_status` + `set_current`
    /// Returns `Ok(true)` if task was found and started, `Ok(false)` if task not found.
    /// The optional branch name is returned separately via `get()` if needed.
    pub fn start(id: &str) -> anyhow::Result<bool> {
        let Some(task) = Self::get(id)? else {
            return Ok(false);
        };

        // Set status and current
        Self::set_status(id, "in_progress")?;
        Self::set_current(id)?;

        // Auto-link to current branch if task is unlinked
        if task.branch.is_none()
            && let Some(branch) = Self::get_current_git_branch()
        {
            Self::link_branch(id, Some(&branch))?;
        }

        Ok(true)
    }

    /// Move task to backlog: set status, unlink branch, clear current if needed
    /// This is the canonical way to move to backlog
    pub fn move_to_backlog(id: &str) -> anyhow::Result<bool> {
        if Self::get(id)?.is_none() {
            return Ok(false);
        }

        Self::set_status(id, "backlog")?;
        Self::link_branch(id, None)?;

        // Clear current if this was the current task
        if Self::current()?.as_deref() == Some(id) {
            Self::clear_current()?;
        }

        Ok(true)
    }

    /// Complete a task: set to done, clear current if needed
    /// This is the canonical way to complete a task
    pub fn complete(id: &str) -> anyhow::Result<bool> {
        if Self::get(id)?.is_none() {
            return Ok(false);
        }

        Self::set_status(id, "done")?;

        // Clear current if this was the current task
        if Self::current()?.as_deref() == Some(id) {
            Self::clear_current()?;
        }

        Ok(true)
    }

    /// Get current git branch name
    fn get_current_git_branch() -> Option<String> {
        let repo = git2::Repository::discover(".").ok()?;
        let head = repo.head().ok()?;
        head.shorthand().map(String::from)
    }

    /// Link or unlink a task to a git branch
    pub fn link_branch(id: &str, branch: Option<&str>) -> anyhow::Result<bool> {
        if let Some(mut task) = Self::get(id)? {
            task.branch = branch.map(String::from);
            Self::write_task(id, &task)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Set a task's concepts (replaces all existing concepts)
    pub fn set_concepts(id: &str, concepts: &[&str]) -> anyhow::Result<bool> {
        if let Some(mut task) = Self::get(id)? {
            task.concepts = concepts.iter().map(|s| (*s).to_string()).collect();
            Self::write_task(id, &task)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Add a concept to a task
    pub fn add_concept(id: &str, concept: &str) -> anyhow::Result<bool> {
        if let Some(mut task) = Self::get(id)? {
            if !task.concepts.contains(&concept.to_string()) {
                task.concepts.push(concept.to_string());
                Self::write_task(id, &task)?;
            }
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Remove a concept from a task
    pub fn remove_concept(id: &str, concept: &str) -> anyhow::Result<bool> {
        if let Some(mut task) = Self::get(id)? {
            task.concepts.retain(|c| c != concept);
            Self::write_task(id, &task)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Set or unset a task's concept (backwards compatible - sets single concept)
    #[deprecated(note = "Use set_concepts, add_concept, or remove_concept instead")]
    pub fn set_concept(id: &str, concept: Option<&str>) -> anyhow::Result<bool> {
        if let Some(mut task) = Self::get(id)? {
            task.concepts = concept.into_iter().map(String::from).collect();
            Self::write_task(id, &task)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Set or unset a task's description
    pub fn set_description(id: &str, description: Option<&str>) -> anyhow::Result<bool> {
        if let Some(mut task) = Self::get(id)? {
            task.description = description.map(String::from);
            Self::write_task(id, &task)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    // === Scope operations ===

    /// Set a task's scope patterns (replaces all existing)
    pub fn set_scope(id: &str, scope: &[&str]) -> anyhow::Result<bool> {
        if let Some(mut task) = Self::get(id)? {
            task.scope = scope.iter().map(|s| (*s).to_string()).collect();
            Self::write_task(id, &task)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Add a scope pattern to a task
    pub fn add_scope(id: &str, pattern: &str) -> anyhow::Result<bool> {
        if let Some(mut task) = Self::get(id)? {
            if !task.scope.contains(&pattern.to_string()) {
                task.scope.push(pattern.to_string());
                Self::write_task(id, &task)?;
            }
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Remove a scope pattern from a task
    pub fn remove_scope(id: &str, pattern: &str) -> anyhow::Result<bool> {
        if let Some(mut task) = Self::get(id)? {
            task.scope.retain(|s| s != pattern);
            Self::write_task(id, &task)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// List tasks filtered by concept (tasks containing this concept)
    pub fn list_by_concept(concept: Option<&str>) -> anyhow::Result<Vec<(String, TaskRef)>> {
        let tasks = Self::list()?;
        Ok(tasks
            .into_iter()
            .filter(|(_, task)| concept.map_or(task.concepts.is_empty(), |c| task.has_concept(c)))
            .collect())
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

    /// Get next pending unblocked task (sorted by priority then id)
    pub fn next_pending_unblocked() -> anyhow::Result<Option<(String, TaskRef)>> {
        let tasks = Self::list()?;

        // Find pending tasks that are not blocked
        let mut candidates: Vec<_> = tasks
            .iter()
            .filter(|(_, task)| task.status == "pending" && !task.is_blocked(&tasks))
            .cloned()
            .collect();

        if candidates.is_empty() {
            return Ok(None);
        }

        // Sort by priority (p0 first), then by id
        candidates.sort_by(|(id_a, a), (id_b, b)| {
            a.priority.cmp(&b.priority).then_with(|| id_a.cmp(id_b))
        });

        Ok(candidates.into_iter().next())
    }

    /// List all pending unblocked tasks
    pub fn list_pending_unblocked() -> anyhow::Result<Vec<(String, TaskRef)>> {
        let tasks = Self::list()?;
        Ok(tasks
            .iter()
            .filter(|(_, task)| task.status == "pending" && !task.is_blocked(&tasks))
            .cloned()
            .collect())
    }

    /// Add a blocker to a task
    pub fn add_blocker(id: &str, blocker_id: &str) -> anyhow::Result<bool> {
        if let Some(mut task) = Self::get(id)? {
            if !task.blocked_by.contains(&blocker_id.to_string()) {
                task.blocked_by.push(blocker_id.to_string());
                Self::write_task(id, &task)?;
            }
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Remove a blocker from a task
    pub fn remove_blocker(id: &str, blocker_id: &str) -> anyhow::Result<bool> {
        if let Some(mut task) = Self::get(id)? {
            task.blocked_by.retain(|b| b != blocker_id);
            Self::write_task(id, &task)?;
            Ok(true)
        } else {
            Ok(false)
        }
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
    ///
    /// Looks for .noslop.toml in the main worktree to ensure
    /// consistent prefixes across all worktrees.
    fn get_prefix() -> anyhow::Result<String> {
        let path = git::get_main_worktree()
            .map_or_else(|| PathBuf::from(".noslop.toml"), |root| root.join(".noslop.toml"));
        if path.exists() {
            let content = fs::read_to_string(&path)?;
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
        assert_eq!(task.status, "backlog");
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

    #[test]
    #[serial]
    fn test_link_branch() {
        let _temp = setup();

        let id = TaskRefs::create("Test", None).unwrap();

        // Initially no branch
        let task = TaskRefs::get(&id).unwrap().unwrap();
        assert!(task.branch.is_none());

        // Link to branch
        TaskRefs::link_branch(&id, Some("feature/foo")).unwrap();
        let task = TaskRefs::get(&id).unwrap().unwrap();
        assert_eq!(task.branch, Some("feature/foo".to_string()));

        // Unlink (set to None)
        TaskRefs::link_branch(&id, None).unwrap();
        let task = TaskRefs::get(&id).unwrap().unwrap();
        assert!(task.branch.is_none());
    }

    #[test]
    #[serial]
    fn test_status_timestamps() {
        let _temp = setup();

        let id = TaskRefs::create("Test", None).unwrap();

        // Initially no timestamps
        let task = TaskRefs::get(&id).unwrap().unwrap();
        assert!(task.started_at.is_none());
        assert!(task.completed_at.is_none());

        // Start task - sets started_at
        TaskRefs::set_status(&id, "in_progress").unwrap();
        let task = TaskRefs::get(&id).unwrap().unwrap();
        assert!(task.started_at.is_some());
        assert!(task.completed_at.is_none());
        let started = task.started_at;

        // Complete task - sets completed_at, keeps started_at
        TaskRefs::set_status(&id, "done").unwrap();
        let task = TaskRefs::get(&id).unwrap().unwrap();
        assert_eq!(task.started_at, started); // unchanged
        assert!(task.completed_at.is_some());

        // Setting in_progress again doesn't change started_at
        TaskRefs::set_status(&id, "in_progress").unwrap();
        let task = TaskRefs::get(&id).unwrap().unwrap();
        assert_eq!(task.started_at, started); // still unchanged
    }
}
