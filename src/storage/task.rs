//! Task storage
//!
//! Stores tasks in branch-scoped TOML files: .noslop/tasks/{branch}.toml
//! Task files are gitignored (local scratch), while completions are recorded in commit trailers.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};

use crate::models::{Priority, Task, TaskStatus};

/// Get the current git branch name
fn get_current_branch() -> Option<String> {
    Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty() && s != "HEAD") // HEAD means detached state
}

/// Convert branch name to safe filename
/// e.g., "feature/oauth" -> "feature-oauth"
fn branch_to_filename(branch: &str) -> String {
    branch
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .to_lowercase()
}

const TASKS_DIR: &str = ".noslop/tasks";

/// Branch task file structure
#[derive(Debug, Serialize, Deserialize)]
pub struct BranchTaskFile {
    /// Branch metadata
    pub branch: BranchMeta,

    /// Tasks for this branch
    #[serde(default, rename = "task")]
    pub tasks: Vec<TaskEntry>,
}

/// Branch metadata
#[derive(Debug, Serialize, Deserialize)]
pub struct BranchMeta {
    /// Branch name
    pub name: String,

    /// When the task file was created
    pub created_at: String,

    /// Optional notes for this branch's work
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

/// Task entry in TOML (serialization format)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskEntry {
    /// Task ID (numeric, branch-local)
    pub id: u32,
    /// Task title
    pub title: String,
    /// Status: pending, in_progress, done
    #[serde(default = "default_status")]
    pub status: String,
    /// Priority: p0-p3
    #[serde(default = "default_priority")]
    pub priority: String,
    /// IDs of tasks that block this one
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blocked_by: Vec<u32>,
    /// Optional notes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    /// When created (RFC3339)
    pub created_at: String,
}

fn default_status() -> String {
    "pending".to_string()
}

fn default_priority() -> String {
    "p1".to_string()
}

impl TaskEntry {
    /// Convert to Task model with branch-specific ID prefix
    pub fn to_task(&self, prefix: &str) -> Task {
        Task {
            id: format!("{}-{}", prefix, self.id),
            title: self.title.clone(),
            status: self.status.parse().unwrap_or(TaskStatus::Pending),
            priority: self.priority.parse().unwrap_or(Priority::P1),
            blocked_by: self
                .blocked_by
                .iter()
                .map(|id| format!("{}-{}", prefix, id))
                .collect(),
            notes: self.notes.clone(),
            created_at: self.created_at.clone(),
        }
    }

    /// Create from Task model, extracting numeric ID
    pub fn from_task(task: &Task) -> Self {
        let numeric_id = task
            .id
            .rsplit('-')
            .next()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        let blocked_by: Vec<u32> = task
            .blocked_by
            .iter()
            .filter_map(|id| id.rsplit('-').next().and_then(|s| s.parse().ok()))
            .collect();

        Self {
            id: numeric_id,
            title: task.title.clone(),
            status: task.status.to_string(),
            priority: task.priority.to_string(),
            blocked_by,
            notes: task.notes.clone(),
            created_at: task.created_at.clone(),
        }
    }
}

/// Minimal config structure just to read the prefix
#[derive(Debug, Deserialize)]
struct NoslopConfig {
    #[serde(default)]
    project: ProjectConfig,
}

#[derive(Debug, Deserialize)]
struct ProjectConfig {
    #[serde(default = "default_prefix")]
    prefix: String,
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            prefix: default_prefix(),
        }
    }
}

fn default_prefix() -> String {
    "TSK".to_string()
}

/// Task storage operations (branch-scoped)
#[derive(Debug, Clone, Copy)]
pub struct TaskStore;

impl TaskStore {
    /// Get the current branch name, or error if not in a git repo
    pub fn current_branch() -> anyhow::Result<String> {
        get_current_branch().ok_or_else(|| anyhow::anyhow!("Not on a git branch"))
    }

    /// Get path to task file for a branch
    fn task_file_path(branch: &str) -> PathBuf {
        let filename = branch_to_filename(branch);
        PathBuf::from(TASKS_DIR).join(format!("{}.toml", filename))
    }

    /// Check if task file exists for current branch
    pub fn has_task_file() -> bool {
        get_current_branch()
            .map(|b| Self::task_file_path(&b).exists())
            .unwrap_or(false)
    }

    /// Initialize task file for current branch
    pub fn init_branch(notes: Option<&str>) -> anyhow::Result<PathBuf> {
        let branch = Self::current_branch()?;
        let path = Self::task_file_path(&branch);

        if path.exists() {
            anyhow::bail!("Task file already exists: {}", path.display());
        }

        // Ensure directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let file = BranchTaskFile {
            branch: BranchMeta {
                name: branch,
                created_at: chrono::Utc::now().to_rfc3339(),
                notes: notes.map(String::from),
            },
            tasks: Vec::new(),
        };

        let content = toml::to_string_pretty(&file)?;
        fs::write(&path, content)?;

        Ok(path)
    }

    /// Load task file for current branch
    fn load_file() -> anyhow::Result<BranchTaskFile> {
        let branch = Self::current_branch()?;
        let path = Self::task_file_path(&branch);

        if !path.exists() {
            // Return empty file structure
            return Ok(BranchTaskFile {
                branch: BranchMeta {
                    name: branch,
                    created_at: chrono::Utc::now().to_rfc3339(),
                    notes: None,
                },
                tasks: Vec::new(),
            });
        }

        let content = fs::read_to_string(&path)?;
        Ok(toml::from_str(&content)?)
    }

    /// Save task file for current branch
    fn save_file(file: &BranchTaskFile) -> anyhow::Result<()> {
        let path = Self::task_file_path(&file.branch.name);

        // Ensure directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = toml::to_string_pretty(file)?;
        fs::write(&path, content)?;
        Ok(())
    }

    /// Load all tasks from current branch
    pub fn load() -> anyhow::Result<Vec<Task>> {
        let file = Self::load_file()?;
        let prefix = Self::get_prefix()?;

        Ok(file.tasks.iter().map(|t| t.to_task(&prefix)).collect())
    }

    /// Save all tasks to current branch
    pub fn save(tasks: &[Task]) -> anyhow::Result<()> {
        let mut file = Self::load_file()?;
        file.tasks = tasks.iter().map(TaskEntry::from_task).collect();
        Self::save_file(&file)
    }

    /// Add a new task
    pub fn add(task: Task) -> anyhow::Result<()> {
        let mut tasks = Self::load()?;
        tasks.push(task);
        Self::save(&tasks)
    }

    /// Get a task by ID
    #[must_use]
    pub fn get(id: &str) -> Option<Task> {
        Self::load().ok()?.into_iter().find(|t| t.id == id)
    }

    /// Update a task's status
    pub fn update_status(id: &str, status: TaskStatus) -> anyhow::Result<bool> {
        let mut tasks = Self::load()?;
        let mut found = false;

        for task in &mut tasks {
            if task.id == id {
                task.status = status;
                found = true;
                break;
            }
        }

        if found {
            Self::save(&tasks)?;
        }

        Ok(found)
    }

    /// Update a task's notes
    pub fn update_notes(id: &str, notes: &str) -> anyhow::Result<bool> {
        let mut tasks = Self::load()?;
        let mut found = false;

        for task in &mut tasks {
            if task.id == id {
                task.notes = Some(notes.to_string());
                found = true;
                break;
            }
        }

        if found {
            Self::save(&tasks)?;
        }

        Ok(found)
    }

    /// Add a blocker to a task
    pub fn add_blocker(id: &str, blocker_id: &str) -> anyhow::Result<bool> {
        let mut tasks = Self::load()?;

        // Verify blocker exists
        if !tasks.iter().any(|t| t.id == blocker_id) {
            anyhow::bail!("Blocker task not found: {blocker_id}");
        }

        let mut found = false;
        for task in &mut tasks {
            if task.id == id {
                if !task.blocked_by.contains(&blocker_id.to_string()) {
                    task.blocked_by.push(blocker_id.to_string());
                }
                found = true;
                break;
            }
        }

        if found {
            Self::save(&tasks)?;
        }

        Ok(found)
    }

    /// Remove a blocker from a task
    pub fn remove_blocker(id: &str, blocker_id: &str) -> anyhow::Result<bool> {
        let mut tasks = Self::load()?;
        let mut found = false;

        for task in &mut tasks {
            if task.id == id {
                task.blocked_by.retain(|b| b != blocker_id);
                found = true;
                break;
            }
        }

        if found {
            Self::save(&tasks)?;
        }

        Ok(found)
    }

    /// Remove a task by ID
    pub fn remove(id: &str) -> anyhow::Result<bool> {
        let mut tasks = Self::load()?;
        let original_len = tasks.len();
        tasks.retain(|t| t.id != id);
        let removed = tasks.len() < original_len;

        if removed {
            Self::save(&tasks)?;
        }

        Ok(removed)
    }

    /// Generate the next task ID using project prefix
    pub fn next_id() -> anyhow::Result<String> {
        let prefix = Self::get_prefix()?;
        let tasks = Self::load()?;

        // Find max existing ID number
        let max_num = tasks
            .iter()
            .filter_map(|t| {
                t.id.strip_prefix(&prefix)
                    .and_then(|s| s.strip_prefix('-'))
                    .and_then(|n| n.parse::<u32>().ok())
            })
            .max()
            .unwrap_or(0);

        Ok(format!("{}-{}", prefix, max_num + 1))
    }

    /// Get project prefix from .noslop.toml or use default
    fn get_prefix() -> anyhow::Result<String> {
        let noslop_path = Path::new(".noslop.toml");
        if noslop_path.exists() {
            let content = fs::read_to_string(noslop_path)?;
            let config: NoslopConfig = toml::from_str(&content)?;
            // Use project prefix if it's not the default
            if config.project.prefix != "NOS" {
                return Ok(config.project.prefix);
            }
        }
        // Fallback to default task prefix
        Ok(default_prefix())
    }

    /// List tasks filtered by status
    #[must_use]
    pub fn list_by_status(status: Option<TaskStatus>) -> Vec<Task> {
        Self::load()
            .unwrap_or_default()
            .into_iter()
            .filter(|t| status.is_none_or(|s| t.status == s))
            .collect()
    }

    /// List ready tasks (pending with no unfinished blockers)
    #[must_use]
    pub fn list_ready() -> Vec<Task> {
        let tasks = Self::load().unwrap_or_default();
        tasks.iter().filter(|t| t.is_ready(&tasks)).cloned().collect()
    }

    /// Get branch metadata for current branch
    pub fn get_branch_meta() -> anyhow::Result<BranchMeta> {
        let file = Self::load_file()?;
        Ok(file.branch)
    }
}
