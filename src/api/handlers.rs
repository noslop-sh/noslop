//! Pure API handlers
//!
//! These handlers contain business logic and are HTTP-agnostic.
//! They take typed input and return `Result<T, ApiError>`.

use std::path::Path;

use crate::config::GlobalConfig;
use crate::noslop_file;
use crate::storage::TaskRefs;

use super::error::ApiError;
use super::types::{
    BlockerRequest, BranchInfo, BranchSelection, CheckCreateData, CheckItem, ChecksData,
    ConfigData, CreateCheckRequest, CreateProjectRequest, CreateTaskRequest, LinkBranchRequest,
    ProjectCreateData, ProjectInfo, ProjectsData, RepoInfo, SelectProjectRequest, StatusData,
    TaskCounts, TaskCreateData, TaskDetailData, TaskItem, TaskMutationData, TasksData,
    UpdateConfigRequest, WorkspaceData,
};

// =============================================================================
// STATUS
// =============================================================================

/// Get overall status
pub fn get_status() -> Result<StatusData, ApiError> {
    let tasks = TaskRefs::list().map_err(|e| ApiError::internal(e.to_string()))?;

    let current = TaskRefs::current().ok().flatten();
    let checks = load_check_count();

    let backlog = tasks.iter().filter(|(_, t)| t.status == "backlog").count();
    let pending = tasks.iter().filter(|(_, t)| t.status == "pending").count();
    let in_progress = tasks.iter().filter(|(_, t)| t.status == "in_progress").count();
    let done = tasks.iter().filter(|(_, t)| t.status == "done").count();

    Ok(StatusData {
        branch: get_current_branch(),
        current_task: current,
        tasks: TaskCounts {
            total: tasks.len(),
            backlog,
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

/// List all tasks, optionally filtered by branch
pub fn list_tasks() -> Result<TasksData, ApiError> {
    list_tasks_filtered(None, None)
}

/// List tasks with optional project and/or branch filter
pub fn list_tasks_filtered(
    project_filter: Option<&str>,
    branch_filter: Option<&str>,
) -> Result<TasksData, ApiError> {
    let tasks = TaskRefs::list().map_err(|e| ApiError::internal(e.to_string()))?;

    let current = TaskRefs::current().ok().flatten();

    let task_items: Vec<TaskItem> = tasks
        .iter()
        .filter(|(_, t)| {
            // If project filter is set, only include tasks with matching project
            let project_match =
                project_filter.is_none_or(|filter| t.project.as_deref() == Some(filter));
            // If branch filter is set, only include tasks with matching branch
            let branch_match =
                branch_filter.is_none_or(|filter| t.branch.as_deref() == Some(filter));
            project_match && branch_match
        })
        .map(|(id, t)| TaskItem {
            id: id.clone(),
            title: t.title.clone(),
            status: t.status.clone(),
            priority: t.priority.clone(),
            blocked_by: t.blocked_by.clone(),
            current: current.as_ref() == Some(id),
            blocked: t.is_blocked(&tasks),
            branch: t.branch.clone(),
            started_at: t.started_at.clone(),
            completed_at: t.completed_at.clone(),
            project: t.project.clone(),
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
            let blocked = task.is_blocked(&tasks);
            Ok(TaskDetailData {
                id: id.to_string(),
                title: task.title,
                status: task.status,
                priority: task.priority,
                blocked_by: task.blocked_by,
                blocked,
                current: current.as_deref() == Some(id),
                created_at: task.created_at,
                notes: task.notes,
                branch: task.branch,
                started_at: task.started_at,
                completed_at: task.completed_at,
                project: task.project,
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

    // Use project from request (don't inherit - let UI handle that)
    let project = req.project.as_deref();

    let id = TaskRefs::create_with_project(&req.title, priority, project)
        .map_err(|e| ApiError::internal(e.to_string()))?;

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

/// Start a task (set to `in_progress`, make current, auto-link to branch)
pub fn start_task(id: &str) -> Result<TaskMutationData, ApiError> {
    match TaskRefs::start(id) {
        Ok(true) => Ok(TaskMutationData {
            id: id.to_string(),
            status: "in_progress".to_string(),
        }),
        Ok(false) => Err(ApiError::not_found(format!("Task '{id}' not found"))),
        Err(e) => Err(ApiError::internal(e.to_string())),
    }
}

/// Complete a task (set to done, clear current if needed)
pub fn complete_task(id: &str) -> Result<TaskMutationData, ApiError> {
    match TaskRefs::complete(id) {
        Ok(true) => Ok(TaskMutationData {
            id: id.to_string(),
            status: "done".to_string(),
        }),
        Ok(false) => Err(ApiError::not_found(format!("Task '{id}' not found"))),
        Err(e) => Err(ApiError::internal(e.to_string())),
    }
}

/// Reset a task (set back to pending and clear current)
pub fn reset_task(id: &str) -> Result<TaskMutationData, ApiError> {
    // Verify task exists
    match TaskRefs::get(id) {
        Ok(None) => return Err(ApiError::not_found(format!("Task '{id}' not found"))),
        Err(e) => return Err(ApiError::internal(e.to_string())),
        Ok(Some(_)) => {},
    }

    TaskRefs::set_status(id, "pending").map_err(|e| ApiError::internal(e.to_string()))?;

    // Clear current if this was the current task
    if TaskRefs::current().ok().flatten().as_deref() == Some(id) {
        let _ = TaskRefs::clear_current();
    }

    Ok(TaskMutationData {
        id: id.to_string(),
        status: "pending".to_string(),
    })
}

/// Move a task to backlog (unlink from branch, set to backlog status, clear current if needed)
pub fn backlog_task(id: &str) -> Result<TaskMutationData, ApiError> {
    match TaskRefs::move_to_backlog(id) {
        Ok(true) => Ok(TaskMutationData {
            id: id.to_string(),
            status: "backlog".to_string(),
        }),
        Ok(false) => Err(ApiError::not_found(format!("Task '{id}' not found"))),
        Err(e) => Err(ApiError::internal(e.to_string())),
    }
}

/// Link or unlink a task to a git branch
pub fn link_branch(id: &str, req: &LinkBranchRequest) -> Result<TaskMutationData, ApiError> {
    // Verify task exists
    let task = match TaskRefs::get(id) {
        Ok(None) => return Err(ApiError::not_found(format!("Task '{id}' not found"))),
        Err(e) => return Err(ApiError::internal(e.to_string())),
        Ok(Some(t)) => t,
    };

    TaskRefs::link_branch(id, req.branch.as_deref())
        .map_err(|e| ApiError::internal(e.to_string()))?;

    Ok(TaskMutationData {
        id: id.to_string(),
        status: task.status,
    })
}

/// Delete a task
pub fn delete_task(id: &str) -> Result<TaskMutationData, ApiError> {
    // Verify task exists first
    match TaskRefs::get(id) {
        Ok(None) => return Err(ApiError::not_found(format!("Task '{id}' not found"))),
        Err(e) => return Err(ApiError::internal(e.to_string())),
        Ok(Some(_)) => {},
    }

    TaskRefs::delete(id).map_err(|e| ApiError::internal(e.to_string()))?;

    Ok(TaskMutationData {
        id: id.to_string(),
        status: "deleted".to_string(),
    })
}

/// Add a blocker to a task
pub fn add_blocker(id: &str, req: &BlockerRequest) -> Result<TaskMutationData, ApiError> {
    // Verify blocker task exists
    if TaskRefs::get(&req.blocker_id)
        .map_err(|e| ApiError::internal(e.to_string()))?
        .is_none()
    {
        return Err(ApiError::not_found(format!("Blocker task '{}' not found", req.blocker_id)));
    }

    match TaskRefs::add_blocker(id, &req.blocker_id) {
        Ok(true) => Ok(TaskMutationData {
            id: id.to_string(),
            status: "blocked".to_string(),
        }),
        Ok(false) => Err(ApiError::not_found(format!("Task '{id}' not found"))),
        Err(e) => Err(ApiError::internal(e.to_string())),
    }
}

/// Remove a blocker from a task
pub fn remove_blocker(id: &str, req: &BlockerRequest) -> Result<TaskMutationData, ApiError> {
    match TaskRefs::remove_blocker(id, &req.blocker_id) {
        Ok(true) => Ok(TaskMutationData {
            id: id.to_string(),
            status: "unblocked".to_string(),
        }),
        Ok(false) => Err(ApiError::not_found(format!("Task '{id}' not found"))),
        Err(e) => Err(ApiError::internal(e.to_string())),
    }
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

// =============================================================================
// WORKSPACE
// =============================================================================

/// Get workspace information (repos and branches)
pub fn get_workspace() -> Result<WorkspaceData, ApiError> {
    let cwd = std::env::current_dir().map_err(|e| ApiError::internal(e.to_string()))?;
    let workspace_str = cwd.to_string_lossy().to_string();

    let mut config = GlobalConfig::load();

    // Discover repos in current workspace
    let repos = discover_repos(&cwd, &mut config)?;

    // Save config to persist any new discoveries
    let _ = config.save();

    Ok(WorkspaceData {
        workspace: workspace_str,
        repos,
    })
}

/// Discover git repositories in the workspace
fn discover_repos(workspace: &Path, config: &mut GlobalConfig) -> Result<Vec<RepoInfo>, ApiError> {
    let mut repos = Vec::new();

    // First, check if current directory is a git repo
    if let Ok(repo) = git2::Repository::discover(workspace)
        && let Some(workdir) = repo.workdir()
    {
        let repo_info = build_repo_info(workdir, &repo, workspace, config)?;
        repos.push(repo_info);

        // Add to config if not already tracked
        config.add_repo(workspace, workdir);
    }

    // TODO: In future, scan for additional repos in workspace subdirectories
    // for multi-repo support

    Ok(repos)
}

/// Build repo info with branches
fn build_repo_info(
    repo_path: &Path,
    repo: &git2::Repository,
    workspace: &Path,
    config: &mut GlobalConfig,
) -> Result<RepoInfo, ApiError> {
    let name = repo_path
        .file_name()
        .map_or_else(|| "unknown".to_string(), |n| n.to_string_lossy().to_string());

    let path_str = repo_path.to_string_lossy().to_string();

    // Get current branch
    let current_branch = repo.head().ok().and_then(|h| h.shorthand().map(String::from));

    // Get all local branches
    let branches = list_repo_branches(repo, workspace, &path_str, config, current_branch.as_ref())?;

    Ok(RepoInfo {
        name,
        path: path_str,
        branches,
        current_branch,
    })
}

/// List all branches in a repo with their config state
fn list_repo_branches(
    repo: &git2::Repository,
    workspace: &Path,
    repo_path: &str,
    config: &mut GlobalConfig,
    current_branch: Option<&String>,
) -> Result<Vec<BranchInfo>, ApiError> {
    let mut branches = Vec::new();

    let branch_iter = repo
        .branches(Some(git2::BranchType::Local))
        .map_err(|e| ApiError::internal(e.to_string()))?;

    for branch_result in branch_iter {
        let (branch, _) = branch_result.map_err(|e| ApiError::internal(e.to_string()))?;

        if let Some(name) = branch.name().ok().flatten() {
            let selected = config.is_branch_selected(workspace, repo_path, name);
            let hidden = config.is_branch_hidden(workspace, repo_path, name);
            let color = config.get_branch_color(workspace, repo_path, name);

            // Auto-select current branch if nothing is selected yet
            let auto_select = current_branch.map(String::as_str) == Some(name)
                && !hidden
                && !config
                    .workspace(workspace)
                    .is_some_and(|ws| ws.branches.contains_key(repo_path));

            if auto_select {
                config.set_branch_selected(workspace, repo_path, name, true);
            }

            branches.push(BranchInfo {
                name: name.to_string(),
                selected: selected || auto_select,
                hidden,
                color,
            });
        }
    }

    // Sort branches: current first, then alphabetically
    branches.sort_by(|a, b| {
        let a_current = current_branch.map(String::as_str) == Some(&a.name);
        let b_current = current_branch.map(String::as_str) == Some(&b.name);
        match (a_current, b_current) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.cmp(&b.name),
        }
    });

    Ok(branches)
}

// =============================================================================
// CONFIG
// =============================================================================

/// Get current config for workspace
pub fn get_config() -> Result<ConfigData, ApiError> {
    let cwd = std::env::current_dir().map_err(|e| ApiError::internal(e.to_string()))?;
    let config = GlobalConfig::load();

    let mut selections = Vec::new();

    if let Some(ws) = config.workspace(&cwd) {
        for (repo, settings) in &ws.branches {
            for branch in &settings.selected {
                let color = ws.colors.get(&format!("{repo}/{branch}")).copied().unwrap_or(0);
                selections.push(BranchSelection {
                    repo: repo.clone(),
                    branch: branch.clone(),
                    selected: true,
                    hidden: false,
                    color,
                });
            }
            for branch in &settings.hidden {
                let color = ws.colors.get(&format!("{repo}/{branch}")).copied().unwrap_or(0);
                selections.push(BranchSelection {
                    repo: repo.clone(),
                    branch: branch.clone(),
                    selected: false,
                    hidden: true,
                    color,
                });
            }
        }
    }

    Ok(ConfigData {
        theme: config.ui.theme,
        selections,
    })
}

/// Update config for workspace
pub fn update_config(req: &UpdateConfigRequest) -> Result<ConfigData, ApiError> {
    let cwd = std::env::current_dir().map_err(|e| ApiError::internal(e.to_string()))?;
    let mut config = GlobalConfig::load();

    if let Some(branch_spec) = &req.branch {
        // Parse "repo/branch" format
        let parts: Vec<&str> = branch_spec.splitn(2, '/').collect();
        if parts.len() != 2 {
            return Err(ApiError::bad_request("Branch must be in 'repo/branch' format"));
        }
        let (repo, branch) = (parts[0], parts[1]);

        if let Some(selected) = req.selected {
            config.set_branch_selected(&cwd, repo, branch, selected);
        }
        if let Some(hidden) = req.hidden {
            config.set_branch_hidden(&cwd, repo, branch, hidden);
        }
    }

    config.save().map_err(|e| ApiError::internal(e.to_string()))?;

    // Return updated config
    get_config()
}

// =============================================================================
// PROJECTS
// =============================================================================

/// List all projects in the workspace
pub fn list_projects() -> Result<ProjectsData, ApiError> {
    let cwd = std::env::current_dir().map_err(|e| ApiError::internal(e.to_string()))?;
    let config = GlobalConfig::load();
    let tasks = TaskRefs::list().map_err(|e| ApiError::internal(e.to_string()))?;

    let projects: Vec<ProjectInfo> = config
        .list_projects(&cwd)
        .into_iter()
        .map(|p| {
            let task_count =
                tasks.iter().filter(|(_, t)| t.project.as_deref() == Some(&p.id)).count();
            ProjectInfo {
                id: p.id.clone(),
                name: p.name.clone(),
                task_count,
                created_at: p.created_at.clone(),
            }
        })
        .collect();

    let current_project = config.current_project(&cwd).map(String::from);

    Ok(ProjectsData {
        projects,
        current_project,
    })
}

/// Create a new project
pub fn create_project(req: &CreateProjectRequest) -> Result<ProjectCreateData, ApiError> {
    if req.name.trim().is_empty() {
        return Err(ApiError::bad_request("Project name cannot be empty"));
    }

    let cwd = std::env::current_dir().map_err(|e| ApiError::internal(e.to_string()))?;
    let mut config = GlobalConfig::load();

    let id = config.create_project(&cwd, &req.name);
    config.save().map_err(|e| ApiError::internal(e.to_string()))?;

    Ok(ProjectCreateData {
        id,
        name: req.name.clone(),
    })
}

/// Delete a project
pub fn delete_project(id: &str) -> Result<(), ApiError> {
    let cwd = std::env::current_dir().map_err(|e| ApiError::internal(e.to_string()))?;
    let mut config = GlobalConfig::load();

    if !config.delete_project(&cwd, id) {
        return Err(ApiError::not_found(format!("Project '{id}' not found")));
    }

    config.save().map_err(|e| ApiError::internal(e.to_string()))?;

    Ok(())
}

/// Select the current project (or None for "view all")
pub fn select_project(req: &SelectProjectRequest) -> Result<ProjectsData, ApiError> {
    let cwd = std::env::current_dir().map_err(|e| ApiError::internal(e.to_string()))?;
    let mut config = GlobalConfig::load();

    // Verify project exists if an ID is provided
    if let Some(id) = &req.id
        && config.get_project(&cwd, id).is_none()
    {
        return Err(ApiError::not_found(format!("Project '{id}' not found")));
    }

    config.set_current_project(&cwd, req.id.as_deref());
    config.save().map_err(|e| ApiError::internal(e.to_string()))?;

    list_projects()
}
