//! Task command - manage tasks using git-like refs storage

use noslop::noslop_file;
use noslop::output::OutputMode;
use noslop::storage::TaskRefs;

use crate::cli::TaskAction;

/// Handle task subcommands
pub fn task_cmd(action: TaskAction, mode: OutputMode) -> anyhow::Result<()> {
    match action {
        TaskAction::Add {
            title,
            priority,
            blocked_by,
        } => add(&title, priority.as_deref(), blocked_by, mode),
        TaskAction::List { status, unblocked } => list(status.as_deref(), unblocked, mode),
        TaskAction::Show { id } => show(&id, mode),
        TaskAction::Current => current(mode),
        TaskAction::Next { start } => next(start, mode),
        TaskAction::Start { id } => start_task(&id, mode),
        TaskAction::Done => done(mode),
        TaskAction::Block { id, by } => block(&id, &by, mode),
        TaskAction::Unblock { id, by } => unblock(&id, &by, mode),
        TaskAction::Remove { id } => remove(&id, mode),
    }
}

fn add(
    title: &str,
    priority: Option<&str>,
    blocked_by: Option<Vec<String>>,
    mode: OutputMode,
) -> anyhow::Result<()> {
    let id = TaskRefs::create(title, priority)?;

    // Add blockers if specified
    if let Some(blockers) = &blocked_by {
        for blocker in blockers {
            TaskRefs::add_blocker(&id, blocker)?;
        }
    }

    if mode == OutputMode::Json {
        println!(
            "{}",
            serde_json::json!({
                "success": true,
                "id": id,
                "title": title,
                "blocked_by": blocked_by.unwrap_or_default(),
            })
        );
    } else {
        println!("Created: {}", id);
        println!("  {}", title);
        if let Some(blockers) = blocked_by
            && !blockers.is_empty()
        {
            println!("  Blocked by: {}", blockers.join(", "));
        }
    }

    Ok(())
}

fn list(status_filter: Option<&str>, unblocked_only: bool, mode: OutputMode) -> anyhow::Result<()> {
    let tasks = TaskRefs::list()?;
    let current = TaskRefs::current()?;

    let filtered: Vec<_> = if unblocked_only {
        tasks
            .iter()
            .filter(|(_, t)| t.status == "pending" && !t.is_blocked(&tasks))
            .collect()
    } else {
        tasks
            .iter()
            .filter(|(_, t)| status_filter.is_none() || t.status == status_filter.unwrap())
            .collect()
    };

    if mode == OutputMode::Json {
        let json_tasks: Vec<_> = filtered
            .iter()
            .map(|(id, t)| {
                let topics_info = get_topics_info(&t.topics);
                serde_json::json!({
                    "id": id,
                    "title": t.title,
                    "description": t.description,
                    "status": t.status,
                    "priority": t.priority,
                    "blocked_by": t.blocked_by,
                    "current": current.as_ref() == Some(id),
                    "blocked": t.is_blocked(&tasks),
                    "topics": topics_info.iter().map(|(tid, name, desc)| {
                        serde_json::json!({
                            "id": tid,
                            "name": name,
                            "description": desc,
                        })
                    }).collect::<Vec<_>>(),
                    "scope": t.scope,
                })
            })
            .collect();
        println!("{}", serde_json::json!({ "tasks": json_tasks }));
    } else {
        if filtered.is_empty() {
            if unblocked_only {
                println!("No pending unblocked tasks");
            } else {
                println!("No tasks");
            }
            return Ok(());
        }

        for (id, task) in &filtered {
            let marker = if current.as_ref() == Some(id) {
                ">"
            } else {
                " "
            };
            let status_icon = match task.status.as_str() {
                "done" => "✓",
                "in_progress" => "●",
                "backlog" => "◌",
                _ if task.is_blocked(&tasks) => "⊘",
                _ => "○",
            };
            let blocked_str = if !task.blocked_by.is_empty() {
                format!(" (blocked by: {})", task.blocked_by.join(", "))
            } else {
                String::new()
            };
            println!("{} [{}] {} {}{}", marker, id, status_icon, task.title, blocked_str);
        }
    }

    Ok(())
}

fn show(id: &str, mode: OutputMode) -> anyhow::Result<()> {
    let tasks = TaskRefs::list()?;
    let task = TaskRefs::get(id)?;
    let current = TaskRefs::current()?;

    if mode == OutputMode::Json {
        if let Some(t) = task {
            let topics_info = get_topics_info(&t.topics);
            println!(
                "{}",
                serde_json::json!({
                    "found": true,
                    "id": id,
                    "title": t.title,
                    "description": t.description,
                    "status": t.status,
                    "priority": t.priority,
                    "blocked_by": t.blocked_by,
                    "blocked": t.is_blocked(&tasks),
                    "current": current.as_deref() == Some(id),
                    "created_at": t.created_at,
                    "notes": t.notes,
                    "topics": topics_info.iter().map(|(tid, name, desc)| {
                        serde_json::json!({
                            "id": tid,
                            "name": name,
                            "description": desc,
                        })
                    }).collect::<Vec<_>>(),
                    "scope": t.scope,
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
        if !t.blocked_by.is_empty() {
            let blocked_status = if t.is_blocked(&tasks) {
                " (blocked)"
            } else {
                " (resolved)"
            };
            println!("  Blocked by: {}{}", t.blocked_by.join(", "), blocked_status);
        }
        println!("  Created:  {}", t.created_at);
        if let Some(notes) = &t.notes {
            println!("  Notes:    {}", notes);
        }
        // Show description if present
        if let Some(desc) = &t.description {
            println!();
            println!("Description:");
            for line in desc.lines() {
                println!("  {}", line);
            }
        }
        // Show topics info if present
        let topics_info = get_topics_info(&t.topics);
        for (topic_id, topic_name, topic_desc) in topics_info {
            println!();
            println!("Topic: {} ({})", topic_name, topic_id);
            if let Some(desc) = topic_desc {
                for line in desc.lines() {
                    println!("  {}", line);
                }
            }
        }
        // Show scope if present
        if !t.scope.is_empty() {
            println!();
            println!("Scope:");
            for pattern in &t.scope {
                println!("  {}", pattern);
            }
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

fn next(start: bool, mode: OutputMode) -> anyhow::Result<()> {
    let result = TaskRefs::next_pending_unblocked()?;

    match result {
        Some((id, task)) => {
            let (linked_branch, task_desc) = if start {
                // Use centralized start logic
                TaskRefs::start(&id)?;
                // Re-fetch task to get linked branch and updated info
                let updated = TaskRefs::get(&id)?;
                (
                    updated.as_ref().and_then(|t| t.branch.clone()),
                    updated.as_ref().and_then(|t| t.description.clone()),
                )
            } else {
                (task.branch.clone(), task.description.clone())
            };

            // Get topics info if task has topics
            let topics_info = get_topics_info(&task.topics);

            if mode == OutputMode::Json {
                println!(
                    "{}",
                    serde_json::json!({
                        "found": true,
                        "id": id,
                        "title": task.title,
                        "description": task_desc,
                        "priority": task.priority,
                        "notes": task.notes,
                        "started": start,
                        "branch": linked_branch,
                        "topics": topics_info.iter().map(|(tid, name, desc)| {
                            serde_json::json!({
                                "id": tid,
                                "name": name,
                                "description": desc,
                            })
                        }).collect::<Vec<_>>(),
                    })
                );
            } else if start {
                println!("Started: {}", id);
                println!("  {}", task.title);
                if let Some(branch) = linked_branch {
                    println!("  Linked to: {}", branch);
                }
                // Show task description if present
                if let Some(desc) = &task_desc {
                    println!();
                    println!("Task context:");
                    for line in desc.lines() {
                        println!("  {}", line);
                    }
                }
                // Show topics context if present
                for (topic_id, topic_name, topic_desc) in &topics_info {
                    println!();
                    println!("Topic: {} ({})", topic_name, topic_id);
                    if let Some(desc) = topic_desc {
                        for line in desc.lines() {
                            println!("  {}", line);
                        }
                    }
                }
            } else {
                println!("Next: {} - {}", id, task.title);
                if let Some(notes) = &task.notes {
                    println!("  {}", notes);
                }
            }
        },
        None => {
            if mode == OutputMode::Json {
                println!("{}", serde_json::json!({ "found": false }));
            } else {
                println!("No pending unblocked tasks");
            }
        },
    }

    Ok(())
}

fn start_task(id: &str, mode: OutputMode) -> anyhow::Result<()> {
    // Use centralized start logic (handles status, current, and auto-linking)
    if TaskRefs::start(id)? {
        // Re-fetch task to get linked branch
        let task = TaskRefs::get(id)?.unwrap();
        let branch = task.branch.clone();

        // Get topics info if task has topics
        let topics_info = get_topics_info(&task.topics);

        if mode == OutputMode::Json {
            println!(
                "{}",
                serde_json::json!({
                    "success": true,
                    "id": id,
                    "title": task.title,
                    "description": task.description,
                    "status": "in_progress",
                    "branch": branch,
                    "topics": topics_info.iter().map(|(tid, name, desc)| {
                        serde_json::json!({
                            "id": tid,
                            "name": name,
                            "description": desc,
                        })
                    }).collect::<Vec<_>>(),
                })
            );
        } else {
            println!("Started: {}", id);
            println!("  {}", task.title);
            if let Some(branch) = branch {
                println!("  Linked to: {}", branch);
            }
            // Show task description if present
            if let Some(desc) = &task.description {
                println!();
                println!("Task context:");
                for line in desc.lines() {
                    println!("  {}", line);
                }
            }
            // Show topics context if present
            for (topic_id, topic_name, topic_desc) in &topics_info {
                println!();
                println!("Topic: {} ({})", topic_name, topic_id);
                if let Some(desc) = topic_desc {
                    for line in desc.lines() {
                        println!("  {}", line);
                    }
                }
            }
        }
    } else if mode == OutputMode::Json {
        println!("{}", serde_json::json!({ "success": false, "error": "Task not found" }));
    } else {
        println!("Task not found: {}", id);
    }

    Ok(())
}

/// Get topic info (id, name, description) for multiple topic IDs
fn get_topics_info(topic_ids: &[String]) -> Vec<(String, String, Option<String>)> {
    if topic_ids.is_empty() {
        return Vec::new();
    }
    let file = noslop_file::load_or_default();
    topic_ids
        .iter()
        .filter_map(|id| {
            file.get_topic(id)
                .map(|c| (c.id.clone(), c.name.clone(), c.description.clone()))
        })
        .collect()
}

fn done(mode: OutputMode) -> anyhow::Result<()> {
    let current_id = TaskRefs::current()?;

    if current_id.is_none() {
        if mode == OutputMode::Json {
            println!("{}", serde_json::json!({ "success": false, "error": "No active task" }));
        } else {
            println!("No active task");
            println!("Use 'noslop task start <id>' to start one first");
        }
        return Ok(());
    }

    let id = current_id.unwrap();
    let task = TaskRefs::get(&id)?.unwrap();

    // Use centralized complete logic (handles status and clearing current)
    TaskRefs::complete(&id)?;

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

fn block(id: &str, blocker_id: &str, mode: OutputMode) -> anyhow::Result<()> {
    // Verify blocker exists
    if TaskRefs::get(blocker_id)?.is_none() {
        if mode == OutputMode::Json {
            println!(
                "{}",
                serde_json::json!({ "success": false, "error": "Blocker task not found" })
            );
        } else {
            println!("Blocker task not found: {}", blocker_id);
        }
        return Ok(());
    }

    let found = TaskRefs::add_blocker(id, blocker_id)?;

    if mode == OutputMode::Json {
        println!(
            "{}",
            serde_json::json!({
                "success": found,
                "id": id,
                "blocked_by": blocker_id,
            })
        );
    } else if found {
        println!("{} now blocked by {}", id, blocker_id);
    } else {
        println!("Task not found: {}", id);
    }

    Ok(())
}

fn unblock(id: &str, blocker_id: &str, mode: OutputMode) -> anyhow::Result<()> {
    let found = TaskRefs::remove_blocker(id, blocker_id)?;

    if mode == OutputMode::Json {
        println!(
            "{}",
            serde_json::json!({
                "success": found,
                "id": id,
                "unblocked": blocker_id,
            })
        );
    } else if found {
        println!("{} no longer blocked by {}", id, blocker_id);
    } else {
        println!("Task not found: {}", id);
    }

    Ok(())
}
