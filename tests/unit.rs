//! Unit tests for noslop
//!
//! These tests verify individual components and functions in isolation.

// Common test utilities
#[path = "unit/common/mod.rs"]
#[allow(dead_code)]
mod common;

#[path = "unit/check_test.rs"]
mod check_test;

#[path = "unit/cli_test.rs"]
mod cli_test;

#[path = "unit/output_test.rs"]
mod output_test;

#[path = "unit/parameterized_test.rs"]
mod parameterized_test;

#[path = "unit/parser_test.rs"]
mod parser_test;

#[path = "unit/proptest_matcher.rs"]
mod proptest_matcher;

#[path = "unit/resolver_test.rs"]
mod resolver_test;

#[path = "unit/storage_test.rs"]
mod storage_test;

#[path = "unit/target_test.rs"]
mod target_test;
