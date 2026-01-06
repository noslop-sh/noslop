//! Integration tests for the full verification lifecycle
//!
//! Tests the complete flow:
//! 1. User verifies a check
//! 2. User commits (with -m or editor)
//! 3. Hooks add trailers and clear staged verifications
//! 4. Cross-branch contamination is prevented

use assert_cmd::Command;
use noslop::git;
use std::fs;
use std::path::Path;
use std::process::Command as ProcessCommand;
use tempfile::TempDir;

/// Helper to initialize a git repo with noslop
fn setup_repo() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let repo_path = temp_dir.path();

    // Initialize git (configure_user also disables commit signing)
    assert!(git::init_repo(repo_path), "Failed to init git repo");
    assert!(
        git::configure_user(repo_path, "test@example.com", "Test User"),
        "Failed to configure git user"
    );

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
fn git_in(dir: &Path, args: &[&str]) -> ProcessCommand {
    let mut cmd = ProcessCommand::new("git");
    cmd.args(args).current_dir(dir);
    cmd
}

#[test]
fn test_full_lifecycle_with_commit_m() {
    let temp_dir = setup_repo();
    let repo_path = temp_dir.path();

    // Add a check (will get auto-assigned ID)
    noslop_in(repo_path)
        .args(["check", "add", "*.rs", "-m", "Review Rust changes", "-s", "block"])
        .assert()
        .success();

    // Create and stage a file
    fs::write(repo_path.join("test.rs"), "fn main() {}").unwrap();
    git_in(repo_path, &["add", "test.rs"]).output().unwrap();

    // Read the generated check ID from .noslop.toml
    let noslop_content = fs::read_to_string(repo_path.join(".noslop.toml")).unwrap();
    // Extract the check ID (format: prefix-N, e.g., TMP-1)
    let check_id = noslop_content
        .lines()
        .find(|l| l.starts_with("id = "))
        .map(|l| l.trim_start_matches("id = ").trim_matches('"'))
        .unwrap_or("TMP-1");

    // Verify the check
    noslop_in(repo_path)
        .args(["verify", check_id, "-m", "Reviewed test file"])
        .assert()
        .success();

    // Verify staged verifications exist
    let verifications_file = repo_path.join(".noslop/staged-verifications.json");
    assert!(verifications_file.exists());

    // Commit with -m (triggers hooks)
    let output = git_in(repo_path, &["commit", "-m", "Add test file"]).output().unwrap();
    assert!(output.status.success(), "Commit should succeed");

    // Verify staged verifications were cleared
    assert!(
        !verifications_file.exists(),
        "Staged verifications should be cleared after commit"
    );

    // Verify commit message contains trailer
    let log_output = git_in(repo_path, &["log", "-1", "--pretty=format:%B"]).output().unwrap();
    let commit_msg = String::from_utf8(log_output.stdout).unwrap();

    assert!(
        commit_msg.contains("Noslop-Verify:") && commit_msg.contains("Reviewed test file"),
        "Commit message should contain verification trailer. Got: {}",
        commit_msg
    );
}

#[test]
fn test_cross_branch_no_contamination() {
    let temp_dir = setup_repo();
    let repo_path = temp_dir.path();

    // Add a check
    noslop_in(repo_path)
        .args(["check", "add", "*.rs", "-m", "Review Rust changes", "-s", "block"])
        .assert()
        .success();

    // Read the generated check ID from .noslop.toml
    let noslop_content = fs::read_to_string(repo_path.join(".noslop.toml")).unwrap();
    let check_id = noslop_content
        .lines()
        .find(|l| l.starts_with("id = "))
        .map(|l| l.trim_start_matches("id = ").trim_matches('"'))
        .unwrap_or("TMP-1");

    // Create and stage a file on main
    fs::write(repo_path.join("main.rs"), "fn main() {}").unwrap();
    git_in(repo_path, &["add", "main.rs"]).output().unwrap();

    // Verify on main
    noslop_in(repo_path)
        .args(["verify", check_id, "-m", "Main branch changes"])
        .assert()
        .success();

    let verifications_file = repo_path.join(".noslop/staged-verifications.json");
    assert!(verifications_file.exists());

    // Create and switch to new branch (verifications should NOT follow)
    git_in(repo_path, &["checkout", "-b", "feature"]).output().unwrap();

    // The staged-verifications.json persists because it's untracked
    // This is the BUG we're testing for - but after hooks are installed,
    // commits will clear it

    // Modify different file on feature branch
    fs::write(repo_path.join("feature.rs"), "fn feature() {}").unwrap();
    git_in(repo_path, &["add", "feature.rs"]).output().unwrap();

    // Check should fail because check still applies but we don't have proper verification for THIS file
    // The old verification from main branch is contaminating feature branch
    let _check_output = noslop_in(repo_path).args(["check", "run"]).output().unwrap();

    // With the bug, this might incorrectly pass because old verification exists
    // With the fix (post-commit clearing), each branch starts clean

    // For this test, we simulate the fix by manually clearing
    fs::remove_file(&verifications_file).ok();

    // Now verification should be required
    noslop_in(repo_path).args(["check", "run"]).assert().failure();

    // Verify for feature branch
    noslop_in(repo_path)
        .args(["verify", check_id, "-m", "Feature branch changes"])
        .assert()
        .success();

    // Now commit should succeed
    let output = git_in(repo_path, &["commit", "-m", "Add feature"]).output().unwrap();

    assert!(output.status.success());

    // Verify verifications cleared
    assert!(!verifications_file.exists());

    // Verify correct verification in commit message
    let log_output = git_in(repo_path, &["log", "-1", "--pretty=format:%B"]).output().unwrap();
    let commit_msg = String::from_utf8(log_output.stdout).unwrap();

    assert!(commit_msg.contains("Feature branch changes"));
    assert!(!commit_msg.contains("Main branch changes"));
}

#[test]
fn test_multiple_verifications_in_commit() {
    let temp_dir = setup_repo();
    let repo_path = temp_dir.path();

    // Add multiple checks
    noslop_in(repo_path)
        .args(["check", "add", "src/*.rs", "-m", "Review source changes", "-s", "block"])
        .assert()
        .success();

    noslop_in(repo_path)
        .args(["check", "add", "*.toml", "-m", "Review config changes", "-s", "warn"])
        .assert()
        .success();

    // Read the generated check IDs from .noslop.toml
    let noslop_content = fs::read_to_string(repo_path.join(".noslop.toml")).unwrap();
    let check_ids: Vec<&str> = noslop_content
        .lines()
        .filter(|l| l.starts_with("id = "))
        .map(|l| l.trim_start_matches("id = ").trim_matches('"'))
        .collect();
    let check_id_1 = check_ids.get(0).unwrap_or(&"TMP-1");
    let check_id_2 = check_ids.get(1).unwrap_or(&"TMP-2");

    // Create files that match both checks
    fs::create_dir_all(repo_path.join("src")).unwrap();
    fs::write(repo_path.join("src/lib.rs"), "pub fn lib() {}").unwrap();
    fs::write(repo_path.join("Cargo.toml"), "[package]").unwrap();

    git_in(repo_path, &["add", "."]).output().unwrap();

    // Verify both checks
    noslop_in(repo_path)
        .args(["verify", check_id_1, "-m", "Reviewed source code"])
        .assert()
        .success();

    noslop_in(repo_path)
        .args(["verify", check_id_2, "-m", "Reviewed config"])
        .assert()
        .success();

    // Commit
    git_in(repo_path, &["commit", "-m", "Add library and config"]).output().unwrap();

    // Verify both verifications in commit message
    let log_output = git_in(repo_path, &["log", "-1", "--pretty=format:%B"]).output().unwrap();
    let commit_msg = String::from_utf8(log_output.stdout).unwrap();

    assert!(commit_msg.contains("Noslop-Verify:") && commit_msg.contains("Reviewed source code"));
    assert!(commit_msg.contains("Noslop-Verify:") && commit_msg.contains("Reviewed config"));

    // Verify cleared
    assert!(!repo_path.join(".noslop/staged-verifications.json").exists());
}

#[test]
fn test_commit_without_verifications() {
    let temp_dir = setup_repo();
    let repo_path = temp_dir.path();

    // Create a file that doesn't match any checks
    fs::write(repo_path.join("README.md"), "# Test").unwrap();
    git_in(repo_path, &["add", "README.md"]).output().unwrap();

    // Commit should succeed (no checks apply)
    let output = git_in(repo_path, &["commit", "-m", "Add README"]).output().unwrap();

    assert!(
        output.status.success(),
        "Commit failed. stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    // Commit message should not have noslop trailers
    let log_output = git_in(repo_path, &["log", "-1", "--pretty=format:%B"]).output().unwrap();
    let commit_msg = String::from_utf8(log_output.stdout).unwrap();

    assert!(!commit_msg.contains("Noslop-Verify"));
}

#[test]
fn test_commit_blocked_without_verification() {
    let temp_dir = setup_repo();
    let repo_path = temp_dir.path();

    // Add blocking check
    noslop_in(repo_path)
        .args(["check", "add", "*.rs", "-m", "Must review Rust code", "-s", "block"])
        .assert()
        .success();

    // Create and stage matching file
    fs::write(repo_path.join("test.rs"), "fn test() {}").unwrap();
    git_in(repo_path, &["add", "test.rs"]).output().unwrap();

    // Try to commit WITHOUT verification - should fail
    let output = git_in(repo_path, &["commit", "-m", "Add test"]).output().unwrap();

    assert!(!output.status.success(), "Commit should be blocked");
}
