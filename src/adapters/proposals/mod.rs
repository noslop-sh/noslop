//! Proposals store
//!
//! Persists discovered check proposals in `.noslop/proposals.toml` while
//! they await human review. Proposals are per-clone working state, like
//! staged acks — they are not tracked and never enforce anything.

use std::fs;

use serde::{Deserialize, Serialize};

use crate::core::models::Proposal;

const PROPOSALS_PATH: &str = ".noslop/proposals.toml";
const REJECTED_PATH: &str = ".noslop/rejected-keys.txt";
const REJECTED_RULES_PATH: &str = ".noslop/rejected-rules.txt";

#[derive(Debug, Default, Serialize, Deserialize)]
struct ProposalsFile {
    #[serde(default, rename = "proposal")]
    proposals: Vec<Proposal>,
}

/// Load staged proposals, or an empty list if none exist.
///
/// # Errors
///
/// Returns an error if the file exists but cannot be read or parsed.
pub fn load() -> anyhow::Result<Vec<Proposal>> {
    let path = crate::adapters::git::state_path(PROPOSALS_PATH);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(path)?;
    let file: ProposalsFile = toml::from_str(&content)?;
    Ok(file.proposals)
}

/// Save proposals, replacing the existing set. An empty set removes the file.
///
/// # Errors
///
/// Returns an error if the file cannot be written.
pub fn save(proposals: &[Proposal]) -> anyhow::Result<()> {
    let path = crate::adapters::git::state_path(PROPOSALS_PATH);

    if proposals.is_empty() {
        if path.exists() {
            fs::remove_file(path)?;
        }
        return Ok(());
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let file = ProposalsFile {
        proposals: proposals.to_vec(),
    };
    fs::write(path, toml::to_string_pretty(&file)?)?;
    Ok(())
}

/// Load dedupe keys of proposals the team has rejected.
///
/// Rejections are tracked in git (unlike staged proposals) so a rejected
/// rule never resurfaces on anyone's re-scan.
///
/// # Errors
///
/// Returns an error if the file exists but cannot be read.
pub fn load_rejected_keys() -> anyhow::Result<Vec<String>> {
    let path = crate::adapters::git::state_path(REJECTED_PATH);
    if !path.exists() {
        return Ok(Vec::new());
    }
    Ok(fs::read_to_string(path)?.lines().map(str::to_string).collect())
}

/// Record dedupe keys of rejected proposals (append-only).
///
/// # Errors
///
/// Returns an error if the file cannot be written.
pub fn append_rejected_keys(keys: &[String]) -> anyhow::Result<()> {
    if keys.is_empty() {
        return Ok(());
    }
    let path = crate::adapters::git::state_path(REJECTED_PATH);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut existing = load_rejected_keys()?;
    existing.extend(keys.iter().cloned());
    existing.dedup();
    fs::write(path, existing.join("\n") + "\n")?;
    Ok(())
}

/// Load the full text of rejected rules.
///
/// Keys (above) give exact-match dedupe; these raw messages are fed into
/// discovery prompts so the model won't re-propose *paraphrases* of rules
/// the team already declined.
///
/// # Errors
///
/// Returns an error if the file exists but cannot be read.
pub fn load_rejected_rules() -> anyhow::Result<Vec<String>> {
    let path = crate::adapters::git::state_path(REJECTED_RULES_PATH);
    if !path.exists() {
        return Ok(Vec::new());
    }
    Ok(fs::read_to_string(path)?
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(str::to_string)
        .collect())
}

/// Record the full text of rejected rules (append-only, newline-flattened).
///
/// # Errors
///
/// Returns an error if the file cannot be written.
pub fn append_rejected_rules(messages: &[String]) -> anyhow::Result<()> {
    if messages.is_empty() {
        return Ok(());
    }
    let path = crate::adapters::git::state_path(REJECTED_RULES_PATH);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut existing = load_rejected_rules()?;
    existing.extend(messages.iter().map(|m| m.replace('\n', " ")));
    existing.dedup();
    fs::write(path, existing.join("\n") + "\n")?;
    Ok(())
}
