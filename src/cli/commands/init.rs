//! Initialize noslop in a repository

use std::fs;
use std::path::Path;

use crate::{git, noslop_file};
use noslop::output::OutputMode;

/// Initialize noslop in the current repository
///
/// Config (`.noslop.toml`) is per-repo and only written when absent (or
/// `--force`). Hooks and `.noslop/` are per-clone and are ALWAYS ensured —
/// a fresh clone of an initialized repo must still get its hooks.
pub fn init(force: bool, _mode: OutputMode) -> anyhow::Result<()> {
    let noslop_path = Path::new(".noslop.toml");

    println!("Initializing noslop...\n");

    if noslop_path.exists() && !force {
        println!("  Kept existing .noslop.toml (use --force to overwrite)");
    } else {
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
    }

    // Create .noslop/ for acknowledgments (pending until committed)
    fs::create_dir_all(".noslop")?;
    fs::write(".noslop/.gitkeep", "")?;
    println!("  Created .noslop/");

    // Staged acks and fire telemetry are per-clone state; the ledger and
    // history are tracked
    ensure_line(".gitignore", ".noslop/staged-acks.json")?;
    ensure_line(".gitignore", ".noslop/events.jsonl")?;
    println!("  Ensured .gitignore covers per-clone .noslop/ state");

    // Parallel branches both append to history.jsonl; union merge never conflicts
    ensure_line(".gitattributes", ".noslop/history.jsonl merge=union")?;
    println!("  Ensured .gitattributes union-merges .noslop/history.jsonl");

    // Install git hooks
    git::hooks::install_pre_commit()?;
    println!("  Installed pre-commit hook");
    git::hooks::install_commit_msg()?;
    println!("  Installed commit-msg hook");
    git::hooks::install_post_commit()?;
    println!("  Installed post-commit hook");

    println!("\nnoslop initialized!");
    println!("\nNext steps:");
    println!("  noslop check add <target> -m \"message\"");
    println!("  git commit  # checks will be validated");

    Ok(())
}

/// Append a line to a file unless it is already present
fn ensure_line(path: &str, line: &str) -> anyhow::Result<()> {
    let existing = if Path::new(path).exists() {
        fs::read_to_string(path)?
    } else {
        String::new()
    };

    if existing.lines().any(|l| l.trim() == line) {
        return Ok(());
    }

    let separator = if existing.is_empty() || existing.ends_with('\n') {
        ""
    } else {
        "\n"
    };
    fs::write(path, format!("{existing}{separator}{line}\n"))?;
    Ok(())
}
