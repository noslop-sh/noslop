//! Git-like refs storage for tasks
//!
//! Stores tasks as individual files, like git refs:
//! - .noslop/HEAD - current active task (per-agent)
//! - .noslop/refs/tasks/TSK-1 - task data (shared)
//!
//! Simple, atomic, no complex parsing.
//!
//! MULTI-AGENT SUPPORT:
//! - Task definitions are shared across all agents (in main worktree)
//! - Each agent has its own HEAD (current task)
//! - Tasks can be "claimed" by an agent to prevent double-assignment
//! - The `claimed_by` field stores the agent name that owns the task

use std::fs;

use serde::{Deserialize, Serialize};

use crate::paths;

/// Task data stored in a ref file
///
/// Note: Uses custom deserialization to support migration from old `topic: Option<String>`
/// to new `topics: Vec<String>` format. When reading old files with `topic`, it will
/// be converted to `topics` array.
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
    /// Topics this task belongs to (supports multiple)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub topics: Vec<String>,
    /// Scope patterns (files this task touches)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scope: Vec<String>,
    /// Agent name that has claimed this task.
    /// Set when an agent starts working on the task.
    /// Prevents multiple agents from working on the same task.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claimed_by: Option<String>,
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
    /// New format: array of topics
    #[serde(default)]
    topics: Vec<String>,
    /// Old format: single topic (for backwards compatibility)
    #[serde(default)]
    topic: Option<String>,
    /// Scope patterns (files this task touches)
    #[serde(default)]
    scope: Vec<String>,
    /// Agent that has claimed this task
    #[serde(default)]
    claimed_by: Option<String>,
}

impl<'de> Deserialize<'de> for TaskRef {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let helper = TaskRefHelper::deserialize(deserializer)?;

        // Merge old `topic` into `topics` if present
        let topics = if helper.topics.is_empty() {
            // If topics is empty, check for old single topic field
            helper.topic.into_iter().collect()
        } else {
            helper.topics
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
            topics,
            scope: helper.scope,
            claimed_by: helper.claimed_by,
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
            topics: Vec::new(),
            scope: Vec::new(),
            claimed_by: None,
        }
    }

    /// Create a new task with topics
    #[must_use]
    pub fn with_topics(title: String, topics: Vec<String>) -> Self {
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
            topics,
            scope: Vec::new(),
            claimed_by: None,
        }
    }

    /// Create a new task with a single topic (convenience method)
    #[must_use]
    pub fn with_topic(title: String, topic: Option<String>) -> Self {
        Self::with_topics(title, topic.into_iter().collect())
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

    /// Check if this task belongs to a specific topic
    #[must_use]
    pub fn has_topic(&self, topic_id: &str) -> bool {
        self.topics.contains(&topic_id.to_string())
    }

    /// Get the primary topic (first one, for backwards compatibility)
    #[must_use]
    pub fn primary_topic(&self) -> Option<&str> {
        self.topics.first().map(String::as_str)
    }
}

/// Refs-based task storage
#[derive(Debug, Clone, Copy)]
pub struct TaskRefs;

impl TaskRefs {
    /// Ensure refs directory exists
    fn ensure_refs_dir() -> anyhow::Result<()> {
        fs::create_dir_all(paths::refs_tasks_dir())?;
        Ok(())
    }

    // === HEAD operations ===

    /// Get current active task ID (from HEAD)
    pub fn current() -> anyhow::Result<Option<String>> {
        let path = paths::head_file();
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
        fs::create_dir_all(paths::noslop_dir())?;
        fs::write(paths::head_file(), format!("{id}\n"))?;
        Ok(())
    }

    /// Clear current active task (remove HEAD)
    pub fn clear_current() -> anyhow::Result<()> {
        let path = paths::head_file();
        if path.exists() {
            fs::remove_file(path)?;
        }
        Ok(())
    }

    // === Task ref operations ===

    /// Create a new task, returns the ID
    pub fn create(title: &str, priority: Option<&str>) -> anyhow::Result<String> {
        Self::create_with_topics(title, priority, &[])
    }

    /// Create a new task with multiple topics, returns the ID
    pub fn create_with_topics(
        title: &str,
        priority: Option<&str>,
        topics: &[String],
    ) -> anyhow::Result<String> {
        Self::ensure_refs_dir()?;

        // Generate next ID
        let id = Self::next_id()?;

        let mut task = TaskRef::with_topics(title.to_string(), topics.to_vec());
        if let Some(p) = priority {
            task.priority = p.to_string();
        }

        Self::write_task(&id, &task)?;
        Ok(id)
    }

    /// Create a new task with an optional topic, returns the ID
    #[deprecated(note = "Use create_with_topics instead")]
    pub fn create_with_topic(
        title: &str,
        priority: Option<&str>,
        topic: Option<&str>,
    ) -> anyhow::Result<String> {
        let topics: Vec<String> = topic.into_iter().map(String::from).collect();
        Self::create_with_topics(title, priority, &topics)
    }

    /// Get a task by ID
    pub fn get(id: &str) -> anyhow::Result<Option<TaskRef>> {
        let path = paths::task_ref(id);
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
        let path = paths::task_ref(id);
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

    /// Start a task: claim it, set to `in_progress`, make current, auto-link to branch
    ///
    /// This is the canonical way to start a task. It will:
    /// 1. Check if the task is already claimed by another agent (returns error)
    /// 2. Claim the task for the current agent
    /// 3. Set status to `in_progress`
    /// 4. Set this task as current (HEAD)
    /// 5. Auto-link to current git branch if unlinked
    ///
    /// Returns `Ok(true)` if task was found and started, `Ok(false)` if task not found.
    /// Returns `Err` if the task is already claimed by a different agent.
    pub fn start(id: &str) -> anyhow::Result<bool> {
        Self::start_with_agent(id, None)
    }

    /// Start a task with a specific agent name.
    ///
    /// If `agent_name` is None, uses the current agent's name (derived from worktree).
    pub fn start_with_agent(id: &str, agent_name: Option<&str>) -> anyhow::Result<bool> {
        let Some(mut task) = Self::get(id)? else {
            return Ok(false);
        };

        // Determine agent name
        let current_agent = agent_name.map(String::from).or_else(Self::get_current_agent_name);

        // Check if already claimed by another agent
        if let Some(ref claimed) = task.claimed_by
            && current_agent.as_ref() != Some(claimed)
        {
            anyhow::bail!(
                "Task {id} is already claimed by agent '{claimed}'. \
                 Use 'noslop agent kill {claimed}' to release it first."
            );
        }

        // Claim the task
        task.claimed_by = current_agent;

        // Set status and timestamp
        let now = chrono::Utc::now().to_rfc3339();
        task.status = "in_progress".to_string();
        if task.started_at.is_none() {
            task.started_at = Some(now);
        }

        // Auto-link to current branch if task is unlinked
        if task.branch.is_none()
            && let Some(branch) = Self::get_current_git_branch()
        {
            task.branch = Some(branch);
        }

        Self::write_task(id, &task)?;
        Self::set_current(id)?;

        Ok(true)
    }

    /// Get the current agent name (derived from worktree path or main).
    fn get_current_agent_name() -> Option<String> {
        let agent_root = paths::agent_root();
        let agents_dir = paths::agents_dir();

        // Check if we're in an agent worktree
        if let Ok(agent_root_canonical) = agent_root.canonicalize()
            && let Ok(agents_dir_canonical) = agents_dir.canonicalize()
            && let Ok(relative) = agent_root_canonical.strip_prefix(&agents_dir_canonical)
            && let Some(name) = relative.components().next()
        {
            return Some(name.as_os_str().to_string_lossy().to_string());
        }

        // We're in the main worktree
        if !paths::is_agent() {
            return Some("main".to_string());
        }

        None
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

    /// Complete a task: set to done, release claim, clear current if needed
    /// This is the canonical way to complete a task
    pub fn complete(id: &str) -> anyhow::Result<bool> {
        let Some(mut task) = Self::get(id)? else {
            return Ok(false);
        };

        // Set status and timestamp
        let now = chrono::Utc::now().to_rfc3339();
        task.status = "done".to_string();
        if task.completed_at.is_none() {
            task.completed_at = Some(now);
        }

        // Release the claim
        task.claimed_by = None;

        Self::write_task(id, &task)?;

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

    // === Task claiming operations ===

    /// Claim a task for an agent without starting it.
    ///
    /// Useful for reserving a task when spawning an agent.
    /// Returns `Err` if already claimed by a different agent.
    pub fn claim(id: &str, agent_name: &str) -> anyhow::Result<bool> {
        let Some(mut task) = Self::get(id)? else {
            return Ok(false);
        };

        if let Some(ref claimed) = task.claimed_by {
            if claimed != agent_name {
                anyhow::bail!("Task {id} is already claimed by agent '{claimed}'");
            }
            return Ok(true); // Already claimed by this agent
        }

        task.claimed_by = Some(agent_name.to_string());
        Self::write_task(id, &task)?;
        Ok(true)
    }

    /// Release claim on a task without completing it.
    ///
    /// Used when an agent is stopped or killed.
    pub fn release(id: &str) -> anyhow::Result<bool> {
        let Some(mut task) = Self::get(id)? else {
            return Ok(false);
        };

        task.claimed_by = None;
        Self::write_task(id, &task)?;
        Ok(true)
    }

    /// Check if a task is claimed by any agent.
    pub fn is_claimed(id: &str) -> anyhow::Result<bool> {
        Ok(Self::get(id)?.is_some_and(|t| t.claimed_by.is_some()))
    }

    /// Get the agent name that has claimed a task.
    pub fn get_claimed_by(id: &str) -> anyhow::Result<Option<String>> {
        Ok(Self::get(id)?.and_then(|t| t.claimed_by))
    }

    /// List all tasks claimed by a specific agent.
    pub fn list_by_agent(agent_name: &str) -> anyhow::Result<Vec<(String, TaskRef)>> {
        Ok(Self::list()?
            .into_iter()
            .filter(|(_, t)| t.claimed_by.as_deref() == Some(agent_name))
            .collect())
    }

    /// List all unclaimed tasks (available for agents to pick up).
    pub fn list_unclaimed() -> anyhow::Result<Vec<(String, TaskRef)>> {
        Ok(Self::list()?
            .into_iter()
            .filter(|(_, t)| t.claimed_by.is_none() && t.status != "done")
            .collect())
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

    /// Set a task's topics (replaces all existing topics)
    pub fn set_topics(id: &str, topics: &[&str]) -> anyhow::Result<bool> {
        if let Some(mut task) = Self::get(id)? {
            task.topics = topics.iter().map(|s| (*s).to_string()).collect();
            Self::write_task(id, &task)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Add a topic to a task
    pub fn add_topic(id: &str, topic: &str) -> anyhow::Result<bool> {
        if let Some(mut task) = Self::get(id)? {
            if !task.topics.contains(&topic.to_string()) {
                task.topics.push(topic.to_string());
                Self::write_task(id, &task)?;
            }
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Remove a topic from a task
    pub fn remove_topic(id: &str, topic: &str) -> anyhow::Result<bool> {
        if let Some(mut task) = Self::get(id)? {
            task.topics.retain(|c| c != topic);
            Self::write_task(id, &task)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Set or unset a task's topic (backwards compatible - sets single topic)
    #[deprecated(note = "Use set_topics, add_topic, or remove_topic instead")]
    pub fn set_topic(id: &str, topic: Option<&str>) -> anyhow::Result<bool> {
        if let Some(mut task) = Self::get(id)? {
            task.topics = topic.into_iter().map(String::from).collect();
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

    /// List tasks filtered by topic (tasks containing this topic)
    pub fn list_by_topic(topic: Option<&str>) -> anyhow::Result<Vec<(String, TaskRef)>> {
        let tasks = Self::list()?;
        Ok(tasks
            .into_iter()
            .filter(|(_, task)| topic.map_or(task.topics.is_empty(), |c| task.has_topic(c)))
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
        let path = paths::task_ref(id);
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
        let refs_dir = paths::refs_tasks_dir();
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

        // Find pending tasks that are not blocked and not claimed
        let mut candidates: Vec<_> = tasks
            .iter()
            .filter(|(_, task)| {
                task.status == "pending" && !task.is_blocked(&tasks) && task.claimed_by.is_none()
            })
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
        let path = paths::noslop_toml();
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
    #[serial(cwd)]
    fn test_create_and_get_task() {
        let _temp = setup();

        let id = TaskRefs::create("Test task", None).unwrap();
        assert!(id.starts_with("TSK-"));

        let task = TaskRefs::get(&id).unwrap().unwrap();
        assert_eq!(task.title, "Test task");
        assert_eq!(task.status, "backlog");
    }

    #[test]
    #[serial(cwd)]
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
    #[serial(cwd)]
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
    #[serial(cwd)]
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
    #[serial(cwd)]
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
