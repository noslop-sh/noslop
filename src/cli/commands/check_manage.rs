//! Check management command - add, list, remove checks

use std::path::Path;

use crate::cli::app::CheckAction;
use crate::noslop_file;
use noslop::output::OutputMode;

/// Handle check management subcommands
pub fn check_manage(action: CheckAction, _mode: OutputMode) -> anyhow::Result<()> {
    match action {
        CheckAction::Add {
            target,
            message,
            severity,
        } => add(&target, &message, &severity),
        CheckAction::List { target } => list(target.as_deref()),
        CheckAction::Remove { id } => remove(&id),
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
        if file.checks.is_empty() {
            continue;
        }

        println!("{}:", path.display());
        for c in file.checks.iter() {
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

    let mut file = noslop_file::load_file(file_path)?;
    if index >= file.checks.len() {
        anyhow::bail!("Index {} out of range (file has {} checks)", index, file.checks.len());
    }

    let removed = file.checks.remove(index);

    // Rewrite file
    let content = format_noslop_file(&file);
    std::fs::write(file_path, content)?;

    println!("Removed check: {}", removed.message);
    Ok(())
}

fn format_noslop_file(file: &noslop_file::NoslopFile) -> String {
    let mut out = String::new();
    out.push_str("# noslop checks\n\n");

    for entry in &file.checks {
        out.push_str("[[check]]\n");
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
