//! Check command - manage checks

use std::path::Path;

use noslop::output::OutputMode;

use crate::cli::CheckAction;
use crate::noslop_file;

/// Handle check subcommands (add, list, remove)
pub fn check_cmd(action: CheckAction, _mode: OutputMode) -> anyhow::Result<()> {
    match action {
        CheckAction::Add {
            target,
            message,
            severity,
        } => add(&target, &message, &severity),
        CheckAction::List { target } => list(target.as_deref()),
        CheckAction::Remove { id } => remove(&id),
        // Run is handled directly in cli.rs before reaching here
        CheckAction::Run { .. } => unreachable!("Run is handled directly in cli.rs"),
    }
}

fn add(target: &str, message: &str, severity: &str) -> anyhow::Result<()> {
    let id = noslop_file::add_check(target, message, severity)?;

    println!("Added check to .noslop.toml");
    println!("  target: {}", target);
    println!("  message: {}", message);
    println!("  severity: {}", severity);
    println!("  id: {}", id);

    Ok(())
}

fn list(target: Option<&str>) -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let search_path = target.map(|t| cwd.join(t)).unwrap_or_else(|| cwd.clone());

    let noslop_files = noslop_file::find_noslop_files(&search_path);

    if noslop_files.is_empty() {
        println!("No .noslop.toml files found.");
        println!("Add a check with: noslop check add <target> -m \"message\"");
        return Ok(());
    }

    let mut total = 0;
    for path in &noslop_files {
        let file = noslop_file::load_file(path)?;
        let checks: Vec<_> = file.all_checks().collect();
        if checks.is_empty() {
            continue;
        }

        println!("{}:", path.display());
        for c in &checks {
            println!("  [{}] {} -> {}", c.severity.to_uppercase(), c.target, c.message);
            total += 1;
        }
        println!();
    }

    if total == 0 {
        println!("No checks defined.");
    } else {
        println!("{} check(s) found.", total);
    }

    Ok(())
}

fn remove(id: &str) -> anyhow::Result<()> {
    // ID format: ".noslop.toml:0" or "path/to/.noslop.toml:2"
    let parts: Vec<&str> = id.rsplitn(2, ':').collect();
    if parts.len() != 2 {
        anyhow::bail!("Invalid ID format. Expected: <file>:<index>");
    }

    let index: usize = parts[0].parse().map_err(|_| anyhow::anyhow!("Invalid index in ID"))?;
    let file_path = Path::new(parts[1]);

    if !file_path.exists() {
        anyhow::bail!("File not found: {}", file_path.display());
    }

    let file = noslop_file::load_file(file_path)?;
    let checks: Vec<_> = file.all_checks().collect();
    if index >= checks.len() {
        anyhow::bail!(
            "Index {} out of range (file has {} checks)",
            index,
            checks.len()
        );
    }

    let removed_message = checks[index].message.clone();

    // Rebuild checks without the removed one
    let new_checks: Vec<_> = checks
        .into_iter()
        .enumerate()
        .filter(|(i, _)| *i != index)
        .map(|(_, c)| c.clone())
        .collect();

    // Rewrite file
    let content = format_noslop_file(&file.project.prefix, &new_checks);
    std::fs::write(file_path, content)?;

    println!("Removed check: {}", removed_message);
    Ok(())
}

fn format_noslop_file(prefix: &str, checks: &[noslop_file::CheckEntry]) -> String {
    let mut out = String::new();
    out.push_str("# noslop checks\n\n");

    // Add project config if prefix is not default
    if prefix != "NOS" {
        out.push_str("[project]\n");
        out.push_str(&format!("prefix = \"{}\"\n\n", prefix));
    }

    for entry in checks {
        out.push_str("[[check]]\n");
        if let Some(id) = &entry.id {
            out.push_str(&format!("id = \"{}\"\n", id));
        }
        out.push_str(&format!("target = \"{}\"\n", entry.target));
        out.push_str(&format!("message = \"{}\"\n", entry.message));
        out.push_str(&format!("severity = \"{}\"\n", entry.severity));
        if !entry.tags.is_empty() {
            out.push_str(&format!("tags = {:?}\n", entry.tags));
        }
        out.push('\n');
    }

    out
}
