//! Tauri IPC command handlers
//!
//! These commands expose noslop functionality to the frontend via Tauri's invoke API.

use noslop::adapters::FileReviewStore;
use noslop::core::models::{Acknowledgment, DiffPosition, Review};
use noslop::core::ports::ReviewStore;
use noslop::storage;
use std::process::Command;

use crate::dto::{CommentDto, ReviewDto};

/// List all reviews, optionally filtering to open only
#[tauri::command]
pub fn list_reviews(open_only: bool) -> Result<Vec<ReviewDto>, String> {
    let store = FileReviewStore::new();
    let reviews = if open_only {
        store.list_open()
    } else {
        store.list_all()
    }
    .map_err(|e| e.to_string())?;

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
    let output = Command::new("git")
        .args(["diff", &format!("{base}..{head}")])
        .output()
        .map_err(|e| e.to_string())?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    String::from_utf8(output.stdout).map_err(|e| e.to_string())
}

/// Start a new review for a commit range
#[tauri::command]
pub fn start_review(base: String, head: String) -> Result<ReviewDto, String> {
    let store = FileReviewStore::new();
    let review = Review::new(base, head);
    store.save(&review).map_err(|e| e.to_string())?;
    Ok(ReviewDto::from(review))
}

/// Add a comment to a review
#[tauri::command]
pub fn add_comment(
    review_id: String,
    target: String,
    message: String,
    line: Option<u32>,
) -> Result<CommentDto, String> {
    let store = FileReviewStore::new();
    let mut review = store
        .load(&review_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Review not found: {review_id}"))?;

    let position = line.map_or(DiffPosition::default(), DiffPosition::new_line);
    review.add_comment(target, message, position);

    let comment = review.comments.last().unwrap();
    let dto = CommentDto::from(comment.clone());

    store.save(&review).map_err(|e| e.to_string())?;
    Ok(dto)
}

/// Resolve a comment
#[tauri::command]
pub fn resolve_comment(comment_id: String, message: Option<String>) -> Result<(), String> {
    // Parse comment ID: "REV-xxxx:N"
    let parts: Vec<&str> = comment_id.rsplitn(2, ':').collect();
    if parts.len() != 2 {
        return Err("Invalid comment ID format. Expected: REV-xxxx:N".to_string());
    }
    let review_id = parts[1];

    let store = FileReviewStore::new();
    let mut review = store
        .load(review_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Review not found: {review_id}"))?;

    let comment = review
        .comments
        .iter_mut()
        .find(|c| c.check.id == comment_id)
        .ok_or_else(|| format!("Comment not found: {comment_id}"))?;

    comment.resolve(message);
    store.save(&review).map_err(|e| e.to_string())?;
    Ok(())
}

/// Close a review (stages acknowledgments for commit trailers)
#[tauri::command]
pub fn close_review(id: String) -> Result<(), String> {
    let store = FileReviewStore::new();
    let mut review = store
        .load(&id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Review not found: {id}"))?;

    let open_count = review.open_comments().len();
    if open_count > 0 {
        return Err(format!("Cannot close with {open_count} unresolved comment(s)"));
    }

    // Stage acknowledgments for all resolved comments
    let ack_store = storage::ack_store();
    for comment in &review.comments {
        let msg = comment.resolution_message.clone().unwrap_or_else(|| "Resolved".to_string());
        let ack = Acknowledgment::new(comment.check.id.clone(), msg, "review".to_string());
        ack_store.stage(&ack).map_err(|e| e.to_string())?;
    }

    review.close();
    store.save(&review).map_err(|e| e.to_string())?;
    Ok(())
}

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
        let result = start_review("abc123".into(), "def456".into());
        assert!(result.is_ok());
        let review = result.unwrap();
        assert!(review.id.starts_with("REV-"));
        assert_eq!(review.base_sha, "abc123");
        assert_eq!(review.head_sha, "def456");
    }

    #[test]
    fn test_add_comment_to_review() {
        let _temp = setup_test_repo();
        let review = start_review("base".into(), "head".into()).unwrap();

        let comment = add_comment(
            review.id.clone(),
            "src/main.rs".into(),
            "Add error handling".into(),
            Some(42),
        )
        .unwrap();

        assert_eq!(comment.target, "src/main.rs");
        assert_eq!(comment.message, "Add error handling");
        assert_eq!(comment.line, Some(42));
        assert_eq!(comment.status, "open");
    }

    #[test]
    fn test_resolve_comment() {
        let _temp = setup_test_repo();
        let review = start_review("base".into(), "head".into()).unwrap();
        add_comment(review.id.clone(), "file.rs".into(), "Fix".into(), None).unwrap();

        let comment_id = format!("{}:1", review.id);
        let result = resolve_comment(comment_id, Some("Done".into()));
        assert!(result.is_ok());

        // Verify comment is resolved
        let updated = get_review(review.id).unwrap();
        assert_eq!(updated.comments[0].status, "resolved");
    }

    #[test]
    fn test_close_review_with_open_comments_fails() {
        let _temp = setup_test_repo();
        let review = start_review("base".into(), "head".into()).unwrap();
        add_comment(review.id.clone(), "file.rs".into(), "Fix".into(), None).unwrap();

        let result = close_review(review.id);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("unresolved"));
    }
}
