//! Initialize noslop in a repository

use std::fs;
use std::path::Path;

use noslop::output::OutputMode;

use crate::{git, noslop_file};

/// Initialize noslop in the current repository
pub fn init(force: bool, _mode: OutputMode) -> anyhow::Result<()> {
    let noslop_path = Path::new(".noslop.toml");
    let _noslop_dir = Path::new(".noslop");

    if noslop_path.exists() && !force {
        println!("Already initialized (.noslop.toml exists).");
        println!("Use --force to reinitialize.");
        return Ok(());
    }

    println!("Initializing noslop...\n");

    // Generate project prefix from repo name
    let prefix = noslop_file::generate_prefix_from_repo();
    println!("  Generated project prefix: {prefix}");

    // Create .noslop.toml with project config and example check
    let noslop_toml = format!(
        r#"# noslop checks

[project]
prefix = "{prefix}"

# Checks are auto-assigned IDs like {prefix}-1, {prefix}-2, etc.
# You can also specify custom IDs:
#   id = "my-custom-id"

# Example check (uncomment to use):
# [[check]]
# target = "*.rs"
# message = "Consider impact on public API"
# severity = "warn"
"#
    );
    fs::write(noslop_path, noslop_toml)?;
    println!("  Created .noslop.toml");

    // Create .noslop/ for verifications and tasks
    fs::create_dir_all(".noslop")?;
    fs::create_dir_all(".noslop/refs/tasks")?;

    // Create .gitignore to exclude local task data
    // - refs/ contains task ref files (one file per task)
    // - HEAD contains current active task
    // - tasks/ is legacy but included for backwards compatibility
    fs::write(".noslop/.gitignore", "refs/\nHEAD\ntasks/\n")?;
    println!("  Created .noslop/");
    println!("  Created .noslop/refs/tasks/ (gitignored)");

    // Install git hooks
    git::hooks::install_pre_commit()?;
    println!("  Installed pre-commit hook");
    git::hooks::install_commit_msg()?;
    println!("  Installed commit-msg hook");
    git::hooks::install_post_commit()?;
    println!("  Installed post-commit hook");

    // Check for existing docs to bootstrap from
    let docs = ["CLAUDE.md", "AGENTS.md", "ARCHITECTURE.md"];
    let found: Vec<_> = docs.iter().filter(|d| Path::new(d).exists()).collect();

    if !found.is_empty() {
        let found_str: Vec<&str> = found.into_iter().copied().collect();
        println!("\n  Found: {}", found_str.join(", "));
        println!("  Run 'noslop check import' to extract checks");
    }

    println!("\nnoslop initialized!");
    println!("\nNext steps:");
    println!("  noslop check add <target> -m \"check message\"");
    println!("  git commit  # checks will be run");

    Ok(())
}
