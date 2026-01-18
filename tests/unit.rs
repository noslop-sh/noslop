//! Unit tests for noslop
//!
//! These tests verify individual components and functions in isolation.

// Common test utilities
#[path = "unit/common/mod.rs"]
#[allow(dead_code)]
mod common;

#[path = "unit/cli_test.rs"]
mod cli_test;

#[path = "unit/output_test.rs"]
mod output_test;

#[path = "unit/parser_test.rs"]
mod parser_test;

#[path = "unit/resolver_test.rs"]
mod resolver_test;

#[path = "unit/storage_test.rs"]
mod storage_test;

#[path = "unit/scope_test.rs"]
mod scope_test;

#[path = "unit/task_test.rs"]
mod task_test;

#[path = "unit/verification_test.rs"]
mod verification_test;

#[path = "unit/refs_test.rs"]
mod refs_test;

#[cfg(feature = "ui")]
#[path = "unit/api_test.rs"]
mod api_test;

#[cfg(feature = "ui")]
#[path = "unit/config_test.rs"]
mod config_test;
