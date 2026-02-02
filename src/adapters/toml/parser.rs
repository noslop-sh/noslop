//! TOML parser for .noslop.toml files
//!
//! Handles reading and deserializing noslop configuration files.

use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

/// A .noslop.toml file structure
#[derive(Debug, Deserialize)]
pub struct NoslopFile {
    /// Project configuration
    #[serde(default)]
    pub project: ProjectConfig,

    /// Assertions in this file
    #[serde(default, rename = "assert")]
    pub assertions: Vec<AssertionEntry>,
}

/// Project-level configuration
#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct ProjectConfig {
    /// 3-letter prefix for assertion IDs (e.g., "NSL" for noslop-123)
    pub prefix: String,

    /// Next assertion ID number
    #[serde(skip)]
    pub next_id: u32,
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            prefix: "AST".to_string(), // Fallback if not set
            next_id: 1,
        }
    }
}

/// An assertion entry in .noslop.toml
#[derive(Debug, Deserialize)]
pub struct AssertionEntry {
    /// Optional custom ID (if not provided, will be auto-generated as PREFIX-N)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Target pattern (glob or path)
    pub target: String,

    /// The assertion message
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

    // Reverse so root comes first (assertions stack up)
    files.reverse();
    files
}

/// Load assertions from a .noslop.toml file
///
/// # Errors
///
/// Returns an error if the file cannot be read or parsed.
pub fn load_file(path: &Path) -> anyhow::Result<NoslopFile> {
    let content = fs::read_to_string(path)?;
    let file: NoslopFile = toml::from_str(&content)?;
    Ok(file)
}
