//! TOML parser for .noslop.toml files
//!
//! Handles reading and deserializing noslop configuration files.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// A .noslop.toml file structure
#[derive(Debug, Deserialize)]
pub struct NoslopFile {
    /// Project configuration
    #[serde(default)]
    pub project: ProjectConfig,

    /// Agent configuration (optional)
    #[serde(default)]
    pub agent: Option<AgentSection>,

    /// Review pipeline configuration (optional)
    #[serde(default)]
    pub review: Option<ReviewSection>,

    /// Checks in this file
    #[serde(default, rename = "check")]
    pub checks: Vec<CheckEntry>,
}

/// Agent configuration section (`[agent]`)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AgentSection {
    /// Agent type: `"claude"` or `"codex"`
    #[serde(rename = "type")]
    pub agent_type: String,
}

/// Review pipeline configuration section (`[review]`)
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct ReviewSection {
    /// Ordered list of analyzer names for the pipeline.
    pub analyzers: Vec<String>,

    /// Catch-all for `agent:*` and `custom:*` sub-tables.
    #[serde(flatten)]
    pub analyzer_configs: HashMap<String, toml::Value>,
}

impl Default for ReviewSection {
    fn default() -> Self {
        Self {
            analyzers: vec!["conventions".to_string()],
            analyzer_configs: HashMap::new(),
        }
    }
}

/// Project-level configuration
#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct ProjectConfig {
    /// 3-letter prefix for check IDs (e.g., "NSL" for noslop-123)
    pub prefix: String,

    /// Next check ID number
    #[serde(skip)]
    pub next_id: u32,
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            prefix: "CHK".to_string(), // Fallback if not set
            next_id: 1,
        }
    }
}

/// A check entry in .noslop.toml
#[derive(Debug, Deserialize)]
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
        if current.join(".git").exists() {
            break;
        }

        // Move to parent directory, or stop if at filesystem root
        if let Some(parent) = current.parent() {
            current = parent.to_path_buf();
        } else {
            break;
        }
    }

    // Reverse so root comes first (checks stack up)
    files.reverse();
    files
}

/// Load checks from a .noslop.toml file
///
/// # Errors
///
/// Returns an error if the file cannot be read or parsed.
pub fn load_file(path: &Path) -> anyhow::Result<NoslopFile> {
    let content = fs::read_to_string(path)?;
    let file: NoslopFile = toml::from_str(&content)?;
    Ok(file)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_minimal_config() {
        let toml_str = r#"
[project]
prefix = "TST"
"#;
        let file: NoslopFile = toml::from_str(toml_str).unwrap();
        assert_eq!(file.project.prefix, "TST");
        assert!(file.agent.is_none());
        assert!(file.review.is_none());
        assert!(file.checks.is_empty());
    }

    #[test]
    fn parse_with_agent_section() {
        let toml_str = r#"
[project]
prefix = "TST"

[agent]
type = "claude"
"#;
        let file: NoslopFile = toml::from_str(toml_str).unwrap();
        let agent = file.agent.unwrap();
        assert_eq!(agent.agent_type, "claude");
    }

    #[test]
    fn parse_with_review_section() {
        let toml_str = r#"
[project]
prefix = "TST"

[review]
analyzers = ["conventions", "formatting"]
"#;
        let file: NoslopFile = toml::from_str(toml_str).unwrap();
        let review = file.review.unwrap();
        assert_eq!(review.analyzers.len(), 2);
        assert!(review.analyzers.contains(&"conventions".to_string()));
        assert!(review.analyzers.contains(&"formatting".to_string()));
    }

    #[test]
    fn parse_full_config() {
        let toml_str = r#"
[project]
prefix = "NOS"

[agent]
type = "claude"

[review]
analyzers = ["conventions", "formatting", "co-change", "agent:security", "agent:quality"]

[review.co-change]
min_correlation = 0.8
lookback = 300
min_commits = 5

[review.formatting]
tools = ["rustfmt", "prettier"]

[review."agent:security"]
agent = "claude"
prompt_template = ".noslop/prompts/security.md"

[review."agent:quality"]
agent = "claude"

[[check]]
target = "*.rs"
message = "Review Rust code"
severity = "block"
"#;
        let file: NoslopFile = toml::from_str(toml_str).unwrap();

        // Project
        assert_eq!(file.project.prefix, "NOS");

        // Agent
        let agent = file.agent.unwrap();
        assert_eq!(agent.agent_type, "claude");

        // Review
        let review = file.review.unwrap();
        assert_eq!(review.analyzers.len(), 5);

        // Co-change and formatting fall into the flattened catch-all
        assert!(review.analyzer_configs.contains_key("co-change"));
        assert!(review.analyzer_configs.contains_key("formatting"));

        // Agent analyzer configs (flattened)
        assert!(review.analyzer_configs.contains_key("agent:security"));
        assert!(review.analyzer_configs.contains_key("agent:quality"));

        // Checks
        assert_eq!(file.checks.len(), 1);
        assert_eq!(file.checks[0].target, "*.rs");
    }

    #[test]
    fn existing_config_without_new_sections() {
        // Old-style TOML with only [project] and [[check]] should parse fine
        let toml_str = r#"
[project]
prefix = "OLD"

[[check]]
target = "*.py"
message = "Review Python"
severity = "warn"
"#;
        let file: NoslopFile = toml::from_str(toml_str).unwrap();
        assert_eq!(file.project.prefix, "OLD");
        assert!(file.agent.is_none());
        assert!(file.review.is_none());
        assert_eq!(file.checks.len(), 1);
    }

    #[test]
    fn review_section_default_analyzers() {
        let review = ReviewSection::default();
        assert_eq!(review.analyzers, vec!["conventions"]);
        assert!(review.analyzer_configs.is_empty());
    }

    #[test]
    fn empty_file_parses_with_defaults() {
        let toml_str = "";
        let file: NoslopFile = toml::from_str(toml_str).unwrap();
        assert_eq!(file.project.prefix, "CHK");
        assert!(file.agent.is_none());
        assert!(file.review.is_none());
        assert!(file.checks.is_empty());
    }
}
