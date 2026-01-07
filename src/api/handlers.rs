//! Pure API handlers
//!
//! These handlers contain business logic and are HTTP-agnostic.
//! They take typed input and return `Result<T, ApiError>`.

use std::path::Path;

use crate::noslop_file;
use crate::storage::TaskRefs;

use super::error::ApiError;
use super::types::{
    CheckCreateData, CheckItem, ChecksData, CreateCheckRequest, CreateTaskRequest, StatusData,
    TaskCounts, TaskCreateData, TaskDetailData, TaskItem, TaskMutationData, TasksData,
};

// =============================================================================
// STATUS
// =============================================================================

/// Get overall status
pub fn get_status() -> Result<StatusData, ApiError> {
    let tasks = TaskRefs::list().map_err(|e| ApiError::internal(e.to_string()))?;

    let current = TaskRefs::current().ok().flatten();
    let checks = load_check_count();

    let pending = tasks.iter().filter(|(_, t)| t.status == "pending").count();
    let in_progress = tasks.iter().filter(|(_, t)| t.status == "in_progress").count();
    let done = tasks.iter().filter(|(_, t)| t.status == "done").count();

    Ok(StatusData {
        branch: get_current_branch(),
        current_task: current,
        tasks: TaskCounts {
            total: tasks.len(),
            pending,
            in_progress,
            done,
        },
        checks,
    })
}

// =============================================================================
// TASKS
// =============================================================================

/// List all tasks
pub fn list_tasks() -> Result<TasksData, ApiError> {
    let tasks = TaskRefs::list().map_err(|e| ApiError::internal(e.to_string()))?;

    let current = TaskRefs::current().ok().flatten();

    let task_items: Vec<TaskItem> = tasks
        .iter()
        .map(|(id, t)| TaskItem {
            id: id.clone(),
            title: t.title.clone(),
            status: t.status.clone(),
            priority: t.priority.clone(),
            blocked_by: t.blocked_by.clone(),
            current: current.as_ref() == Some(id),
            ready: t.is_ready(&tasks),
        })
        .collect();

    Ok(TasksData { tasks: task_items })
}

/// Get a single task by ID
pub fn get_task(id: &str) -> Result<TaskDetailData, ApiError> {
    let tasks = TaskRefs::list().map_err(|e| ApiError::internal(e.to_string()))?;

    match TaskRefs::get(id) {
        Ok(Some(task)) => {
            let current = TaskRefs::current().ok().flatten();
            let ready = task.is_ready(&tasks);
            Ok(TaskDetailData {
                id: id.to_string(),
                title: task.title,
                status: task.status,
                priority: task.priority,
                blocked_by: task.blocked_by,
                ready,
                current: current.as_deref() == Some(id),
                created_at: task.created_at,
                notes: task.notes,
            })
        },
        Ok(None) => Err(ApiError::not_found(format!("Task '{id}' not found"))),
        Err(e) => Err(ApiError::internal(e.to_string())),
    }
}

/// Create a new task
pub fn create_task(req: &CreateTaskRequest) -> Result<TaskCreateData, ApiError> {
    if req.title.trim().is_empty() {
        return Err(ApiError::bad_request("Task title cannot be empty"));
    }

    let priority = req.priority.as_deref();
    let id =
        TaskRefs::create(&req.title, priority).map_err(|e| ApiError::internal(e.to_string()))?;

    let task = TaskRefs::get(&id)
        .map_err(|e| ApiError::internal(e.to_string()))?
        .ok_or_else(|| ApiError::internal("Task created but could not be read back"))?;

    Ok(TaskCreateData {
        id,
        title: task.title,
        status: task.status,
        priority: task.priority,
    })
}

/// Start a task (set to `in_progress` and make current)
pub fn start_task(id: &str) -> Result<TaskMutationData, ApiError> {
    // Verify task exists
    match TaskRefs::get(id) {
        Ok(None) => return Err(ApiError::not_found(format!("Task '{id}' not found"))),
        Err(e) => return Err(ApiError::internal(e.to_string())),
        Ok(Some(_)) => {},
    }

    TaskRefs::set_status(id, "in_progress").map_err(|e| ApiError::internal(e.to_string()))?;

    TaskRefs::set_current(id).map_err(|e| ApiError::internal(e.to_string()))?;

    Ok(TaskMutationData {
        id: id.to_string(),
        status: "in_progress".to_string(),
    })
}

/// Complete a task (set to done)
pub fn complete_task(id: &str) -> Result<TaskMutationData, ApiError> {
    // Verify task exists
    match TaskRefs::get(id) {
        Ok(None) => return Err(ApiError::not_found(format!("Task '{id}' not found"))),
        Err(e) => return Err(ApiError::internal(e.to_string())),
        Ok(Some(_)) => {},
    }

    TaskRefs::set_status(id, "done").map_err(|e| ApiError::internal(e.to_string()))?;

    // Clear current if this was the current task
    if TaskRefs::current().ok().flatten().as_deref() == Some(id) {
        let _ = TaskRefs::clear_current();
    }

    Ok(TaskMutationData {
        id: id.to_string(),
        status: "done".to_string(),
    })
}

// =============================================================================
// CHECKS
// =============================================================================

/// List all checks
pub fn list_checks() -> Result<ChecksData, ApiError> {
    let path = Path::new(".noslop.toml");
    if !path.exists() {
        return Ok(ChecksData { checks: vec![] });
    }

    match noslop_file::load_file(path) {
        Ok(file) => {
            let checks: Vec<CheckItem> = file
                .checks
                .iter()
                .enumerate()
                .map(|(i, c)| CheckItem {
                    id: c.id.clone().unwrap_or_else(|| format!("CHK-{}", i + 1)),
                    target: c.target.clone(),
                    message: c.message.clone(),
                    severity: c.severity.clone(),
                })
                .collect();
            Ok(ChecksData { checks })
        },
        Err(e) => Err(ApiError::internal(e.to_string())),
    }
}

/// Create a new check
pub fn create_check(req: &CreateCheckRequest) -> Result<CheckCreateData, ApiError> {
    if req.target.trim().is_empty() {
        return Err(ApiError::bad_request("Check target cannot be empty"));
    }
    if req.message.trim().is_empty() {
        return Err(ApiError::bad_request("Check message cannot be empty"));
    }

    // Validate severity
    let severity = match req.severity.as_str() {
        "block" | "warn" | "info" => req.severity.clone(),
        _ => return Err(ApiError::bad_request("Severity must be 'block', 'warn', or 'info'")),
    };

    let id = noslop_file::add_check(&req.target, &req.message, &severity)
        .map_err(|e| ApiError::internal(e.to_string()))?;

    Ok(CheckCreateData {
        id,
        target: req.target.clone(),
        message: req.message.clone(),
        severity,
    })
}

// =============================================================================
// HELPERS
// =============================================================================

fn get_current_branch() -> Option<String> {
    let repo = git2::Repository::discover(".").ok()?;
    let head = repo.head().ok()?;
    head.shorthand().map(String::from)
}

fn load_check_count() -> usize {
    let path = Path::new(".noslop.toml");
    if !path.exists() {
        return 0;
    }
    noslop_file::load_file(path).map(|f| f.checks.len()).unwrap_or(0)
}
