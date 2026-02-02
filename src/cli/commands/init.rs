//! Initialize noslop in a repository

use std::fs;
use std::path::Path;

use crate::{git, noslop_file};
use noslop::output::OutputMode;

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

    // Create .noslop.toml with project config and example assertion
    let noslop_toml = format!(
        r#"# noslop assertions

[project]
prefix = "{prefix}"

# Assertions are auto-assigned IDs like {prefix}-1, {prefix}-2, etc.
# You can also specify custom IDs:
#   id = "my-custom-id"

# Example assertion (uncomment to use):
# [[assert]]
# target = "*.rs"
# message = "Consider impact on public API"
# severity = "warn"
"#
    );
    fs::write(noslop_path, noslop_toml)?;
    println!("  Created .noslop.toml");

    // Create .noslop/ for attestations (pending until committed)
    fs::create_dir_all(".noslop")?;
    fs::write(".noslop/.gitkeep", "")?;
    println!("  Created .noslop/");

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
        println!("  Run 'noslop assert import' to extract assertions");
    }

    println!("\nnoslop initialized!");
    println!("\nNext steps:");
    println!("  noslop assert add <target> -m \"assertion\"");
    println!("  git commit  # assertions will be checked");

    Ok(())
}
