//! Task command - manage tasks

use noslop::models::{Priority, Task, TaskStatus};
use noslop::output::{OutputMode, TaskInfo, TaskListResult, TaskShowResult};
use noslop::storage::TaskStore;

use crate::cli::TaskAction;

/// Handle task subcommands
pub fn task_cmd(action: TaskAction, mode: OutputMode) -> anyhow::Result<()> {
    match action {
        TaskAction::Add {
            title,
            priority,
            blocked_by,
            note,
        } => add(&title, priority.as_deref(), blocked_by, note.as_deref(), mode),
        TaskAction::List { status, ready } => list(status.as_deref(), ready, mode),
        TaskAction::Show { id } => show(&id, mode),
        TaskAction::Start { id } => start(&id, mode),
        TaskAction::Done { id } => done(&id, mode),
        TaskAction::Note { id, message } => note(&id, &message, mode),
        TaskAction::Block { id, blocker_ids } => block(&id, &blocker_ids, mode),
        TaskAction::Unblock { id, blocker_ids } => unblock(&id, &blocker_ids, mode),
        TaskAction::Remove { id } => remove(&id, mode),
    }
}

fn add(
    title: &str,
    priority: Option<&str>,
    blocked_by: Vec<String>,
    note: Option<&str>,
    mode: OutputMode,
) -> anyhow::Result<()> {
    let id = TaskStore::next_id()?;
    let priority: Priority = priority
        .map(|p| p.parse())
        .transpose()
        .map_err(|e: String| anyhow::anyhow!(e))?
        .unwrap_or_default();

    let task = Task::with_options(
        id.clone(),
        title.to_string(),
        priority,
        blocked_by,
        note.map(String::from),
    );

    TaskStore::add(task)?;

    if mode == OutputMode::Json {
        println!(
            "{}",
            serde_json::json!({
                "success": true,
                "id": id,
                "title": title,
                "priority": priority.to_string(),
            })
        );
    } else {
        println!("Created task: {}", id);
        println!("  Title:    {}", title);
        println!("  Priority: {}", priority);
    }

    Ok(())
}

fn list(status: Option<&str>, ready: bool, mode: OutputMode) -> anyhow::Result<()> {
    let tasks = if ready {
        TaskStore::list_ready()
    } else {
        let status_filter: Option<TaskStatus> =
            status.map(|s| s.parse()).transpose().map_err(|e: String| anyhow::anyhow!(e))?;
        TaskStore::list_by_status(status_filter)
    };

    let all_tasks = TaskStore::load()?;

    let task_infos: Vec<TaskInfo> = tasks
        .iter()
        .map(|t| TaskInfo {
            id: t.id.clone(),
            title: t.title.clone(),
            status: t.status.to_string(),
            priority: t.priority.to_string(),
            blocked_by: t.blocked_by.clone(),
            ready: t.is_ready(&all_tasks),
            notes: t.notes.clone(),
            created_at: t.created_at.clone(),
        })
        .collect();

    let result = TaskListResult {
        total: task_infos.len(),
        tasks: task_infos,
    };

    result.render(mode);
    Ok(())
}

fn show(id: &str, mode: OutputMode) -> anyhow::Result<()> {
    let task = TaskStore::get(id);
    let all_tasks = TaskStore::load()?;

    let result = TaskShowResult {
        found: task.is_some(),
        task: task.map(|t| TaskInfo {
            id: t.id.clone(),
            title: t.title.clone(),
            status: t.status.to_string(),
            priority: t.priority.to_string(),
            blocked_by: t.blocked_by.clone(),
            ready: t.is_ready(&all_tasks),
            notes: t.notes.clone(),
            created_at: t.created_at.clone(),
        }),
    };

    result.render(mode);
    Ok(())
}

fn start(id: &str, mode: OutputMode) -> anyhow::Result<()> {
    let found = TaskStore::update_status(id, TaskStatus::InProgress)?;

    if mode == OutputMode::Json {
        println!(
            "{}",
            serde_json::json!({
                "success": found,
                "id": id,
                "status": "in_progress",
            })
        );
    } else if found {
        println!("Started: {}", id);
    } else {
        println!("Task not found: {}", id);
    }

    Ok(())
}

fn done(id: &str, mode: OutputMode) -> anyhow::Result<()> {
    let found = TaskStore::update_status(id, TaskStatus::Done)?;

    if mode == OutputMode::Json {
        println!(
            "{}",
            serde_json::json!({
                "success": found,
                "id": id,
                "status": "done",
            })
        );
    } else if found {
        println!("Completed: {}", id);
    } else {
        println!("Task not found: {}", id);
    }

    Ok(())
}

fn note(id: &str, message: &str, mode: OutputMode) -> anyhow::Result<()> {
    let found = TaskStore::update_notes(id, message)?;

    if mode == OutputMode::Json {
        println!(
            "{}",
            serde_json::json!({
                "success": found,
                "id": id,
                "notes": message,
            })
        );
    } else if found {
        println!("Updated notes for: {}", id);
    } else {
        println!("Task not found: {}", id);
    }

    Ok(())
}

fn block(id: &str, blocker_ids: &[String], mode: OutputMode) -> anyhow::Result<()> {
    for blocker_id in blocker_ids {
        TaskStore::add_blocker(id, blocker_id)?;
    }

    if mode == OutputMode::Json {
        println!(
            "{}",
            serde_json::json!({
                "success": true,
                "id": id,
                "blocked_by": blocker_ids,
            })
        );
    } else {
        println!("{} is now blocked by: {}", id, blocker_ids.join(", "));
    }

    Ok(())
}

fn unblock(id: &str, blocker_ids: &[String], mode: OutputMode) -> anyhow::Result<()> {
    for blocker_id in blocker_ids {
        TaskStore::remove_blocker(id, blocker_id)?;
    }

    if mode == OutputMode::Json {
        println!(
            "{}",
            serde_json::json!({
                "success": true,
                "id": id,
                "unblocked": blocker_ids,
            })
        );
    } else {
        println!("{} is no longer blocked by: {}", id, blocker_ids.join(", "));
    }

    Ok(())
}

fn remove(id: &str, mode: OutputMode) -> anyhow::Result<()> {
    let found = TaskStore::remove(id)?;

    if mode == OutputMode::Json {
        println!(
            "{}",
            serde_json::json!({
                "success": found,
                "id": id,
                "removed": found,
            })
        );
    } else if found {
        println!("Removed: {}", id);
    } else {
        println!("Task not found: {}", id);
    }

    Ok(())
}
