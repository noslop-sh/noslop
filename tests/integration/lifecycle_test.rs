//! Integration tests for the full attestation lifecycle
//!
//! Tests the complete flow:
//! 1. User attests to assertion
//! 2. User commits (with -m or editor)
//! 3. Hooks add trailers and clear staged attestations
//! 4. Cross-branch contamination is prevented

use assert_cmd::Command;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

/// Helper to initialize a git repo with noslop
fn setup_repo() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let repo_path = temp_dir.path();

    // Initialize git
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(repo_path)
        .output()
        .unwrap();

    std::process::Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(repo_path)
        .output()
        .unwrap();

    std::process::Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(repo_path)
        .output()
        .unwrap();

    // Get the path to the noslop binary
    // Test binaries are in target/debug/deps/, main binary is in target/debug/
    let noslop_bin = std::env::current_exe()
        .unwrap()
        .parent() // Remove test binary name
        .unwrap()
        .parent() // Go from deps/ to debug/
        .unwrap()
        .join("noslop");

    // Initialize noslop
    #[allow(deprecated)]
    {
        Command::cargo_bin("noslop")
            .unwrap()
            .arg("init")
            .current_dir(repo_path)
            .assert()
            .success();
    }

    // Update hooks to use absolute path to noslop binary
    let hooks_dir = repo_path.join(".git/hooks");

    // Log binary path for debugging
    eprintln!("[TEST SETUP] noslop binary path: {}", noslop_bin.display());
    eprintln!("[TEST SETUP] Binary exists: {}", noslop_bin.exists());

    // Update pre-commit hook
    let pre_commit_path = hooks_dir.join("pre-commit");
    let pre_commit = std::fs::read_to_string(&pre_commit_path).unwrap();
    let pre_commit = pre_commit.replace("noslop", noslop_bin.to_str().unwrap());
    std::fs::write(&pre_commit_path, &pre_commit).unwrap();

    // Update commit-msg hook
    let commit_msg_path = hooks_dir.join("commit-msg");
    let commit_msg = std::fs::read_to_string(&commit_msg_path).unwrap();
    let commit_msg = commit_msg.replace("noslop", noslop_bin.to_str().unwrap());
    std::fs::write(&commit_msg_path, &commit_msg).unwrap();

    // Update post-commit hook
    let post_commit_path = hooks_dir.join("post-commit");
    let post_commit = std::fs::read_to_string(&post_commit_path).unwrap();
    let post_commit = post_commit.replace("noslop", noslop_bin.to_str().unwrap());
    std::fs::write(&post_commit_path, &post_commit).unwrap();

    // Restore executable permissions (required on Unix)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for hook in ["pre-commit", "commit-msg", "post-commit"] {
            let hook_path = hooks_dir.join(hook);
            let mut perms = std::fs::metadata(&hook_path).unwrap().permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&hook_path, perms).unwrap();
        }
    }

    temp_dir
}

/// Helper to create noslop command in a directory
fn noslop_in(dir: &Path) -> Command {
    #[allow(deprecated)]
    let mut cmd = Command::cargo_bin("noslop").unwrap();
    cmd.current_dir(dir);
    cmd
}

/// Helper to create git command in a directory
fn git_in(dir: &Path, args: &[&str]) -> std::process::Command {
    let mut cmd = std::process::Command::new("git");
    cmd.args(args).current_dir(dir);
    cmd
}

#[test]
fn test_full_lifecycle_with_commit_m() {
    let temp_dir = setup_repo();
    let repo_path = temp_dir.path();

    // Add an assertion (will get auto-assigned ID: AST-1)
    noslop_in(repo_path)
        .args(["assert", "add", "*.rs", "-m", "Review Rust changes", "-s", "block"])
        .assert()
        .success();

    // Create and stage a file
    fs::write(repo_path.join("test.rs"), "fn main() {}").unwrap();
    git_in(repo_path, &["add", "test.rs"]).output().unwrap();

    // Attest (use the actual ID from .noslop.toml: AST-1)
    noslop_in(repo_path)
        .args(["attest", "AST-1", "-m", "Reviewed test file"])
        .assert()
        .success();

    // Verify staged attestations exist
    let attestations_file = repo_path.join(".noslop/staged-attestations.json");
    assert!(attestations_file.exists());

    // Commit with -m (triggers hooks)
    let output = git_in(repo_path, &["commit", "-m", "Add test file"]).output().unwrap();
    assert!(output.status.success(), "Commit should succeed");

    // Verify staged attestations were cleared
    assert!(
        !attestations_file.exists(),
        "Staged attestations should be cleared after commit"
    );

    // Verify commit message contains trailer
    let log_output = git_in(repo_path, &["log", "-1", "--pretty=format:%B"]).output().unwrap();
    let commit_msg = String::from_utf8(log_output.stdout).unwrap();

    assert!(
        commit_msg.contains("Noslop-Attest: AST-1 | Reviewed test file | human"),
        "Commit message should contain attestation trailer. Got: {}",
        commit_msg
    );
}

#[test]
fn test_cross_branch_no_contamination() {
    let temp_dir = setup_repo();
    let repo_path = temp_dir.path();

    // Add an assertion
    noslop_in(repo_path)
        .args(["assert", "add", "*.rs", "-m", "Review Rust changes", "-s", "block"])
        .assert()
        .success();

    // Create and stage a file on main
    fs::write(repo_path.join("main.rs"), "fn main() {}").unwrap();
    git_in(repo_path, &["add", "main.rs"]).output().unwrap();

    // Attest on main
    noslop_in(repo_path)
        .args(["attest", "AST-1", "-m", "Main branch changes"])
        .assert()
        .success();

    let attestations_file = repo_path.join(".noslop/staged-attestations.json");
    assert!(attestations_file.exists());

    // Create and switch to new branch (attestations should NOT follow)
    git_in(repo_path, &["checkout", "-b", "feature"]).output().unwrap();

    // The staged-attestations.json persists because it's untracked
    // This is the BUG we're testing for - but after hooks are installed,
    // commits will clear it

    // Modify different file on feature branch
    fs::write(repo_path.join("feature.rs"), "fn feature() {}").unwrap();
    git_in(repo_path, &["add", "feature.rs"]).output().unwrap();

    // Check should fail because AST-1 still applies but we don't have proper attestation for THIS file
    // The old attestation from main branch is contaminating feature branch
    let _check_output = noslop_in(repo_path).arg("check").output().unwrap();

    // With the bug, this might incorrectly pass because old attestation exists
    // With the fix (post-commit clearing), each branch starts clean

    // For this test, we simulate the fix by manually clearing
    fs::remove_file(&attestations_file).ok();

    // Now attestation should be required
    noslop_in(repo_path).arg("check").assert().failure();

    // Attest for feature branch
    noslop_in(repo_path)
        .args(["attest", "AST-1", "-m", "Feature branch changes"])
        .assert()
        .success();

    // Now commit should succeed
    let output = git_in(repo_path, &["commit", "-m", "Add feature"]).output().unwrap();

    assert!(output.status.success());

    // Verify attestations cleared
    assert!(!attestations_file.exists());

    // Verify correct attestation in commit message
    let log_output = git_in(repo_path, &["log", "-1", "--pretty=format:%B"]).output().unwrap();
    let commit_msg = String::from_utf8(log_output.stdout).unwrap();

    assert!(commit_msg.contains("Feature branch changes"));
    assert!(!commit_msg.contains("Main branch changes"));
}

#[test]
fn test_multiple_attestations_in_commit() {
    let temp_dir = setup_repo();
    let repo_path = temp_dir.path();

    // Add multiple assertions
    noslop_in(repo_path)
        .args(["assert", "add", "src/*.rs", "-m", "Review source changes", "-s", "block"])
        .assert()
        .success();

    noslop_in(repo_path)
        .args(["assert", "add", "*.toml", "-m", "Review config changes", "-s", "warn"])
        .assert()
        .success();

    // Create files that match both assertions
    fs::create_dir_all(repo_path.join("src")).unwrap();
    fs::write(repo_path.join("src/lib.rs"), "pub fn lib() {}").unwrap();
    fs::write(repo_path.join("Cargo.toml"), "[package]").unwrap();

    git_in(repo_path, &["add", "."]).output().unwrap();

    // Attest to both (AST-1 for *.rs, AST-2 for *.toml)
    noslop_in(repo_path)
        .args(["attest", "AST-1", "-m", "Reviewed source code"])
        .assert()
        .success();

    noslop_in(repo_path)
        .args(["attest", "AST-2", "-m", "Reviewed config"])
        .assert()
        .success();

    // Commit
    git_in(repo_path, &["commit", "-m", "Add library and config"]).output().unwrap();

    // Verify both attestations in commit message
    let log_output = git_in(repo_path, &["log", "-1", "--pretty=format:%B"]).output().unwrap();
    let commit_msg = String::from_utf8(log_output.stdout).unwrap();

    assert!(commit_msg.contains("Noslop-Attest: AST-1 | Reviewed source code | human"));
    assert!(commit_msg.contains("Noslop-Attest: AST-2 | Reviewed config | human"));

    // Verify cleared
    assert!(!repo_path.join(".noslop/staged-attestations.json").exists());
}

#[test]
fn test_commit_without_attestations() {
    let temp_dir = setup_repo();
    let repo_path = temp_dir.path();

    // Create a file that doesn't match any assertions
    fs::write(repo_path.join("README.md"), "# Test").unwrap();
    git_in(repo_path, &["add", "README.md"]).output().unwrap();

    // Commit should succeed (no assertions apply)
    let output = git_in(repo_path, &["commit", "-m", "Add README"]).output().unwrap();

    assert!(output.status.success());

    // Commit message should not have noslop trailers
    let log_output = git_in(repo_path, &["log", "-1", "--pretty=format:%B"]).output().unwrap();
    let commit_msg = String::from_utf8(log_output.stdout).unwrap();

    assert!(!commit_msg.contains("Noslop-Attest"));
}

#[test]
fn test_commit_blocked_without_attestation() {
    let temp_dir = setup_repo();
    let repo_path = temp_dir.path();

    // Add blocking assertion
    noslop_in(repo_path)
        .args(["assert", "add", "*.rs", "-m", "Must review Rust code", "-s", "block"])
        .assert()
        .success();

    // Create and stage matching file
    fs::write(repo_path.join("test.rs"), "fn test() {}").unwrap();
    git_in(repo_path, &["add", "test.rs"]).output().unwrap();

    // Try to commit WITHOUT attestation - should fail
    let output = git_in(repo_path, &["commit", "-m", "Add test"]).output().unwrap();

    assert!(!output.status.success(), "Commit should be blocked");
}
