//! TOML writer for .noslop.toml files
//!
//! Handles creating and modifying noslop configuration files.

use std::fmt::Write;
use std::fs;
use std::path::Path;

use super::parser::{CheckEntry, NoslopFile, ProjectConfig, load_file};

/// Create or update a .noslop.toml file with a new check
///
/// # Errors
///
/// Returns an error if the file cannot be read or written.
pub fn add_check(target: &str, message: &str, severity: &str) -> anyhow::Result<String> {
    let path = Path::new(".noslop.toml");

    let mut file = if path.exists() {
        load_file(path)?
    } else {
        NoslopFile {
            project: ProjectConfig::default(),
            checks: Vec::new(),
        }
    };

    // Calculate next ID number (max existing ID + 1)
    let next_num = file
        .checks
        .iter()
        .filter_map(|c| c.id.as_ref())
        .filter_map(|id| {
            // Parse PREFIX-123 format
            id.split('-').nth(1).and_then(|n| n.parse::<u32>().ok())
        })
        .max()
        .map_or(1, |n| n + 1);

    // Generate JIRA-style ID
    let generated_id = format!("{}-{}", file.project.prefix, next_num);

    let entry = CheckEntry {
        id: Some(generated_id.clone()),
        target: target.to_string(),
        message: message.to_string(),
        severity: severity.to_string(),
        tags: Vec::new(),
    };

    file.checks.push(entry);

    // Write back
    let content = format_noslop_file(&file);
    fs::write(path, content)?;

    Ok(generated_id)
}

/// Format a `NoslopFile` as TOML
#[must_use]
pub fn format_noslop_file(file: &NoslopFile) -> String {
    let mut out = String::new();
    out.push_str("# noslop checks\n\n");

    // Add project config if prefix is not default
    if file.project.prefix != "CHK" {
        out.push_str("[project]\n");
        let _ = writeln!(out, "prefix = \"{}\"", file.project.prefix);
        out.push('\n');
    }

    for entry in &file.checks {
        out.push_str("[[check]]\n");
        if let Some(id) = &entry.id {
            let _ = writeln!(out, "id = \"{id}\"");
        }
        let _ = writeln!(out, "target = \"{}\"", entry.target);
        let _ = writeln!(out, "message = \"{}\"", entry.message);
        let _ = writeln!(out, "severity = \"{}\"", entry.severity);
        if !entry.tags.is_empty() {
            let _ = writeln!(out, "tags = {:?}", entry.tags);
        }
        out.push('\n');
    }

    out
}

/// Generate a 3-letter prefix from git repository name
///
/// Examples: "noslop" -> "NOS", "my-awesome-project" -> "MAP"
#[must_use]
pub fn generate_prefix_from_repo(repo_name: &str) -> String {
    generate_3_letter_prefix(repo_name)
}

/// Generate a 3-letter acronym from a project name
///
/// Examples:
/// - "noslop" -> "NSL"
/// - "my-awesome-project" -> "MAP"
/// - "ab" -> "AB"
fn generate_3_letter_prefix(name: &str) -> String {
    // Remove special chars and split by separators
    let words: Vec<&str> =
        name.split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()).collect();

    let prefix = if words.len() >= 3 {
        // Take first letter of first 3 words
        words.iter().take(3).filter_map(|w| w.chars().next()).collect()
    } else if words.len() == 2 {
        // Two words: first letter of each + second letter of first
        let mut chars: Vec<char> = words.iter().filter_map(|w| w.chars().next()).collect();
        if let Some(second) = words[0].chars().nth(1) {
            chars.insert(1, second);
        }
        chars.into_iter().collect()
    } else if !words.is_empty() {
        // Single word: take first 3 letters
        words[0].chars().take(3).collect()
    } else {
        "CHK".to_string() // Fallback
    };

    prefix.to_uppercase()
}
