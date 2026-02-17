//! Core domain logic for noslop
//!
//! This module contains pure business logic with no I/O dependencies.
//! All external interactions are abstracted through port traits.
//!
//! # Hexagonal Architecture
//!
//! ```text
//!                  +-------------------------------------+
//!                  |           Adapters (I/O)            |
//!                  |  +-----+  +-----+                  |
//!                  |  |TOML |  | Git |                  |
//!                  |  +--+--+  +--+--+                  |
//!                  +-----+--------+--------------------+
//!                        |        |
//!                  +-----v--------v--------------------+
//!                  |              Ports (Traits)        |
//!                  |  CheckRepository                   |
//!                  |  ReviewStore                       |
//!                  |  VersionControl                    |
//!                  +-----------------+-----------------+
//!                                   |
//!                  +-----------------v-----------------+
//!                  |           Core (Pure Logic)       |
//!                  |  +----------+                     |
//!                  |  |  Models  |                     |
//!                  |  |  Check   |                     |
//!                  |  |  Finding |                     |
//!                  |  |  Review  |                     |
//!                  |  | Severity |                     |
//!                  |  |  Target  |                     |
//!                  |  +----------+                     |
//!                  +-----------------------------------+
//! ```
//!
//! # Modules
//!
//! - [`models`] - Domain types (Check, Finding, Review, Target, Severity)
//! - [`services`] - Business logic (pipeline, analyzers) -- added in later phases
//! - [`ports`] - Trait definitions for external dependencies

pub mod models;
pub mod ports;
pub mod services;
