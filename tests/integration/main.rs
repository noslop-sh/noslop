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

// =============================================================================
// CHECKPOINT INTEGRATION TESTS
// =============================================================================

/// Test checkpoint with uncommitted changes creates a commit
#[test]
fn test_checkpoint_with_changes() {
    let temp = TempDir::new().unwrap();
    let repo_path = temp.path();

    // Setup
    init_git_repo(repo_path);

    // Initial commit so HEAD exists
    fs::write(repo_path.join("initial.txt"), "initial").unwrap();
    git_add(repo_path, "initial.txt");
    git_commit(repo_path, "Initial commit");

    // Create a file to checkpoint
    fs::write(repo_path.join("new_file.txt"), "some content").unwrap();

    // Run checkpoint
    noslop()
        .args(["checkpoint", "test checkpoint"])
        .current_dir(repo_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("Checkpoint:"))
        .stdout(predicate::str::contains("test checkpoint"));
}

/// Test checkpoint on a clean tree reports nothing to do
#[test]
fn test_checkpoint_clean_tree() {
    let temp = TempDir::new().unwrap();
    let repo_path = temp.path();

    // Setup
    init_git_repo(repo_path);

    // Initial commit
    fs::write(repo_path.join("initial.txt"), "initial").unwrap();
    git_add(repo_path, "initial.txt");
    git_commit(repo_path, "Initial commit");

    // No changes -- checkpoint should report clean
    noslop()
        .args(["checkpoint"])
        .current_dir(repo_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("Nothing to checkpoint"));
}

// =============================================================================
// REVIEW RUN INTEGRATION TESTS
// =============================================================================

/// Test review run on a commit range with matching checks produces findings
#[test]
fn test_review_run_with_checks() {
    let temp = TempDir::new().unwrap();
    let repo_path = temp.path();

    // Setup
    init_git_repo(repo_path);

    // Create .noslop.toml with a check and .noslop directory
    fs::write(
        repo_path.join(".noslop.toml"),
        r#"[project]
prefix = "NOS"

[[check]]
id = "NOS-1"
target = "*.rs"
message = "Rust files need security review"
severity = "block"
"#,
    )
    .unwrap();
    fs::create_dir_all(repo_path.join(".noslop")).unwrap();

    // Initial commit
    git_add(repo_path, ".noslop.toml");
    git_commit(repo_path, "Initial commit");

    // Get base SHA
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to get HEAD");
    let base_sha = String::from_utf8_lossy(&output.stdout).trim().to_string();

    // Create a Rust file and commit
    fs::write(repo_path.join("main.rs"), "fn main() {}").unwrap();
    git_add(repo_path, "main.rs");
    git_commit(repo_path, "Add main.rs");

    // Run review
    noslop()
        .args(["review", "run", "--base", &base_sha])
        .current_dir(repo_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("Review: REV-"))
        .stdout(predicate::str::contains("Findings: 1"))
        .stdout(predicate::str::contains("Blocking: 1"));
}

/// Test review run with no matching checks produces no findings
#[test]
fn test_review_run_no_findings() {
    let temp = TempDir::new().unwrap();
    let repo_path = temp.path();

    // Setup
    init_git_repo(repo_path);

    // Create .noslop.toml with a check for *.py only
    fs::write(
        repo_path.join(".noslop.toml"),
        r#"[project]
prefix = "NOS"

[[check]]
id = "NOS-1"
target = "*.py"
message = "Python needs review"
severity = "block"
"#,
    )
    .unwrap();
    fs::create_dir_all(repo_path.join(".noslop")).unwrap();

    // Initial commit
    git_add(repo_path, ".noslop.toml");
    git_commit(repo_path, "Initial commit");

    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to get HEAD");
    let base_sha = String::from_utf8_lossy(&output.stdout).trim().to_string();

    // Create a Rust file (not Python) and commit
    fs::write(repo_path.join("main.rs"), "fn main() {}").unwrap();
    git_add(repo_path, "main.rs");
    git_commit(repo_path, "Add main.rs");

    // Run review -- no check matches .rs files
    noslop()
        .args(["review", "run", "--base", &base_sha])
        .current_dir(repo_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("Findings: 0"))
        .stdout(predicate::str::contains("No blocking findings"));
}

/// Test review run --check exits 1 when blocking findings exist
#[test]
fn test_review_run_check_flag_blocks() {
    let temp = TempDir::new().unwrap();
    let repo_path = temp.path();

    // Setup
    init_git_repo(repo_path);

    fs::write(
        repo_path.join(".noslop.toml"),
        r#"[project]
prefix = "NOS"

[[check]]
id = "NOS-1"
target = "*.rs"
message = "Rust files need review"
severity = "block"
"#,
    )
    .unwrap();
    fs::create_dir_all(repo_path.join(".noslop")).unwrap();

    git_add(repo_path, ".noslop.toml");
    git_commit(repo_path, "Initial commit");

    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to get HEAD");
    let base_sha = String::from_utf8_lossy(&output.stdout).trim().to_string();

    fs::write(repo_path.join("main.rs"), "fn main() {}").unwrap();
    git_add(repo_path, "main.rs");
    git_commit(repo_path, "Add main.rs");

    // With --check, should exit 1 because blocking findings exist
    noslop()
        .args(["review", "run", "--base", &base_sha, "--check"])
        .current_dir(repo_path)
        .assert()
        .failure()
        .stdout(predicate::str::contains("BLOCKED"));
}

// =============================================================================
// FINDINGS INTEGRATION TESTS
// =============================================================================

/// Test findings list, resolve, and dismiss workflow
#[test]
fn test_findings_workflow() {
    let temp = TempDir::new().unwrap();
    let repo_path = temp.path();

    // Setup
    init_git_repo(repo_path);

    fs::write(
        repo_path.join(".noslop.toml"),
        r#"[project]
prefix = "NOS"

[[check]]
id = "NOS-1"
target = "*.rs"
message = "Rust files need review"
severity = "block"
"#,
    )
    .unwrap();
    fs::create_dir_all(repo_path.join(".noslop")).unwrap();

    git_add(repo_path, ".noslop.toml");
    git_commit(repo_path, "Initial commit");

    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to get HEAD");
    let base_sha = String::from_utf8_lossy(&output.stdout).trim().to_string();

    fs::write(repo_path.join("main.rs"), "fn main() {}").unwrap();
    git_add(repo_path, "main.rs");
    git_commit(repo_path, "Add main.rs");

    // Run review to create findings
    let review_output = noslop()
        .args(["review", "run", "--base", &base_sha])
        .current_dir(repo_path)
        .assert()
        .success();

    // Extract review ID
    let stdout = String::from_utf8_lossy(&review_output.get_output().stdout);
    let review_id = stdout
        .lines()
        .find(|l| l.contains("REV-"))
        .and_then(|l| l.split_whitespace().find(|w| w.starts_with("REV-")))
        .expect("Should find review ID in output");

    // List findings
    let findings_output = noslop()
        .args(["findings", "list", review_id])
        .current_dir(repo_path)
        .assert()
        .success();

    let findings_stdout = String::from_utf8_lossy(&findings_output.get_output().stdout);
    assert!(findings_stdout.contains("Rust files need review"));

    // Extract finding ID
    let finding_id = findings_stdout
        .lines()
        .find(|l| l.contains("F-"))
        .and_then(|l| l.split_whitespace().find(|w| w.starts_with("F-")))
        .expect("Should find finding ID in output");

    // Resolve the finding
    noslop()
        .args(["findings", "resolve", review_id, finding_id])
        .current_dir(repo_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("Resolved finding"));

    // Now the review should be closable
    noslop()
        .args(["review", "close", review_id])
        .current_dir(repo_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("Closed review"));
}

/// Test findings dismiss with reason
#[test]
fn test_findings_dismiss() {
    let temp = TempDir::new().unwrap();
    let repo_path = temp.path();

    // Setup
    init_git_repo(repo_path);

    fs::write(
        repo_path.join(".noslop.toml"),
        r#"[project]
prefix = "NOS"

[[check]]
id = "NOS-1"
target = "*.rs"
message = "Rust review"
severity = "warn"
"#,
    )
    .unwrap();
    fs::create_dir_all(repo_path.join(".noslop")).unwrap();

    git_add(repo_path, ".noslop.toml");
    git_commit(repo_path, "Initial commit");

    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to get HEAD");
    let base_sha = String::from_utf8_lossy(&output.stdout).trim().to_string();

    fs::write(repo_path.join("lib.rs"), "pub fn hello() {}").unwrap();
    git_add(repo_path, "lib.rs");
    git_commit(repo_path, "Add lib.rs");

    // Run review
    let review_output = noslop()
        .args(["review", "run", "--base", &base_sha, "--json"])
        .current_dir(repo_path)
        .assert()
        .success();

    let json_stdout = String::from_utf8_lossy(&review_output.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&json_stdout).unwrap();
    let review_id = json["review_id"].as_str().unwrap();

    // List findings in JSON mode
    let findings_output = noslop()
        .args(["findings", "list", review_id, "--json"])
        .current_dir(repo_path)
        .assert()
        .success();

    let findings_json_str = String::from_utf8_lossy(&findings_output.get_output().stdout);
    let findings_json: serde_json::Value = serde_json::from_str(&findings_json_str).unwrap();
    let finding_id = findings_json[0]["id"].as_str().unwrap();

    // Dismiss the finding
    noslop()
        .args(["findings", "dismiss", review_id, finding_id, "--reason", "false_positive"])
        .current_dir(repo_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("Dismissed finding"));
}

// =============================================================================
// FULL LIFECYCLE INTEGRATION TESTS
// =============================================================================

/// Full workflow: init -> add check -> commit -> review run -> findings resolve -> review close
#[test]
fn test_full_lifecycle_init_to_close() {
    let temp = TempDir::new().unwrap();
    let repo_path = temp.path();

    // Phase 1: Init repo and noslop
    init_git_repo(repo_path);
    noslop().arg("init").current_dir(repo_path).assert().success();

    // Phase 2: Add a blocking check
    noslop()
        .args(["check", "add", "*.rs", "-m", "Rust files need security review", "-s", "block"])
        .current_dir(repo_path)
        .assert()
        .success();

    // Phase 3: Create initial commit with noslop config
    // Use --no-verify to bypass the hooks that noslop init installed
    git_add(repo_path, ".noslop.toml");
    git_add(repo_path, ".noslop");
    Command::new("git")
        .args(["commit", "--no-verify", "-m", "Initial commit with noslop config"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to create commit");

    // Get base SHA
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to get HEAD");
    let base_sha = String::from_utf8_lossy(&output.stdout).trim().to_string();

    // Phase 4: Create some code and commit (bypass hooks since they'd block)
    fs::write(repo_path.join("main.rs"), "fn main() { println!(\"hello\"); }").unwrap();
    git_add(repo_path, "main.rs");
    Command::new("git")
        .args(["commit", "--no-verify", "-m", "Add main.rs"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to create commit");

    // Phase 5: Run review (JSON) to produce findings
    let review_output = noslop()
        .args(["review", "run", "--base", &base_sha, "--json"])
        .current_dir(repo_path)
        .assert()
        .success();

    let json_stdout = String::from_utf8_lossy(&review_output.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&json_stdout).unwrap();
    let review_id = json["review_id"].as_str().unwrap();
    assert!(json["blocked"].as_bool().unwrap());
    assert_eq!(json["findings"].as_u64().unwrap(), 1);

    // Phase 6: review --check should fail because there are blocking findings
    noslop()
        .args(["review", "run", "--base", &base_sha, "--check"])
        .current_dir(repo_path)
        .assert()
        .failure();

    // Phase 7: Resolve the finding
    let findings_output = noslop()
        .args(["findings", "list", review_id, "--json"])
        .current_dir(repo_path)
        .assert()
        .success();

    let findings_json_str = String::from_utf8_lossy(&findings_output.get_output().stdout);
    let findings_json: serde_json::Value = serde_json::from_str(&findings_json_str).unwrap();
    let finding_id = findings_json[0]["id"].as_str().unwrap();

    noslop()
        .args(["findings", "resolve", review_id, finding_id])
        .current_dir(repo_path)
        .assert()
        .success();

    // Phase 8: Close review (now succeeds since blocking findings are resolved)
    noslop()
        .args(["review", "close", review_id])
        .current_dir(repo_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("Closed review"));
}

// =============================================================================
// ERROR HANDLING TESTS (REVIEW / FINDINGS)
// =============================================================================

/// Closing a review with blocking findings should fail
#[test]
fn test_review_close_blocked_by_findings() {
    let temp = TempDir::new().unwrap();
    let repo_path = temp.path();

    init_git_repo(repo_path);

    fs::write(
        repo_path.join(".noslop.toml"),
        r#"[project]
prefix = "NOS"

[[check]]
id = "NOS-1"
target = "*.rs"
message = "Rust review"
severity = "block"
"#,
    )
    .unwrap();
    fs::create_dir_all(repo_path.join(".noslop")).unwrap();

    git_add(repo_path, ".noslop.toml");
    git_commit(repo_path, "Initial commit");

    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repo_path)
        .output()
        .unwrap();
    let base_sha = String::from_utf8_lossy(&output.stdout).trim().to_string();

    fs::write(repo_path.join("main.rs"), "fn main() {}").unwrap();
    git_add(repo_path, "main.rs");
    git_commit(repo_path, "Add code");

    // Run review to create findings
    let review_output = noslop()
        .args(["review", "run", "--base", &base_sha, "--json"])
        .current_dir(repo_path)
        .assert()
        .success();

    let json_stdout = String::from_utf8_lossy(&review_output.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&json_stdout).unwrap();
    let review_id = json["review_id"].as_str().unwrap();

    // Try to close -- should fail because there are blocking findings
    noslop()
        .args(["review", "close", review_id])
        .current_dir(repo_path)
        .assert()
        .failure()
        .stderr(predicate::str::contains("blocking finding"));
}

/// Findings list on a nonexistent review should fail
#[test]
fn test_findings_list_nonexistent_review() {
    let temp = TempDir::new().unwrap();
    let repo_path = temp.path();

    init_git_repo(repo_path);
    fs::create_dir_all(repo_path.join(".noslop")).unwrap();

    noslop()
        .args(["findings", "list", "REV-nonexistent"])
        .current_dir(repo_path)
        .assert()
        .failure()
        .stderr(predicate::str::contains("Review not found"));
}

/// Resolve finding on a nonexistent review should fail
#[test]
fn test_findings_resolve_nonexistent_review() {
    let temp = TempDir::new().unwrap();
    let repo_path = temp.path();

    init_git_repo(repo_path);
    fs::create_dir_all(repo_path.join(".noslop")).unwrap();

    noslop()
        .args(["findings", "resolve", "REV-nonexistent", "F-nonexistent"])
        .current_dir(repo_path)
        .assert()
        .failure()
        .stderr(predicate::str::contains("Review not found"));
}

/// Dismiss finding on a nonexistent review should fail
#[test]
fn test_findings_dismiss_nonexistent_review() {
    let temp = TempDir::new().unwrap();
    let repo_path = temp.path();

    init_git_repo(repo_path);
    fs::create_dir_all(repo_path.join(".noslop")).unwrap();

    noslop()
        .args(["findings", "dismiss", "REV-nonexistent", "F-nonexistent"])
        .current_dir(repo_path)
        .assert()
        .failure()
        .stderr(predicate::str::contains("Review not found"));
}

/// Review show on a nonexistent review should fail
#[test]
fn test_review_show_nonexistent() {
    let temp = TempDir::new().unwrap();
    let repo_path = temp.path();

    init_git_repo(repo_path);
    fs::create_dir_all(repo_path.join(".noslop")).unwrap();

    noslop()
        .args(["review", "show", "REV-nonexistent"])
        .current_dir(repo_path)
        .assert()
        .failure()
        .stderr(predicate::str::contains("Review not found"));
}

/// Review close on a nonexistent review should fail
#[test]
fn test_review_close_nonexistent() {
    let temp = TempDir::new().unwrap();
    let repo_path = temp.path();

    init_git_repo(repo_path);
    fs::create_dir_all(repo_path.join(".noslop")).unwrap();

    noslop()
        .args(["review", "close", "REV-nonexistent"])
        .current_dir(repo_path)
        .assert()
        .failure()
        .stderr(predicate::str::contains("Review not found"));
}

// =============================================================================
// JSON OUTPUT MODE TESTS
// =============================================================================

/// Review run in JSON mode produces valid parseable JSON
#[test]
fn test_review_run_json_output() {
    let temp = TempDir::new().unwrap();
    let repo_path = temp.path();

    init_git_repo(repo_path);

    fs::write(
        repo_path.join(".noslop.toml"),
        r#"[project]
prefix = "NOS"

[[check]]
id = "NOS-1"
target = "*.rs"
message = "Review Rust"
severity = "block"
"#,
    )
    .unwrap();
    fs::create_dir_all(repo_path.join(".noslop")).unwrap();

    git_add(repo_path, ".noslop.toml");
    git_commit(repo_path, "Initial commit");

    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repo_path)
        .output()
        .unwrap();
    let base_sha = String::from_utf8_lossy(&output.stdout).trim().to_string();

    fs::write(repo_path.join("lib.rs"), "pub fn lib() {}").unwrap();
    git_add(repo_path, "lib.rs");
    git_commit(repo_path, "Add lib.rs");

    let review_output = noslop()
        .args(["review", "run", "--base", &base_sha, "--json"])
        .current_dir(repo_path)
        .assert()
        .success();

    let json_stdout = String::from_utf8_lossy(&review_output.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&json_stdout).unwrap();

    // Verify JSON structure
    assert!(json["success"].as_bool().unwrap());
    assert!(json["review_id"].as_str().unwrap().starts_with("REV-"));
    assert!(json["findings"].is_number());
    assert!(json["blocking"].is_number());
    assert!(json["blocked"].is_boolean());
    assert!(json["base"].is_string());
    assert!(json["head"].is_string());
}

/// Review list in JSON mode produces valid parseable JSON
#[test]
fn test_review_list_json_output() {
    let temp = TempDir::new().unwrap();
    let repo_path = temp.path();

    init_git_repo(repo_path);

    fs::write(repo_path.join("main.rs"), "fn main() {}").unwrap();
    git_add(repo_path, "main.rs");
    git_commit(repo_path, "Initial commit");

    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repo_path)
        .output()
        .unwrap();
    let head_sha = String::from_utf8_lossy(&output.stdout).trim().to_string();

    // Start a review
    noslop()
        .args(["review", "start", &head_sha, &head_sha])
        .current_dir(repo_path)
        .assert()
        .success();

    // List in JSON mode
    let list_output = noslop()
        .args(["--json", "review", "list"])
        .current_dir(repo_path)
        .assert()
        .success();

    let json_stdout = String::from_utf8_lossy(&list_output.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&json_stdout).unwrap();

    // Should be an array with at least one review
    assert!(json.is_array());
    assert!(!json.as_array().unwrap().is_empty());
    assert!(json[0]["id"].as_str().unwrap().starts_with("REV-"));
    assert_eq!(json[0]["status"].as_str().unwrap(), "open");
}

/// Review show in JSON mode produces valid parseable JSON
#[test]
fn test_review_show_json_output() {
    let temp = TempDir::new().unwrap();
    let repo_path = temp.path();

    init_git_repo(repo_path);

    fs::write(repo_path.join("main.rs"), "fn main() {}").unwrap();
    git_add(repo_path, "main.rs");
    git_commit(repo_path, "Initial commit");

    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repo_path)
        .output()
        .unwrap();
    let head_sha = String::from_utf8_lossy(&output.stdout).trim().to_string();

    // Start a review (JSON mode to get the ID)
    let start_output = noslop()
        .args(["--json", "review", "start", &head_sha, &head_sha])
        .current_dir(repo_path)
        .assert()
        .success();

    let start_json_str = String::from_utf8_lossy(&start_output.get_output().stdout);
    let start_json: serde_json::Value = serde_json::from_str(&start_json_str).unwrap();
    let review_id = start_json["review_id"].as_str().unwrap();

    // Show in JSON mode
    let show_output = noslop()
        .args(["--json", "review", "show", review_id])
        .current_dir(repo_path)
        .assert()
        .success();

    let show_json_str = String::from_utf8_lossy(&show_output.get_output().stdout);
    let show_json: serde_json::Value = serde_json::from_str(&show_json_str).unwrap();

    assert_eq!(show_json["id"].as_str().unwrap(), review_id);
    assert_eq!(show_json["status"].as_str().unwrap(), "open");
    assert!(show_json["findings"].is_array());
}

/// Findings list in JSON mode produces valid parseable JSON
#[test]
fn test_findings_list_json_output() {
    let temp = TempDir::new().unwrap();
    let repo_path = temp.path();

    init_git_repo(repo_path);

    fs::write(
        repo_path.join(".noslop.toml"),
        r#"[project]
prefix = "NOS"

[[check]]
id = "NOS-1"
target = "*.rs"
message = "Rust review"
severity = "warn"
"#,
    )
    .unwrap();
    fs::create_dir_all(repo_path.join(".noslop")).unwrap();

    git_add(repo_path, ".noslop.toml");
    git_commit(repo_path, "Initial commit");

    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repo_path)
        .output()
        .unwrap();
    let base_sha = String::from_utf8_lossy(&output.stdout).trim().to_string();

    fs::write(repo_path.join("lib.rs"), "pub fn lib() {}").unwrap();
    git_add(repo_path, "lib.rs");
    git_commit(repo_path, "Add lib.rs");

    // Run review
    let review_output = noslop()
        .args(["review", "run", "--base", &base_sha, "--json"])
        .current_dir(repo_path)
        .assert()
        .success();

    let json_stdout = String::from_utf8_lossy(&review_output.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&json_stdout).unwrap();
    let review_id = json["review_id"].as_str().unwrap();

    // List findings in JSON
    let findings_output = noslop()
        .args(["findings", "list", review_id, "--json"])
        .current_dir(repo_path)
        .assert()
        .success();

    let findings_json_str = String::from_utf8_lossy(&findings_output.get_output().stdout);
    let findings_json: serde_json::Value = serde_json::from_str(&findings_json_str).unwrap();

    // Should be an array with findings
    assert!(findings_json.is_array());
    assert!(!findings_json.as_array().unwrap().is_empty());
    // Each finding should have id, severity, message
    let first = &findings_json[0];
    assert!(first["id"].as_str().unwrap().starts_with("F-"));
    assert!(first["severity"].is_string());
    assert!(first["message"].is_string());
}

// =============================================================================
// CHECKPOINT JSON MODE TESTS
// =============================================================================

/// Checkpoint with custom message in JSON mode
#[test]
fn test_checkpoint_custom_message_json() {
    let temp = TempDir::new().unwrap();
    let repo_path = temp.path();

    init_git_repo(repo_path);

    fs::write(repo_path.join("initial.txt"), "initial").unwrap();
    git_add(repo_path, "initial.txt");
    git_commit(repo_path, "Initial commit");

    // Create a change
    fs::write(repo_path.join("changed.txt"), "content").unwrap();

    // Run checkpoint with custom message in JSON mode
    let cp_output = noslop()
        .args(["--json", "checkpoint", "before refactoring"])
        .current_dir(repo_path)
        .assert()
        .success();

    let json_stdout = String::from_utf8_lossy(&cp_output.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&json_stdout).unwrap();

    assert!(json["success"].as_bool().unwrap());
    assert!(json["sha"].is_string());
    assert_eq!(json["message"].as_str().unwrap(), "before refactoring");
}

/// Checkpoint on clean tree in JSON mode
#[test]
fn test_checkpoint_clean_tree_json() {
    let temp = TempDir::new().unwrap();
    let repo_path = temp.path();

    init_git_repo(repo_path);

    fs::write(repo_path.join("initial.txt"), "initial").unwrap();
    git_add(repo_path, "initial.txt");
    git_commit(repo_path, "Initial commit");

    // No changes -- checkpoint in JSON mode
    let cp_output = noslop()
        .args(["--json", "checkpoint"])
        .current_dir(repo_path)
        .assert()
        .success();

    let json_stdout = String::from_utf8_lossy(&cp_output.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&json_stdout).unwrap();

    assert!(json["success"].as_bool().unwrap());
    assert!(json["sha"].is_null());
}

// =============================================================================
// MULTIPLE REVIEWS / SESSION PERSISTENCE TESTS
// =============================================================================

/// Multiple reviews create separate session files
#[test]
fn test_multiple_reviews_separate_sessions() {
    let temp = TempDir::new().unwrap();
    let repo_path = temp.path();

    init_git_repo(repo_path);

    fs::write(repo_path.join("main.rs"), "fn main() {}").unwrap();
    git_add(repo_path, "main.rs");
    git_commit(repo_path, "Initial commit");

    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repo_path)
        .output()
        .unwrap();
    let head_sha = String::from_utf8_lossy(&output.stdout).trim().to_string();

    // Start two reviews with JSON to get IDs
    let r1_output = noslop()
        .args(["--json", "review", "start", &head_sha, &head_sha])
        .current_dir(repo_path)
        .assert()
        .success();
    let r1_json: serde_json::Value =
        serde_json::from_str(&String::from_utf8_lossy(&r1_output.get_output().stdout)).unwrap();
    let id1 = r1_json["review_id"].as_str().unwrap().to_string();

    let r2_output = noslop()
        .args(["--json", "review", "start", &head_sha, &head_sha])
        .current_dir(repo_path)
        .assert()
        .success();
    let r2_json: serde_json::Value =
        serde_json::from_str(&String::from_utf8_lossy(&r2_output.get_output().stdout)).unwrap();
    let id2 = r2_json["review_id"].as_str().unwrap().to_string();

    // IDs should be different
    assert_ne!(id1, id2);

    // List should show both
    let list_output = noslop()
        .args(["--json", "review", "list"])
        .current_dir(repo_path)
        .assert()
        .success();
    let list_json: serde_json::Value =
        serde_json::from_str(&String::from_utf8_lossy(&list_output.get_output().stdout)).unwrap();

    assert!(list_json.as_array().unwrap().len() >= 2);

    // Both IDs should appear
    let ids: Vec<&str> = list_json
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["id"].as_str().unwrap())
        .collect();
    assert!(ids.contains(&id1.as_str()));
    assert!(ids.contains(&id2.as_str()));
}

/// Review list --open filters closed reviews
#[test]
fn test_review_list_open_filter() {
    let temp = TempDir::new().unwrap();
    let repo_path = temp.path();

    init_git_repo(repo_path);

    fs::write(repo_path.join("main.rs"), "fn main() {}").unwrap();
    git_add(repo_path, "main.rs");
    git_commit(repo_path, "Initial commit");

    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repo_path)
        .output()
        .unwrap();
    let head_sha = String::from_utf8_lossy(&output.stdout).trim().to_string();

    // Start two reviews
    let r1_output = noslop()
        .args(["--json", "review", "start", &head_sha, &head_sha])
        .current_dir(repo_path)
        .assert()
        .success();
    let r1_json: serde_json::Value =
        serde_json::from_str(&String::from_utf8_lossy(&r1_output.get_output().stdout)).unwrap();
    let id1 = r1_json["review_id"].as_str().unwrap().to_string();

    noslop()
        .args(["review", "start", &head_sha, &head_sha])
        .current_dir(repo_path)
        .assert()
        .success();

    // Close the first review
    noslop()
        .args(["review", "close", &id1])
        .current_dir(repo_path)
        .assert()
        .success();

    // List --open should show only 1
    let list_output = noslop()
        .args(["--json", "review", "list", "--open"])
        .current_dir(repo_path)
        .assert()
        .success();
    let list_json: serde_json::Value =
        serde_json::from_str(&String::from_utf8_lossy(&list_output.get_output().stdout)).unwrap();

    let open_reviews = list_json.as_array().unwrap();
    assert_eq!(open_reviews.len(), 1);
    assert_ne!(open_reviews[0]["id"].as_str().unwrap(), id1);
}

/// Review run with no changes between base and head
#[test]
fn test_review_run_no_changes() {
    let temp = TempDir::new().unwrap();
    let repo_path = temp.path();

    init_git_repo(repo_path);

    fs::write(
        repo_path.join(".noslop.toml"),
        r#"[project]
prefix = "NOS"

[[check]]
id = "NOS-1"
target = "*.rs"
message = "Review Rust"
severity = "block"
"#,
    )
    .unwrap();
    fs::create_dir_all(repo_path.join(".noslop")).unwrap();

    git_add(repo_path, ".noslop.toml");
    git_commit(repo_path, "Initial commit");

    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repo_path)
        .output()
        .unwrap();
    let head_sha = String::from_utf8_lossy(&output.stdout).trim().to_string();

    // Run review with base=head (no changes)
    noslop()
        .args(["review", "run", "--base", &head_sha, "--head", &head_sha])
        .current_dir(repo_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("Findings: 0"));
}
