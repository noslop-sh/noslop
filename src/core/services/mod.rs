//! Business logic services
//!
//! Pure orchestration logic that operates on domain models
//! and interacts with the outside world through port traits.

mod checker;
mod matcher;

pub use checker::{CheckResult, CheckService};
pub use matcher::TargetMatcher;
