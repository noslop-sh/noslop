//! Discovery service - extracts check proposals from rules files
//!
//! Pure decomposition logic: markdown/mdc content in, proposals out.
//! No filesystem access; adapters feed content in and persist results.
//!
//! Extraction rules (deterministic, no LLM):
//! - Bullet lines (`-`, `*`, `1.`) outside code fences are candidates
//! - A candidate survives only if it reads like an obligation
//!   (imperative opener or a modal keyword)
//! - Glob inference: explicit path/glob tokens in the text win; a path in
//!   the enclosing heading is the fallback; otherwise `**/*`
//! - Severity: hard words (never/must/banned/required) => block, else warn

use crate::core::models::{Proposal, Severity};

/// Words that mark a bullet as an obligation when they open the sentence.
const IMPERATIVE_OPENERS: &[&str] = &[
    "use", "never", "always", "don't", "do", "avoid", "prefer", "keep", "run", "add", "ensure",
    "make", "write", "test", "check", "update", "include", "wrap", "return", "delete", "no",
    "only", "commit", "branch", "name", "document", "validate", "handle",
];

/// Modal keywords that mark an obligation anywhere in the sentence.
const MODAL_KEYWORDS: &[&str] = &[
    " must ",
    " should ",
    " never ",
    " always ",
    " required",
    " banned",
    " do not ",
    " don't ",
    " needs to ",
    " have to ",
    " only ",
];

/// Hard words that escalate severity to block.
const BLOCK_WORDS: &[&str] = &[
    "never",
    "must",
    "banned",
    "required",
    "do not",
    "don't",
    "forbidden",
    "prohibited",
];

/// Extract check proposals from a markdown rules file (CLAUDE.md, AGENTS.md).
///
/// `source_name` is used for provenance (`CLAUDE.md:12`).
#[must_use]
pub fn extract_from_markdown(content: &str, source_name: &str) -> Vec<Proposal> {
    let mut proposals = Vec::new();
    let mut in_fence = false;
    let mut heading_path_hint: Option<String> = None;

    for (idx, raw_line) in content.lines().enumerate() {
        let line = raw_line.trim();

        if line.starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }

        if line.starts_with('#') {
            heading_path_hint = extract_path_token(line);
            continue;
        }

        let Some(text) = bullet_text(line) else {
            continue;
        };

        if !is_obligation(text) {
            continue;
        }

        let target = extract_path_token(text)
            .or_else(|| heading_path_hint.clone())
            .unwrap_or_else(|| "**/*".to_string());

        proposals.push(Proposal {
            target,
            message: text.to_string(),
            severity: infer_severity(text),
            source: format!("{}:{}", source_name, idx + 1),
        });
    }

    proposals
}

/// Extract proposals from a Cursor `.mdc` rules file.
///
/// The frontmatter `globs:` value scopes every rule in the body; body
/// bullets are extracted like markdown.
#[must_use]
pub fn extract_from_mdc(content: &str, source_name: &str) -> Vec<Proposal> {
    let (frontmatter, body) = split_frontmatter(content);

    let glob = frontmatter
        .lines()
        .find_map(|l| l.strip_prefix("globs:"))
        .map(|v| v.trim().trim_matches(|c| c == '"' || c == '\'').to_string())
        .filter(|v| !v.is_empty());

    let mut proposals = extract_from_markdown(body, source_name);
    if let Some(glob) = glob {
        // Frontmatter scope beats inference for rules that fell back to **/*
        for p in &mut proposals {
            if p.target == "**/*" {
                p.target.clone_from(&glob);
            }
        }
    }
    proposals
}

/// Dedupe proposals against each other and against already-known keys
/// (existing checks and previously staged proposals).
#[must_use]
pub fn dedupe(proposals: Vec<Proposal>, known_keys: &[String]) -> Vec<Proposal> {
    let mut seen: Vec<String> = known_keys.to_vec();
    let mut result = Vec::new();

    for p in proposals {
        let key = p.dedupe_key();
        if key.is_empty() || seen.contains(&key) {
            continue;
        }
        seen.push(key);
        result.push(p);
    }
    result
}

/// Return the bullet text if the line is a list item.
fn bullet_text(line: &str) -> Option<&str> {
    let text = line
        .strip_prefix("- ")
        .or_else(|| line.strip_prefix("* "))
        .or_else(|| {
            line.split_once(". ")
                .filter(|(n, _)| n.chars().all(|c| c.is_ascii_digit()) && !n.is_empty())
                .map(|(_, rest)| rest)
        })?
        .trim();

    // Too short to be a rule, or a bare link/reference
    if text.len() < 15 || text.starts_with('[') || text.starts_with("See ") {
        return None;
    }
    Some(text)
}

/// Does this text read like an obligation rather than prose or navigation?
fn is_obligation(text: &str) -> bool {
    let lower = text.to_lowercase();
    let first_word = lower.split_whitespace().next().unwrap_or("");

    IMPERATIVE_OPENERS.contains(&first_word) || MODAL_KEYWORDS.iter().any(|k| lower.contains(k))
}

/// Find an explicit path or glob token in the text, if any.
fn extract_path_token(text: &str) -> Option<String> {
    // Prefer backtick-wrapped tokens, then bare tokens
    let candidates = text.split('`').skip(1).step_by(2).chain(text.split_whitespace());

    for token in candidates {
        let token =
            token.trim_matches(|c: char| !c.is_ascii_alphanumeric() && !"*./_-".contains(c));
        // Only chars that can appear in a real path glob; rejects prose and
        // placeholder patterns like "sp/no-<issue>-<slug>"
        if token.chars().any(|c| !c.is_ascii_alphanumeric() && !"*./_-".contains(c)) {
            continue;
        }
        // A directory ("src/adapters/"), a glob ("src/**/*.rs", "*.py"), or a
        // file path ("config/prod.yaml") — but not prose like "and/or"
        let dir_like = token.contains('/') && token.ends_with('/');
        let glob_or_file = token.contains('/') && (token.contains('.') || token.contains('*'));
        let ext_glob = token.starts_with("*.");
        if (dir_like || glob_or_file || ext_glob) && token.len() > 3 && !token.starts_with("http") {
            return Some(
                token.trim_end_matches('/').to_string()
                    + if token.ends_with('/') { "/**/*" } else { "" },
            );
        }
    }
    None
}

fn infer_severity(text: &str) -> Severity {
    let lower = text.to_lowercase();
    if BLOCK_WORDS.iter().any(|w| lower.contains(w)) {
        Severity::Block
    } else {
        Severity::Warn
    }
}

/// Split an `.mdc` file into (frontmatter, body).
fn split_frontmatter(content: &str) -> (&str, &str) {
    let Some(rest) = content.strip_prefix("---") else {
        return ("", content);
    };
    rest.split_once("\n---").map_or(("", content), |(fm, body)| (fm, body))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_imperative_bullets() {
        let md = "# Conventions\n\n- Use git flow: work on feature branches\n- some prose that is not a rule at all\n- Never commit directly to main\n";
        let props = extract_from_markdown(md, "CLAUDE.md");
        assert_eq!(props.len(), 2);
        assert_eq!(props[0].source, "CLAUDE.md:3");
        assert_eq!(props[0].severity, Severity::Warn);
        assert_eq!(props[1].severity, Severity::Block);
    }

    #[test]
    fn skips_code_fences_and_links() {
        let md = "- Run `make check` before every commit\n```\n- Use fake bullet inside fence\n```\n- [link only](https://x.com)\n";
        let props = extract_from_markdown(md, "AGENTS.md");
        assert_eq!(props.len(), 1);
        assert!(props[0].message.contains("make check"));
    }

    #[test]
    fn infers_glob_from_text_and_heading() {
        let md = "## Rules for `src/adapters/`\n\n- Always add integration tests for adapter changes\n- Use `#[must_use]` on all public src/core/**/*.rs constructors\n";
        let props = extract_from_markdown(md, "CLAUDE.md");
        assert_eq!(props[0].target, "src/adapters/**/*");
        assert_eq!(props[1].target, "src/core/**/*.rs");
    }

    #[test]
    fn falls_back_to_wildcard() {
        let md = "- Always write type hints for functions\n";
        let props = extract_from_markdown(md, "CLAUDE.md");
        assert_eq!(props[0].target, "**/*");
    }

    #[test]
    fn placeholder_patterns_are_not_globs() {
        // Regression: branch-name templates looked like paths
        let md = "- Branches: `sp/no-<linear-issue>-<slug>`. Never commit to main.\n";
        let props = extract_from_markdown(md, "CLAUDE.md");
        assert_eq!(props[0].target, "**/*");
    }

    #[test]
    fn mdc_frontmatter_glob_scopes_body() {
        let mdc = "---\nglobs: \"src/**/*.ts\"\n---\n- Always validate request bodies with zod\n";
        let props = extract_from_mdc(mdc, "api.mdc");
        assert_eq!(props.len(), 1);
        assert_eq!(props[0].target, "src/**/*.ts");
    }

    #[test]
    fn dedupes_by_normalized_message() {
        let a = Proposal {
            target: "**/*".into(),
            message: "Never commit to main".into(),
            severity: Severity::Block,
            source: "a:1".into(),
        };
        let b = Proposal {
            target: "**/*".into(),
            message: "never commit to MAIN!".into(),
            severity: Severity::Block,
            source: "b:2".into(),
        };
        let out = dedupe(vec![a.clone(), b], &[]);
        assert_eq!(out.len(), 1);

        let out = dedupe(vec![a.clone()], &[a.dedupe_key()]);
        assert!(out.is_empty());
    }
}
