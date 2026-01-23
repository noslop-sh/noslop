//! Integration tests for agent commands
//!
//! Tests the agent workflow:
//! - Spawning agents creates git worktrees
//! - Listing agents shows all active agents
//! - Killing agents removes worktrees and releases tasks
//! - Task claiming prevents conflicts

use assert_cmd::Command;
use noslop::git;
use predicates::prelude::*;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

/// Helper to initialize a git repo with noslop
fn setup_repo() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let repo_path = temp_dir.path();

    // Initialize git
    assert!(git::init_repo(repo_path), "Failed to init git repo");
    assert!(
        git::configure_user(repo_path, "test@example.com", "Test User"),
        "Failed to configure git user"
    );

    // Create initial commit so we have a branch
    fs::write(repo_path.join("README.md"), "# Test").unwrap();
    assert!(git::add_file(repo_path, "README.md"), "Failed to stage README");
    assert!(git::commit(repo_path, "Initial commit", false), "Failed to create initial commit");

    // Initialize noslop
    noslop_in(repo_path).arg("init").assert().success();

    temp_dir
}

/// Helper to create noslop command in a directory
fn noslop_in(dir: &Path) -> Command {
    #[allow(deprecated)]
    let mut cmd = Command::cargo_bin("noslop").unwrap();
    cmd.current_dir(dir);
    cmd
}

// =============================================================================
// AGENT SPAWN TESTS
// =============================================================================

/// Test that spawning an idle agent creates a worktree
#[test]
fn test_agent_spawn_idle() {
    let temp_dir = setup_repo();
    let repo_path = temp_dir.path();

    // Spawn an idle agent (no task assignment)
    noslop_in(repo_path)
        .args(["agent", "spawn", "--idle", "--no-tmux"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Spawned agent: agent-1"))
        .stdout(predicate::str::contains("(idle)"));

    // Verify worktree directory exists
    let agent_path = repo_path.join(".noslop/agents/agent-1");
    assert!(agent_path.exists(), "Agent worktree should exist");

    // Verify it's a valid git worktree
    assert!(
        agent_path.join(".git").exists(),
        "Agent should have .git file (worktree marker)"
    );
}

/// Test spawning a named agent
#[test]
fn test_agent_spawn_named() {
    let temp_dir = setup_repo();
    let repo_path = temp_dir.path();

    // Spawn a named agent
    noslop_in(repo_path)
        .args(["agent", "spawn", "--name", "my-agent", "--idle", "--no-tmux"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Spawned agent: my-agent"));

    // Verify worktree directory exists with correct name
    let agent_path = repo_path.join(".noslop/agents/my-agent");
    assert!(agent_path.exists(), "Named agent worktree should exist");
}

/// Test that spawning with tasks auto-assigns a task
#[test]
fn test_agent_spawn_with_task() {
    let temp_dir = setup_repo();
    let repo_path = temp_dir.path();

    // Add a pending task first
    noslop_in(repo_path)
        .args(["task", "add", "Test task for agent"])
        .assert()
        .success();

    // Move task to pending (backlog tasks aren't auto-assigned)
    fs::write(
        repo_path.join(".noslop/refs/tasks/TSK-1"),
        r#"{"title":"Test task for agent","status":"pending","priority":"p2","created_at":"2025-01-01T00:00:00Z","blocked_by":[],"branch":null,"topics":[],"claimed_by":null}"#,
    )
    .unwrap();

    // Spawn an agent (should auto-assign the task)
    noslop_in(repo_path)
        .args(["agent", "spawn", "--no-tmux"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Task:"))
        .stdout(predicate::str::contains("TSK-1"));

    // Verify task is claimed
    let task_content = fs::read_to_string(repo_path.join(".noslop/refs/tasks/TSK-1")).unwrap();
    assert!(task_content.contains("claimed_by"), "Task should be claimed");
    assert!(task_content.contains("agent-1"), "Task should be claimed by agent-1");
}

/// Test spawning duplicate agent fails
#[test]
fn test_agent_spawn_duplicate_fails() {
    let temp_dir = setup_repo();
    let repo_path = temp_dir.path();

    // Spawn first agent
    noslop_in(repo_path)
        .args(["agent", "spawn", "--name", "dup-agent", "--idle", "--no-tmux"])
        .assert()
        .success();

    // Try to spawn again with same name - should fail
    noslop_in(repo_path)
        .args(["agent", "spawn", "--name", "dup-agent", "--idle", "--no-tmux"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("already exists"));
}

// =============================================================================
// AGENT LIST TESTS
// =============================================================================

/// Test listing agents when none exist
#[test]
fn test_agent_list_empty() {
    let temp_dir = setup_repo();
    let repo_path = temp_dir.path();

    noslop_in(repo_path)
        .args(["agent", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No agents"));
}

/// Test listing multiple agents
#[test]
fn test_agent_list_multiple() {
    let temp_dir = setup_repo();
    let repo_path = temp_dir.path();

    // Spawn two agents
    noslop_in(repo_path)
        .args(["agent", "spawn", "--name", "agent-a", "--idle", "--no-tmux"])
        .assert()
        .success();

    noslop_in(repo_path)
        .args(["agent", "spawn", "--name", "agent-b", "--idle", "--no-tmux"])
        .assert()
        .success();

    // List should show both
    noslop_in(repo_path)
        .args(["agent", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("agent-a"))
        .stdout(predicate::str::contains("agent-b"));
}

/// Test listing agents in JSON format
#[test]
fn test_agent_list_json() {
    let temp_dir = setup_repo();
    let repo_path = temp_dir.path();

    // Spawn an agent
    noslop_in(repo_path)
        .args(["agent", "spawn", "--name", "json-agent", "--idle", "--no-tmux"])
        .assert()
        .success();

    // List in JSON
    noslop_in(repo_path)
        .args(["--json", "agent", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"name\""))
        .stdout(predicate::str::contains("\"json-agent\""))
        .stdout(predicate::str::contains("\"agents\""));
}

// =============================================================================
// AGENT KILL TESTS
// =============================================================================

/// Test killing an agent removes worktree
#[test]
fn test_agent_kill_removes_worktree() {
    let temp_dir = setup_repo();
    let repo_path = temp_dir.path();

    // Spawn an agent
    noslop_in(repo_path)
        .args(["agent", "spawn", "--name", "kill-test", "--idle", "--no-tmux"])
        .assert()
        .success();

    let agent_path = repo_path.join(".noslop/agents/kill-test");
    assert!(agent_path.exists(), "Agent should exist before kill");

    // Kill the agent
    noslop_in(repo_path)
        .args(["agent", "kill", "kill-test"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Agent killed: kill-test"));

    // Worktree should be gone
    assert!(!agent_path.exists(), "Agent worktree should be removed after kill");
}

/// Test killing an agent releases claimed tasks
#[test]
fn test_agent_kill_releases_task() {
    let temp_dir = setup_repo();
    let repo_path = temp_dir.path();

    // Add a pending task
    noslop_in(repo_path)
        .args(["task", "add", "Task for killing agent"])
        .assert()
        .success();

    // Manually make it pending and claimed
    fs::write(
        repo_path.join(".noslop/refs/tasks/TSK-1"),
        r#"{"title":"Task for killing agent","status":"pending","priority":"p2","created_at":"2025-01-01T00:00:00Z","blocked_by":[],"branch":null,"topics":[],"claimed_by":"release-agent"}"#,
    )
    .unwrap();

    // Create the agent directory manually (simulating a spawned agent)
    let agent_path = repo_path.join(".noslop/agents/release-agent");
    fs::create_dir_all(&agent_path).unwrap();

    // Kill the agent
    noslop_in(repo_path)
        .args(["agent", "kill", "release-agent", "--force"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Released task: TSK-1"));

    // Task should be unclaimed (claimed_by absent or null means released)
    let task_content = fs::read_to_string(repo_path.join(".noslop/refs/tasks/TSK-1")).unwrap();
    assert!(
        !task_content.contains("\"claimed_by\":\"release-agent\""),
        "Task should no longer be claimed by release-agent. Got: {}",
        task_content
    );
}

/// Test killing non-existent agent
#[test]
fn test_agent_kill_nonexistent() {
    let temp_dir = setup_repo();
    let repo_path = temp_dir.path();

    // Try to kill non-existent agent - should still succeed (idempotent)
    noslop_in(repo_path)
        .args(["agent", "kill", "ghost-agent"])
        .assert()
        .success();
}

// =============================================================================
// AGENT STATUS TESTS
// =============================================================================

/// Test agent status in main worktree
#[test]
fn test_agent_status_main() {
    let temp_dir = setup_repo();
    let repo_path = temp_dir.path();

    noslop_in(repo_path)
        .args(["agent", "status"])
        .assert()
        .success()
        .stdout(predicate::str::contains("main"));
}

/// Test agent status JSON output
#[test]
fn test_agent_status_json() {
    let temp_dir = setup_repo();
    let repo_path = temp_dir.path();

    noslop_in(repo_path)
        .args(["--json", "agent", "status"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"is_main\""));
}

// =============================================================================
// AGENT ASSIGN TESTS
// =============================================================================

/// Test assigning a task to an agent
#[test]
fn test_agent_assign_task() {
    let temp_dir = setup_repo();
    let repo_path = temp_dir.path();

    // Spawn an idle agent
    noslop_in(repo_path)
        .args(["agent", "spawn", "--name", "assign-agent", "--idle", "--no-tmux"])
        .assert()
        .success();

    // Add a task
    noslop_in(repo_path)
        .args(["task", "add", "Task to assign"])
        .assert()
        .success();

    // Assign task to agent
    noslop_in(repo_path)
        .args(["agent", "assign", "assign-agent", "TSK-1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Assigned task TSK-1 to agent assign-agent"));

    // Verify task is claimed
    let task_content = fs::read_to_string(repo_path.join(".noslop/refs/tasks/TSK-1")).unwrap();
    assert!(task_content.contains("assign-agent"), "Task should be claimed by assign-agent");

    // Verify HEAD in agent's workspace
    let agent_head = repo_path.join(".noslop/agents/assign-agent/.noslop/HEAD");
    let head_content = fs::read_to_string(&agent_head).unwrap();
    assert!(head_content.contains("TSK-1"), "Agent HEAD should point to TSK-1");
}

/// Test assigning already-claimed task fails
#[test]
fn test_agent_assign_claimed_task_fails() {
    let temp_dir = setup_repo();
    let repo_path = temp_dir.path();

    // Spawn two agents
    noslop_in(repo_path)
        .args(["agent", "spawn", "--name", "agent-x", "--idle", "--no-tmux"])
        .assert()
        .success();

    noslop_in(repo_path)
        .args(["agent", "spawn", "--name", "agent-y", "--idle", "--no-tmux"])
        .assert()
        .success();

    // Add a task
    noslop_in(repo_path)
        .args(["task", "add", "Contested task"])
        .assert()
        .success();

    // Assign to agent-x
    noslop_in(repo_path)
        .args(["agent", "assign", "agent-x", "TSK-1"])
        .assert()
        .success();

    // Try to assign to agent-y - should fail
    noslop_in(repo_path)
        .args(["agent", "assign", "agent-y", "TSK-1"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("already claimed"));
}

/// Test assigning to non-existent agent fails
#[test]
fn test_agent_assign_nonexistent_agent_fails() {
    let temp_dir = setup_repo();
    let repo_path = temp_dir.path();

    // Add a task
    noslop_in(repo_path)
        .args(["task", "add", "Orphan task"])
        .assert()
        .success();

    // Try to assign to non-existent agent
    noslop_in(repo_path)
        .args(["agent", "assign", "ghost-agent", "TSK-1"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("does not exist"));
}

// =============================================================================
// TASK CLAIMING INTEGRATION TESTS
// =============================================================================

/// Test that next_pending_unblocked skips claimed tasks
#[test]
fn test_next_task_skips_claimed() {
    let temp_dir = setup_repo();
    let repo_path = temp_dir.path();

    // Add two tasks
    noslop_in(repo_path)
        .args(["task", "add", "High priority task", "-p", "p0"])
        .assert()
        .success();

    noslop_in(repo_path)
        .args(["task", "add", "Lower priority task", "-p", "p1"])
        .assert()
        .success();

    // Make TSK-1 pending and claimed by writing proper JSON
    fs::write(
        repo_path.join(".noslop/refs/tasks/TSK-1"),
        r#"{"title":"High priority task","status":"pending","priority":"p0","created_at":"2025-01-01T00:00:00Z","blocked_by":[],"topics":[],"claimed_by":"other-agent"}"#,
    )
    .unwrap();

    // Make TSK-2 pending and unclaimed
    fs::write(
        repo_path.join(".noslop/refs/tasks/TSK-2"),
        r#"{"title":"Lower priority task","status":"pending","priority":"p1","created_at":"2025-01-01T00:00:00Z","blocked_by":[],"topics":[],"claimed_by":null}"#,
    )
    .unwrap();

    // Next task should be TSK-2 (TSK-1 is claimed)
    noslop_in(repo_path)
        .args(["task", "next"])
        .assert()
        .success()
        .stdout(predicate::str::contains("TSK-2"));
}

/// Test spawning multiple agents
#[test]
fn test_agent_spawn_count() {
    let temp_dir = setup_repo();
    let repo_path = temp_dir.path();

    // Spawn 3 agents
    noslop_in(repo_path)
        .args(["agent", "spawn", "--count", "3", "--idle", "--no-tmux"])
        .assert()
        .success();

    // Verify all 3 exist
    for i in 1..=3 {
        let agent_path = repo_path.join(format!(".noslop/agents/agent-{}", i));
        assert!(agent_path.exists(), "Agent {} should exist", i);
    }

    // List should show all 3
    noslop_in(repo_path)
        .args(["agent", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("agent-1"))
        .stdout(predicate::str::contains("agent-2"))
        .stdout(predicate::str::contains("agent-3"));
}
