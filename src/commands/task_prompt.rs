//! Task prompt command - interactive prompt for task status at commit time
//!
//! Called by the pre-commit hook after checks pass to ask the user
//! what this commit does to the current task.

use std::env;
use std::io::{self, BufRead, Write};

use noslop::storage::TaskRefs;

/// Task action chosen by user
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskAction {
    /// Task is complete with this commit
    Done,
    /// Still working on task (partial progress)
    Continue,
    /// Commit is unrelated to task
    Skip,
}

impl std::str::FromStr for TaskAction {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().trim() {
            "d" | "done" | "complete" | "finished" => Ok(Self::Done),
            "c" | "continue" | "partial" | "wip" | "" => Ok(Self::Continue), // empty = default
            "s" | "skip" | "unrelated" | "none" => Ok(Self::Skip),
            _ => Err(format!(
                "Invalid action: {s}. Use: done, continue, skip (or d/c/s)"
            )),
        }
    }
}

/// Prompt user for task status at commit time
///
/// Returns Ok(true) if a task was prompted for, Ok(false) if no current task.
pub fn task_prompt() -> anyhow::Result<bool> {
    // Check if there's a current task
    let current_id = TaskRefs::current()?;
    let id = match current_id {
        Some(id) => id,
        None => return Ok(false), // No current task, nothing to prompt
    };

    // Get task details
    let task = match TaskRefs::get(&id)? {
        Some(t) => t,
        None => {
            // Task file missing, clear HEAD
            TaskRefs::clear_current()?;
            return Ok(false);
        }
    };

    // Check for environment variable override (for CI)
    if let Ok(action_str) = env::var("NOSLOP_TASK_ACTION") {
        let action: TaskAction = action_str
            .parse()
            .map_err(|e: String| anyhow::anyhow!(e))?;
        apply_action(&id, &task.title, action)?;
        return Ok(true);
    }

    // Check if running in non-interactive mode
    if !is_interactive() {
        // Non-interactive: default to continue (safe default)
        TaskRefs::set_pending_trailer(&id, "partial")?;
        eprintln!("  (non-interactive: continuing task {})", id);
        return Ok(true);
    }

    // Show the prompt
    print_prompt(&task.title);

    // Read user input
    let stdin = io::stdin();
    let mut input = String::new();
    stdin.lock().read_line(&mut input)?;

    // Parse action (empty defaults to continue)
    let action: TaskAction = input
        .parse()
        .map_err(|e: String| anyhow::anyhow!(e))?;

    apply_action(&id, &task.title, action)?;

    Ok(true)
}

/// Check if stdin is interactive (TTY)
fn is_interactive() -> bool {
    // Simple check: try to detect if stdin is a terminal
    // In real impl, use atty crate or similar
    env::var("NOSLOP_INTERACTIVE").map(|v| v != "0").unwrap_or(true)
        && env::var("CI").is_err()
}

/// Print the descriptive task prompt
fn print_prompt(task_title: &str) {
    eprintln!();
    eprintln!("\x1b[1m━━━ Task in progress ━━━━━━━━━━━━━━━━━━━━━\x1b[0m");
    eprintln!("  {}", task_title);
    eprintln!();
    eprintln!("What does this commit do?");
    eprintln!("  \x1b[1m[d]\x1b[0mone     \x1b[2m→\x1b[0m Mark task complete");
    eprintln!("  \x1b[1m[c]\x1b[0montinue \x1b[2m→\x1b[0m Still working on it");
    eprintln!("  \x1b[1m[s]\x1b[0mkip     \x1b[2m→\x1b[0m Unrelated to this task");
    eprintln!();
    eprint!("Choice \x1b[2m(default: c)\x1b[0m: ");
    io::stderr().flush().ok();
}

/// Apply the chosen action
fn apply_action(id: &str, title: &str, action: TaskAction) -> anyhow::Result<()> {
    match action {
        TaskAction::Done => {
            // Mark task as done, set pending trailer, clear HEAD
            TaskRefs::set_status(id, "done")?;
            TaskRefs::set_pending_trailer(id, "done")?;
            TaskRefs::clear_current()?;
            eprintln!("  \x1b[32m✓\x1b[0m Completing: {} - {}", id, title);
        }
        TaskAction::Continue => {
            // Set pending trailer for partial progress
            TaskRefs::set_pending_trailer(id, "partial")?;
            eprintln!("  \x1b[33m→\x1b[0m Continuing: {} - {}", id, title);
        }
        TaskAction::Skip => {
            // No trailer, task stays in progress
            eprintln!("  \x1b[2m○\x1b[0m Skipped task prompt");
        }
    }
    Ok(())
}
