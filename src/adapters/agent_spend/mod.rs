//! Agent session spend reading
//!
//! noslop owns no LLM access, so token spend comes from the agent's own
//! session records. Claude Code writes transcripts under
//! `~/.claude/projects/<munged-cwd>/<session>.jsonl`; each assistant entry
//! carries a `message.usage` block and the model id. Summing usage over a
//! transcript yields a cumulative session counter: snapshot at fire,
//! snapshot at ack, ship the delta.
//!
//! PRIVACY INVARIANT: only token counts and the model id are read. Message
//! content never leaves the transcript.
//!
//! Everything here is fail-open: unknown agent, opted out, no transcript,
//! unreadable lines — all yield `None`, never an error and never a zero.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// Cumulative spend of one agent session at a moment in time.
///
/// Fresh and cached are tracked separately because they differ ~10x in
/// price: merging them made a mostly-cache-reads span read as 10-30x
/// more expensive than it was.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionSpend {
    /// Fresh work so far: input + output + cache creation tokens.
    pub fresh: u64,
    /// Context re-read from cache so far (cheap re-processing).
    pub cached: u64,
    /// Model id of the most recent assistant message, when present.
    pub model: Option<String>,
}

/// Transcripts older than this are stale sessions, not the live one.
const FRESH_WINDOW: Duration = Duration::from_hours(24);

/// Opt-out env var: set to `0`/`false` to disable capture entirely.
const CAPTURE_ENV: &str = "NOSLOP_TOKEN_CAPTURE";

/// Read the current session's cumulative spend for the given actor.
///
/// Only Claude Code exposes transcripts today; other actors return `None`
/// (absent upstream, never a fake zero).
#[must_use]
pub fn cumulative_spend(actor: &str) -> Option<SessionSpend> {
    if actor != "claude-code" || opted_out() {
        return None;
    }
    let base = home_dir()?.join(".claude").join("projects");
    let roots = candidate_roots();
    cumulative_spend_in(&base, &roots, SystemTime::now())
}

fn opted_out() -> bool {
    matches!(std::env::var(CAPTURE_ENV).ok().as_deref(), Some("0" | "false" | "off"))
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

/// Directories the live session may be keyed under, most specific first:
/// the repo root, the cwd, and their ancestors up to (excluding) $HOME.
/// Agents regularly start sessions in a workspace directory ABOVE the
/// repo they later run the gate in, so ancestors are first-class
/// candidates, not a fallback.
fn candidate_roots() -> Vec<PathBuf> {
    let home = home_dir();
    let mut roots: Vec<PathBuf> = Vec::new();
    let mut push_chain = |start: PathBuf| {
        let mut current = Some(start);
        while let Some(dir) = current {
            if home.as_deref() == Some(dir.as_path()) || dir.parent().is_none() {
                break;
            }
            if !roots.contains(&dir) {
                roots.push(dir.clone());
            }
            current = dir.parent().map(Path::to_path_buf);
        }
    };
    push_chain(crate::adapters::git::repo_root_or_cwd());
    if let Ok(cwd) = std::env::current_dir() {
        push_chain(cwd);
    }
    roots
}

/// Claude Code munges a project path into a directory name by replacing
/// `/` and `.` with `-`.
fn munge(path: &Path) -> String {
    path.to_string_lossy()
        .chars()
        .map(|c| if c == '/' || c == '.' { '-' } else { c })
        .collect()
}

/// Testable core: pick the live session's transcript and sum it.
///
/// Roots are ordered most-specific-first; the first project dir holding a
/// fresh transcript wins (specificity beats recency across dirs — a fresh
/// transcript in an unrelated ancestor project must not shadow the repo's
/// own session). Within the winning dir, the freshest transcript is the
/// live session.
#[must_use]
pub fn cumulative_spend_in(
    base: &Path,
    roots: &[PathBuf],
    now: SystemTime,
) -> Option<SessionSpend> {
    for root in roots {
        let dir = base.join(munge(root));
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        let mut newest: Option<(SystemTime, PathBuf)> = None;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_none_or(|e| e != "jsonl") {
                continue;
            }
            let Ok(modified) = entry.metadata().and_then(|m| m.modified()) else {
                continue;
            };
            if now.duration_since(modified).unwrap_or_default() > FRESH_WINDOW {
                continue;
            }
            if newest.as_ref().is_none_or(|(t, _)| modified > *t) {
                newest = Some((modified, path));
            }
        }
        if let Some((_, path)) = newest {
            return sum_transcript(&path);
        }
    }
    None
}

/// Sum `message.usage` over a transcript's assistant entries, tolerantly:
/// unparsable lines are skipped, exactly like `telemetry::load_events`.
fn sum_transcript(path: &Path) -> Option<SessionSpend> {
    let content = std::fs::read_to_string(path).ok()?;
    let mut fresh: u64 = 0;
    let mut cached: u64 = 0;
    let mut model: Option<String> = None;
    let mut saw_usage = false;
    for line in content.lines().filter(|l| !l.trim().is_empty()) {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        if value.get("type").and_then(|t| t.as_str()) != Some("assistant") {
            continue;
        }
        let Some(message) = value.get("message") else {
            continue;
        };
        if let Some(m) = message.get("model").and_then(|m| m.as_str()) {
            model = Some(m.to_string());
        }
        let Some(usage) = message.get("usage") else {
            continue;
        };
        saw_usage = true;
        for key in ["input_tokens", "output_tokens", "cache_creation_input_tokens"] {
            fresh += usage.get(key).and_then(serde_json::Value::as_u64).unwrap_or(0);
        }
        cached += usage
            .get("cache_read_input_tokens")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0);
    }
    saw_usage.then_some(SessionSpend {
        fresh,
        cached,
        model,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_transcript(dir: &Path, name: &str, lines: &[&str]) -> PathBuf {
        std::fs::create_dir_all(dir).unwrap();
        let path = dir.join(name);
        std::fs::write(&path, lines.join("\n")).unwrap();
        path
    }

    const ASSISTANT_A: &str = r#"{"type":"assistant","message":{"model":"claude-fable-5","usage":{"input_tokens":100,"output_tokens":50,"cache_creation_input_tokens":10,"cache_read_input_tokens":40}}}"#;
    const ASSISTANT_B: &str = r#"{"type":"assistant","message":{"model":"claude-opus-4-8","usage":{"input_tokens":200,"output_tokens":100}}}"#;
    const USER_LINE: &str = r#"{"type":"user","message":{"content":"never read"}}"#;

    #[test]
    fn sums_usage_and_takes_last_model() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("-repo-root");
        write_transcript(&project, "s1.jsonl", &[ASSISTANT_A, USER_LINE, "not json", ASSISTANT_B]);

        let spend =
            cumulative_spend_in(tmp.path(), &[PathBuf::from("/repo.root")], SystemTime::now())
                .unwrap();
        assert_eq!(spend.fresh, 460);
        assert_eq!(spend.cached, 40);
        assert_eq!(spend.model.as_deref(), Some("claude-opus-4-8"));
    }

    #[test]
    fn picks_the_freshest_transcript_and_ignores_stale_sessions() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("-repo-root");
        let old = write_transcript(&project, "old.jsonl", &[ASSISTANT_B]);
        let stale_time = SystemTime::now() - Duration::from_hours(48);
        let file = std::fs::File::options().append(true).open(&old).unwrap();
        file.set_modified(stale_time).unwrap();
        write_transcript(&project, "live.jsonl", &[ASSISTANT_A]);

        let spend =
            cumulative_spend_in(tmp.path(), &[PathBuf::from("/repo.root")], SystemTime::now())
                .unwrap();
        // Only the live session counts; the stale one is another day's work
        assert_eq!(spend.fresh, 160);
        assert_eq!(spend.cached, 40);
    }

    #[test]
    fn absent_when_no_transcript_or_no_usage() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(
            cumulative_spend_in(tmp.path(), &[PathBuf::from("/repo")], SystemTime::now()).is_none()
        );
        let project = tmp.path().join("-repo");
        write_transcript(&project, "s.jsonl", &[USER_LINE]);
        assert!(
            cumulative_spend_in(tmp.path(), &[PathBuf::from("/repo")], SystemTime::now()).is_none()
        );
    }

    #[test]
    fn prefers_the_most_specific_project_dir_over_fresher_ancestors() {
        let tmp = tempfile::tempdir().unwrap();
        // Ancestor workspace session (fresher) and the repo's own session
        write_transcript(&tmp.path().join("-ws"), "other.jsonl", &[ASSISTANT_B]);
        write_transcript(&tmp.path().join("-ws-repo"), "own.jsonl", &[ASSISTANT_A]);

        // Roots ordered most specific first, as candidate_roots produces
        let spend = cumulative_spend_in(
            tmp.path(),
            &[PathBuf::from("/ws/repo"), PathBuf::from("/ws")],
            SystemTime::now(),
        )
        .unwrap();
        assert_eq!(spend.fresh, 160, "repo-specific session wins");

        // Without a repo-specific dir, the ancestor session is found
        let spend = cumulative_spend_in(
            tmp.path(),
            &[PathBuf::from("/ws/other-repo"), PathBuf::from("/ws")],
            SystemTime::now(),
        )
        .unwrap();
        assert_eq!(spend.fresh, 300);
    }

    #[test]
    fn munges_slashes_and_dots() {
        assert_eq!(munge(Path::new("/home/x/code.dir")), "-home-x-code-dir");
    }
}
