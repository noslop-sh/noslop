//! GitHub review-history adapter
//!
//! Pulls human review comments from merged PRs via the `gh` CLI (which
//! carries its own authentication). noslop never talks to the GitHub API
//! directly and holds no tokens.

use std::process::Command;

use crate::core::services::mining::ReviewComment;

/// Bot author fragments filtered out of mining input.
const BOT_LOGIN_FRAGMENTS: &[&str] = &["bot", "copilot", "coderabbit", "dependabot"];

/// Max characters of a single comment fed to mining.
const MAX_COMMENT_LEN: usize = 500;

/// The `owner/repo` slug of the current repository.
///
/// # Errors
///
/// Returns an error if `gh` is missing, unauthenticated, or there is no
/// GitHub remote.
pub fn repo_slug() -> anyhow::Result<String> {
    let output = gh(&["repo", "view", "--json", "nameWithOwner", "--jq", ".nameWithOwner"])?;
    let slug = output.trim().to_string();
    if slug.is_empty() {
        anyhow::bail!("could not resolve GitHub repository (is there a GitHub remote?)");
    }
    Ok(slug)
}

/// Fetch recent human review comments (newest first), up to `max_pages`
/// pages of 100.
///
/// # Errors
///
/// Returns an error if `gh` is unavailable or the API response is not JSON.
pub fn fetch_review_comments(slug: &str, max_pages: usize) -> anyhow::Result<Vec<ReviewComment>> {
    let mut comments = Vec::new();

    for page in 1..=max_pages {
        let endpoint = format!(
            "repos/{slug}/pulls/comments?per_page=100&sort=created&direction=desc&page={page}"
        );
        let raw = gh(&["api", &endpoint])?;
        let items: serde_json::Value = serde_json::from_str(&raw)?;
        let Some(items) = items.as_array() else {
            anyhow::bail!("unexpected response from GitHub API");
        };

        for item in items {
            let login = item["user"]["login"].as_str().unwrap_or("").to_lowercase();
            let is_bot = item["user"]["type"].as_str() == Some("Bot")
                || BOT_LOGIN_FRAGMENTS.iter().any(|f| login.contains(f));
            if is_bot {
                continue;
            }
            let (Some(path), Some(body)) = (item["path"].as_str(), item["body"].as_str()) else {
                continue;
            };
            comments.push(ReviewComment {
                path: path.to_string(),
                body: body.chars().take(MAX_COMMENT_LEN).collect(),
            });
        }

        if items.len() < 100 {
            break; // last page
        }
    }

    Ok(comments)
}

/// Run a `gh` subcommand and capture stdout.
fn gh(args: &[&str]) -> anyhow::Result<String> {
    let output = Command::new("gh").args(args).output().map_err(|_| {
        anyhow::anyhow!(
            "mining needs the GitHub CLI: install `gh` and run `gh auth login`, \
             or mine from an export with --from-file"
        )
    })?;

    if !output.status.success() {
        anyhow::bail!(
            "gh {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}
