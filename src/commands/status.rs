//! Status command - show overview of current noslop state

use noslop::models::TaskStatus;
use noslop::output::OutputMode;
use noslop::storage::TaskStore;

use crate::noslop_file;

/// Show current noslop status
pub fn status(output_mode: OutputMode) -> anyhow::Result<()> {
    // Get git info
    let branch = get_current_branch();

    // Load tasks
    let tasks = TaskStore::load().unwrap_or_default();
    let pending = tasks.iter().filter(|t| t.status == TaskStatus::Pending).count();
    let in_progress = tasks.iter().filter(|t| t.status == TaskStatus::InProgress).count();
    let done = tasks.iter().filter(|t| t.status == TaskStatus::Done).count();
    let ready = tasks.iter().filter(|t| t.is_ready(&tasks)).count();

    // Load checks
    let checks = load_check_count();

    if output_mode == OutputMode::Json {
        let json = serde_json::json!({
            "branch": branch,
            "tasks": {
                "total": tasks.len(),
                "pending": pending,
                "in_progress": in_progress,
                "done": done,
                "ready": ready
            },
            "checks": checks
        });
        println!("{}", serde_json::to_string_pretty(&json)?);
    } else {
        println!("Branch: {}", branch.as_deref().unwrap_or("(not in git repo)"));
        println!();

        if tasks.is_empty() {
            println!("Tasks: none");
        } else {
            println!("Tasks: {} total", tasks.len());
            if in_progress > 0 {
                println!("  • {} in progress", in_progress);
            }
            if ready > 0 {
                println!("  • {} ready", ready);
            }
            if pending > ready {
                println!("  • {} blocked", pending - ready);
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
    let path = std::path::Path::new(".noslop.toml");
    if !path.exists() {
        return 0;
    }

    noslop_file::load_file(path).map(|f| f.checks.len()).unwrap_or(0)
}
