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
            agent: None,
            review: None,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::toml::parser::{CheckEntry, NoslopFile, ProjectConfig};

    // --- generate_3_letter_prefix tests ---

    #[test]
    fn prefix_single_word() {
        assert_eq!(generate_3_letter_prefix("noslop"), "NOS");
    }

    #[test]
    fn prefix_two_words() {
        let prefix = generate_3_letter_prefix("my-project");
        assert_eq!(prefix.len(), 3);
        // Two words: first letter of each + second letter of first word
        // m,y -> m, o -> wait: "my" "project" -> [m,p] then insert y at pos 1 -> [m,y,p] -> "MYP"
        assert_eq!(prefix, "MYP");
    }

    #[test]
    fn prefix_three_words() {
        assert_eq!(generate_3_letter_prefix("my-awesome-project"), "MAP");
    }

    #[test]
    fn prefix_more_than_three_words() {
        // Only takes first 3 words
        assert_eq!(generate_3_letter_prefix("the-very-awesome-project"), "TVA");
    }

    #[test]
    fn prefix_short_single_word() {
        // "ab" -> only 2 chars -> "AB"
        assert_eq!(generate_3_letter_prefix("ab"), "AB");
    }

    #[test]
    fn prefix_single_char_word() {
        assert_eq!(generate_3_letter_prefix("x"), "X");
    }

    #[test]
    fn prefix_empty_string() {
        assert_eq!(generate_3_letter_prefix(""), "CHK");
    }

    #[test]
    fn prefix_only_separators() {
        assert_eq!(generate_3_letter_prefix("---"), "CHK");
    }

    #[test]
    fn prefix_uppercase_input() {
        assert_eq!(generate_3_letter_prefix("MyProject"), "MYP");
    }

    #[test]
    fn prefix_underscore_separator() {
        assert_eq!(generate_3_letter_prefix("my_awesome_project"), "MAP");
    }

    #[test]
    fn generate_prefix_from_repo_delegates() {
        assert_eq!(generate_prefix_from_repo("noslop"), "NOS");
    }

    // --- format_noslop_file tests ---

    #[test]
    fn format_empty_file_default_prefix() {
        let file = NoslopFile {
            project: ProjectConfig::default(), // "CHK"
            agent: None,
            review: None,
            checks: Vec::new(),
        };
        let out = format_noslop_file(&file);
        assert!(out.starts_with("# noslop checks"));
        // Default prefix "CHK" should not emit [project] section
        assert!(!out.contains("[project]"));
    }

    #[test]
    fn format_file_custom_prefix() {
        let file = NoslopFile {
            project: ProjectConfig {
                prefix: "NOS".to_string(),
                next_id: 1,
            },
            agent: None,
            review: None,
            checks: Vec::new(),
        };
        let out = format_noslop_file(&file);
        assert!(out.contains("[project]"));
        assert!(out.contains("prefix = \"NOS\""));
    }

    #[test]
    fn format_file_with_checks() {
        let file = NoslopFile {
            project: ProjectConfig {
                prefix: "TST".to_string(),
                next_id: 2,
            },
            agent: None,
            review: None,
            checks: vec![
                CheckEntry {
                    id: Some("TST-1".to_string()),
                    target: "*.rs".to_string(),
                    message: "Review Rust".to_string(),
                    severity: "block".to_string(),
                    tags: vec![],
                },
                CheckEntry {
                    id: None,
                    target: "*.py".to_string(),
                    message: "Review Python".to_string(),
                    severity: "warn".to_string(),
                    tags: vec!["security".to_string()],
                },
            ],
        };
        let out = format_noslop_file(&file);
        assert!(out.contains("[[check]]"));
        assert!(out.contains("id = \"TST-1\""));
        assert!(out.contains("target = \"*.rs\""));
        assert!(out.contains("message = \"Review Rust\""));
        assert!(out.contains("severity = \"block\""));
        // Second check has no id
        assert!(out.contains("target = \"*.py\""));
        // Second check has tags
        assert!(out.contains("tags = "));
        assert!(out.contains("security"));
    }

    #[test]
    fn format_roundtrip_parses() {
        let file = NoslopFile {
            project: ProjectConfig {
                prefix: "RND".to_string(),
                next_id: 1,
            },
            agent: None,
            review: None,
            checks: vec![CheckEntry {
                id: Some("RND-1".to_string()),
                target: "src/**/*.rs".to_string(),
                message: "Review code".to_string(),
                severity: "block".to_string(),
                tags: vec![],
            }],
        };
        let out = format_noslop_file(&file);
        // Should parse back without error
        let parsed: NoslopFile = toml::from_str(&out).unwrap();
        assert_eq!(parsed.project.prefix, "RND");
        assert_eq!(parsed.checks.len(), 1);
        assert_eq!(parsed.checks[0].target, "src/**/*.rs");
    }
}
