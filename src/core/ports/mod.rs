//! Port traits (interfaces) for external dependencies
//!
//! These traits define the boundaries between core business logic
//! and external systems (filesystem, git, storage backends).
//!
//! Implementations live in the `adapters` module.

mod check_repo;
mod review_store;
mod vcs;

pub use check_repo::CheckRepository;
pub use review_store::ReviewStore;
pub use vcs::VersionControl;
