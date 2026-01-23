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
    /// Optional task description
    #[serde(default)]
    pub description: Option<String>,
    /// Optional priority (p0, p1, p2, p3)
    #[serde(default)]
    pub priority: Option<String>,
    /// Topics to assign task to
    #[serde(default)]
    pub topics: Vec<String>,
}

/// Request body for creating a check
#[derive(Debug, Deserialize)]
pub struct CreateCheckRequest {
    /// Scope file/pattern
    #[serde(alias = "target")]
    pub scope: String,
    /// Check message
    pub message: String,
    /// Severity (block, warn, info)
    #[serde(default = "default_check_severity")]
    pub severity: String,
}

fn default_check_severity() -> String {
    "block".to_string()
}

/// Request body for linking a task to a branch
#[derive(Debug, Deserialize)]
pub struct LinkBranchRequest {
    /// Branch name to link (None to unlink)
    pub branch: Option<String>,
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
    /// Backlog tasks (not yet committed to a branch)
    pub backlog: usize,
    /// Pending tasks (committed to a branch)
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
    /// Task description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Status (pending, `in_progress`, done)
    pub status: String,
    /// Priority (p0, p1, p2, p3)
    pub priority: String,
    /// IDs of blocking tasks
    pub blocked_by: Vec<String>,
    /// Whether this is the current task
    pub current: bool,
    /// Whether this task is blocked by unfinished tasks
    pub blocked: bool,
    /// Optional associated git branch
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    /// When work started (RFC3339)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<String>,
    /// When completed (RFC3339)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<String>,
    /// Topics this task belongs to
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub topics: Vec<String>,
    /// Scope patterns (files this task touches)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub scope: Vec<String>,
    /// Total number of checks that apply to this task (derived from scope overlap)
    pub check_count: usize,
    /// Number of verified checks
    pub checks_verified: usize,
}

/// Single task detail response
#[derive(Debug, Serialize)]
pub struct TaskDetailData {
    /// Task ID
    pub id: String,
    /// Task title
    pub title: String,
    /// Task description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Status
    pub status: String,
    /// Priority
    pub priority: String,
    /// Blocking task IDs
    pub blocked_by: Vec<String>,
    /// Whether blocked by unfinished tasks
    pub blocked: bool,
    /// Whether this is the current task
    pub current: bool,
    /// Creation timestamp (RFC3339)
    pub created_at: String,
    /// Optional notes
    pub notes: Option<String>,
    /// Optional associated git branch
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    /// When work started (RFC3339)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<String>,
    /// When completed (RFC3339)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<String>,
    /// Topics this task belongs to
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub topics: Vec<String>,
    /// Scope patterns (files this task touches)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub scope: Vec<String>,
    /// Total number of checks that apply to this task (derived from scope overlap)
    pub check_count: usize,
    /// Number of verified checks
    pub checks_verified: usize,
    /// Actual checks that apply to this task
    pub checks: Vec<TaskCheckItem>,
}

/// A check as it relates to a specific task
#[derive(Debug, Serialize)]
pub struct TaskCheckItem {
    /// Check ID
    pub id: String,
    /// Check message
    pub message: String,
    /// Severity
    pub severity: String,
    /// Whether verified for this task
    pub verified: bool,
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
    /// Scope file/pattern
    pub scope: String,
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
    /// Scope
    pub scope: String,
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

// =============================================================================
// WORKSPACE TYPES
// =============================================================================

/// Workspace data showing repos and branches
#[derive(Debug, Serialize)]
pub struct WorkspaceData {
    /// Current working directory (workspace root)
    pub workspace: String,
    /// Repos in this workspace
    pub repos: Vec<RepoInfo>,
}

/// Information about a single repository
#[derive(Debug, Serialize)]
pub struct RepoInfo {
    /// Repository name (directory name)
    pub name: String,
    /// Full path to the repository
    pub path: String,
    /// All branches in the repo
    pub branches: Vec<BranchInfo>,
    /// Currently checked out branch
    pub current_branch: Option<String>,
}

/// Information about a single branch
#[derive(Debug, Serialize)]
pub struct BranchInfo {
    /// Branch name
    pub name: String,
    /// Whether this branch is selected (shown in kanban)
    pub selected: bool,
    /// Whether user has hidden this branch
    pub hidden: bool,
    /// Color index assigned to this branch
    pub color: usize,
}

// =============================================================================
// CONFIG TYPES
// =============================================================================

/// Config data returned by API
#[derive(Debug, Serialize)]
pub struct ConfigData {
    /// UI theme
    pub theme: String,
    /// Branch selections for current workspace
    pub selections: Vec<BranchSelection>,
}

/// A branch selection entry
#[derive(Debug, Serialize)]
pub struct BranchSelection {
    /// Repository path
    pub repo: String,
    /// Branch name
    pub branch: String,
    /// Whether selected
    pub selected: bool,
    /// Whether hidden
    pub hidden: bool,
    /// Color index
    pub color: usize,
}

/// Request to update config
#[derive(Debug, Deserialize)]
pub struct UpdateConfigRequest {
    /// Branch to update (repo/branch format)
    #[serde(default)]
    pub branch: Option<String>,
    /// Set selected state
    #[serde(default)]
    pub selected: Option<bool>,
    /// Set hidden state
    #[serde(default)]
    pub hidden: Option<bool>,
}

/// Request to add or remove a blocker
#[derive(Debug, Deserialize)]
pub struct BlockerRequest {
    /// ID of the task that blocks
    pub blocker_id: String,
}

// =============================================================================
// TOPIC TYPES
// =============================================================================

/// Topic info
#[derive(Debug, Serialize)]
pub struct TopicInfo {
    /// Topic ID
    pub id: String,
    /// Topic name
    pub name: String,
    /// Topic description (context for LLMs)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Scope patterns for this topic
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub scope: Vec<String>,
    /// Number of tasks in topic
    pub task_count: usize,
    /// When created (RFC3339)
    pub created_at: String,
}

/// Topics list response
#[derive(Debug, Serialize)]
pub struct TopicsData {
    /// List of topics
    pub topics: Vec<TopicInfo>,
    /// Currently selected topic ID (None = view all)
    pub current_topic: Option<String>,
}

/// Create topic request
#[derive(Debug, Deserialize)]
pub struct CreateTopicRequest {
    /// Topic name
    pub name: String,
    /// Optional description
    #[serde(default)]
    pub description: Option<String>,
}

/// Create topic response
#[derive(Debug, Serialize)]
pub struct TopicCreateData {
    /// Created topic ID
    pub id: String,
    /// Topic name
    pub name: String,
}

/// Select topic request
#[derive(Debug, Deserialize)]
pub struct SelectTopicRequest {
    /// Topic ID to select (None = view all)
    #[serde(default)]
    pub id: Option<String>,
}

/// Update topic request
#[derive(Debug, Deserialize)]
pub struct UpdateTopicRequest {
    /// New description (None to clear)
    #[serde(default)]
    pub description: Option<String>,
}

/// Update task request
#[derive(Debug, Deserialize)]
pub struct UpdateTaskRequest {
    /// New description (None to clear)
    #[serde(default)]
    pub description: Option<String>,
    /// New topics list (None means no change)
    #[serde(default)]
    pub topics: Option<Vec<String>>,
}
