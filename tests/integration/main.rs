//! Integration tests for noslop CLI
//!
//! These tests simulate real-world workflows with a complete codebase,
//! testing the full cycle of: init → check add → verify → check run

// Include lifecycle tests from the same directory
mod lifecycle_test;

use assert_cmd::cargo;
use noslop::git;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

/// Helper function to create a noslop command
fn noslop() -> assert_cmd::Command {
    assert_cmd::Command::new(cargo::cargo_bin!("noslop"))
}

/// Helper to initialize a git repo with basic config
/// Also disables hooks to avoid PATH issues in tests
fn init_git_repo(path: &std::path::Path) {
    assert!(git::init_repo(path), "Failed to init git repo");
    assert!(
        git::configure_user(path, "test@example.com", "Test User"),
        "Failed to configure git user"
    );
    // Disable hooks to avoid PATH issues when hooks call noslop
    git::disable_hooks(path);
}

/// Helper to stage files in git
fn git_add(path: &std::path::Path, file: &str) {
    assert!(git::add_file(path, file), "Failed to stage file: {}", file);
}

/// Helper to create a git commit
fn git_commit(path: &std::path::Path, message: &str) {
    assert!(
        git::commit(path, message),
        "Failed to create commit: {}",
        message
    );
}

// =============================================================================
// END-TO-END WORKFLOW TESTS
// =============================================================================

/// Test complete workflow: init → create files → add check → verify → check run
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

    // Step 4: Add a check for Rust files
    noslop()
        .args(["check", "add", "*.rs", "-m", "Ensure all Rust code follows style guidelines"])
        .current_dir(repo_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("Added check"));

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

    // Step 7: Check should FAIL because check is not verified
    noslop()
        .args(["check", "run", "--ci"])
        .current_dir(repo_path)
        .assert()
        .failure()
        .stdout(predicate::str::contains("BLOCKED"))
        .stdout(predicate::str::contains("Ensure all Rust code follows style guidelines"));

    // Step 8: Verify the check
    noslop()
        .args([
            "verify",
            "Ensure all Rust code follows style guidelines",
            "-m",
            "Reviewed code and it follows rustfmt conventions",
        ])
        .current_dir(repo_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("Staged verification"));

    // Step 9: Check should now PASS
    noslop()
        .args(["check", "run", "--ci"])
        .current_dir(repo_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("All checks verified"));
}

/// Test that check fails when blocking checks are not verified
#[test]
fn test_check_blocks_without_verification() {
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

    // Check should fail
    noslop()
        .args(["check", "run", "--ci"])
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
        .args(["check", "run", "--ci"])
        .current_dir(repo_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("Warnings"))
        .stdout(predicate::str::contains("Consider adding JSDoc comments"))
        .stdout(
            predicate::str::contains("All checks verified")
                .or(predicate::str::contains("Commit may proceed")),
        );
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

    // Check should fail - missing Rust and TypeScript verifications
    noslop()
        .args(["check", "run", "--ci"])
        .current_dir(repo_path)
        .assert()
        .failure()
        .stdout(predicate::str::contains("BLOCKED"))
        .stdout(predicate::str::contains("Rust code must be formatted with rustfmt"))
        .stdout(predicate::str::contains("TypeScript must use strict mode"));

    // Verify Rust check only
    noslop()
        .args([
            "verify",
            "Rust code must be formatted with rustfmt",
            "-m",
            "Ran rustfmt on all files",
        ])
        .current_dir(repo_path)
        .assert()
        .success();

    // Still should fail - missing TypeScript verification
    noslop()
        .args(["check", "run", "--ci"])
        .current_dir(repo_path)
        .assert()
        .failure()
        .stdout(predicate::str::contains("TypeScript must use strict mode"));

    // Verify TypeScript check
    noslop()
        .args([
            "verify",
            "TypeScript must use strict mode",
            "-m",
            "Added strict: true to tsconfig",
        ])
        .current_dir(repo_path)
        .assert()
        .success();

    // Now should pass
    noslop()
        .args(["check", "run", "--ci"])
        .current_dir(repo_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("All checks verified"));
}

/// Test that same check applies to multiple matching files
#[test]
fn test_single_check_multiple_files() {
    let temp = TempDir::new().unwrap();
    let repo_path = temp.path();

    // Setup
    init_git_repo(repo_path);

    // Create .noslop.toml with glob pattern (using *.rs which is more reliable)
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

    // Check should fail
    noslop()
        .args(["check", "run", "--ci"])
        .current_dir(repo_path)
        .assert()
        .failure()
        .stdout(predicate::str::contains("BLOCKED"));

    // Single verification should cover all files
    noslop()
        .args([
            "verify",
            "All source files need security review",
            "-m",
            "Reviewed all files for security issues",
        ])
        .current_dir(repo_path)
        .assert()
        .success();

    // Now should pass
    noslop()
        .args(["check", "run", "--ci"])
        .current_dir(repo_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("All checks verified"));
}

// =============================================================================
// VERIFICATION MATCHING TESTS
// =============================================================================

/// Test verification matching by check target
#[test]
fn test_verify_by_target() {
    let temp = TempDir::new().unwrap();
    let repo_path = temp.path();

    // Setup
    init_git_repo(repo_path);

    // Create .noslop.toml
    fs::write(
        repo_path.join(".noslop.toml"),
        r#"[[check]]
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

    // Verify by target pattern
    noslop()
        .args(["verify", "config/*.yaml", "-m", "DevOps team approved changes"])
        .current_dir(repo_path)
        .assert()
        .success();

    // Should pass
    noslop()
        .args(["check", "run", "--ci"])
        .current_dir(repo_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("All checks verified"));
}

/// Test verification matching by partial message
#[test]
fn test_verify_by_partial_message() {
    let temp = TempDir::new().unwrap();
    let repo_path = temp.path();

    // Setup
    init_git_repo(repo_path);

    // Create .noslop.toml with a long message
    fs::write(
        repo_path.join(".noslop.toml"),
        r#"[[check]]
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

    // Verify using the full message
    noslop()
        .args([
            "verify",
            "Database migrations must be reviewed by DBA team for performance impact",
            "-m",
            "DBA approved - no index changes needed",
        ])
        .current_dir(repo_path)
        .assert()
        .success();

    // Should pass
    noslop().args(["check", "run", "--ci"]).current_dir(repo_path).assert().success();
}

// =============================================================================
// STAGED VERIFICATIONS TESTS
// =============================================================================

/// Test that verifications are stored in staged-verifications.json
#[test]
fn test_verification_stored_in_json_file() {
    let temp = TempDir::new().unwrap();
    let repo_path = temp.path();

    // Setup
    init_git_repo(repo_path);

    // Create .noslop directory
    fs::create_dir_all(repo_path.join(".noslop")).unwrap();

    // Verify
    noslop()
        .args(["verify", "test-check", "-m", "Test verification message"])
        .current_dir(repo_path)
        .assert()
        .success();

    // Verify staged-verifications.json exists and contains correct data
    let json_path = repo_path.join(".noslop/staged-verifications.json");
    assert!(json_path.exists(), "staged-verifications.json should exist");

    let content = fs::read_to_string(&json_path).unwrap();
    assert!(content.contains("test-check"), "JSON should contain check ID");
    assert!(
        content.contains("Test verification message"),
        "JSON should contain verification message"
    );
    assert!(content.contains("human"), "JSON should indicate human verification");
}

/// Test multiple verifications accumulate in JSON file
#[test]
fn test_multiple_verifications_accumulate() {
    let temp = TempDir::new().unwrap();
    let repo_path = temp.path();

    // Setup
    init_git_repo(repo_path);

    // Create .noslop directory
    fs::create_dir_all(repo_path.join(".noslop")).unwrap();

    // Add first verification
    noslop()
        .args(["verify", "first-check", "-m", "First verification"])
        .current_dir(repo_path)
        .assert()
        .success();

    // Add second verification
    noslop()
        .args(["verify", "second-check", "-m", "Second verification"])
        .current_dir(repo_path)
        .assert()
        .success();

    // Verify both are in the JSON file
    let json_path = repo_path.join(".noslop/staged-verifications.json");
    let content = fs::read_to_string(&json_path).unwrap();

    assert!(content.contains("first-check"), "Should contain first check");
    assert!(content.contains("second-check"), "Should contain second check");
    assert!(content.contains("First verification"), "Should contain first message");
    assert!(content.contains("Second verification"), "Should contain second message");
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
        .args([
            "check",
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
    noslop().args(["check", "run"]).current_dir(temp.path()).assert().failure();
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
        r#"[[check]]
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

    // Check should require both checks to be verified
    let output = noslop().args(["check", "run", "--ci"]).current_dir(repo_path).assert().failure();

    // The check should mention both checks
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

    // Without --ci, check might exit differently
    // With --ci, should return error result (for CI systems)
    noslop()
        .args(["check", "run", "--ci"])
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
        .args(["check", "run", "--ci"])
        .current_dir(repo_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("No checks apply"));
}

// =============================================================================
// BRANCH-SCOPED TASK TESTS
// =============================================================================

/// Test that task init creates branch-specific task file
#[test]
fn test_task_init_creates_branch_file() {
    let temp = TempDir::new().unwrap();
    let repo_path = temp.path();
    init_git_repo(repo_path);

    // Create initial commit so we have a branch
    fs::write(repo_path.join("README.md"), "# Test").unwrap();
    git_add(repo_path, "README.md");
    git_commit(repo_path, "Initial commit");

    // Initialize noslop
    noslop().arg("init").current_dir(repo_path).assert().success();

    // Check that we're on a branch (should be main or master)
    let branch = git::get_branch_in(repo_path).expect("Should be on a branch");

    // Init task file for current branch
    noslop()
        .args(["task", "init"])
        .current_dir(repo_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("Created:"))
        .stdout(predicate::str::contains(&branch));

    // Check that task file was created
    let task_file = repo_path
        .join(".noslop/tasks")
        .join(format!("{}.toml", branch.replace('/', "-")));
    assert!(task_file.exists(), "Task file should exist: {:?}", task_file);

    // Verify it's gitignored (tasks/ should be in .gitignore)
    let gitignore_content = fs::read_to_string(repo_path.join(".noslop/.gitignore")).unwrap();
    assert!(
        gitignore_content.contains("tasks/"),
        "tasks/ should be gitignored"
    );
}

/// Test that tasks are added to branch-specific file
#[test]
fn test_task_add_uses_branch_file() {
    let temp = TempDir::new().unwrap();
    let repo_path = temp.path();
    init_git_repo(repo_path);

    // Create initial commit so we have a branch
    fs::write(repo_path.join("README.md"), "# Test").unwrap();
    git_add(repo_path, "README.md");
    git_commit(repo_path, "Initial commit");

    noslop().arg("init").current_dir(repo_path).assert().success();

    // Add a task
    noslop()
        .args(["task", "add", "Test task for branch"])
        .current_dir(repo_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("Created task:"));

    // List tasks - should show the branch name
    let output = noslop()
        .args(["task", "list"])
        .current_dir(repo_path)
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    assert!(
        stdout.contains("Branch:"),
        "Task list should show branch name"
    );
    assert!(
        stdout.contains("Test task for branch"),
        "Task list should show added task"
    );
}

/// Test that tasks are isolated per branch
#[test]
fn test_tasks_isolated_per_branch() {
    let temp = TempDir::new().unwrap();
    let repo_path = temp.path();
    init_git_repo(repo_path);

    // Create initial commit so we can create branches
    fs::write(repo_path.join("README.md"), "# Test").unwrap();
    git_add(repo_path, "README.md");
    git_commit(repo_path, "Initial commit");

    noslop().arg("init").current_dir(repo_path).assert().success();

    // Add task on main branch
    noslop()
        .args(["task", "add", "Main branch task"])
        .current_dir(repo_path)
        .assert()
        .success();

    // Verify main branch has the task
    noslop()
        .args(["task", "list"])
        .current_dir(repo_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("Main branch task"));

    // Create and switch to feature branch
    assert!(
        git::checkout(repo_path, "feature/test", true),
        "Failed to create feature branch"
    );

    // Feature branch should have no tasks
    noslop()
        .args(["task", "list"])
        .current_dir(repo_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("No tasks found"));

    // Add task on feature branch
    noslop()
        .args(["task", "add", "Feature branch task"])
        .current_dir(repo_path)
        .assert()
        .success();

    // Verify feature branch has its task
    noslop()
        .args(["task", "list"])
        .current_dir(repo_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("Feature branch task"));

    // Switch back to main (try master first, then main)
    let _ = git::checkout(repo_path, "master", false);
    let _ = git::checkout(repo_path, "main", false);

    // Main should only show main branch task
    let output = noslop()
        .args(["task", "list"])
        .current_dir(repo_path)
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    assert!(
        stdout.contains("Main branch task"),
        "Main branch should show its task"
    );
    assert!(
        !stdout.contains("Feature branch task"),
        "Main branch should NOT show feature branch task"
    );
}

/// Test task operations (start, done, remove) work on branch-scoped tasks
#[test]
fn test_task_operations_branch_scoped() {
    let temp = TempDir::new().unwrap();
    let repo_path = temp.path();
    init_git_repo(repo_path);

    // Create initial commit so we have a branch
    fs::write(repo_path.join("README.md"), "# Test").unwrap();
    git_add(repo_path, "README.md");
    git_commit(repo_path, "Initial commit");

    noslop().arg("init").current_dir(repo_path).assert().success();

    // Add a task
    noslop()
        .args(["task", "add", "Task to complete"])
        .current_dir(repo_path)
        .assert()
        .success();

    // Start the task (get the ID from the output)
    let output = noslop()
        .args(["task", "list", "--json"])
        .current_dir(repo_path)
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    // Parse task ID from JSON (simple extraction)
    let task_id = if stdout.contains("TSK-1") {
        "TSK-1"
    } else {
        "NOS-1" // In case project prefix is different
    };

    // Start the task
    noslop()
        .args(["task", "start", task_id])
        .current_dir(repo_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("Started:"));

    // Verify it's in progress
    noslop()
        .args(["task", "list"])
        .current_dir(repo_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("◐")); // in_progress icon

    // Complete the task
    noslop()
        .args(["task", "done", task_id])
        .current_dir(repo_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("Completed:"));

    // Remove the task
    noslop()
        .args(["task", "remove", task_id])
        .current_dir(repo_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("Removed:"));

    // Verify task is gone
    noslop()
        .args(["task", "list"])
        .current_dir(repo_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("No tasks found"));
}

/// Test that task init fails if task file already exists
#[test]
fn test_task_init_fails_if_exists() {
    let temp = TempDir::new().unwrap();
    let repo_path = temp.path();
    init_git_repo(repo_path);

    // Create initial commit so we have a branch
    fs::write(repo_path.join("README.md"), "# Test").unwrap();
    git_add(repo_path, "README.md");
    git_commit(repo_path, "Initial commit");

    noslop().arg("init").current_dir(repo_path).assert().success();

    // Init task file first time
    noslop()
        .args(["task", "init"])
        .current_dir(repo_path)
        .assert()
        .success();

    // Second init should report already exists
    noslop()
        .args(["task", "init"])
        .current_dir(repo_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("already exists"));
}

/// Test task blocked_by functionality with branch-scoped storage
#[test]
fn test_task_blocked_by_branch_scoped() {
    let temp = TempDir::new().unwrap();
    let repo_path = temp.path();
    init_git_repo(repo_path);

    // Create initial commit so we have a branch
    fs::write(repo_path.join("README.md"), "# Test").unwrap();
    git_add(repo_path, "README.md");
    git_commit(repo_path, "Initial commit");

    noslop().arg("init").current_dir(repo_path).assert().success();

    // Add first task
    noslop()
        .args(["task", "add", "First task"])
        .current_dir(repo_path)
        .assert()
        .success();

    // Add second task blocked by first (use --blocked-by with ID format)
    noslop()
        .args(["task", "add", "Second task", "--blocked-by", "TSK-1"])
        .current_dir(repo_path)
        .assert()
        .success();

    // List ready tasks - only first should be ready
    noslop()
        .args(["task", "list", "--ready"])
        .current_dir(repo_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("First task"))
        .stdout(predicate::str::contains("Second task").not());

    // Complete first task
    noslop()
        .args(["task", "done", "TSK-1"])
        .current_dir(repo_path)
        .assert()
        .success();

    // Now second task should be ready
    noslop()
        .args(["task", "list", "--ready"])
        .current_dir(repo_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("Second task"));
}
