//! Task model
//!
//! A task represents a unit of work for an agent to complete.
//! Tasks can have dependencies (`blocked_by`) forming a directed graph.

use serde::{Deserialize, Serialize};

/// A task - a unit of work to be completed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// Unique identifier (auto-generated: PREFIX-N)
    pub id: String,

    /// What needs to be done
    pub title: String,

    /// Current status
    pub status: TaskStatus,

    /// Priority level (p0 = critical, p3 = low)
    pub priority: Priority,

    /// Task IDs that block this one (must be done first)
    #[serde(default)]
    pub blocked_by: Vec<String>,

    /// Optional notes/context that can evolve over time
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,

    /// When this task was created
    pub created_at: String,
}

/// Task status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    /// In the backlog - not yet committed to any branch
    #[default]
    Backlog,
    /// Committed to work on (linked to a branch)
    Pending,
    /// Currently being worked on
    InProgress,
    /// Completed
    Done,
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Backlog => write!(f, "backlog"),
            Self::Pending => write!(f, "pending"),
            Self::InProgress => write!(f, "in_progress"),
            Self::Done => write!(f, "done"),
        }
    }
}

impl std::str::FromStr for TaskStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().replace('-', "_").as_str() {
            "backlog" => Ok(Self::Backlog),
            "pending" => Ok(Self::Pending),
            "in_progress" | "inprogress" | "started" => Ok(Self::InProgress),
            "done" | "complete" | "completed" => Ok(Self::Done),
            _ => Err(format!("Invalid status: {s}. Use: backlog, pending, in_progress, done")),
        }
    }
}

/// Task priority (p0 = most critical)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    /// Critical - must be done immediately
    P0,
    /// High priority (default)
    #[default]
    P1,
    /// Medium priority
    P2,
    /// Low priority
    P3,
}

impl std::fmt::Display for Priority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::P0 => write!(f, "p0"),
            Self::P1 => write!(f, "p1"),
            Self::P2 => write!(f, "p2"),
            Self::P3 => write!(f, "p3"),
        }
    }
}

impl std::str::FromStr for Priority {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "p0" | "0" | "critical" => Ok(Self::P0),
            "p1" | "1" | "high" => Ok(Self::P1),
            "p2" | "2" | "medium" | "med" => Ok(Self::P2),
            "p3" | "3" | "low" => Ok(Self::P3),
            _ => Err(format!("Invalid priority: {s}. Use: p0, p1, p2, p3 (or 0-3)")),
        }
    }
}

impl Task {
    /// Create a new task with the given title
    #[must_use]
    #[allow(dead_code)] // Used by tests and external consumers
    pub fn new(id: String, title: String) -> Self {
        Self {
            id,
            title,
            status: TaskStatus::default(),
            priority: Priority::default(),
            blocked_by: Vec::new(),
            notes: None,
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// Create a new task with all options
    #[must_use]
    #[allow(dead_code)] // Used by commands and external consumers
    pub fn with_options(
        id: String,
        title: String,
        priority: Priority,
        blocked_by: Vec<String>,
        notes: Option<String>,
    ) -> Self {
        Self {
            id,
            title,
            status: TaskStatus::default(),
            priority,
            blocked_by,
            notes,
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// Check if this task is blocked by unfinished tasks
    #[must_use]
    pub fn is_blocked(&self, all_tasks: &[Self]) -> bool {
        self.blocked_by.iter().any(|blocker_id| {
            all_tasks
                .iter()
                .find(|t| t.id == *blocker_id)
                .is_some_and(|t| t.status != TaskStatus::Done)
        })
    }
}
