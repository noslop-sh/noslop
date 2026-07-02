//! Mining service - turn review comments into check proposals via an LLM
//!
//! Pure logic only: prompt construction, comment chunking, and parsing/
//! validating the LLM's TOML output. The subprocess that actually runs the
//! prompt lives in `adapters::runner`; comment fetching in `adapters::gh`.
//!
//! The prompt is the one validated in the NO-6 discovery-quality spike.

use serde::Deserialize;

use crate::core::models::{Proposal, Severity, Target};

/// A single human review comment fed into mining
#[derive(Debug, Clone, Deserialize)]
pub struct ReviewComment {
    /// File path the comment was left on
    pub path: String,
    /// Comment text
    pub body: String,
}

/// Why the LLM output could not be turned into proposals
#[derive(Debug, thiserror::Error)]
pub enum MiningParseError {
    /// No TOML content found in the output
    #[error("no [[check]] TOML found in runner output")]
    NoToml,
    /// TOML was present but malformed
    #[error("invalid TOML: {0}")]
    Invalid(String),
    /// TOML parsed but every entry failed validation
    #[error("no valid checks: {0}")]
    AllRejected(String),
}

/// Build the mining prompt for one chunk of comments.
#[must_use]
pub fn mining_prompt(repo: &str, comments: &[ReviewComment]) -> String {
    use std::fmt::Write as _;
    let mut serialized = String::new();
    for c in comments {
        let _ = writeln!(serialized, "PATH: {}\nCOMMENT: {}\n---", c.path, c.body);
    }

    format!(
        "The following are inline code-review comments (file path + comment text) written by \
         humans on merged pull requests of the {repo} repository.\n\n\
         You are discovering the team's unwritten conventions — the things reviewers keep \
         enforcing by hand. Ignore one-off nitpicks, questions, praise, and anything a standard \
         linter/formatter already catches.\n\n\
         Output: a noslop TOML check list — up to 15 [[check]] entries in exactly this shape:\n\n\
         [[check]]\n\
         target = \"<glob, scoped as tightly as the evidence supports>\"\n\
         message = \"<one specific obligation, phrased as a question an agent can act on at \
         commit time>\"\n\
         severity = \"block\" # or \"warn\"\n\
         # evidence: <2-3 short quotes/paraphrases of supporting comments + rough count>\n\n\
         Rules: one obligation per check; a check must be justified by at least 2 separate \
         comments; prefer concrete paths/patterns that appear in the comments; target globs may \
         only use *, ** and literal paths (no {{a,b}} brace expansion — emit separate checks \
         instead); if fewer than 15 real conventions exist, output fewer rather than padding. \
         Respond with ONLY the TOML.\n\n\
         COMMENTS:\n{serialized}"
    )
}

/// Build the merge prompt that consolidates per-chunk results.
#[must_use]
pub fn merge_prompt(repo: &str, chunk_outputs: &[String]) -> String {
    format!(
        "Below are several TOML [[check]] lists mined from different slices of the {repo} \
         repository's review history. Merge them: combine duplicates (keep the tightest glob \
         and clearest message), drop weakly-evidenced entries, and return at most 15 [[check]] \
         entries in the same TOML shape. Respond with ONLY the TOML.\n\n{}",
        chunk_outputs.join("\n\n# --- next chunk ---\n\n")
    )
}

/// Build the one-shot retry prompt after a parse failure.
#[must_use]
pub fn retry_prompt(original_prompt: &str, error: &MiningParseError) -> String {
    format!(
        "{original_prompt}\n\nYour previous response could not be used: {error}. \
         Respond again with ONLY valid TOML [[check]] entries and nothing else."
    )
}

/// Greedily pack comments into chunks of at most `max_bytes` serialized size.
#[must_use]
pub fn chunk_comments(comments: Vec<ReviewComment>, max_bytes: usize) -> Vec<Vec<ReviewComment>> {
    let mut chunks = Vec::new();
    let mut current = Vec::new();
    let mut current_size = 0_usize;

    for c in comments {
        let size = c.path.len() + c.body.len() + 20;
        if current_size + size > max_bytes && !current.is_empty() {
            chunks.push(std::mem::take(&mut current));
            current_size = 0;
        }
        current_size += size;
        current.push(c);
    }
    if !current.is_empty() {
        chunks.push(current);
    }
    chunks
}

#[derive(Debug, Deserialize)]
struct MinedDoc {
    #[serde(default, rename = "check")]
    checks: Vec<MinedEntry>,
}

#[derive(Debug, Deserialize)]
struct MinedEntry {
    target: String,
    message: String,
    #[serde(default)]
    severity: Option<String>,
}

/// Parse LLM output into validated proposals.
///
/// Accepts output wrapped in fenced code blocks or bare TOML. Entries with
/// an unparsable glob or an empty/trivial message are dropped; if every
/// entry is dropped the whole output is rejected.
///
/// # Errors
///
/// Returns a [`MiningParseError`] describing why nothing usable was found.
pub fn parse_proposals(output: &str, source: &str) -> Result<Vec<Proposal>, MiningParseError> {
    let toml_text = extract_toml(output).ok_or(MiningParseError::NoToml)?;

    let doc: MinedDoc =
        toml::from_str(toml_text).map_err(|e| MiningParseError::Invalid(e.to_string()))?;

    if doc.checks.is_empty() {
        return Err(MiningParseError::NoToml);
    }

    let total = doc.checks.len();
    let mut rejected_reasons = Vec::new();
    let mut proposals = Vec::new();

    for entry in doc.checks {
        if entry.message.trim().len() < 10 {
            rejected_reasons.push(format!("'{}': message too short", entry.target));
            continue;
        }
        // The matcher has no brace expansion: a {a,b} glob would parse but
        // silently never match — reject so the retry loop corrects it.
        if entry.target.contains(['{', '}']) {
            rejected_reasons.push(format!(
                "'{}': brace globs are unsupported, split into separate checks",
                entry.target
            ));
            continue;
        }
        if Target::parse(&entry.target).is_err() {
            rejected_reasons.push(format!("'{}': invalid glob", entry.target));
            continue;
        }
        let severity =
            entry.severity.as_deref().and_then(|s| s.parse().ok()).unwrap_or(Severity::Warn);
        proposals.push(Proposal {
            target: entry.target,
            message: entry.message.trim().to_string(),
            severity,
            source: source.to_string(),
        });
    }

    if proposals.is_empty() {
        return Err(MiningParseError::AllRejected(format!(
            "{total} entries, all invalid: {}",
            rejected_reasons.join("; ")
        )));
    }
    Ok(proposals)
}

/// Pull the TOML payload out of the runner's output.
fn extract_toml(output: &str) -> Option<&str> {
    // Fenced block first: ```toml ... ``` or ``` ... ```
    if let Some(start) = output.find("```") {
        let after_fence = &output[start + 3..];
        let body_start = after_fence.find('\n').map_or(0, |i| i + 1);
        let body = &after_fence[body_start..];
        if let Some(end) = body.find("```") {
            let fenced = &body[..end];
            if fenced.contains("[[check]]") {
                return Some(fenced);
            }
        }
    }
    // Bare TOML: from the first [[check]] to the end
    output.find("[[check]]").map(|i| &output[i..])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn comment(path: &str, body: &str) -> ReviewComment {
        ReviewComment {
            path: path.to_string(),
            body: body.to_string(),
        }
    }

    #[test]
    fn prompt_contains_comments_and_contract() {
        let p = mining_prompt("acme/api", &[comment("src/a.py", "add rate limiting")]);
        assert!(p.contains("PATH: src/a.py"));
        assert!(p.contains("ONLY the TOML"));
        assert!(p.contains("acme/api"));
    }

    #[test]
    fn chunking_respects_max_bytes() {
        let comments: Vec<_> =
            (0..10).map(|i| comment(&format!("f{i}.rs"), &"x".repeat(100))).collect();
        let chunks = chunk_comments(comments, 300);
        assert!(chunks.len() > 1);
        assert_eq!(chunks.iter().map(Vec::len).sum::<usize>(), 10);
    }

    #[test]
    fn parses_fenced_toml() {
        let out = "Here you go:\n```toml\n[[check]]\ntarget = \"src/**/*.py\"\nmessage = \"Rate limiting decorator added to public routes?\"\nseverity = \"block\"\n```\n";
        let props = parse_proposals(out, "mining:acme").unwrap();
        assert_eq!(props.len(), 1);
        assert_eq!(props[0].severity, Severity::Block);
        assert_eq!(props[0].source, "mining:acme");
    }

    #[test]
    fn parses_bare_toml_and_defaults_severity() {
        let out = "[[check]]\ntarget = \"migrations/*.py\"\nmessage = \"Generated with alembic autogenerate?\"\n";
        let props = parse_proposals(out, "mining:x").unwrap();
        assert_eq!(props[0].severity, Severity::Warn);
    }

    #[test]
    fn rejects_output_without_toml() {
        assert!(matches!(
            parse_proposals("I could not find any conventions.", "m"),
            Err(MiningParseError::NoToml)
        ));
    }

    #[test]
    fn drops_invalid_entries_keeps_valid() {
        let out = "[[check]]\ntarget = \"src/**/*.rs\"\nmessage = \"Public API changes need docs update?\"\n\n[[check]]\ntarget = \"ok/*.rs\"\nmessage = \"short\"\n";
        let props = parse_proposals(out, "m").unwrap();
        assert_eq!(props.len(), 1);
    }

    #[test]
    fn all_invalid_is_an_error() {
        let out = "[[check]]\ntarget = \"x/*.rs\"\nmessage = \"tiny\"\n";
        assert!(matches!(parse_proposals(out, "m"), Err(MiningParseError::AllRejected(_))));
    }

    #[test]
    fn brace_globs_are_rejected() {
        // Regression: matcher has no brace expansion; such checks never fire
        let out = "[[check]]\ntarget = \"src/{a,b}/**/*.ts\"\nmessage = \"Transactions must use the injected EntityManager?\"\n";
        let err = parse_proposals(out, "m").unwrap_err();
        assert!(err.to_string().contains("brace globs"));
    }
}
