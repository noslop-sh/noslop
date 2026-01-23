//! .noslop.toml file loading
//!
//! Scans for .noslop.toml files from a path up to the repo root,
//! collecting all checks that apply.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::models::{Check, Severity};
use crate::paths;

/// A .noslop.toml file structure
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct NoslopFile {
    /// Project configuration
    #[serde(default)]
    pub project: ProjectConfig,

    /// Agent configuration for multi-agent workflows
    #[serde(default)]
    pub agent: AgentConfig,

    /// Checks defined in this file via [[check]] sections
    #[serde(default, rename = "check")]
    pub checks: Vec<CheckEntry>,

    /// Topics defined in this file via [[topic]] sections
    #[serde(default, rename = "topic")]
    pub topics: Vec<TopicEntry>,
}

impl NoslopFile {
    /// Get all checks
    pub fn all_checks(&self) -> impl Iterator<Item = &CheckEntry> {
        self.checks.iter()
    }

    /// Get all topics
    pub fn all_topics(&self) -> impl Iterator<Item = &TopicEntry> {
        self.topics.iter()
    }

    /// Get a topic by ID
    #[must_use]
    pub fn get_topic(&self, id: &str) -> Option<&TopicEntry> {
        self.topics.iter().find(|c| c.id == id)
    }

    /// Get a mutable topic by ID
    pub fn get_topic_mut(&mut self, id: &str) -> Option<&mut TopicEntry> {
        self.topics.iter_mut().find(|c| c.id == id)
    }
}

/// Project-level configuration
#[derive(Debug, Deserialize, Serialize)]
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
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CheckEntry {
    /// Optional custom ID (if not provided, will be auto-generated as PREFIX-N)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Scope pattern (glob or path) - accepts both "scope" and legacy "target" in TOML
    #[serde(alias = "target")]
    pub scope: String,
    /// The check message
    pub message: String,
    /// Severity: info, warn, block
    #[serde(default = "default_severity")]
    pub severity: String,
    /// Optional tags
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

fn default_severity() -> String {
    "block".to_string()
}

/// A topic entry in .noslop.toml
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TopicEntry {
    /// Topic ID (e.g., "TOP-1")
    pub id: String,
    /// Topic name
    pub name: String,
    /// Description providing context for LLMs
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Scope patterns (files/directories this topic applies to)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scope: Vec<String>,
    /// When created (RFC3339)
    pub created_at: String,
}

/// Agent configuration for multi-agent workflows
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct AgentConfig {
    /// Command to run for the AI agent (e.g., "claude", "codex", "cursor")
    pub command: String,

    /// Whether to use tmux for session management
    pub use_tmux: bool,

    /// Additional arguments to pass to the agent command
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,

    /// Auto-assign next task when spawning an agent
    #[serde(default = "default_auto_assign")]
    pub auto_assign: bool,
}

const fn default_auto_assign() -> bool {
    true
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            command: "claude".to_string(),
            use_tmux: true,
            args: Vec::new(),
            auto_assign: true,
        }
    }
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
                if matches_scope(&entry.scope, file, noslop_dir, &cwd) {
                    let check = Check::new(
                        entry.id.clone(),
                        entry.scope.clone(),
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

/// Check if a scope pattern matches a file
fn matches_scope(scope: &str, file: &str, noslop_dir: &Path, cwd: &Path) -> bool {
    // Get relative path from noslop_dir
    let file_abs = cwd.join(file);
    let file_rel = file_abs
        .strip_prefix(noslop_dir)
        .map_or_else(|_| file.to_string(), |p| p.to_string_lossy().to_string());

    // Simple matching for now
    if scope == "*" {
        return true;
    }

    // Glob-style: *.rs matches any .rs file
    if scope.starts_with("*.") {
        let ext = &scope[1..]; // ".rs"
        return file_rel.ends_with(ext);
    }

    // Glob-style: dir/*.ext matches files in dir with extension
    if let Some(star_pos) = scope.find("/*") {
        let prefix = &scope[..=star_pos]; // "src/"
        let suffix = &scope[star_pos + 2..]; // ".rs" or ""

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
    if let Some(doublestar_pos) = scope.find("/**") {
        let prefix = &scope[..=doublestar_pos]; // "src/"
        let suffix = scope.strip_suffix(".rs").map_or("", |_| ".rs");

        if (file_rel.starts_with(prefix) || file.starts_with(prefix)) && file_rel.ends_with(suffix)
        {
            return true;
        }
    }

    // Exact or prefix match
    file_rel == scope || file_rel.starts_with(scope) || file.contains(scope)
}

/// Load the noslop file or create a default one
#[must_use]
pub fn load_or_default() -> NoslopFile {
    let path = paths::noslop_toml();
    if path.exists() {
        load_file(&path).unwrap_or_default()
    } else {
        NoslopFile::default()
    }
}

/// Save the noslop file
pub fn save_file(file: &NoslopFile) -> anyhow::Result<()> {
    let path = paths::noslop_toml();
    let content = format_noslop_file(file);
    fs::write(&path, content)?;
    Ok(())
}

/// Create or update a .noslop.toml file with a new check
pub fn add_check(scope: &str, message: &str, severity: &str) -> anyhow::Result<String> {
    let path = paths::noslop_toml();

    let mut file = if path.exists() {
        load_file(&path)?
    } else {
        NoslopFile::default()
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
        scope: scope.to_string(),
        message: message.to_string(),
        severity: severity.to_string(),
        tags: Vec::new(),
    };

    file.checks.push(entry);

    // Write back using new [[check]] format
    let content = format_noslop_file(&file);
    fs::write(&path, content)?;

    Ok(generated_id)
}

/// Format a `NoslopFile` as TOML
fn format_noslop_file(file: &NoslopFile) -> String {
    use std::fmt::Write as _;

    let mut out = String::new();
    out.push_str("# noslop configuration\n\n");

    // Add project config if prefix is not default
    if file.project.prefix != "NOS" {
        out.push_str("[project]\n");
        let _ = writeln!(out, "prefix = \"{}\"", file.project.prefix);
        out.push('\n');
    }

    // Add agent config if non-default
    let default_agent = AgentConfig::default();
    if file.agent.command != default_agent.command
        || file.agent.use_tmux != default_agent.use_tmux
        || !file.agent.args.is_empty()
        || file.agent.auto_assign != default_agent.auto_assign
    {
        out.push_str("[agent]\n");
        if file.agent.command != default_agent.command {
            let _ = writeln!(out, "command = \"{}\"", file.agent.command);
        }
        if file.agent.use_tmux != default_agent.use_tmux {
            let _ = writeln!(out, "use_tmux = {}", file.agent.use_tmux);
        }
        if !file.agent.args.is_empty() {
            let _ = writeln!(out, "args = {:?}", file.agent.args);
        }
        if file.agent.auto_assign != default_agent.auto_assign {
            let _ = writeln!(out, "auto_assign = {}", file.agent.auto_assign);
        }
        out.push('\n');
    }

    // Write checks
    for entry in file.all_checks() {
        out.push_str("[[check]]\n");
        if let Some(id) = &entry.id {
            let _ = writeln!(out, "id = \"{id}\"");
        }
        let _ = writeln!(out, "scope = \"{}\"", entry.scope);
        let _ = writeln!(out, "message = \"{}\"", entry.message);
        let _ = writeln!(out, "severity = \"{}\"", entry.severity);
        if !entry.tags.is_empty() {
            let _ = writeln!(out, "tags = {:?}", entry.tags);
        }
        out.push('\n');
    }

    // Write topics
    for entry in file.all_topics() {
        out.push_str("[[topic]]\n");
        let _ = writeln!(out, "id = \"{}\"", entry.id);
        let _ = writeln!(out, "name = \"{}\"", entry.name);
        if let Some(desc) = &entry.description {
            let _ = writeln!(out, "description = \"{desc}\"");
        }
        if !entry.scope.is_empty() {
            let _ = writeln!(out, "scope = {:?}", entry.scope);
        }
        let _ = writeln!(out, "created_at = \"{}\"", entry.created_at);
        out.push('\n');
    }

    out
}

// =============================================================================
// TOPIC OPERATIONS
// =============================================================================

/// Create a new topic, returns the topic ID
pub fn create_topic(name: &str, description: Option<&str>) -> anyhow::Result<String> {
    let mut file = load_or_default();

    // Generate next ID
    let max_num = file
        .topics
        .iter()
        .filter_map(|c| c.id.strip_prefix("TOP-").and_then(|n| n.parse::<u32>().ok()))
        .max()
        .unwrap_or(0);

    let id = format!("TOP-{}", max_num + 1);

    file.topics.push(TopicEntry {
        id: id.clone(),
        name: name.to_string(),
        description: description.map(String::from),
        scope: Vec::new(),
        created_at: chrono::Utc::now().to_rfc3339(),
    });

    save_file(&file)?;
    Ok(id)
}

/// Update a topic's description
pub fn update_topic_description(id: &str, description: Option<&str>) -> anyhow::Result<bool> {
    let mut file = load_or_default();
    if let Some(topic) = file.get_topic_mut(id) {
        topic.description = description.map(String::from);
        save_file(&file)?;
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Delete a topic by ID, returns true if found and deleted
pub fn delete_topic(id: &str) -> anyhow::Result<bool> {
    let mut file = load_or_default();
    let len_before = file.topics.len();
    file.topics.retain(|c| c.id != id);

    if file.topics.len() < len_before {
        save_file(&file)?;

        // Clear current topic if it was the deleted one
        if current_topic()?.as_deref() == Some(id) {
            set_current_topic(None)?;
        }
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Get the current topic ID (from .noslop/current-topic)
pub fn current_topic() -> anyhow::Result<Option<String>> {
    let path = paths::current_topic_file();
    if !path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(&path)?;
    let id = content.trim();
    if id.is_empty() {
        Ok(None)
    } else {
        Ok(Some(id.to_string()))
    }
}

/// Set the current topic (None to clear)
pub fn set_current_topic(id: Option<&str>) -> anyhow::Result<()> {
    let path = paths::current_topic_file();
    if let Some(id) = id {
        // Ensure parent dir exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, format!("{id}\n"))?;
    } else if path.exists() {
        fs::remove_file(&path)?;
    }
    Ok(())
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
