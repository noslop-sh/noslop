//! Business logic services
//!
//! Pure orchestration logic that operates on domain models.
//! These services have no I/O dependencies - they operate on
//! data passed in and return results.
//!
//! - [`checker`] - Check assertions against attestations
//! - [`matcher`] - Match target patterns to file paths

pub mod checker;
pub mod matcher;

pub use checker::{AssertionCheckResult, CheckResult, check_assertions};
pub use matcher::matches_target;
