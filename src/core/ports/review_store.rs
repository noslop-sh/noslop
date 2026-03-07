//! Review storage port
//!
//! Trait for persisting and retrieving review sessions.

use crate::core::models::Review;

/// Storage backend for reviews
pub trait ReviewStore: Send + Sync {
    /// Save a review (create or update)
    fn save(&self, review: &Review) -> anyhow::Result<()>;

    /// Load a review by ID
    fn load(&self, id: &str) -> anyhow::Result<Option<Review>>;

    /// List all reviews
    fn list_all(&self) -> anyhow::Result<Vec<Review>>;

    /// List only open reviews
    fn list_open(&self) -> anyhow::Result<Vec<Review>>;

    /// Delete a review by ID
    fn delete(&self, id: &str) -> anyhow::Result<bool>;

    /// Find reviews with open comments that apply to a file
    fn find_blocking_for_file(&self, file: &str) -> anyhow::Result<Vec<Review>>;
}
