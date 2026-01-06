//! Task storage
//!
//! Stores tasks in .noslop/tasks.json

use std::fs;
use std::path::Path;

use serde::Deserialize;

use crate::models::{Task, TaskStatus};

const TASKS_PATH: &str = ".noslop/tasks.json";

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

/// Task storage operations
#[derive(Debug, Clone, Copy)]
pub struct TaskStore;

impl TaskStore {
    /// Load all tasks from storage
    pub fn load() -> anyhow::Result<Vec<Task>> {
        let path = Path::new(TASKS_PATH);
        if !path.exists() {
            return Ok(Vec::new());
        }
        let content = fs::read_to_string(path)?;
        if content.trim().is_empty() {
            return Ok(Vec::new());
        }
        Ok(serde_json::from_str(&content)?)
    }

    /// Save all tasks to storage
    pub fn save(tasks: &[Task]) -> anyhow::Result<()> {
        // Ensure directory exists
        if let Some(parent) = Path::new(TASKS_PATH).parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(tasks)?;
        fs::write(TASKS_PATH, content)?;
        Ok(())
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
}
