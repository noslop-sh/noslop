//! Task command - manage tasks using git-like refs storage

use noslop::output::OutputMode;
use noslop::storage::TaskRefs;

use crate::cli::TaskAction;

/// Handle task subcommands
pub fn task_cmd(action: TaskAction, mode: OutputMode) -> anyhow::Result<()> {
    match action {
        TaskAction::Add { title, priority } => add(&title, priority.as_deref(), mode),
        TaskAction::List { status } => list(status.as_deref(), mode),
        TaskAction::Show { id } => show(&id, mode),
        TaskAction::Current => current(mode),
        TaskAction::Start { id } => start(&id, mode),
        TaskAction::Done => done(mode),
        TaskAction::Remove { id } => remove(&id, mode),
    }
}

fn add(title: &str, priority: Option<&str>, mode: OutputMode) -> anyhow::Result<()> {
    let id = TaskRefs::create(title, priority)?;

    if mode == OutputMode::Json {
        println!(
            "{}",
            serde_json::json!({
                "success": true,
                "id": id,
                "title": title,
            })
        );
    } else {
        println!("Created: {}", id);
        println!("  {}", title);
    }

    Ok(())
}

fn list(status_filter: Option<&str>, mode: OutputMode) -> anyhow::Result<()> {
    let tasks = TaskRefs::list()?;
    let current = TaskRefs::current()?;

    let filtered: Vec<_> = tasks
        .iter()
        .filter(|(_, t)| status_filter.is_none() || t.status == status_filter.unwrap())
        .collect();

    if mode == OutputMode::Json {
        let json_tasks: Vec<_> = filtered
            .iter()
            .map(|(id, t)| {
                serde_json::json!({
                    "id": id,
                    "title": t.title,
                    "status": t.status,
                    "priority": t.priority,
                    "current": current.as_ref() == Some(id),
                })
            })
            .collect();
        println!("{}", serde_json::json!({ "tasks": json_tasks }));
    } else {
        if filtered.is_empty() {
            println!("No tasks");
            return Ok(());
        }

        for (id, task) in &filtered {
            let marker = if current.as_ref() == Some(id) { ">" } else { " " };
            let status_icon = match task.status.as_str() {
                "done" => "✓",
                "in_progress" => "●",
                _ => "○",
            };
            println!("{} [{}] {} {}", marker, id, status_icon, task.title);
        }
    }

    Ok(())
}

fn show(id: &str, mode: OutputMode) -> anyhow::Result<()> {
    let task = TaskRefs::get(id)?;
    let current = TaskRefs::current()?;

    if mode == OutputMode::Json {
        if let Some(t) = task {
            println!(
                "{}",
                serde_json::json!({
                    "found": true,
                    "id": id,
                    "title": t.title,
                    "status": t.status,
                    "priority": t.priority,
                    "current": current.as_deref() == Some(id),
                    "created_at": t.created_at,
                    "notes": t.notes,
                })
            );
        } else {
            println!("{}", serde_json::json!({ "found": false, "id": id }));
        }
    } else if let Some(t) = task {
        let is_current = current.as_deref() == Some(id);
        println!("{}{}", id, if is_current { " (current)" } else { "" });
        println!("  Title:    {}", t.title);
        println!("  Status:   {}", t.status);
        println!("  Priority: {}", t.priority);
        println!("  Created:  {}", t.created_at);
        if let Some(notes) = &t.notes {
            println!("  Notes:    {}", notes);
        }
    } else {
        println!("Task not found: {}", id);
    }

    Ok(())
}

fn current(mode: OutputMode) -> anyhow::Result<()> {
    let current_id = TaskRefs::current()?;

    if mode == OutputMode::Json {
        if let Some(id) = &current_id {
            let task = TaskRefs::get(id)?;
            println!(
                "{}",
                serde_json::json!({
                    "current": id,
                    "title": task.map(|t| t.title),
                })
            );
        } else {
            println!("{}", serde_json::json!({ "current": null }));
        }
    } else if let Some(id) = &current_id {
        if let Some(task) = TaskRefs::get(id)? {
            println!("{}: {}", id, task.title);
        } else {
            println!("{} (task file missing)", id);
        }
    } else {
        println!("No active task");
        println!("Use 'noslop task start <id>' to start one");
    }

    Ok(())
}

fn start(id: &str, mode: OutputMode) -> anyhow::Result<()> {
    // Check task exists
    let task = TaskRefs::get(id)?;
    if task.is_none() {
        if mode == OutputMode::Json {
            println!(
                "{}",
                serde_json::json!({ "success": false, "error": "Task not found" })
            );
        } else {
            println!("Task not found: {}", id);
        }
        return Ok(());
    }

    // Set status to in_progress and set HEAD
    TaskRefs::set_status(id, "in_progress")?;
    TaskRefs::set_current(id)?;

    if mode == OutputMode::Json {
        println!(
            "{}",
            serde_json::json!({
                "success": true,
                "id": id,
                "status": "in_progress",
            })
        );
    } else {
        let task = TaskRefs::get(id)?.unwrap();
        println!("Started: {}", id);
        println!("  {}", task.title);
    }

    Ok(())
}

fn done(mode: OutputMode) -> anyhow::Result<()> {
    let current_id = TaskRefs::current()?;

    if current_id.is_none() {
        if mode == OutputMode::Json {
            println!(
                "{}",
                serde_json::json!({ "success": false, "error": "No active task" })
            );
        } else {
            println!("No active task");
            println!("Use 'noslop task start <id>' to start one first");
        }
        return Ok(());
    }

    let id = current_id.unwrap();

    // Set status to done and clear HEAD
    TaskRefs::set_status(&id, "done")?;
    TaskRefs::clear_current()?;

    if mode == OutputMode::Json {
        println!(
            "{}",
            serde_json::json!({
                "success": true,
                "id": id,
                "status": "done",
            })
        );
    } else {
        let task = TaskRefs::get(&id)?.unwrap();
        println!("Completed: {}", id);
        println!("  {}", task.title);
    }

    Ok(())
}

fn remove(id: &str, mode: OutputMode) -> anyhow::Result<()> {
    let found = TaskRefs::delete(id)?;

    if mode == OutputMode::Json {
        println!(
            "{}",
            serde_json::json!({
                "success": found,
                "id": id,
            })
        );
    } else if found {
        println!("Removed: {}", id);
    } else {
        println!("Task not found: {}", id);
    }

    Ok(())
}
