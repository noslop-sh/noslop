//! Tauri IPC command handlers
//!
//! These commands expose noslop functionality to the frontend via Tauri's invoke API.

use noslop::Review;
use noslop::adapters::FileReviewStore;
use noslop::core::models::{Finding, FindingSource, Severity, Target};
use noslop::core::ports::ReviewStore;
use std::process::Command;

use crate::dto::{FindingDto, ReviewDto};

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

/// Add a finding to a review
#[tauri::command]
pub fn add_finding(
    review_id: String,
    target: String,
    message: String,
    severity: Option<String>,
) -> Result<FindingDto, String> {
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

    let finding = Finding::new(Target::file(target), sev, message, FindingSource::Human);
    let dto = FindingDto::from(finding.clone());
    review.add_finding(finding);

    store.save(&review).map_err(|e| e.to_string())?;
    Ok(dto)
}

/// Resolve a finding
#[tauri::command]
pub fn resolve_finding(review_id: String, finding_id: String) -> Result<(), String> {
    let store = FileReviewStore::new();
    let mut review = store
        .load(&review_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Review not found: {review_id}"))?;

    let finding = review
        .findings
        .iter_mut()
        .find(|f| f.id == finding_id)
        .ok_or_else(|| format!("Finding not found: {finding_id}"))?;

    finding.resolve();
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

    let blocking_count = review.blocking_findings().len();
    if blocking_count > 0 {
        return Err(format!("Cannot close with {blocking_count} blocking finding(s)"));
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
        assert_eq!(review.base, "abc123");
        assert_eq!(review.head, "def456");
    }

    #[test]
    fn test_add_finding_to_review() {
        let _temp = setup_test_repo();
        let review = start_review("base".into(), "head".into()).unwrap();

        let finding = add_finding(
            review.id.clone(),
            "src/main.rs".into(),
            "Add error handling".into(),
            Some("block".into()),
        )
        .unwrap();

        assert_eq!(finding.target, "src/main.rs");
        assert_eq!(finding.message, "Add error handling");
        assert_eq!(finding.status, "open");
    }

    #[test]
    fn test_resolve_finding() {
        let _temp = setup_test_repo();
        let review = start_review("base".into(), "head".into()).unwrap();
        let finding = add_finding(review.id.clone(), "file.rs".into(), "Fix".into(), None).unwrap();

        let result = resolve_finding(review.id.clone(), finding.id);
        assert!(result.is_ok());

        // Verify finding is resolved
        let updated = get_review(review.id).unwrap();
        assert_eq!(updated.findings[0].status, "resolved");
    }

    #[test]
    fn test_close_review_with_blocking_findings_fails() {
        let _temp = setup_test_repo();
        let review = start_review("base".into(), "head".into()).unwrap();
        add_finding(review.id.clone(), "file.rs".into(), "Fix".into(), Some("block".into()))
            .unwrap();

        let result = close_review(review.id);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("blocking"));
    }
}
