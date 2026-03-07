//! Centralized prompt builder for agent review invocations.
//!
//! Single place to craft and evolve the prompt sent to AI agents.
//! The agent uses `noslop` CLI commands to write feedbacks directly
//! into the review — no output parsing required.

/// Build the review prompt for an agent invocation.
///
/// The prompt instructs the agent to:
/// 1. Analyze the diff for issues
/// 2. Use `noslop feedback add` to record each finding
/// 3. Use `noslop review summary` to set an overall summary
#[must_use]
pub fn build_review_prompt(review_id: &str, diff: &str) -> String {
    format!(
        r#"You are a code reviewer. Analyze the following diff and provide feedback.

## Review Context

Review ID: {review_id}

## Instructions

For each issue you find, run this command:

```
noslop feedback add {review_id} "<description of the issue>" --file <path> --line <line_number> --severity <info|warn|block>
```

Severity guide:
- **block**: Security vulnerabilities, data loss, correctness bugs, broken APIs
- **warn**: Code smells, performance concerns, missing error handling, style issues
- **info**: Suggestions, nitpicks, documentation improvements

Optional flags:
- `--end_line <n>` for multi-line spans
- `--suggestion "<suggested fix>"` to propose a concrete change

After reviewing all files, set an overall summary:

```
noslop review summary {review_id} "<1-2 paragraph summary of the changes and your assessment>"
```

## Guidelines

- Focus on substantive issues, not formatting
- Be specific: reference exact file paths and line numbers
- For blocking issues, explain the impact clearly
- Keep feedback messages concise but actionable
- The summary should cover: what changed, what's good, what needs attention

## Diff

```diff
{diff}
```"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_contains_review_id() {
        let prompt = build_review_prompt("42", "some diff");
        assert!(prompt.contains("Review ID: 42"));
        assert!(prompt.contains("noslop feedback add 42"));
        assert!(prompt.contains("noslop review summary 42"));
    }

    #[test]
    fn prompt_contains_diff() {
        let diff = "+fn new_function() {}";
        let prompt = build_review_prompt("1", diff);
        assert!(prompt.contains(diff));
    }

    #[test]
    fn prompt_contains_severity_guide() {
        let prompt = build_review_prompt("1", "");
        assert!(prompt.contains("block"));
        assert!(prompt.contains("warn"));
        assert!(prompt.contains("info"));
    }
}
