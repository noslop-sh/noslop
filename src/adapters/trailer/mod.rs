//! Commit trailer attestation storage
//!
//! Implements `AttestationStore` using git commit trailers.

mod store;

pub use store::TrailerAttestationStore;
