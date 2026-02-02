//! Port traits (interfaces) for external dependencies
//!
//! These traits define the boundaries between core business logic
//! and external systems (filesystem, git, storage backends).
//!
//! Implementations live in the `adapters` module.

mod assertion_repo;
mod attestation_store;
mod vcs;

pub use assertion_repo::AssertionRepository;
pub use attestation_store::AttestationStore;
pub use vcs::VersionControl;
