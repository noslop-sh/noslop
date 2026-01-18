//! Clear staged verifications and pending task trailers
//!
//! This command is called by the post-commit hook to remove
//! staged verifications and pending task trailers after they've
//! been added to the commit.

use noslop::paths;
use noslop::storage::TaskRefs;

/// Clear staged verifications and pending task trailers
///
/// Called by post-commit hook to:
/// 1. Delete .noslop/staged-verifications.json
/// 2. Clear pending_trailer field from all tasks
/// 3. Auto-enrich current task's scope with committed files
///
/// This runs after the commit has been created with all trailers.
pub fn clear_staged() -> anyhow::Result<()> {
    // Delete staged verifications file
    let verifications_file = paths::staged_verifications();
    if verifications_file.exists() {
        std::fs::remove_file(verifications_file)?;
    }

    // Clear pending task trailers
    TaskRefs::clear_all_pending_trailers()?;

    // Auto-enrich current task's scope with committed files
    auto_enrich_task_scope()?;

    Ok(())
}

/// Auto-enrich the current task's scope with files from the latest commit
fn auto_enrich_task_scope() -> anyhow::Result<()> {
    // Only enrich if there's a current task
    let Some(current_id) = TaskRefs::current()? else {
        return Ok(());
    };

    // Get files from the latest commit
    let committed_files = get_last_commit_files()?;

    // Add each file to the task's scope
    for file in committed_files {
        TaskRefs::add_scope(&current_id, &file)?;
    }

    Ok(())
}

/// Get the list of files modified in the last commit
fn get_last_commit_files() -> anyhow::Result<Vec<String>> {
    // Use git diff-tree to get files from HEAD
    let output = std::process::Command::new("git")
        .args(["diff-tree", "--no-commit-id", "--name-only", "-r", "HEAD"])
        .output()?;

    if !output.status.success() {
        return Ok(Vec::new());
    }

    let files = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|line| !line.is_empty())
        .map(String::from)
        .collect();

    Ok(files)
}
