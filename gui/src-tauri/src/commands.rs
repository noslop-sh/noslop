//! Tauri IPC command handlers
//!
//! These commands expose noslop functionality to the frontend via Tauri's invoke API.

use noslop::Review;
use noslop::adapters::FileReviewStore;
use noslop::core::models::{
    DismissReason, Feedback, FeedbackSource, ResolutionReason, Severity, Span, Target,
};
use noslop::core::ports::ReviewStore;
use serde::Serialize;
use std::process::Command;

use crate::dto::{FeedbackDto, FeedbackNoteDto, ReviewDto};

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// Run a git command and return its stdout as a String.
fn run_git(args: &[&str]) -> Result<String, String> {
    let output = Command::new("git").args(args).output().map_err(|e| e.to_string())?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    String::from_utf8(output.stdout).map_err(|e| e.to_string())
}

/// Parse a dismiss reason string into its enum variant.
fn parse_dismiss_reason(reason: &str) -> Result<DismissReason, String> {
    reason.parse()
}

// ---------------------------------------------------------------------------
// Existing commands
// ---------------------------------------------------------------------------

/// List all reviews, optionally filtering to open only
#[tauri::command]
pub fn list_reviews(open_only: bool) -> Result<Vec<ReviewDto>, String> {
    let store = FileReviewStore::new();
    let reviews = if open_only {
        store.list_open()
    } else {
        store.list_all()
    }
    .map_err(|e| {
        log::error!("list_reviews failed: {e}");
        e.to_string()
    })?;

    Ok(reviews.into_iter().map(ReviewDto::from).collect())
}

/// Get a single review by ID
#[tauri::command]
pub fn get_review(id: String) -> Result<ReviewDto, String> {
    let store = FileReviewStore::new();
    store
        .load(&id)
        .map_err(|e| e.to_string())?
        .map(ReviewDto::from)
        .ok_or_else(|| format!("Review not found: {id}"))
}

/// Get git diff between two commits
#[tauri::command]
pub fn get_diff(base: String, head: String) -> Result<String, String> {
    run_git(&["diff", &format!("{base}..{head}")])
}

/// Start a new review for a commit range
#[tauri::command]
pub fn start_review(
    base: String,
    head: String,
    branch: Option<String>,
) -> Result<ReviewDto, String> {
    let store = FileReviewStore::new();
    let mut review = Review::new(base, head);
    review.branch = branch;
    store.save(&review).map_err(|e| e.to_string())?;
    Ok(ReviewDto::from(review))
}

/// Get the default branch name (main, master, or first available)
#[tauri::command]
pub fn get_default_branch() -> Result<String, String> {
    let stdout = run_git(&["branch", "--format=%(refname:short)"])?;
    let branches: Vec<&str> = stdout.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();

    if branches.contains(&"main") {
        return Ok("main".to_string());
    }
    if branches.contains(&"master") {
        return Ok("master".to_string());
    }
    branches
        .first()
        .map(|s| s.to_string())
        .ok_or_else(|| "No branches found".to_string())
}

/// Add a feedback to a review
#[tauri::command]
pub fn add_feedback(
    review_id: String,
    target: String,
    message: String,
    severity: Option<String>,
    start_line: Option<u32>,
    end_line: Option<u32>,
) -> Result<FeedbackDto, String> {
    let store = FileReviewStore::new();
    let mut review = store
        .load(&review_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Review not found: {review_id}"))?;

    let sev = severity
        .as_deref()
        .unwrap_or("block")
        .parse::<Severity>()
        .map_err(|e| e.to_string())?;

    let mut target_obj = Target::file(target);
    if let (Some(start), Some(end)) = (start_line, end_line) {
        target_obj = target_obj.with_span(Span::range(start, end));
    } else if let Some(start) = start_line {
        target_obj = target_obj.with_span(Span::line(start));
    }

    let feedback = Feedback::new(target_obj, sev, message, FeedbackSource::Human);
    let dto = FeedbackDto::from(feedback.clone());
    review.add_feedback(feedback);

    store.save(&review).map_err(|e| e.to_string())?;
    Ok(dto)
}

/// Resolve a feedback
#[tauri::command]
pub fn resolve_feedback(review_id: String, feedback_id: String) -> Result<(), String> {
    let store = FileReviewStore::new();
    let mut review = store
        .load(&review_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Review not found: {review_id}"))?;

    let feedback = review
        .feedbacks
        .iter_mut()
        .find(|f| f.id == feedback_id)
        .ok_or_else(|| format!("Feedback not found: {feedback_id}"))?;

    feedback.resolve(None);
    store.save(&review).map_err(|e| e.to_string())?;
    Ok(())
}

/// Close a review
#[tauri::command]
pub fn close_review(id: String) -> Result<(), String> {
    let store = FileReviewStore::new();
    let mut review = store
        .load(&id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Review not found: {id}"))?;

    let blocking_count = review.blocking_feedbacks().len();
    if blocking_count > 0 {
        return Err(format!("Cannot close with {blocking_count} blocking feedback(s)"));
    }

    review.close();
    store.save(&review).map_err(|e| e.to_string())?;
    Ok(())
}

// ---------------------------------------------------------------------------
// New commands
// ---------------------------------------------------------------------------

/// Dismiss a feedback with a reason
#[tauri::command]
pub fn dismiss_feedback(
    review_id: String,
    feedback_id: String,
    reason: String,
) -> Result<(), String> {
    let dismiss_reason = parse_dismiss_reason(&reason)?;

    let store = FileReviewStore::new();
    let mut review = store
        .load(&review_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Review not found: {review_id}"))?;

    let feedback = review
        .feedbacks
        .iter_mut()
        .find(|f| f.id == feedback_id)
        .ok_or_else(|| format!("Feedback not found: {feedback_id}"))?;

    feedback.dismiss(dismiss_reason);
    store.save(&review).map_err(|e| e.to_string())?;
    Ok(())
}

/// Reopen a closed review
#[tauri::command]
pub fn reopen_review(id: String) -> Result<(), String> {
    let store = FileReviewStore::new();
    let mut review = store
        .load(&id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Review not found: {id}"))?;

    review.reopen();
    store.save(&review).map_err(|e| e.to_string())?;
    Ok(())
}

/// Get the current git branch name
#[tauri::command]
pub fn get_current_branch() -> Result<String, String> {
    Ok(run_git(&["rev-parse", "--abbrev-ref", "HEAD"])?.trim().to_string())
}

/// Get list of local branches
#[tauri::command]
pub fn get_branches() -> Result<Vec<String>, String> {
    let stdout = run_git(&["branch", "--format=%(refname:short)"])?;
    Ok(stdout.lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect())
}

/// Get the merge base between HEAD and a branch
#[tauri::command]
pub fn get_merge_base(branch: String) -> Result<String, String> {
    Ok(run_git(&["merge-base", "HEAD", &branch])?.trim().to_string())
}

/// Get file content at a specific commit, optionally limited to a line range
#[tauri::command]
pub fn get_file_content(
    path: String,
    commit: String,
    start_line: u32,
    end_line: u32,
) -> Result<String, String> {
    let content = run_git(&["show", &format!("{commit}:{path}")])?;

    // If start and end are both 0, return full content
    if start_line == 0 && end_line == 0 {
        return Ok(content);
    }

    // Extract requested line range (1-indexed, inclusive)
    let lines: Vec<&str> = content.lines().collect();
    let start = (start_line as usize).saturating_sub(1);
    let end = (end_line as usize).min(lines.len());

    if start >= lines.len() {
        return Ok(String::new());
    }

    Ok(lines[start..end].join("\n"))
}

/// Add a note to a feedback
#[tauri::command]
pub fn add_feedback_note(
    review_id: String,
    feedback_id: String,
    content: String,
) -> Result<FeedbackNoteDto, String> {
    let store = FileReviewStore::new();
    let mut review = store
        .load(&review_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Review not found: {review_id}"))?;

    let feedback = review
        .feedbacks
        .iter_mut()
        .find(|f| f.id == feedback_id)
        .ok_or_else(|| format!("Feedback not found: {feedback_id}"))?;

    let note = feedback.add_note(content);
    let dto = FeedbackNoteDto {
        id: note.id.clone(),
        content: note.content.clone(),
        created_at: note.created_at.clone(),
    };

    store.save(&review).map_err(|e| e.to_string())?;
    Ok(dto)
}

/// Toggle a file as viewed/unviewed in a review
#[tauri::command]
pub fn mark_file_viewed(review_id: String, path: String) -> Result<(), String> {
    let store = FileReviewStore::new();
    let mut review = store
        .load(&review_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Review not found: {review_id}"))?;

    review.mark_file_viewed(path);
    store.save(&review).map_err(|e| e.to_string())?;
    Ok(())
}

/// Apply a feedback's suggestion by replacing the target span in the source file
#[tauri::command]
pub fn apply_suggestion(review_id: String, feedback_id: String) -> Result<(), String> {
    let store = FileReviewStore::new();
    let mut review = store
        .load(&review_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Review not found: {review_id}"))?;

    // Find the feedback and extract the data we need before mutating
    let feedback_idx = review
        .feedbacks
        .iter()
        .position(|f| f.id == feedback_id)
        .ok_or_else(|| format!("Feedback not found: {feedback_id}"))?;

    let replacement = review.feedbacks[feedback_idx]
        .suggestion
        .clone()
        .ok_or_else(|| format!("Feedback {feedback_id} has no suggestion"))?;

    let span = review.feedbacks[feedback_idx]
        .target
        .span
        .ok_or_else(|| format!("Feedback {feedback_id} has no target span"))?;

    let path = review.feedbacks[feedback_idx].target.path.clone();

    // Read the file
    let content =
        std::fs::read_to_string(&path).map_err(|e| format!("Failed to read file {path}: {e}"))?;

    let lines: Vec<&str> = content.lines().collect();

    // Validate span is within file bounds (1-indexed, inclusive)
    let start = span.start as usize;
    let end = span.end as usize;

    if start == 0 || start > lines.len() || end > lines.len() || end < start {
        return Err(format!(
            "Span L{}-L{} is out of bounds for file {path} ({} lines)",
            span.start,
            span.end,
            lines.len()
        ));
    }

    // Build the new file content:
    // - lines before the span (0..start-1)
    // - the replacement text
    // - lines after the span (end..)
    let mut new_lines: Vec<&str> = Vec::new();
    new_lines.extend_from_slice(&lines[..start - 1]);

    // Split replacement into lines and add them
    let replacement_lines: Vec<&str> = replacement.lines().collect();
    new_lines.extend_from_slice(&replacement_lines);

    new_lines.extend_from_slice(&lines[end..]);

    // Preserve trailing newline if original file had one
    let mut output = new_lines.join("\n");
    if content.ends_with('\n') {
        output.push('\n');
    }

    // Write the modified file back
    std::fs::write(&path, &output).map_err(|e| format!("Failed to write file {path}: {e}"))?;

    // Resolve the feedback with SuggestionApplied reason
    review.feedbacks[feedback_idx].resolve(Some(ResolutionReason::SuggestionApplied));

    store.save(&review).map_err(|e| e.to_string())?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Structured diff types and command
// ---------------------------------------------------------------------------

/// Structured diff output for the frontend
#[derive(Debug, Clone, Serialize)]
pub struct StructuredDiff {
    pub files: Vec<FileDiff>,
    pub stats: DiffStats,
}

/// Aggregate diff statistics
#[derive(Debug, Clone, Serialize)]
pub struct DiffStats {
    pub files_changed: usize,
    pub additions: usize,
    pub deletions: usize,
}

/// Per-file diff
#[derive(Debug, Clone, Serialize)]
pub struct FileDiff {
    pub path: String,
    pub old_path: Option<String>,
    pub change_type: FileChangeType,
    pub hunks: Vec<DiffHunk>,
    pub additions: usize,
    pub deletions: usize,
    pub is_binary: bool,
    pub language: Option<String>,
}

/// How a file changed
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum FileChangeType {
    Added,
    Modified,
    Deleted,
    Renamed { similarity: u8 },
}

/// A diff hunk
#[derive(Debug, Clone, Serialize)]
pub struct DiffHunk {
    pub old_start: u32,
    pub old_count: u32,
    pub new_start: u32,
    pub new_count: u32,
    pub header: String,
    pub lines: Vec<DiffLineEntry>,
}

/// A single line in a diff
#[derive(Debug, Clone, Serialize)]
pub struct DiffLineEntry {
    pub kind: String,
    pub old_line_no: Option<u32>,
    pub new_line_no: Option<u32>,
    pub content: String,
    pub char_changes: Option<Vec<CharChangeEntry>>,
}

/// Character-level change within a line
#[derive(Debug, Clone, Serialize)]
pub struct CharChangeEntry {
    pub start: usize,
    pub end: usize,
    pub kind: String,
}

/// Detect language from file extension
fn detect_language(path: &str) -> Option<String> {
    let ext = path.rsplit('.').next()?;
    let lang = match ext {
        "rs" => "rust",
        "ts" | "tsx" => "typescript",
        "js" | "jsx" => "javascript",
        "py" => "python",
        "rb" => "ruby",
        "go" => "go",
        "java" => "java",
        "c" | "h" => "c",
        "cpp" | "cc" | "cxx" | "hpp" => "cpp",
        "cs" => "csharp",
        "swift" => "swift",
        "kt" | "kts" => "kotlin",
        "html" | "htm" => "html",
        "css" => "css",
        "scss" | "sass" => "scss",
        "json" => "json",
        "yaml" | "yml" => "yaml",
        "toml" => "toml",
        "md" | "markdown" => "markdown",
        "sh" | "bash" | "zsh" => "shell",
        "sql" => "sql",
        "xml" => "xml",
        "svelte" => "svelte",
        "vue" => "vue",
        "zig" => "zig",
        _ => return None,
    };
    Some(lang.to_string())
}

/// Parse a unified diff into structured types
fn parse_unified_diff(raw: &str) -> StructuredDiff {
    let mut files: Vec<FileDiff> = Vec::new();
    let mut total_additions: usize = 0;
    let mut total_deletions: usize = 0;

    let lines: Vec<&str> = raw.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];

        // Start of a new file diff
        if line.starts_with("diff --git ") {
            let (a_path, b_path) = parse_diff_header(line);
            let mut file = FileDiff {
                path: b_path.clone(),
                old_path: None,
                change_type: FileChangeType::Modified,
                hunks: Vec::new(),
                additions: 0,
                deletions: 0,
                is_binary: false,
                language: detect_language(&b_path),
            };

            i += 1;

            // Parse metadata lines before hunks
            while i < lines.len()
                && !lines[i].starts_with("@@")
                && !lines[i].starts_with("diff --git ")
            {
                let meta = lines[i];
                if meta.starts_with("new file") {
                    file.change_type = FileChangeType::Added;
                } else if meta.starts_with("deleted file") {
                    file.change_type = FileChangeType::Deleted;
                    file.path = a_path.clone();
                } else if meta.starts_with("rename from ") {
                    file.old_path = Some(meta.trim_start_matches("rename from ").to_string());
                } else if meta.starts_with("rename to ") {
                    file.path = meta.trim_start_matches("rename to ").to_string();
                    file.language = detect_language(&file.path);
                } else if meta.starts_with("similarity index ") {
                    let sim_str =
                        meta.trim_start_matches("similarity index ").trim_end_matches('%');
                    let sim: u8 = sim_str.parse().unwrap_or(100);
                    file.change_type = FileChangeType::Renamed { similarity: sim };
                } else if meta.contains("Binary files") {
                    file.is_binary = true;
                }
                i += 1;
            }

            // Parse hunks
            while i < lines.len() && !lines[i].starts_with("diff --git ") {
                if lines[i].starts_with("@@") {
                    let (hunk, consumed) = parse_hunk(&lines[i..]);
                    file.additions += hunk.lines.iter().filter(|l| l.kind == "add").count();
                    file.deletions += hunk.lines.iter().filter(|l| l.kind == "delete").count();
                    file.hunks.push(hunk);
                    i += consumed;
                } else {
                    i += 1;
                }
            }

            total_additions += file.additions;
            total_deletions += file.deletions;
            files.push(file);
        } else {
            i += 1;
        }
    }

    // Compute character-level diffs for add/delete pairs
    for file in &mut files {
        for hunk in &mut file.hunks {
            compute_char_changes(&mut hunk.lines);
        }
    }

    StructuredDiff {
        stats: DiffStats {
            files_changed: files.len(),
            additions: total_additions,
            deletions: total_deletions,
        },
        files,
    }
}

/// Parse `diff --git a/path b/path` header
fn parse_diff_header(line: &str) -> (String, String) {
    // Format: diff --git a/path b/path
    let rest = line.strip_prefix("diff --git ").unwrap_or(line);
    // Split on " b/" - handle paths with spaces by finding the last " b/"
    if let Some(idx) = rest.rfind(" b/") {
        let a = rest[..idx].strip_prefix("a/").unwrap_or(&rest[..idx]);
        let b = &rest[idx + 3..]; // skip " b/"
        (a.to_string(), b.to_string())
    } else {
        // Fallback: split on space
        let parts: Vec<&str> = rest.splitn(2, ' ').collect();
        let a = parts
            .first()
            .unwrap_or(&"")
            .strip_prefix("a/")
            .unwrap_or(parts.first().unwrap_or(&""));
        let b = parts
            .get(1)
            .unwrap_or(&"")
            .strip_prefix("b/")
            .unwrap_or(parts.get(1).unwrap_or(&""));
        (a.to_string(), b.to_string())
    }
}

/// Parse a single hunk starting at `@@`, return the hunk and how many lines were consumed
fn parse_hunk(lines: &[&str]) -> (DiffHunk, usize) {
    let header_line = lines[0];
    let (old_start, old_count, new_start, new_count) = parse_hunk_header(header_line);

    let mut hunk = DiffHunk {
        old_start,
        old_count,
        new_start,
        new_count,
        header: header_line.to_string(),
        lines: Vec::new(),
    };

    let mut old_line = old_start;
    let mut new_line = new_start;
    let mut consumed = 1; // header line

    for &line in &lines[1..] {
        if line.starts_with("diff --git ") || line.starts_with("@@") {
            break;
        }
        consumed += 1;

        if let Some(content) = line.strip_prefix('+') {
            hunk.lines.push(DiffLineEntry {
                kind: "add".to_string(),
                old_line_no: None,
                new_line_no: Some(new_line),
                content: content.to_string(),
                char_changes: None,
            });
            new_line += 1;
        } else if let Some(content) = line.strip_prefix('-') {
            hunk.lines.push(DiffLineEntry {
                kind: "delete".to_string(),
                old_line_no: Some(old_line),
                new_line_no: None,
                content: content.to_string(),
                char_changes: None,
            });
            old_line += 1;
        } else if let Some(content) = line.strip_prefix(' ') {
            hunk.lines.push(DiffLineEntry {
                kind: "context".to_string(),
                old_line_no: Some(old_line),
                new_line_no: Some(new_line),
                content: content.to_string(),
                char_changes: None,
            });
            old_line += 1;
            new_line += 1;
        } else if line == "\\ No newline at end of file" {
            // Skip this marker
        } else {
            // Treat unknown lines as context
            hunk.lines.push(DiffLineEntry {
                kind: "context".to_string(),
                old_line_no: Some(old_line),
                new_line_no: Some(new_line),
                content: line.to_string(),
                char_changes: None,
            });
            old_line += 1;
            new_line += 1;
        }
    }

    (hunk, consumed)
}

/// Parse `@@ -old_start,old_count +new_start,new_count @@ ...`
fn parse_hunk_header(line: &str) -> (u32, u32, u32, u32) {
    // Strip leading @@ and trailing @@ + optional context
    let inner = line.strip_prefix("@@ ").and_then(|s| s.split(" @@").next()).unwrap_or("");

    let parts: Vec<&str> = inner.split_whitespace().collect();

    let (old_start, old_count) = parse_range(parts.first().unwrap_or(&"-0,0"));
    let (new_start, new_count) = parse_range(parts.get(1).unwrap_or(&"+0,0"));

    (old_start, old_count, new_start, new_count)
}

/// Parse `-start,count` or `+start,count` (count defaults to 1 if omitted)
fn parse_range(s: &str) -> (u32, u32) {
    let s = s.trim_start_matches(['-', '+']);
    if let Some((start, count)) = s.split_once(',') {
        (start.parse().unwrap_or(0), count.parse().unwrap_or(0))
    } else {
        (s.parse().unwrap_or(0), 1)
    }
}

/// Compute character-level changes for consecutive delete+add pairs
fn compute_char_changes(lines: &mut [DiffLineEntry]) {
    let mut i = 0;
    while i < lines.len() {
        // Find consecutive delete lines followed by consecutive add lines
        if lines[i].kind == "delete" {
            let del_start = i;
            while i < lines.len() && lines[i].kind == "delete" {
                i += 1;
            }
            let del_end = i;

            let add_start = i;
            while i < lines.len() && lines[i].kind == "add" {
                i += 1;
            }
            let add_end = i;

            // Pair up delete and add lines for character-level diffing
            let pairs = (del_end - del_start).min(add_end - add_start);
            for p in 0..pairs {
                let del_idx = del_start + p;
                let add_idx = add_start + p;

                let old_text = &lines[del_idx].content;
                let new_text = &lines[add_idx].content;

                let text_diff = similar::TextDiff::from_chars(old_text, new_text);

                let mut del_changes: Vec<CharChangeEntry> = Vec::new();
                let mut add_changes: Vec<CharChangeEntry> = Vec::new();
                let mut old_pos: usize = 0;
                let mut new_pos: usize = 0;

                for op in text_diff.ops() {
                    match op {
                        similar::DiffOp::Equal {
                            old_index: _,
                            new_index: _,
                            len,
                        } => {
                            old_pos += len;
                            new_pos += len;
                        },
                        similar::DiffOp::Delete {
                            old_index: _,
                            old_len,
                            new_index: _,
                        } => {
                            del_changes.push(CharChangeEntry {
                                start: old_pos,
                                end: old_pos + old_len,
                                kind: "delete".to_string(),
                            });
                            old_pos += old_len;
                        },
                        similar::DiffOp::Insert {
                            old_index: _,
                            new_index: _,
                            new_len,
                        } => {
                            add_changes.push(CharChangeEntry {
                                start: new_pos,
                                end: new_pos + new_len,
                                kind: "add".to_string(),
                            });
                            new_pos += new_len;
                        },
                        similar::DiffOp::Replace {
                            old_index: _,
                            old_len,
                            new_index: _,
                            new_len,
                        } => {
                            del_changes.push(CharChangeEntry {
                                start: old_pos,
                                end: old_pos + old_len,
                                kind: "delete".to_string(),
                            });
                            add_changes.push(CharChangeEntry {
                                start: new_pos,
                                end: new_pos + new_len,
                                kind: "add".to_string(),
                            });
                            old_pos += old_len;
                            new_pos += new_len;
                        },
                    }
                }

                if !del_changes.is_empty() {
                    lines[del_idx].char_changes = Some(del_changes);
                }
                if !add_changes.is_empty() {
                    lines[add_idx].char_changes = Some(add_changes);
                }
            }
        } else {
            i += 1;
        }
    }
}

/// Get a structured diff between two commits
#[tauri::command]
pub fn get_structured_diff(base: String, head: String) -> Result<StructuredDiff, String> {
    let raw = run_git(&["diff", &format!("{base}..{head}")])?;
    Ok(parse_unified_diff(&raw))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use tempfile::TempDir;

    fn setup_test_repo() -> TempDir {
        let temp = TempDir::new().unwrap();
        env::set_current_dir(temp.path()).unwrap();

        // Initialize git repo
        Command::new("git").args(["init"]).current_dir(temp.path()).output().unwrap();

        // Create .noslop directory
        std::fs::create_dir_all(temp.path().join(".noslop/reviews")).unwrap();

        temp
    }

    #[test]
    fn test_list_reviews_empty() {
        let _temp = setup_test_repo();
        let result = list_reviews(true);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_start_review_creates_review() {
        let _temp = setup_test_repo();
        let result = start_review("abc123".into(), "def456".into(), None);
        assert!(result.is_ok());
        let review = result.unwrap();
        assert!(review.id.starts_with("REV-"));
        assert_eq!(review.base, "abc123");
        assert_eq!(review.head, "def456");
    }

    #[test]
    fn test_add_feedback_to_review() {
        let _temp = setup_test_repo();
        let review = start_review("base".into(), "head".into(), None).unwrap();

        let feedback = add_feedback(
            review.id.clone(),
            "src/main.rs".into(),
            "Add error handling".into(),
            Some("block".into()),
            None,
            None,
        )
        .unwrap();

        assert_eq!(feedback.target.path, "src/main.rs");
        assert_eq!(feedback.message, "Add error handling");
        assert_eq!(feedback.status, "open");
    }

    #[test]
    fn test_resolve_feedback() {
        let _temp = setup_test_repo();
        let review = start_review("base".into(), "head".into(), None).unwrap();
        let feedback =
            add_feedback(review.id.clone(), "file.rs".into(), "Fix".into(), None, None, None)
                .unwrap();

        let result = resolve_feedback(review.id.clone(), feedback.id);
        assert!(result.is_ok());

        // Verify feedback is resolved
        let updated = get_review(review.id).unwrap();
        assert_eq!(updated.feedbacks[0].status, "resolved");
    }

    #[test]
    fn test_close_review_with_blocking_feedbacks_fails() {
        let _temp = setup_test_repo();
        let review = start_review("base".into(), "head".into(), None).unwrap();
        add_feedback(
            review.id.clone(),
            "file.rs".into(),
            "Fix".into(),
            Some("block".into()),
            None,
            None,
        )
        .unwrap();

        let result = close_review(review.id);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("blocking"));
    }

    #[test]
    fn test_dismiss_feedback() {
        let _temp = setup_test_repo();
        let review = start_review("base".into(), "head".into(), None).unwrap();
        let feedback = add_feedback(
            review.id.clone(),
            "file.rs".into(),
            "Not applicable".into(),
            None,
            None,
            None,
        )
        .unwrap();

        let result = dismiss_feedback(review.id.clone(), feedback.id, "false_positive".into());
        assert!(result.is_ok());

        let updated = get_review(review.id).unwrap();
        assert_eq!(updated.feedbacks[0].status, "dismissed");
        assert_eq!(updated.feedbacks[0].dismiss_reason.as_deref(), Some("false_positive"));
    }

    #[test]
    fn test_dismiss_feedback_invalid_reason() {
        let _temp = setup_test_repo();
        let review = start_review("base".into(), "head".into(), None).unwrap();
        let feedback =
            add_feedback(review.id.clone(), "file.rs".into(), "Test".into(), None, None, None)
                .unwrap();

        let result = dismiss_feedback(review.id, feedback.id, "bad_reason".into());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid dismiss reason"));
    }

    #[test]
    fn test_reopen_review() {
        let _temp = setup_test_repo();
        let review = start_review("base".into(), "head".into(), None).unwrap();

        // Close it first (no blocking feedbacks)
        close_review(review.id.clone()).unwrap();
        let closed = get_review(review.id.clone()).unwrap();
        assert_eq!(closed.status, "closed");

        // Reopen
        reopen_review(review.id.clone()).unwrap();
        let reopened = get_review(review.id).unwrap();
        assert_eq!(reopened.status, "open");
    }

    #[test]
    fn test_add_feedback_note() {
        let _temp = setup_test_repo();
        let review = start_review("base".into(), "head".into(), None).unwrap();
        let feedback =
            add_feedback(review.id.clone(), "file.rs".into(), "Test".into(), None, None, None)
                .unwrap();

        let note = add_feedback_note(review.id.clone(), feedback.id, "Needs investigation".into())
            .unwrap();
        assert!(note.id.starts_with("N-"));
        assert_eq!(note.content, "Needs investigation");

        let updated = get_review(review.id).unwrap();
        assert_eq!(updated.feedbacks[0].notes.len(), 1);
    }

    #[test]
    fn test_mark_file_viewed() {
        let _temp = setup_test_repo();
        let review = start_review("base".into(), "head".into(), None).unwrap();

        // Mark viewed
        mark_file_viewed(review.id.clone(), "src/main.rs".into()).unwrap();
        let updated = get_review(review.id.clone()).unwrap();
        assert_eq!(updated.viewed_files, vec!["src/main.rs"]);

        // Toggle off
        mark_file_viewed(review.id.clone(), "src/main.rs".into()).unwrap();
        let updated = get_review(review.id).unwrap();
        assert!(updated.viewed_files.is_empty());
    }

    #[test]
    fn test_detect_language() {
        assert_eq!(detect_language("src/main.rs"), Some("rust".to_string()));
        assert_eq!(detect_language("app.ts"), Some("typescript".to_string()));
        assert_eq!(detect_language("script.py"), Some("python".to_string()));
        assert_eq!(detect_language("page.svelte"), Some("svelte".to_string()));
        assert_eq!(detect_language("noext"), None);
    }

    #[test]
    fn test_parse_unified_diff_simple() {
        let diff = "\
diff --git a/src/main.rs b/src/main.rs
index abc1234..def5678 100644
--- a/src/main.rs
+++ b/src/main.rs
@@ -1,3 +1,4 @@
 fn main() {
+    println!(\"Hello\");
     let x = 1;
 }
";
        let result = parse_unified_diff(diff);
        assert_eq!(result.files.len(), 1);
        assert_eq!(result.files[0].path, "src/main.rs");
        assert_eq!(result.stats.files_changed, 1);
        assert_eq!(result.stats.additions, 1);
        assert_eq!(result.stats.deletions, 0);
        assert_eq!(result.files[0].hunks.len(), 1);
    }

    #[test]
    fn test_parse_unified_diff_new_file() {
        let diff = "\
diff --git a/new_file.rs b/new_file.rs
new file mode 100644
index 0000000..abc1234
--- /dev/null
+++ b/new_file.rs
@@ -0,0 +1,3 @@
+fn hello() {
+    println!(\"hi\");
+}
";
        let result = parse_unified_diff(diff);
        assert_eq!(result.files.len(), 1);
        assert!(matches!(result.files[0].change_type, FileChangeType::Added));
        assert_eq!(result.stats.additions, 3);
    }

    #[test]
    fn test_parse_unified_diff_deleted_file() {
        let diff = "\
diff --git a/old_file.rs b/old_file.rs
deleted file mode 100644
index abc1234..0000000
--- a/old_file.rs
+++ /dev/null
@@ -1,2 +0,0 @@
-fn old() {
-}
";
        let result = parse_unified_diff(diff);
        assert_eq!(result.files.len(), 1);
        assert!(matches!(result.files[0].change_type, FileChangeType::Deleted));
        assert_eq!(result.stats.deletions, 2);
    }

    #[test]
    fn test_parse_hunk_header() {
        assert_eq!(parse_hunk_header("@@ -1,3 +1,4 @@ fn main"), (1, 3, 1, 4));
        assert_eq!(parse_hunk_header("@@ -10 +10 @@"), (10, 1, 10, 1));
        assert_eq!(parse_hunk_header("@@ -0,0 +1,5 @@"), (0, 0, 1, 5));
    }

    #[test]
    fn test_char_changes_computed() {
        let diff = "\
diff --git a/test.rs b/test.rs
index abc..def 100644
--- a/test.rs
+++ b/test.rs
@@ -1,1 +1,1 @@
-let x = 1;
+let x = 2;
";
        let result = parse_unified_diff(diff);
        let hunk = &result.files[0].hunks[0];
        // delete line should have char_changes
        assert!(hunk.lines[0].char_changes.is_some());
        // add line should have char_changes
        assert!(hunk.lines[1].char_changes.is_some());
    }

    #[test]
    fn test_parse_empty_diff() {
        let result = parse_unified_diff("");
        assert!(result.files.is_empty());
        assert_eq!(result.stats.files_changed, 0);
    }
}
