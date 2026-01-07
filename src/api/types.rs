//! API request and response types
//!
//! All types are framework-agnostic and can be used by any client.

use serde::{Deserialize, Serialize};

use super::error::ApiErrorData;

// =============================================================================
// RESPONSE ENVELOPE
// =============================================================================

/// Standard API response envelope
#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    /// Whether the request succeeded
    pub success: bool,
    /// Response data (present on success)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    /// Error details (present on failure)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ApiErrorData>,
}

impl<T: Serialize> ApiResponse<T> {
    /// Create a successful response
    #[must_use]
    pub const fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }
}

impl ApiResponse<()> {
    /// Create an error response
    #[must_use]
    pub fn error(code: &str, message: &str) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(ApiErrorData {
                code: code.to_string(),
                message: message.to_string(),
            }),
        }
    }
}

// =============================================================================
// REQUEST TYPES
// =============================================================================

/// Request body for creating a task
#[derive(Debug, Deserialize)]
pub struct CreateTaskRequest {
    /// Task title
    pub title: String,
    /// Optional priority (p0, p1, p2, p3)
    #[serde(default)]
    pub priority: Option<String>,
}

/// Request body for creating a check
#[derive(Debug, Deserialize)]
pub struct CreateCheckRequest {
    /// Target file/pattern
    pub target: String,
    /// Check message
    pub message: String,
    /// Severity (block, warn, info)
    #[serde(default = "default_check_severity")]
    pub severity: String,
}

fn default_check_severity() -> String {
    "block".to_string()
}

// =============================================================================
// RESPONSE DATA TYPES
// =============================================================================

/// Status endpoint response data
#[derive(Debug, Serialize)]
pub struct StatusData {
    /// Current git branch
    pub branch: Option<String>,
    /// Current active task ID
    pub current_task: Option<String>,
    /// Task counts by status
    pub tasks: TaskCounts,
    /// Number of checks
    pub checks: usize,
}

/// Task count breakdown
#[derive(Debug, Clone, Copy, Serialize)]
pub struct TaskCounts {
    /// Total number of tasks
    pub total: usize,
    /// Pending tasks
    pub pending: usize,
    /// In-progress tasks
    pub in_progress: usize,
    /// Completed tasks
    pub done: usize,
}

/// Tasks list endpoint response data
#[derive(Debug, Serialize)]
pub struct TasksData {
    /// List of tasks
    pub tasks: Vec<TaskItem>,
}

/// Single task in a list
#[derive(Debug, Serialize)]
pub struct TaskItem {
    /// Task ID
    pub id: String,
    /// Task title
    pub title: String,
    /// Status (pending, `in_progress`, done)
    pub status: String,
    /// Priority (p0, p1, p2, p3)
    pub priority: String,
    /// IDs of blocking tasks
    pub blocked_by: Vec<String>,
    /// Whether this is the current task
    pub current: bool,
    /// Whether this task is ready to work on
    pub ready: bool,
}

/// Single task detail response
#[derive(Debug, Serialize)]
pub struct TaskDetailData {
    /// Task ID
    pub id: String,
    /// Task title
    pub title: String,
    /// Status
    pub status: String,
    /// Priority
    pub priority: String,
    /// Blocking task IDs
    pub blocked_by: Vec<String>,
    /// Whether ready to work on
    pub ready: bool,
    /// Whether this is the current task
    pub current: bool,
    /// Creation timestamp (RFC3339)
    pub created_at: String,
    /// Optional notes
    pub notes: Option<String>,
}

/// Task mutation (start/done/create) response
#[derive(Debug, Serialize)]
pub struct TaskMutationData {
    /// Task ID
    pub id: String,
    /// New status
    pub status: String,
}

/// Response for task creation
#[derive(Debug, Serialize)]
pub struct TaskCreateData {
    /// Created task ID
    pub id: String,
    /// Task title
    pub title: String,
    /// Initial status
    pub status: String,
    /// Priority
    pub priority: String,
}

/// Checks list endpoint response data
#[derive(Debug, Serialize)]
pub struct ChecksData {
    /// List of checks
    pub checks: Vec<CheckItem>,
}

/// Single check item
#[derive(Debug, Serialize)]
pub struct CheckItem {
    /// Check ID
    pub id: String,
    /// Target file/pattern
    pub target: String,
    /// Check message
    pub message: String,
    /// Severity
    pub severity: String,
}

/// Response for check creation
#[derive(Debug, Serialize)]
pub struct CheckCreateData {
    /// Created check ID
    pub id: String,
    /// Target
    pub target: String,
    /// Message
    pub message: String,
    /// Severity
    pub severity: String,
}

/// Long-polling events response
#[derive(Debug, Clone, Copy, Serialize)]
pub struct EventsData {
    /// Whether data has changed since last poll
    pub changed: bool,
    /// Current change counter
    pub counter: u64,
}
