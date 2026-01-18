//! Tests for git worktree support
//!
//! These tests verify that noslop correctly handles git worktrees by:
//! - Detecting when running in a linked worktree vs main worktree
//! - Resolving all .noslop/ paths to the main worktree
//! - Sharing task state across worktrees

use serial_test::serial;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

use noslop::git;
use noslop::paths;

/// Helper to create a git repository in a temp directory and cd into it
fn setup_git_repo() -> (TempDir, PathBuf) {
    let temp = TempDir::new().unwrap();
    let path = temp.path().to_path_buf();

    // Initialize git repo
    Command::new("git")
        .args(["init"])
        .current_dir(&path)
        .output()
        .expect("Failed to init git repo");

    // Configure git user
    Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(&path)
        .output()
        .expect("Failed to configure git email");

    Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(&path)
        .output()
        .expect("Failed to configure git name");

    // Create initial commit
    fs::write(path.join("README.md"), "# Test\n").unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(&path)
        .output()
        .expect("Failed to git add");

    Command::new("git")
        .args(["commit", "-m", "Initial commit"])
        .current_dir(&path)
        .output()
        .expect("Failed to git commit");

    // Set current directory to the repo
    std::env::set_current_dir(&path).unwrap();

    (temp, path)
}

/// Helper to create an empty temp directory and cd into it (non-git)
fn setup_non_git() -> TempDir {
    let temp = TempDir::new().unwrap();
    std::env::set_current_dir(temp.path()).unwrap();
    temp
}

// =============================================================================
// git module tests
// =============================================================================

#[test]
#[serial(cwd)]
fn test_get_main_worktree_in_regular_repo() {
    let (_temp, repo_path) = setup_git_repo();

    let main_worktree = git::get_main_worktree();
    assert!(main_worktree.is_some());

    let main_worktree = main_worktree.unwrap();
    // Canonicalize both for comparison (handles symlinks on macOS)
    let expected = repo_path.canonicalize().unwrap();
    assert_eq!(main_worktree, expected);
}

#[test]
#[serial(cwd)]
fn test_is_linked_worktree_false_in_main() {
    let _temp = setup_git_repo();

    // Main worktree should NOT be detected as linked
    assert!(!git::is_linked_worktree());
}

#[test]
#[serial(cwd)]
#[ignore] // This test creates git worktrees which can fail in nested git environments
fn test_worktree_detection_in_linked_worktree() {
    let (temp, repo_path) = setup_git_repo();

    // Create a linked worktree
    let worktree_path = temp.path().join("linked-worktree");
    let output = Command::new("git")
        .args(["worktree", "add", worktree_path.to_str().unwrap(), "HEAD", "--detach"])
        .current_dir(&repo_path)
        .output()
        .expect("Failed to create worktree");

    if !output.status.success() {
        eprintln!("git worktree add failed: {}", String::from_utf8_lossy(&output.stderr));
        panic!("Failed to create worktree");
    }

    // Test from linked worktree
    std::env::set_current_dir(&worktree_path).unwrap();

    // Should detect as linked worktree
    assert!(git::is_linked_worktree(), "Should detect linked worktree");

    // get_main_worktree should return the MAIN repo, not the linked one
    let main_worktree = git::get_main_worktree().expect("Should get main worktree");
    let expected_main = repo_path.canonicalize().unwrap();
    assert_eq!(main_worktree, expected_main, "Main worktree should point to original repo");
}

#[test]
#[serial(cwd)]
fn test_get_main_worktree_outside_git_repo() {
    let _temp = setup_non_git();

    // Should return None outside a git repo
    assert!(git::get_main_worktree().is_none());
    assert!(!git::is_linked_worktree());
}

#[test]
#[serial(cwd)]
#[ignore] // This test creates git worktrees which can fail in nested git environments
fn test_worktrees_share_common_git_dir() {
    let (temp, repo_path) = setup_git_repo();

    // Create a linked worktree
    let worktree_path = temp.path().join("worktree-2");
    let output = Command::new("git")
        .args(["worktree", "add", worktree_path.to_str().unwrap(), "HEAD", "--detach"])
        .current_dir(&repo_path)
        .output()
        .expect("Failed to create worktree");

    if !output.status.success() {
        panic!("Failed to create worktree");
    }

    // From main worktree (already there from setup)
    let main_from_main = git::get_main_worktree().unwrap();

    // From linked worktree
    std::env::set_current_dir(&worktree_path).unwrap();
    let main_from_linked = git::get_main_worktree().unwrap();

    // Both should point to the same main worktree
    assert_eq!(main_from_main, main_from_linked);
}

// =============================================================================
// paths module tests
// =============================================================================

#[test]
#[serial(cwd)]
#[ignore] // This test creates git worktrees which can fail in nested git environments
fn test_paths_in_worktree_point_to_main() {
    let (temp, repo_path) = setup_git_repo();

    // Create .noslop/ structure in main repo
    let main_noslop_dir = repo_path.join(".noslop");
    fs::create_dir_all(main_noslop_dir.join("refs/tasks")).unwrap();
    fs::write(main_noslop_dir.join("HEAD"), "TSK-1\n").unwrap();

    // Create a linked worktree
    let worktree_path = temp.path().join("linked");
    let output = Command::new("git")
        .args(["worktree", "add", worktree_path.to_str().unwrap(), "HEAD", "--detach"])
        .current_dir(&repo_path)
        .output()
        .expect("Failed to create worktree");

    if !output.status.success() {
        panic!("Failed to create worktree: {}", String::from_utf8_lossy(&output.stderr));
    }

    // Change to linked worktree
    std::env::set_current_dir(&worktree_path).unwrap();

    // Verify paths point to main worktree
    let noslop = paths::noslop_dir();
    let canonical_main = main_noslop_dir.canonicalize().unwrap();
    let canonical_noslop = noslop.canonicalize().unwrap();
    assert_eq!(
        canonical_noslop, canonical_main,
        "noslop_dir() should point to main worktree's .noslop/"
    );

    // head_file should be the one in main worktree
    let head = paths::head_file();
    assert!(head.exists(), "HEAD file should exist (pointing to main worktree)");
    let content = fs::read_to_string(&head).unwrap();
    assert_eq!(content.trim(), "TSK-1");
}

#[test]
#[serial(cwd)]
#[ignore] // This test creates git worktrees which can fail in nested git environments
fn test_task_ref_path_in_worktree() {
    let (temp, repo_path) = setup_git_repo();

    // Create .noslop/ structure in main repo with a task
    let main_noslop_dir = repo_path.join(".noslop");
    let refs_dir = main_noslop_dir.join("refs/tasks");
    fs::create_dir_all(&refs_dir).unwrap();
    fs::write(refs_dir.join("TSK-1"), r#"{"title":"Test task","status":"pending"}"#).unwrap();

    // Create linked worktree
    let worktree_path = temp.path().join("worktree");
    let output = Command::new("git")
        .args(["worktree", "add", worktree_path.to_str().unwrap(), "HEAD", "--detach"])
        .current_dir(&repo_path)
        .output()
        .expect("Failed to create worktree");

    if !output.status.success() {
        panic!("Failed to create worktree");
    }

    std::env::set_current_dir(&worktree_path).unwrap();

    // task_ref should resolve to main worktree
    let task_path = paths::task_ref("TSK-1");
    assert!(task_path.exists(), "Task should be readable from worktree");
    let content = fs::read_to_string(&task_path).unwrap();
    assert!(content.contains("Test task"));
}

#[test]
#[serial(cwd)]
fn test_project_root_fallback_outside_git() {
    let _temp = setup_non_git();

    // Should fall back to current directory
    let root = paths::project_root();
    assert_eq!(root, PathBuf::from("."));
}
