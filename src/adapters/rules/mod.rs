//! Rules-file discovery adapter
//!
//! Finds and reads the agent rules files that `noslop discover` imports:
//! `CLAUDE.md`, `AGENTS.md`, `AGENT.md`, `.cursor/rules/*.mdc`, and
//! `.claude/rules/*.md` at the repository root. Files are passed to the
//! LLM verbatim (it reads `.mdc` frontmatter itself), so no format
//! distinction is needed here.

use std::fs;
use std::path::Path;

/// A rules file found in the repository
#[derive(Debug, Clone)]
pub struct RulesFile {
    /// Path relative to the repo root, used for provenance
    pub name: String,
    /// Raw file content
    pub content: String,
}

/// Root-level markdown rules files to look for.
const ROOT_FILES: &[&str] = &["CLAUDE.md", "AGENTS.md", "AGENT.md"];

/// Find all rules files under `root`.
///
/// # Errors
///
/// Returns an error only if a file that exists cannot be read.
pub fn find_rules_files(root: &Path) -> anyhow::Result<Vec<RulesFile>> {
    let mut files = Vec::new();

    for name in ROOT_FILES {
        let path = root.join(name);
        if path.is_file() {
            files.push(RulesFile {
                name: (*name).to_string(),
                content: fs::read_to_string(&path)?,
            });
        }
    }

    collect_dir(root, ".cursor/rules", "mdc", &mut files)?;
    collect_dir(root, ".claude/rules", "md", &mut files)?;

    Ok(files)
}

/// Collect files with `ext` from `dir` (non-recursive), sorted by name.
fn collect_dir(root: &Path, dir: &str, ext: &str, out: &mut Vec<RulesFile>) -> anyhow::Result<()> {
    let dir_path = root.join(dir);
    if !dir_path.is_dir() {
        return Ok(());
    }

    let mut entries: Vec<_> = fs::read_dir(&dir_path)?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.is_file() && p.extension().is_some_and(|e| e == ext))
        .collect();
    entries.sort();

    for path in entries {
        let file_name =
            path.file_name().map_or_else(String::new, |n| n.to_string_lossy().into_owned());
        out.push(RulesFile {
            name: format!("{dir}/{file_name}"),
            content: fs::read_to_string(&path)?,
        });
    }
    Ok(())
}
