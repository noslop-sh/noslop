//! Agent command - manage AI agents in isolated workspaces
//!
//! Provides commands to spawn, list, and manage AI agents working in
//! isolated git worktrees. Each agent works on one task at a time in its
//! own workspace with its own branch.

use std::fs;
use std::process::Command;

use anyhow::{anyhow, bail};

use noslop::git;
use noslop::noslop_file;
use noslop::output::OutputMode;
use noslop::paths;
use noslop::storage::TaskRefs;

use crate::cli::AgentAction;

/// Handle agent subcommands
pub fn agent_cmd(action: AgentAction, mode: OutputMode) -> anyhow::Result<()> {
    match action {
        AgentAction::Spawn {
            name,
            count,
            no_tmux,
            idle,
        } => spawn(name, count, no_tmux, idle, mode),
        AgentAction::List => list(mode),
        AgentAction::Kill { name, force } => kill(&name, force, mode),
        AgentAction::Status => status(mode),
        AgentAction::Assign { name, task_id } => assign(&name, &task_id, mode),
    }
}

/// Generate the next agent name (agent-1, agent-2, etc.)
fn next_agent_name() -> anyhow::Result<String> {
    let agents_dir = paths::agents_dir();
    if !agents_dir.exists() {
        return Ok("agent-1".to_string());
    }

    let mut max_num = 0;
    for entry in fs::read_dir(&agents_dir)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        if let Some(num_str) = name.strip_prefix("agent-")
            && let Ok(num) = num_str.parse::<u32>()
        {
            max_num = max_num.max(num);
        }
    }

    Ok(format!("agent-{}", max_num + 1))
}

/// Spawn one or more agents
fn spawn(
    name: Option<String>,
    count: Option<u32>,
    no_tmux: bool,
    idle: bool,
    mode: OutputMode,
) -> anyhow::Result<()> {
    let config = noslop_file::load_or_default();
    let use_tmux = !no_tmux && config.agent.use_tmux;
    let auto_assign = !idle && config.agent.auto_assign;

    let count = count.unwrap_or(1);

    // Ensure agents directory exists
    fs::create_dir_all(paths::agents_dir())?;

    let mut spawned = Vec::new();

    for i in 0..count {
        // Determine agent name
        let agent_name = if count == 1 {
            name.clone()
                .unwrap_or_else(|| next_agent_name().unwrap_or_else(|_| "agent-1".to_string()))
        } else {
            // For multiple agents, generate sequential names
            if let Some(ref base) = name {
                format!("{}-{}", base, i + 1)
            } else {
                next_agent_name()?
            }
        };

        // Check if agent already exists
        let agent_path = paths::agent_path(&agent_name);
        if agent_path.exists() {
            if count == 1 {
                bail!("Agent '{}' already exists at: {}", agent_name, agent_path.display());
            } else {
                // Skip existing agents when spawning multiple
                continue;
            }
        }

        // Find a task to assign (if auto_assign is enabled)
        let task = if auto_assign {
            TaskRefs::next_pending_unblocked()?
        } else {
            None
        };

        // Check if task is already claimed
        if let Some((ref id, ref t)) = task
            && t.claimed_by.is_some()
        {
            if count == 1 {
                bail!("Task {} is already claimed", id);
            } else {
                continue;
            }
        }

        // Create branch name
        let branch = if let Some((ref id, _)) = task {
            format!("agent/{}/{}", agent_name, id.to_lowercase())
        } else {
            format!("agent/{}", agent_name)
        };

        // Create worktree
        git::create_worktree(&agent_path, &branch)?;

        // Initialize .noslop/ in the new agent workspace
        let agent_noslop = agent_path.join(".noslop");
        fs::create_dir_all(&agent_noslop)?;

        // Claim and set HEAD if we have a task
        if let Some((ref id, _)) = task {
            TaskRefs::claim(id, &agent_name)?;

            // Set HEAD in the new agent's workspace
            let head_path = agent_noslop.join("HEAD");
            fs::write(&head_path, format!("{}\n", id))?;
        }

        spawned.push((agent_name.clone(), task.clone(), agent_path.clone(), branch.clone()));

        // Start tmux session if enabled
        if use_tmux {
            let session_name = format!("noslop-{}", agent_name);

            // Check if session already exists
            let check = Command::new("tmux").args(["has-session", "-t", &session_name]).output();

            if check.map(|o| o.status.success()).unwrap_or(false) {
                // Session exists, skip
                continue;
            }

            // Build command to run in tmux
            let tmux_cmd = if let Some((ref id, _)) = task {
                format!(
                    "cd '{}' && noslop task start '{}' && {}",
                    agent_path.display(),
                    id,
                    config.agent.command
                )
            } else {
                format!("cd '{}' && {}", agent_path.display(), config.agent.command)
            };

            Command::new("tmux")
                .args([
                    "new-session",
                    "-d",
                    "-s",
                    &session_name,
                    "-c",
                    &agent_path.to_string_lossy(),
                    "sh",
                    "-c",
                    &tmux_cmd,
                ])
                .output()?;
        }
    }

    // Output results
    if mode == OutputMode::Json {
        let agents: Vec<_> = spawned
            .iter()
            .map(|(name, task, path, branch)| {
                serde_json::json!({
                    "name": name,
                    "path": path.to_string_lossy(),
                    "branch": branch,
                    "task_id": task.as_ref().map(|(id, _)| id),
                    "task_title": task.as_ref().map(|(_, t)| &t.title),
                    "tmux_session": if use_tmux { Some(format!("noslop-{}", name)) } else { None },
                })
            })
            .collect();
        println!("{}", serde_json::json!({ "spawned": agents }));
    } else {
        for (name, task, path, branch) in &spawned {
            println!("Spawned agent: {}", name);
            println!("  Path:   {}", path.display());
            println!("  Branch: {}", branch);
            if let Some((id, t)) = task {
                println!("  Task:   {} - {}", id, t.title);
            } else {
                println!("  Task:   (idle)");
            }
            if use_tmux {
                println!("  Tmux:   noslop-{}", name);
            }
            println!();
        }

        if !spawned.is_empty() && use_tmux {
            println!("To attach to an agent:");
            println!("  tmux attach -t noslop-<name>");
        }
    }

    Ok(())
}

/// List all agents
fn list(mode: OutputMode) -> anyhow::Result<()> {
    let agents_dir = paths::agents_dir();
    let tasks = TaskRefs::list()?;

    let mut agents = Vec::new();

    if agents_dir.exists() {
        for entry in fs::read_dir(&agents_dir)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().to_string();
            let path = entry.path();

            // Find task claimed by this agent
            let claimed_task = tasks
                .iter()
                .find(|(_, t)| t.claimed_by.as_deref() == Some(&name))
                .map(|(id, t)| (id.clone(), t.clone()));

            // Check for tmux session
            let session_name = format!("noslop-{}", name);
            let tmux_running = Command::new("tmux")
                .args(["has-session", "-t", &session_name])
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false);

            // Get branch
            let branch = git::get_branch_in(&path);

            agents.push((name, path, claimed_task, branch, tmux_running));
        }
    }

    if mode == OutputMode::Json {
        let json_agents: Vec<_> = agents
            .iter()
            .map(|(name, path, task, branch, tmux_running)| {
                serde_json::json!({
                    "name": name,
                    "path": path.to_string_lossy(),
                    "branch": branch,
                    "task_id": task.as_ref().map(|(id, _)| id),
                    "task_title": task.as_ref().map(|(_, t)| &t.title),
                    "task_status": task.as_ref().map(|(_, t)| &t.status),
                    "tmux_session": format!("noslop-{}", name),
                    "tmux_running": tmux_running,
                })
            })
            .collect();
        println!("{}", serde_json::json!({ "agents": json_agents }));
    } else {
        if agents.is_empty() {
            println!("No agents");
            println!();
            println!("To spawn an agent:");
            println!("  noslop agent spawn");
            return Ok(());
        }

        println!("{:<15} {:<15} {:<12} {:<30} TMUX", "AGENT", "TASK", "STATUS", "BRANCH");
        println!("{}", "-".repeat(85));

        for (name, _path, task, branch, tmux_running) in &agents {
            let task_col = task.as_ref().map(|(id, _)| id.as_str()).unwrap_or("(idle)");
            let status_col = task.as_ref().map(|(_, t)| t.status.as_str()).unwrap_or("-");
            let branch_col = branch.as_deref().unwrap_or("-");
            let tmux_col = if *tmux_running {
                format!("noslop-{}", name)
            } else {
                "(not running)".to_string()
            };

            println!(
                "{:<15} {:<15} {:<12} {:<30} {}",
                name, task_col, status_col, branch_col, tmux_col
            );
        }

        println!();
        println!("To attach: tmux attach -t <session>");
        println!("To stop:   noslop agent kill <name>");
    }

    Ok(())
}

/// Kill an agent
fn kill(name: &str, force: bool, mode: OutputMode) -> anyhow::Result<()> {
    let agent_path = paths::agent_path(name);
    let session_name = format!("noslop-{}", name);

    // Kill tmux session if running
    let tmux_killed = Command::new("tmux")
        .args(["kill-session", "-t", &session_name])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    // Release any claimed tasks
    let tasks = TaskRefs::list()?;
    let mut released_tasks = Vec::new();
    for (id, task) in &tasks {
        if task.claimed_by.as_deref() == Some(name) {
            TaskRefs::release(id)?;
            released_tasks.push(id.clone());
        }
    }

    // Remove worktree
    let worktree_removed = if agent_path.exists() {
        git::remove_worktree(&agent_path, force).is_ok()
    } else {
        false
    };

    if mode == OutputMode::Json {
        println!(
            "{}",
            serde_json::json!({
                "success": true,
                "name": name,
                "tmux_killed": tmux_killed,
                "worktree_removed": worktree_removed,
                "released_tasks": released_tasks,
            })
        );
    } else {
        println!("Agent killed: {}", name);
        if tmux_killed {
            println!("  Killed tmux session: {}", session_name);
        }
        for task_id in &released_tasks {
            println!("  Released task: {}", task_id);
        }
        if worktree_removed {
            println!("  Removed workspace: {}", agent_path.display());
        } else if agent_path.exists() {
            println!(
                "  Workspace not removed (has uncommitted changes?). Use --force to remove anyway."
            );
        }
    }

    Ok(())
}

/// Show status of current agent
fn status(mode: OutputMode) -> anyhow::Result<()> {
    let agent_root = paths::agent_root();
    let agents_dir = paths::agents_dir();
    let current_task = TaskRefs::current()?;

    // Determine agent name
    let agent_name = if let Ok(agent_root_canonical) = agent_root.canonicalize() {
        if let Ok(agents_dir_canonical) = agents_dir.canonicalize() {
            if let Ok(relative) = agent_root_canonical.strip_prefix(&agents_dir_canonical) {
                relative
                    .components()
                    .next()
                    .map(|c| c.as_os_str().to_string_lossy().to_string())
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    let is_main = agent_name.is_none() && !paths::is_agent();

    let task_info = current_task
        .as_ref()
        .and_then(|id| TaskRefs::get(id).ok().flatten().map(|t| (id.clone(), t)));

    if mode == OutputMode::Json {
        println!(
            "{}",
            serde_json::json!({
                "agent_name": agent_name.as_deref().unwrap_or(if is_main { "main" } else { "unknown" }),
                "path": agent_root.to_string_lossy(),
                "is_main": is_main,
                "task_id": current_task,
                "task_title": task_info.as_ref().map(|(_, t)| &t.title),
                "task_status": task_info.as_ref().map(|(_, t)| &t.status),
            })
        );
    } else {
        if let Some(name) = &agent_name {
            println!("Agent: {}", name);
        } else if is_main {
            println!("Agent: main (primary worktree)");
        } else {
            println!("Agent: unknown");
        }
        println!("  Path: {}", agent_root.display());

        if let Some((id, task)) = task_info {
            println!();
            println!("Current task: {}", id);
            println!("  Title:  {}", task.title);
            println!("  Status: {}", task.status);
        } else {
            println!();
            println!("No active task");
            println!();
            println!("To pick a task:");
            println!("  noslop task next --start");
        }
    }

    Ok(())
}

/// Assign a specific task to an agent
fn assign(name: &str, task_id: &str, mode: OutputMode) -> anyhow::Result<()> {
    let agent_path = paths::agent_path(name);

    // Verify agent exists
    if !agent_path.exists() {
        bail!("Agent '{}' does not exist", name);
    }

    // Verify task exists
    let task = TaskRefs::get(task_id)?.ok_or_else(|| anyhow!("Task not found: {}", task_id))?;

    // Check if task is already claimed by someone else
    if let Some(ref claimed) = task.claimed_by
        && claimed != name
    {
        bail!(
            "Task {} is already claimed by agent '{}'. \
             Use 'noslop agent kill {}' to release it first.",
            task_id,
            claimed,
            claimed
        );
    }

    // Release any current task from this agent
    let tasks = TaskRefs::list()?;
    for (id, t) in &tasks {
        if t.claimed_by.as_deref() == Some(name) && id != task_id {
            TaskRefs::release(id)?;
        }
    }

    // Claim the new task
    TaskRefs::claim(task_id, name)?;

    // Update HEAD in agent's workspace
    let head_path = agent_path.join(".noslop").join("HEAD");
    fs::create_dir_all(head_path.parent().unwrap())?;
    fs::write(&head_path, format!("{}\n", task_id))?;

    if mode == OutputMode::Json {
        println!(
            "{}",
            serde_json::json!({
                "success": true,
                "agent": name,
                "task_id": task_id,
                "task_title": task.title,
            })
        );
    } else {
        println!("Assigned task {} to agent {}", task_id, name);
        println!("  Title: {}", task.title);
    }

    Ok(())
}
