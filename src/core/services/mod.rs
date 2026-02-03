//! Business logic services
//!
//! Pure orchestration logic that operates on domain models.
//! These services have no I/O dependencies - they operate on
//! data passed in and return results.
//!
//! - [`checker`] - Check checks against acknowledgments
//! - [`matcher`] - Match target patterns to file paths

pub mod checker;
pub mod matcher;

pub use checker::{CheckItemResult, CheckResult, check_items};
pub use matcher::matches_target;
