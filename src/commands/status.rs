//! Status command - show overview of current noslop state

use noslop::noslop_file;
use noslop::output::OutputMode;
use noslop::paths;
use noslop::storage::TaskRefs;

/// Show current noslop status
pub fn status(output_mode: OutputMode) -> anyhow::Result<()> {
    // Get git info
    let branch = get_current_branch();

    // Load tasks from refs storage
    let tasks = TaskRefs::list().unwrap_or_default();
    let pending = tasks.iter().filter(|(_, t)| t.status == "pending").count();
    let in_progress = tasks.iter().filter(|(_, t)| t.status == "in_progress").count();
    let done = tasks.iter().filter(|(_, t)| t.status == "done").count();

    // Get current active task
    let current = TaskRefs::current().ok().flatten();

    // Load checks
    let checks = load_check_count();

    if output_mode == OutputMode::Json {
        let json = serde_json::json!({
            "branch": branch,
            "current_task": current,
            "tasks": {
                "total": tasks.len(),
                "pending": pending,
                "in_progress": in_progress,
                "done": done
            },
            "checks": checks
        });
        println!("{}", serde_json::to_string_pretty(&json)?);
    } else {
        println!("Branch: {}", branch.as_deref().unwrap_or("(not in git repo)"));
        println!();

        if let Some(task_id) = &current
            && let Some((_, task)) = tasks.iter().find(|(id, _)| id == task_id)
        {
            println!("Current task: {} - {}", task_id, task.title);
            println!();
        }

        if tasks.is_empty() {
            println!("Tasks: none");
        } else {
            println!("Tasks: {} total", tasks.len());
            if in_progress > 0 {
                println!("  • {} in progress", in_progress);
            }
            if pending > 0 {
                println!("  • {} pending", pending);
            }
            if done > 0 {
                println!("  • {} done", done);
            }
        }

        println!();
        println!("Checks: {}", checks);
    }

    Ok(())
}

fn get_current_branch() -> Option<String> {
    let repo = git2::Repository::discover(".").ok()?;
    let head = repo.head().ok()?;
    head.shorthand().map(String::from)
}

fn load_check_count() -> usize {
    let path = paths::noslop_toml();
    if !path.exists() {
        return 0;
    }

    noslop_file::load_file(&path).map(|f| f.checks.len()).unwrap_or(0)
}
