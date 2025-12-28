//! Integration tests for noslop CLI
//!
//! These tests simulate real-world workflows with a complete codebase,
//! testing the full cycle of: init → assert → attest → check

// Include lifecycle tests from the same directory
mod lifecycle_test;

use assert_cmd::cargo;
use predicates::prelude::*;
use std::fs;
use std::process::Command;
use tempfile::TempDir;

/// Helper function to create a noslop command
fn noslop() -> assert_cmd::Command {
    assert_cmd::Command::new(cargo::cargo_bin!("noslop"))
}

/// Helper to initialize a git repo with basic config
fn init_git_repo(path: &std::path::Path) {
    Command::new("git")
        .args(["init"])
        .current_dir(path)
        .output()
        .expect("Failed to init git repo");

    // Configure git user for commits
    Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(path)
        .output()
        .expect("Failed to configure git email");

    Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(path)
        .output()
        .expect("Failed to configure git name");
}

/// Helper to stage files in git
fn git_add(path: &std::path::Path, file: &str) {
    Command::new("git")
        .args(["add", file])
        .current_dir(path)
        .output()
        .expect("Failed to stage file");
}

/// Helper to create a git commit
fn git_commit(path: &std::path::Path, message: &str) {
    Command::new("git")
        .args(["commit", "-m", message])
        .current_dir(path)
        .output()
        .expect("Failed to create commit");
}

// =============================================================================
// END-TO-END WORKFLOW TESTS
// =============================================================================

/// Test complete workflow: init → create files → add assertion → attest → check
#[test]
fn test_e2e_complete_workflow() {
    let temp = TempDir::new().unwrap();
    let repo_path = temp.path();

    // Step 1: Initialize git repo
    init_git_repo(repo_path);

    // Step 2: Initialize noslop
    noslop()
        .arg("init")
        .current_dir(repo_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("Created .noslop.toml"));

    // Step 3: Create initial commit so we have a HEAD
    git_add(repo_path, ".noslop.toml");
    git_commit(repo_path, "Initial commit");

    // Step 4: Add an assertion for Rust files
    noslop()
        .args(["assert", "add", "*.rs", "-m", "Ensure all Rust code follows style guidelines"])
        .current_dir(repo_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("Added assertion"));

    // Step 5: Create a Rust source file
    let src_dir = repo_path.join("src");
    fs::create_dir_all(&src_dir).unwrap();
    fs::write(
        src_dir.join("main.rs"),
        r#"fn main() {
    println!("Hello, noslop!");
}
"#,
    )
    .unwrap();

    // Step 6: Stage the new file
    git_add(repo_path, "src/main.rs");

    // Step 7: Check should FAIL because assertion is not attested
    noslop()
        .args(["check", "--ci"])
        .current_dir(repo_path)
        .assert()
        .failure()
        .stdout(predicate::str::contains("BLOCKED"))
        .stdout(predicate::str::contains("Ensure all Rust code follows style guidelines"));

    // Step 8: Attest to the assertion
    noslop()
        .args([
            "attest",
            "Ensure all Rust code follows style guidelines",
            "-m",
            "Reviewed code and it follows rustfmt conventions",
        ])
        .current_dir(repo_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("Staged attestation"));

    // Step 9: Check should now PASS
    noslop()
        .args(["check", "--ci"])
        .current_dir(repo_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("All assertions attested"));
}

/// Test that check fails when blocking assertions are unattested
#[test]
fn test_check_blocks_without_attestation() {
    let temp = TempDir::new().unwrap();
    let repo_path = temp.path();

    // Setup
    init_git_repo(repo_path);

    // Create .noslop.toml with a blocking assertion
    fs::write(
        repo_path.join(".noslop.toml"),
        r#"[[assert]]
target = "*.py"
message = "Python code must have type hints"
severity = "block"
"#,
    )
    .unwrap();

    // Initial commit
    git_add(repo_path, ".noslop.toml");
    git_commit(repo_path, "Initial commit");

    // Create Python file
    fs::write(
        repo_path.join("app.py"),
        r#"def greet(name):
    return f"Hello, {name}!"
"#,
    )
    .unwrap();

    // Stage the file
    git_add(repo_path, "app.py");

    // Check should fail
    noslop()
        .args(["check", "--ci"])
        .current_dir(repo_path)
        .assert()
        .failure()
        .stdout(predicate::str::contains("BLOCKED"))
        .stdout(predicate::str::contains("Python code must have type hints"));
}

/// Test that warn severity shows warnings but doesn't block
#[test]
fn test_warn_severity_shows_warning_but_passes() {
    let temp = TempDir::new().unwrap();
    let repo_path = temp.path();

    // Setup
    init_git_repo(repo_path);

    // Create .noslop.toml with a warn assertion
    fs::write(
        repo_path.join(".noslop.toml"),
        r#"[[assert]]
target = "*.js"
message = "Consider adding JSDoc comments"
severity = "warn"
"#,
    )
    .unwrap();

    // Initial commit
    git_add(repo_path, ".noslop.toml");
    git_commit(repo_path, "Initial commit");

    // Create JS file
    fs::write(
        repo_path.join("index.js"),
        r#"function add(a, b) {
    return a + b;
}
"#,
    )
    .unwrap();

    // Stage the file
    git_add(repo_path, "index.js");

    // Check should pass but show warning
    noslop()
        .args(["check", "--ci"])
        .current_dir(repo_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("Warnings"))
        .stdout(predicate::str::contains("Consider adding JSDoc comments"))
        .stdout(
            predicate::str::contains("All assertions attested")
                .or(predicate::str::contains("Commit may proceed")),
        );
}

// =============================================================================
// MULTIPLE ASSERTIONS TESTS
// =============================================================================

/// Test handling multiple assertions for different file types
#[test]
fn test_multiple_assertions_different_targets() {
    let temp = TempDir::new().unwrap();
    let repo_path = temp.path();

    // Setup
    init_git_repo(repo_path);

    // Create .noslop.toml with multiple assertions
    fs::write(
        repo_path.join(".noslop.toml"),
        r#"[[assert]]
target = "*.rs"
message = "Rust code must be formatted with rustfmt"
severity = "block"

[[assert]]
target = "*.ts"
message = "TypeScript must use strict mode"
severity = "block"

[[assert]]
target = "*.md"
message = "Documentation should be reviewed"
severity = "warn"
"#,
    )
    .unwrap();

    // Create .noslop directory for staging
    fs::create_dir_all(repo_path.join(".noslop")).unwrap();

    // Initial commit
    git_add(repo_path, ".noslop.toml");
    git_commit(repo_path, "Initial commit");

    // Create multiple files
    fs::write(repo_path.join("lib.rs"), "pub fn hello() {}").unwrap();
    fs::write(repo_path.join("app.ts"), "const x: number = 1;").unwrap();
    fs::write(repo_path.join("README.md"), "# My Project").unwrap();

    // Stage all files
    git_add(repo_path, "lib.rs");
    git_add(repo_path, "app.ts");
    git_add(repo_path, "README.md");

    // Check should fail - missing Rust and TypeScript attestations
    noslop()
        .args(["check", "--ci"])
        .current_dir(repo_path)
        .assert()
        .failure()
        .stdout(predicate::str::contains("BLOCKED"))
        .stdout(predicate::str::contains("Rust code must be formatted with rustfmt"))
        .stdout(predicate::str::contains("TypeScript must use strict mode"));

    // Attest to Rust assertion only
    noslop()
        .args([
            "attest",
            "Rust code must be formatted with rustfmt",
            "-m",
            "Ran rustfmt on all files",
        ])
        .current_dir(repo_path)
        .assert()
        .success();

    // Still should fail - missing TypeScript attestation
    noslop()
        .args(["check", "--ci"])
        .current_dir(repo_path)
        .assert()
        .failure()
        .stdout(predicate::str::contains("TypeScript must use strict mode"));

    // Attest to TypeScript assertion
    noslop()
        .args([
            "attest",
            "TypeScript must use strict mode",
            "-m",
            "Added strict: true to tsconfig",
        ])
        .current_dir(repo_path)
        .assert()
        .success();

    // Now should pass
    noslop()
        .args(["check", "--ci"])
        .current_dir(repo_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("All assertions attested"));
}

/// Test that same assertion applies to multiple matching files
#[test]
fn test_single_assertion_multiple_files() {
    let temp = TempDir::new().unwrap();
    let repo_path = temp.path();

    // Setup
    init_git_repo(repo_path);

    // Create .noslop.toml with glob pattern (using *.rs which is more reliable)
    fs::write(
        repo_path.join(".noslop.toml"),
        r#"[[assert]]
target = "*.rs"
message = "All source files need security review"
severity = "block"
"#,
    )
    .unwrap();

    // Initial commit
    git_add(repo_path, ".noslop.toml");
    git_commit(repo_path, "Initial commit");

    // Create multiple Rust files at root level
    fs::write(repo_path.join("main.rs"), "fn main() {}").unwrap();
    fs::write(repo_path.join("lib.rs"), "pub fn lib() {}").unwrap();
    fs::write(repo_path.join("utils.rs"), "pub fn util() {}").unwrap();

    // Stage all files
    git_add(repo_path, "main.rs");
    git_add(repo_path, "lib.rs");
    git_add(repo_path, "utils.rs");

    // Check should fail
    noslop()
        .args(["check", "--ci"])
        .current_dir(repo_path)
        .assert()
        .failure()
        .stdout(predicate::str::contains("BLOCKED"));

    // Single attestation should cover all files
    noslop()
        .args([
            "attest",
            "All source files need security review",
            "-m",
            "Reviewed all files for security issues",
        ])
        .current_dir(repo_path)
        .assert()
        .success();

    // Now should pass
    noslop()
        .args(["check", "--ci"])
        .current_dir(repo_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("All assertions attested"));
}

// =============================================================================
// ATTESTATION MATCHING TESTS
// =============================================================================

/// Test attestation matching by assertion target
#[test]
fn test_attest_by_target() {
    let temp = TempDir::new().unwrap();
    let repo_path = temp.path();

    // Setup
    init_git_repo(repo_path);

    // Create .noslop.toml
    fs::write(
        repo_path.join(".noslop.toml"),
        r#"[[assert]]
target = "config/*.yaml"
message = "Config changes require DevOps review"
severity = "block"
"#,
    )
    .unwrap();

    // Create config directory
    let config_dir = repo_path.join("config");
    fs::create_dir_all(&config_dir).unwrap();
    fs::write(config_dir.join("app.yaml"), "key: value").unwrap();

    // Initial commit
    git_add(repo_path, ".noslop.toml");
    git_commit(repo_path, "Initial commit");

    // Stage config file
    git_add(repo_path, "config/app.yaml");

    // Attest by target pattern
    noslop()
        .args(["attest", "config/*.yaml", "-m", "DevOps team approved changes"])
        .current_dir(repo_path)
        .assert()
        .success();

    // Should pass
    noslop()
        .args(["check", "--ci"])
        .current_dir(repo_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("All assertions attested"));
}

/// Test attestation matching by partial message
#[test]
fn test_attest_by_partial_message() {
    let temp = TempDir::new().unwrap();
    let repo_path = temp.path();

    // Setup
    init_git_repo(repo_path);

    // Create .noslop.toml with a long message
    fs::write(
        repo_path.join(".noslop.toml"),
        r#"[[assert]]
target = "*.sql"
message = "Database migrations must be reviewed by DBA team for performance impact"
severity = "block"
"#,
    )
    .unwrap();

    // Create SQL file
    fs::write(repo_path.join("migration.sql"), "CREATE TABLE users (id INT);").unwrap();

    // Initial commit
    git_add(repo_path, ".noslop.toml");
    git_commit(repo_path, "Initial commit");

    // Stage SQL file
    git_add(repo_path, "migration.sql");

    // Attest using the full message
    noslop()
        .args([
            "attest",
            "Database migrations must be reviewed by DBA team for performance impact",
            "-m",
            "DBA approved - no index changes needed",
        ])
        .current_dir(repo_path)
        .assert()
        .success();

    // Should pass
    noslop().args(["check", "--ci"]).current_dir(repo_path).assert().success();
}

// =============================================================================
// STAGED ATTESTATIONS TESTS
// =============================================================================

/// Test that attestations are stored in staged-attestations.json
#[test]
fn test_attestation_stored_in_json_file() {
    let temp = TempDir::new().unwrap();
    let repo_path = temp.path();

    // Setup
    init_git_repo(repo_path);

    // Create .noslop directory
    fs::create_dir_all(repo_path.join(".noslop")).unwrap();

    // Attest
    noslop()
        .args(["attest", "test-assertion", "-m", "Test attestation message"])
        .current_dir(repo_path)
        .assert()
        .success();

    // Verify staged-attestations.json exists and contains correct data
    let json_path = repo_path.join(".noslop/staged-attestations.json");
    assert!(json_path.exists(), "staged-attestations.json should exist");

    let content = fs::read_to_string(&json_path).unwrap();
    assert!(content.contains("test-assertion"), "JSON should contain assertion ID");
    assert!(
        content.contains("Test attestation message"),
        "JSON should contain attestation message"
    );
    assert!(content.contains("human"), "JSON should indicate human attestation");
}

/// Test multiple attestations accumulate in JSON file
#[test]
fn test_multiple_attestations_accumulate() {
    let temp = TempDir::new().unwrap();
    let repo_path = temp.path();

    // Setup
    init_git_repo(repo_path);

    // Create .noslop directory
    fs::create_dir_all(repo_path.join(".noslop")).unwrap();

    // Add first attestation
    noslop()
        .args(["attest", "first-assertion", "-m", "First attestation"])
        .current_dir(repo_path)
        .assert()
        .success();

    // Add second attestation
    noslop()
        .args(["attest", "second-assertion", "-m", "Second attestation"])
        .current_dir(repo_path)
        .assert()
        .success();

    // Verify both are in the JSON file
    let json_path = repo_path.join(".noslop/staged-attestations.json");
    let content = fs::read_to_string(&json_path).unwrap();

    assert!(content.contains("first-assertion"), "Should contain first assertion");
    assert!(content.contains("second-assertion"), "Should contain second assertion");
    assert!(content.contains("First attestation"), "Should contain first message");
    assert!(content.contains("Second attestation"), "Should contain second message");
}

// =============================================================================
// ASSERT COMMAND TESTS
// =============================================================================

/// Test assert add with different severity levels
#[test]
fn test_assert_add_with_severity() {
    let temp = TempDir::new().unwrap();
    let repo_path = temp.path();

    // Setup
    init_git_repo(repo_path);
    noslop().arg("init").current_dir(repo_path).assert().success();

    // Add block severity assertion
    noslop()
        .args(["assert", "add", "*.secret", "-m", "Never commit secrets", "-s", "block"])
        .current_dir(repo_path)
        .assert()
        .success();

    // Add warn severity assertion
    noslop()
        .args([
            "assert",
            "add",
            "*.log",
            "-m",
            "Log files should not be committed",
            "-s",
            "warn",
        ])
        .current_dir(repo_path)
        .assert()
        .success();

    // Verify .noslop.toml contains both
    let content = fs::read_to_string(repo_path.join(".noslop.toml")).unwrap();
    assert!(content.contains("Never commit secrets"));
    assert!(content.contains("Log files should not be committed"));
    assert!(content.contains("block"));
    assert!(content.contains("warn"));
}

/// Test assert list filters by target
#[test]
fn test_assert_list_filter_by_target() {
    let temp = TempDir::new().unwrap();
    let repo_path = temp.path();

    // Setup
    init_git_repo(repo_path);

    // Create .noslop.toml with multiple assertions
    fs::write(
        repo_path.join(".noslop.toml"),
        r#"[[assert]]
target = "*.rs"
message = "Rust assertion"
severity = "block"

[[assert]]
target = "*.py"
message = "Python assertion"
severity = "block"

[[assert]]
target = "*.rs"
message = "Another Rust assertion"
severity = "warn"
"#,
    )
    .unwrap();

    // List all assertions
    noslop()
        .args(["assert", "list"])
        .current_dir(repo_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("Rust assertion"))
        .stdout(predicate::str::contains("Python assertion"))
        .stdout(predicate::str::contains("Another Rust assertion"));

    // List filtered by target
    noslop()
        .args(["assert", "list", "-t", "*.rs"])
        .current_dir(repo_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("Rust assertion"))
        .stdout(predicate::str::contains("Another Rust assertion"));
}

// =============================================================================
// ERROR HANDLING TESTS
// =============================================================================

/// Test check in non-git directory fails gracefully
#[test]
fn test_check_in_non_git_directory() {
    let temp = TempDir::new().unwrap();

    // Don't initialize git - just run check
    noslop().arg("check").current_dir(temp.path()).assert().failure();
}

/// Test init fails gracefully in non-git directory
#[test]
fn test_init_requires_git_repo() {
    let temp = TempDir::new().unwrap();

    noslop().arg("init").current_dir(temp.path()).assert().failure();
}

// =============================================================================
// NESTED DIRECTORY TESTS
// =============================================================================

/// Test assertions in nested .noslop.toml files
#[test]
fn test_nested_noslop_toml_files() {
    let temp = TempDir::new().unwrap();
    let repo_path = temp.path();

    // Setup
    init_git_repo(repo_path);

    // Root .noslop.toml
    fs::write(
        repo_path.join(".noslop.toml"),
        r#"[[assert]]
target = "*.rs"
message = "Global Rust check"
severity = "block"
"#,
    )
    .unwrap();

    // Create nested directory with its own .noslop.toml
    let api_dir = repo_path.join("api");
    fs::create_dir_all(&api_dir).unwrap();
    fs::write(
        api_dir.join(".noslop.toml"),
        r#"[[assert]]
target = "*.rs"
message = "API-specific security check"
severity = "block"
"#,
    )
    .unwrap();

    // Initial commit
    git_add(repo_path, ".noslop.toml");
    git_add(repo_path, "api/.noslop.toml");
    git_commit(repo_path, "Initial commit");

    // Create API Rust file
    fs::write(api_dir.join("handler.rs"), "fn handle() {}").unwrap();

    // Stage the file
    git_add(repo_path, "api/handler.rs");

    // Check should require both assertions to be attested
    let output = noslop().args(["check", "--ci"]).current_dir(repo_path).assert().failure();

    // The check should mention both assertions
    output.stdout(
        predicate::str::contains("Global Rust check").or(predicate::str::contains("API-specific")),
    );
}

// =============================================================================
// CI MODE TESTS
// =============================================================================

/// Test --ci flag returns proper exit code
#[test]
fn test_ci_mode_exit_codes() {
    let temp = TempDir::new().unwrap();
    let repo_path = temp.path();

    // Setup
    init_git_repo(repo_path);

    // Create blocking assertion
    fs::write(
        repo_path.join(".noslop.toml"),
        r#"[[assert]]
target = "*.go"
message = "Go code review required"
severity = "block"
"#,
    )
    .unwrap();

    git_add(repo_path, ".noslop.toml");
    git_commit(repo_path, "Initial commit");

    // Create Go file
    fs::write(repo_path.join("main.go"), "package main").unwrap();
    git_add(repo_path, "main.go");

    // Without --ci, check might exit differently
    // With --ci, should return error result (for CI systems)
    noslop()
        .args(["check", "--ci"])
        .current_dir(repo_path)
        .assert()
        .failure()
        .stdout(predicate::str::contains("BLOCKED"));
}

/// Test check succeeds in CI when no assertions apply
#[test]
fn test_ci_no_applicable_assertions() {
    let temp = TempDir::new().unwrap();
    let repo_path = temp.path();

    // Setup
    init_git_repo(repo_path);

    // Create assertion for .rs files only
    fs::write(
        repo_path.join(".noslop.toml"),
        r#"[[assert]]
target = "*.rs"
message = "Rust check"
severity = "block"
"#,
    )
    .unwrap();

    git_add(repo_path, ".noslop.toml");
    git_commit(repo_path, "Initial commit");

    // Create and stage a Python file (doesn't match assertion)
    fs::write(repo_path.join("script.py"), "print('hello')").unwrap();
    git_add(repo_path, "script.py");

    // Check should pass - no assertions apply to .py files
    noslop()
        .args(["check", "--ci"])
        .current_dir(repo_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("No assertions apply"));
}
