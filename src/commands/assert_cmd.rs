//! Assert command - manage assertions

use std::path::Path;

use noslop::output::OutputMode;

use crate::cli::AssertAction;
use crate::noslop_file;

/// Handle assert subcommands
pub fn assert_cmd(action: AssertAction, _mode: OutputMode) -> anyhow::Result<()> {
    match action {
        AssertAction::Add {
            target,
            message,
            severity,
        } => add(&target, &message, &severity),
        AssertAction::List { target } => list(target.as_deref()),
        AssertAction::Remove { id } => remove(&id),
    }
}

fn add(target: &str, message: &str, severity: &str) -> anyhow::Result<()> {
    let id = noslop_file::add_assertion(target, message, severity)?;

    println!("Added assertion to .noslop.toml");
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
        println!("Add an assertion with: noslop assert add <target> -m \"message\"");
        return Ok(());
    }

    let mut total = 0;
    for path in &noslop_files {
        let file = noslop_file::load_file(path)?;
        if file.assertions.is_empty() {
            continue;
        }

        println!("{}:", path.display());
        for a in file.assertions.iter() {
            println!("  [{}] {} -> {}", a.severity.to_uppercase(), a.target, a.message);
            total += 1;
        }
        println!();
    }

    if total == 0 {
        println!("No assertions defined.");
    } else {
        println!("{} assertion(s) found.", total);
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
    if index >= file.assertions.len() {
        anyhow::bail!(
            "Index {} out of range (file has {} assertions)",
            index,
            file.assertions.len()
        );
    }

    let removed = file.assertions.remove(index);

    // Rewrite file
    let content = format_noslop_file(&file);
    std::fs::write(file_path, content)?;

    println!("Removed assertion: {}", removed.message);
    Ok(())
}

fn format_noslop_file(file: &noslop_file::NoslopFile) -> String {
    let mut out = String::new();
    out.push_str("# noslop assertions\n\n");

    for entry in &file.assertions {
        out.push_str("[[assert]]\n");
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
