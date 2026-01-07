//! .noslop.toml file loading
//!
//! Scans for .noslop.toml files from a path up to the repo root,
//! collecting all checks that apply.

use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::models::{Check, Severity};

/// A .noslop.toml file structure
#[derive(Debug, Deserialize)]
pub struct NoslopFile {
    /// Project configuration
    #[serde(default)]
    pub project: ProjectConfig,

    /// Checks defined in this file via [[check]] sections
    #[serde(default, rename = "check")]
    pub checks: Vec<CheckEntry>,
}

impl NoslopFile {
    /// Get all checks
    pub fn all_checks(&self) -> impl Iterator<Item = &CheckEntry> {
        self.checks.iter()
    }
}

/// Project-level configuration
#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct ProjectConfig {
    /// 3-letter prefix for check IDs (e.g., "NOS" for NOS-123)
    pub prefix: String,

    /// Next check ID number
    #[serde(skip)]
    pub next_id: u32,
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            prefix: "NOS".to_string(), // New default prefix
            next_id: 1,
        }
    }
}

/// A check entry in .noslop.toml
#[derive(Debug, Clone, Deserialize)]
pub struct CheckEntry {
    /// Optional custom ID (if not provided, will be auto-generated as PREFIX-N)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Target pattern (glob or path)
    pub target: String,
    /// The check message
    pub message: String,
    /// Severity: info, warn, block
    #[serde(default = "default_severity")]
    pub severity: String,
    /// Optional tags
    #[serde(default)]
    pub tags: Vec<String>,
}

fn default_severity() -> String {
    "block".to_string()
}

/// Find all .noslop.toml files from path up to repo root
///
/// # Panics
///
/// Panics if the file path has no parent directory
#[must_use]
pub fn find_noslop_files(from: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut current = from.to_path_buf();

    // Normalize: if it's a file, start from parent
    if current.is_file() {
        current = current.parent().unwrap_or(from).to_path_buf();
    }

    loop {
        let noslop_file = current.join(".noslop.toml");
        if noslop_file.exists() {
            files.push(noslop_file);
        }

        // Stop at repo root (.git) or filesystem root
        if current.join(".git").exists() || current.parent().is_none() {
            break;
        }

        current = current.parent().unwrap().to_path_buf();
    }

    // Reverse so root comes first (checks stack up)
    files.reverse();
    files
}

/// Load checks from a .noslop.toml file
pub fn load_file(path: &Path) -> anyhow::Result<NoslopFile> {
    let content = fs::read_to_string(path)?;
    let file: NoslopFile = toml::from_str(&content)?;
    Ok(file)
}

/// Load all checks applicable to a set of files
pub fn load_checks_for_files(files: &[String]) -> anyhow::Result<Vec<(Check, String)>> {
    let mut result = Vec::new();
    let cwd = std::env::current_dir()?;

    for file in files {
        let file_path = cwd.join(file);
        let noslop_files = find_noslop_files(&file_path);

        for noslop_path in noslop_files {
            let noslop_file = load_file(&noslop_path)?;
            let noslop_dir = noslop_path.parent().unwrap_or(&cwd);

            for entry in noslop_file.all_checks() {
                if matches_target(&entry.target, file, noslop_dir, &cwd) {
                    let check = Check::new(
                        entry.id.clone(),
                        entry.target.clone(),
                        entry.message.clone(),
                        entry.severity.parse().unwrap_or(Severity::Block),
                    );
                    result.push((check, file.clone()));
                }
            }
        }
    }

    // Dedupe by check message + file (same check might match from multiple noslop files)
    result.sort_by(|a, b| (&a.0.message, &a.1).cmp(&(&b.0.message, &b.1)));
    result.dedup_by(|a, b| a.0.message == b.0.message && a.1 == b.1);

    Ok(result)
}

/// Check if a target pattern matches a file
fn matches_target(target: &str, file: &str, noslop_dir: &Path, cwd: &Path) -> bool {
    // Get relative path from noslop_dir
    let file_abs = cwd.join(file);
    let file_rel = file_abs
        .strip_prefix(noslop_dir)
        .map_or_else(|_| file.to_string(), |p| p.to_string_lossy().to_string());

    // Simple matching for now
    if target == "*" {
        return true;
    }

    // Glob-style: *.rs matches any .rs file
    if target.starts_with("*.") {
        let ext = &target[1..]; // ".rs"
        return file_rel.ends_with(ext);
    }

    // Glob-style: dir/*.ext matches files in dir with extension
    if let Some(star_pos) = target.find("/*") {
        let prefix = &target[..=star_pos]; // "src/"
        let suffix = &target[star_pos + 2..]; // ".rs" or ""

        // File must start with prefix
        if !file_rel.starts_with(prefix) && !file.starts_with(prefix) {
            return false;
        }

        // Get the part after prefix
        let remainder = file_rel
            .strip_prefix(prefix)
            .or_else(|| file.strip_prefix(prefix))
            .unwrap_or("");

        // For /*.ext, remainder must not contain / (direct children only) and must end with ext
        if !remainder.contains('/') {
            if suffix.is_empty() || suffix == "*" {
                return true;
            }
            if suffix.starts_with('.') {
                return remainder.ends_with(suffix);
            }
        }
        return false;
    }

    // Glob-style: dir/**/*.ext matches all files recursively
    if let Some(doublestar_pos) = target.find("/**") {
        let prefix = &target[..=doublestar_pos]; // "src/"
        let suffix = target.strip_suffix(".rs").map_or("", |_| ".rs");

        if (file_rel.starts_with(prefix) || file.starts_with(prefix)) && file_rel.ends_with(suffix)
        {
            return true;
        }
    }

    // Exact or prefix match
    file_rel == target || file_rel.starts_with(target) || file.contains(target)
}

/// Create or update a .noslop.toml file with a new check
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
        .all_checks()
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

    // Write back using new [[check]] format
    let content = format_noslop_file(&file);
    fs::write(path, content)?;

    Ok(generated_id)
}

/// Format a `NoslopFile` as TOML (using new [[check]] format)
fn format_noslop_file(file: &NoslopFile) -> String {
    use std::fmt::Write as _;

    let mut out = String::new();
    out.push_str("# noslop checks\n\n");

    // Add project config if prefix is not default
    if file.project.prefix != "NOS" {
        out.push_str("[project]\n");
        let _ = writeln!(out, "prefix = \"{}\"", file.project.prefix);
        out.push('\n');
    }

    for entry in file.all_checks() {
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
/// Examples: "noslop" -> "NOS", "my-awesome-project" -> "MAP"
#[must_use]
pub fn generate_prefix_from_repo() -> String {
    use crate::git;
    let repo_name = git::get_repo_name();
    generate_3_letter_prefix(&repo_name)
}

/// Generate a 3-letter acronym from a project name
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
        "NOS".to_string() // Fallback
    };

    prefix.to_uppercase()
}
