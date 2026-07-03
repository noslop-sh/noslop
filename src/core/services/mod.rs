//! Business logic services
//!
//! Pure orchestration logic that operates on domain models.
//! These services have no I/O dependencies - they operate on
//! data passed in and return results.
//!
//! - [`checker`] - Check checks against acknowledgments
//! - [`matcher`] - Match target patterns to file paths

pub mod checker;
pub mod curate;
pub mod discovery;
pub mod matcher;
pub mod merge;
pub mod stats;

pub use checker::{CheckItemResult, CheckResult, check_items};
pub use matcher::matches_target;
pub use merge::merge_checks;
