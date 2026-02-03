//! File-based review storage
//!
//! Reviews are stored in `.noslop/reviews/` directory, one JSON file per review.

use std::fs;
use std::path::PathBuf;

use crate::core::models::{Review, ReviewStatus};
use crate::core::ports::ReviewStore;

const REVIEWS_DIR: &str = ".noslop/reviews";

/// File-based review storage
#[derive(Debug, Clone)]
pub struct FileReviewStore {
    base_path: PathBuf,
}

impl Default for FileReviewStore {
    fn default() -> Self {
        Self::new()
    }
}

impl FileReviewStore {
    /// Create a new file review store using the default path
    #[must_use]
    pub fn new() -> Self {
        Self {
            base_path: PathBuf::from(REVIEWS_DIR),
        }
    }

    /// Create a file review store with a custom base path (for testing)
    #[must_use]
    pub const fn with_path(base_path: PathBuf) -> Self {
        Self { base_path }
    }

    fn review_path(&self, id: &str) -> PathBuf {
        self.base_path.join(format!("{id}.json"))
    }

    fn ensure_dir(&self) -> anyhow::Result<()> {
        fs::create_dir_all(&self.base_path)?;
        Ok(())
    }
}

impl ReviewStore for FileReviewStore {
    fn save(&self, review: &Review) -> anyhow::Result<()> {
        self.ensure_dir()?;
        let path = self.review_path(&review.id);
        let content = serde_json::to_string_pretty(review)?;
        fs::write(path, content)?;
        Ok(())
    }

    fn load(&self, id: &str) -> anyhow::Result<Option<Review>> {
        let path = self.review_path(id);
        if !path.exists() {
            return Ok(None);
        }
        let content = fs::read_to_string(path)?;
        let review: Review = serde_json::from_str(&content)?;
        Ok(Some(review))
    }

    fn list_all(&self) -> anyhow::Result<Vec<Review>> {
        if !self.base_path.exists() {
            return Ok(Vec::new());
        }

        let mut reviews = Vec::new();
        for entry in fs::read_dir(&self.base_path)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "json") {
                let content = fs::read_to_string(&path)?;
                let review: Review = serde_json::from_str(&content)?;
                reviews.push(review);
            }
        }

        // Sort by created_at descending (newest first)
        reviews.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(reviews)
    }

    fn list_open(&self) -> anyhow::Result<Vec<Review>> {
        let all = self.list_all()?;
        Ok(all.into_iter().filter(|r| r.status == ReviewStatus::Open).collect())
    }

    fn delete(&self, id: &str) -> anyhow::Result<bool> {
        let path = self.review_path(id);
        if path.exists() {
            fs::remove_file(path)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn find_blocking_for_file(&self, file: &str) -> anyhow::Result<Vec<Review>> {
        let open = self.list_open()?;
        let blocking: Vec<Review> = open
            .into_iter()
            .filter(|r| !r.open_comments_for_file(file).is_empty())
            .collect();
        Ok(blocking)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::models::DiffPosition;
    use tempfile::TempDir;

    fn test_store() -> (FileReviewStore, TempDir) {
        let dir = TempDir::new().unwrap();
        let store = FileReviewStore::with_path(dir.path().join("reviews"));
        (store, dir)
    }

    #[test]
    fn test_save_and_load() {
        let (store, _dir) = test_store();

        let review = Review::new("abc123".to_string(), "def456".to_string());
        let id = review.id.clone();

        store.save(&review).unwrap();

        let loaded = store.load(&id).unwrap();
        assert!(loaded.is_some());
        let loaded = loaded.unwrap();
        assert_eq!(loaded.id, id);
        assert_eq!(loaded.base_sha, "abc123");
    }

    #[test]
    fn test_load_nonexistent() {
        let (store, _dir) = test_store();
        let loaded = store.load("nonexistent").unwrap();
        assert!(loaded.is_none());
    }

    #[test]
    fn test_list_all() {
        let (store, _dir) = test_store();

        let r1 = Review::new("a".to_string(), "b".to_string());
        let r2 = Review::new("c".to_string(), "d".to_string());

        store.save(&r1).unwrap();
        store.save(&r2).unwrap();

        let all = store.list_all().unwrap();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_list_open() {
        let (store, _dir) = test_store();

        let r1 = Review::new("a".to_string(), "b".to_string());
        let mut r2 = Review::new("c".to_string(), "d".to_string());
        r2.close();

        store.save(&r1).unwrap();
        store.save(&r2).unwrap();

        let open = store.list_open().unwrap();
        assert_eq!(open.len(), 1);
        assert_eq!(open[0].status, ReviewStatus::Open);
    }

    #[test]
    fn test_delete() {
        let (store, _dir) = test_store();

        let review = Review::new("a".to_string(), "b".to_string());
        let id = review.id.clone();

        store.save(&review).unwrap();
        assert!(store.load(&id).unwrap().is_some());

        let deleted = store.delete(&id).unwrap();
        assert!(deleted);
        assert!(store.load(&id).unwrap().is_none());
    }

    #[test]
    fn test_find_blocking_for_file() {
        let (store, _dir) = test_store();

        let mut r1 = Review::new("a".to_string(), "b".to_string());
        r1.add_comment(
            "src/main.rs".to_string(),
            "Fix this".to_string(),
            DiffPosition::new_line(10),
        );

        let mut r2 = Review::new("c".to_string(), "d".to_string());
        r2.add_comment(
            "src/lib.rs".to_string(),
            "Other file".to_string(),
            DiffPosition::new_line(20),
        );

        store.save(&r1).unwrap();
        store.save(&r2).unwrap();

        let blocking = store.find_blocking_for_file("src/main.rs").unwrap();
        assert_eq!(blocking.len(), 1);
        assert_eq!(blocking[0].comments[0].check.target, "src/main.rs");
    }
}
