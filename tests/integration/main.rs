//! Integration tests for noslop CLI
//!
//! These tests simulate real-world workflows with a complete codebase,
//! testing the full cycle of: init, check add, check, review.

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

/// Test that blocking checks report BLOCKED status for staged files
#[test]
fn test_check_blocks_staged_files() {
    let temp = TempDir::new().unwrap();
    let repo_path = temp.path();

    // Setup
    init_git_repo(repo_path);

    // Create .noslop.toml with a blocking check
    fs::write(
        repo_path.join(".noslop.toml"),
        r#"[[check]]
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

    // Check should fail - blocking check applies
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

    // Create .noslop.toml with a warn check
    fs::write(
        repo_path.join(".noslop.toml"),
        r#"[[check]]
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
        .stdout(predicate::str::contains("Commit may proceed"));
}

// =============================================================================
// MULTIPLE CHECKS TESTS
// =============================================================================

/// Test handling multiple checks for different file types
#[test]
fn test_multiple_checks_different_targets() {
    let temp = TempDir::new().unwrap();
    let repo_path = temp.path();

    // Setup
    init_git_repo(repo_path);

    // Create .noslop.toml with multiple checks
    fs::write(
        repo_path.join(".noslop.toml"),
        r#"[[check]]
target = "*.rs"
message = "Rust code must be formatted with rustfmt"
severity = "block"

[[check]]
target = "*.ts"
message = "TypeScript must use strict mode"
severity = "block"

[[check]]
target = "*.md"
message = "Documentation should be reviewed"
severity = "warn"
"#,
    )
    .unwrap();

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

    // Check should fail - blocking checks for Rust and TypeScript
    noslop()
        .args(["check", "--ci"])
        .current_dir(repo_path)
        .assert()
        .failure()
        .stdout(predicate::str::contains("BLOCKED"))
        .stdout(predicate::str::contains("Rust code must be formatted with rustfmt"))
        .stdout(predicate::str::contains("TypeScript must use strict mode"));
}

/// Test that same check applies to multiple matching files
#[test]
fn test_single_check_multiple_files() {
    let temp = TempDir::new().unwrap();
    let repo_path = temp.path();

    // Setup
    init_git_repo(repo_path);

    // Create .noslop.toml with glob pattern
    fs::write(
        repo_path.join(".noslop.toml"),
        r#"[[check]]
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

    // Check should fail - blocking check matches all files
    noslop()
        .args(["check", "--ci"])
        .current_dir(repo_path)
        .assert()
        .failure()
        .stdout(predicate::str::contains("BLOCKED"));
}

// =============================================================================
// CHECK COMMAND TESTS
// =============================================================================

/// Test check add with different severity levels
#[test]
fn test_check_add_with_severity() {
    let temp = TempDir::new().unwrap();
    let repo_path = temp.path();

    // Setup
    init_git_repo(repo_path);
    noslop().arg("init").current_dir(repo_path).assert().success();

    // Add block severity check
    noslop()
        .args(["check", "add", "*.secret", "-m", "Never commit secrets", "-s", "block"])
        .current_dir(repo_path)
        .assert()
        .success();

    // Add warn severity check
    noslop()
        .args(["check", "add", "*.log", "-m", "Log files should not be committed", "-s", "warn"])
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

/// Test check list filters by target
#[test]
fn test_check_list_filter_by_target() {
    let temp = TempDir::new().unwrap();
    let repo_path = temp.path();

    // Setup
    init_git_repo(repo_path);

    // Create .noslop.toml with multiple checks
    fs::write(
        repo_path.join(".noslop.toml"),
        r#"[[check]]
target = "*.rs"
message = "Rust check"
severity = "block"

[[check]]
target = "*.py"
message = "Python check"
severity = "block"

[[check]]
target = "*.rs"
message = "Another Rust check"
severity = "warn"
"#,
    )
    .unwrap();

    // List all checks
    noslop()
        .args(["check", "list"])
        .current_dir(repo_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("Rust check"))
        .stdout(predicate::str::contains("Python check"))
        .stdout(predicate::str::contains("Another Rust check"));

    // List filtered by target
    noslop()
        .args(["check", "list", "-t", "*.rs"])
        .current_dir(repo_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("Rust check"))
        .stdout(predicate::str::contains("Another Rust check"));
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

/// Test checks in nested .noslop.toml files
#[test]
fn test_nested_noslop_toml_files() {
    let temp = TempDir::new().unwrap();
    let repo_path = temp.path();

    // Setup
    init_git_repo(repo_path);

    // Root .noslop.toml
    fs::write(
        repo_path.join(".noslop.toml"),
        r#"[[check]]
target = "**/*.rs"
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
        r#"[[check]]
target = "api/*.rs"
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

    // Check should report blocking checks
    let output = noslop().args(["check", "--ci"]).current_dir(repo_path).assert().failure();

    // The check should mention at least one of the checks
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

    // Create blocking check
    fs::write(
        repo_path.join(".noslop.toml"),
        r#"[[check]]
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

    // With --ci, should return error result (for CI systems)
    noslop()
        .args(["check", "--ci"])
        .current_dir(repo_path)
        .assert()
        .failure()
        .stdout(predicate::str::contains("BLOCKED"));
}

/// Test check succeeds in CI when no checks apply
#[test]
fn test_ci_no_applicable_checks() {
    let temp = TempDir::new().unwrap();
    let repo_path = temp.path();

    // Setup
    init_git_repo(repo_path);

    // Create check for .rs files only
    fs::write(
        repo_path.join(".noslop.toml"),
        r#"[[check]]
target = "*.rs"
message = "Rust check"
severity = "block"
"#,
    )
    .unwrap();

    git_add(repo_path, ".noslop.toml");
    git_commit(repo_path, "Initial commit");

    // Create and stage a Python file (doesn't match check)
    fs::write(repo_path.join("script.py"), "print('hello')").unwrap();
    git_add(repo_path, "script.py");

    // Check should pass - no checks apply to .py files
    noslop()
        .args(["check", "--ci"])
        .current_dir(repo_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("No checks apply"));
}

// =============================================================================
// REVIEW INTEGRATION TESTS
// =============================================================================

/// Test starting and listing reviews
#[test]
fn test_review_start_and_list() {
    let temp = TempDir::new().unwrap();
    let repo_path = temp.path();

    // Setup
    init_git_repo(repo_path);

    // Create initial file and commit
    fs::write(repo_path.join("main.rs"), "fn main() {}").unwrap();
    git_add(repo_path, "main.rs");
    git_commit(repo_path, "Initial commit");

    // Get commit SHA
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to get HEAD");
    let head_sha = String::from_utf8_lossy(&output.stdout).trim().to_string();

    // Start a review
    noslop()
        .args(["review", "start", &head_sha, &head_sha])
        .current_dir(repo_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("Started review"));

    // List reviews should show the review
    noslop()
        .args(["review", "list"])
        .current_dir(repo_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("OPEN"));
}

/// Test closing a review without blocking findings
#[test]
fn test_review_close() {
    let temp = TempDir::new().unwrap();
    let repo_path = temp.path();

    // Setup
    init_git_repo(repo_path);

    // Create initial file and commit
    fs::write(repo_path.join("app.rs"), "fn app() {}").unwrap();
    git_add(repo_path, "app.rs");
    git_commit(repo_path, "Initial commit");

    // Get commit SHA
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to get HEAD");
    let head_sha = String::from_utf8_lossy(&output.stdout).trim().to_string();

    // Start a review
    let review_output = noslop()
        .args(["review", "start", &head_sha, &head_sha])
        .current_dir(repo_path)
        .assert()
        .success();

    // Extract review ID from output
    let stdout = String::from_utf8_lossy(&review_output.get_output().stdout);
    let review_id = stdout
        .lines()
        .find(|l| l.contains("REV-"))
        .and_then(|l| l.split_whitespace().find(|w| w.starts_with("REV-")))
        .expect("Should find review ID in output");

    // Close the review (no findings, so it should succeed)
    noslop()
        .args(["review", "close", review_id])
        .current_dir(repo_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("Closed review"));
}
