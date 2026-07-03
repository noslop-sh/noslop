//! Integration tests for noslop CLI

use assert_cmd::cargo;
use predicates::prelude::*;
use tempfile::TempDir;

fn noslop() -> assert_cmd::Command {
    assert_cmd::Command::new(cargo::cargo_bin!("noslop"))
}

#[test]
fn test_version() {
    noslop()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("noslop"));
}

#[test]
fn test_help() {
    noslop()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Checks declare what must be reviewed"));
}

#[test]
fn test_no_args_shows_info() {
    noslop().assert().success().stdout(predicate::str::contains("noslop"));
}

#[test]
fn test_init_creates_noslop_toml() {
    let temp = TempDir::new().unwrap();

    // Initialize a git repo first
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    noslop()
        .arg("init")
        .current_dir(temp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Created .noslop.toml"));

    assert!(temp.path().join(".noslop.toml").exists());
}

#[test]
fn test_check_add_creates_check() {
    let temp = TempDir::new().unwrap();

    // Initialize git repo
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    // Initialize noslop
    noslop().arg("init").current_dir(temp.path()).assert().success();

    // Add check
    noslop()
        .args(["check", "add", "*.rs", "-m", "Test check"])
        .current_dir(temp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Added check"));

    // Verify .noslop.toml contains the check
    let content = std::fs::read_to_string(temp.path().join(".noslop.toml")).unwrap();
    assert!(content.contains("Test check"));
}

#[test]
fn test_check_list_shows_checks() {
    let temp = TempDir::new().unwrap();

    // Initialize git repo
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    // Create .noslop.toml with a check
    std::fs::write(
        temp.path().join(".noslop.toml"),
        r#"
[[check]]
target = "*.rs"
message = "Check this thing"
severity = "block"
"#,
    )
    .unwrap();

    noslop()
        .args(["check", "list"])
        .current_dir(temp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Check this thing"));
}

#[test]
fn test_check_with_no_staged_files() {
    let temp = TempDir::new().unwrap();

    // Initialize git repo
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    // Create .noslop.toml
    std::fs::write(temp.path().join(".noslop.toml"), "").unwrap();

    noslop()
        .arg("check")
        .current_dir(temp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("No staged changes"));
}

#[test]
fn test_ack_stages_acknowledgment() {
    let temp = TempDir::new().unwrap();

    // Initialize git repo
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    // Create .noslop directory and a check to acknowledge
    std::fs::create_dir_all(temp.path().join(".noslop")).unwrap();
    std::fs::write(
        temp.path().join(".noslop.toml"),
        "[[check]]\nid = \"test-check\"\ntarget = \"*.rs\"\nmessage = \"Check it\"\nseverity = \"block\"\n",
    )
    .unwrap();

    // Acking an unknown ID fails with guidance
    noslop()
        .args(["ack", "nonexistent", "-m", "I checked it"])
        .current_dir(temp.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("No check with ID"))
        .stderr(predicate::str::contains("test-check"));

    noslop()
        .args(["ack", "test-check", "-m", "I checked it"])
        .current_dir(temp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Staged acknowledgment"));

    // Verify staged acks file exists
    assert!(temp.path().join(".noslop/staged-acks.json").exists());
}

#[test]
fn test_check_list_with_no_checks() {
    let temp = TempDir::new().unwrap();

    // Initialize git repo
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    // Create empty .noslop.toml
    std::fs::write(temp.path().join(".noslop.toml"), "[project]\nprefix = \"TST\"\n").unwrap();

    noslop()
        .args(["check", "list"])
        .current_dir(temp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("No checks defined"));
}

#[test]
fn test_check_list_with_no_noslop_files() {
    let temp = TempDir::new().unwrap();

    noslop()
        .args(["check", "list"])
        .current_dir(temp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("No .noslop.toml files found"));
}

#[test]
fn test_check_remove() {
    let temp = TempDir::new().unwrap();

    // Initialize git repo
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    // Initialize noslop
    noslop().arg("init").current_dir(temp.path()).assert().success();

    // Add two checks
    noslop()
        .args(["check", "add", "*.rs", "-m", "First check"])
        .current_dir(temp.path())
        .assert()
        .success();

    noslop()
        .args(["check", "add", "*.ts", "-m", "Second check"])
        .current_dir(temp.path())
        .assert()
        .success();

    // List should show both
    noslop()
        .args(["check", "list"])
        .current_dir(temp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("2 check(s) found"));

    // Remove the first check - need to use the correct ID format
    let content = std::fs::read_to_string(temp.path().join(".noslop.toml")).unwrap();
    assert!(content.contains("First check"));

    noslop()
        .args(["check", "remove", ".noslop.toml:0"])
        .current_dir(temp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Removed check"));

    // Verify first check is gone
    let content = std::fs::read_to_string(temp.path().join(".noslop.toml")).unwrap();
    assert!(!content.contains("First check"));
    assert!(content.contains("Second check"));
}

#[test]
fn test_version_command() {
    noslop()
        .arg("version")
        .assert()
        .success()
        .stdout(predicate::str::contains("noslop v"));
}

#[test]
fn test_json_output_version() {
    noslop()
        .args(["--json", "version"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"version\""));
}

#[test]
fn test_json_output_no_args() {
    noslop()
        .arg("--json")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"version\""))
        .stdout(predicate::str::contains("\"hint\""));
}

#[test]
fn test_check_add_with_severity() {
    let temp = TempDir::new().unwrap();

    // Initialize git repo
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    // Initialize noslop
    noslop().arg("init").current_dir(temp.path()).assert().success();

    // Add check with block severity
    noslop()
        .args(["check", "add", "*.rs", "-m", "Test check", "--severity", "block"])
        .current_dir(temp.path())
        .assert()
        .success();

    // Verify severity in config
    let content = std::fs::read_to_string(temp.path().join(".noslop.toml")).unwrap();
    assert!(content.contains("block"));
}

#[test]
fn test_init_force_reinitialize() {
    let temp = TempDir::new().unwrap();

    // Initialize git repo
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    // Initialize once
    noslop().arg("init").current_dir(temp.path()).assert().success();

    // Re-init without force keeps the config
    noslop()
        .arg("init")
        .current_dir(temp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Kept existing .noslop.toml"));

    // Initialize with force
    noslop()
        .args(["init", "--force"])
        .current_dir(temp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Created .noslop.toml"));
}

#[test]
fn test_init_fresh_clone_installs_hooks() {
    // Regression (NO-11): a fresh clone already has .noslop.toml (tracked)
    // but no hooks (per-clone). init must install hooks anyway.
    let temp = TempDir::new().unwrap();

    std::process::Command::new("git")
        .args(["init"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    // Simulate the fresh clone: tracked config exists, hooks don't
    std::fs::write(
        temp.path().join(".noslop.toml"),
        "[project]\nprefix = \"TST\"\n\n[[check]]\nid = \"TST-1\"\ntarget = \"*.rs\"\nmessage = \"Existing team check\"\nseverity = \"block\"\n",
    )
    .unwrap();
    assert!(!temp.path().join(".git/hooks/pre-commit").exists());

    noslop()
        .arg("init")
        .current_dir(temp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Kept existing .noslop.toml"))
        .stdout(predicate::str::contains("Installed pre-commit hook"));

    // Hooks installed, team config untouched
    for hook in ["pre-commit", "commit-msg", "post-commit"] {
        let path = temp.path().join(".git/hooks").join(hook);
        assert!(path.exists(), "{hook} hook must exist after init on fresh clone");
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("noslop"), "{hook} hook must invoke noslop");
    }
    let config = std::fs::read_to_string(temp.path().join(".noslop.toml")).unwrap();
    assert!(config.contains("Existing team check"));
}

#[test]
fn test_discover_imports_rules_and_review_accepts() {
    let temp = TempDir::new().unwrap();

    std::process::Command::new("git")
        .args(["init"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    std::fs::write(
        temp.path().join("CLAUDE.md"),
        "# Conventions\n\n- Never commit directly to main\n- Run `make check` before every commit\n",
    )
    .unwrap();

    // Fake agent CLI: consumes stdin, emits two decomposed checks
    let script = temp.path().join("fake-agent.sh");
    std::fs::write(
        &script,
        concat!(
            "#!/bin/sh\ncat > /dev/null\n",
            "echo '[[check]]'\n",
            "echo 'target = \"**/*\"'\n",
            "echo 'message = \"Never commit directly to main\"'\n",
            "echo 'severity = \"block\"'\n",
            "echo 'source = \"CLAUDE.md\"'\n",
            "echo '[[check]]'\n",
            "echo 'target = \"**/*.rs\"'\n",
            "echo 'message = \"Did make check pass before this commit?\"'\n",
            "echo 'severity = \"warn\"'\n",
            "echo 'source = \"CLAUDE.md\"'\n",
        ),
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    std::fs::write(
        temp.path().join(".noslop.toml"),
        "[project]\nprefix = \"TST\"\n\n[discover]\nrunner = \"./fake-agent.sh\"\n",
    )
    .unwrap();

    // Import decomposes rules files through the runner
    noslop()
        .arg("discover")
        .current_dir(temp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("2 new proposal(s)"));
    assert!(temp.path().join(".noslop/proposals.toml").exists());

    // Model-provided source survives into the proposal
    let staged = std::fs::read_to_string(temp.path().join(".noslop/proposals.toml")).unwrap();
    assert!(staged.contains("source = \"CLAUDE.md\""));

    // Re-scan is idempotent
    noslop()
        .arg("discover")
        .current_dir(temp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("0 new proposal(s)"));

    // Review: accept first, reject second
    noslop()
        .args(["discover", "--review"])
        .current_dir(temp.path())
        .write_stdin("a\nr\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("1 accepted, 1 rejected"));

    // Accepted proposal became a check; proposals file is gone
    let toml = std::fs::read_to_string(temp.path().join(".noslop.toml")).unwrap();
    assert!(toml.contains("Never commit directly to main"));
    assert!(!temp.path().join(".noslop/proposals.toml").exists());

    // Accepted and rejected proposals both dedupe against future scans:
    // the accepted one is now a check, the rejected one is in rejected-keys
    noslop()
        .arg("discover")
        .current_dir(temp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("0 new proposal(s)"));
    assert!(temp.path().join(".noslop/rejected-keys.txt").exists());
}

#[test]
fn test_discover_mine_from_file_with_fake_runner() {
    let temp = TempDir::new().unwrap();

    std::process::Command::new("git")
        .args(["init"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    // Fake agent CLI: consumes stdin, emits one valid check
    let script = temp.path().join("fake-agent.sh");
    std::fs::write(
        &script,
        "#!/bin/sh\ncat > /dev/null\necho '[[check]]'\necho 'target = \"api/**/*.py\"'\necho 'message = \"Rate limiting decorator added to public routes?\"'\necho 'severity = \"block\"'\n",
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    }

    std::fs::write(
        temp.path().join(".noslop.toml"),
        "[project]\nprefix = \"TST\"\n\n[discover]\nrunner = \"./fake-agent.sh\"\n",
    )
    .unwrap();

    std::fs::write(
        temp.path().join("comments.jsonl"),
        "{\"path\": \"api/routes.py\", \"body\": \"needs rate limiting\"}\n{\"path\": \"api/users.py\", \"body\": \"rate limit missing again\"}\n",
    )
    .unwrap();

    // Mine from the JSONL export through the fake runner
    noslop()
        .args(["discover", "--from-file", "comments.jsonl"])
        .current_dir(temp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("1 new proposal(s) mined"));

    // Accept it; the check lands with a TST id and runner config survives
    noslop()
        .args(["discover", "--review"])
        .current_dir(temp.path())
        .write_stdin("a\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("Added as TST-1"));

    let toml = std::fs::read_to_string(temp.path().join(".noslop.toml")).unwrap();
    assert!(toml.contains("Rate limiting decorator"));
    assert!(
        toml.contains("runner = \"./fake-agent.sh\""),
        "discover config must survive rewrites"
    );
}

#[test]
fn test_stats_tracks_fires_acks_and_rubber_stamps() {
    let temp = TempDir::new().unwrap();

    std::process::Command::new("git")
        .args(["init"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    std::fs::write(
        temp.path().join(".noslop.toml"),
        "[project]\nprefix = \"TST\"\n\n[[check]]\nid = \"TST-1\"\ntarget = \"*.rs\"\nmessage = \"Reviewed the rust change?\"\nseverity = \"block\"\n\n[[check]]\nid = \"TST-2\"\ntarget = \"ghost/**/*.py\"\nmessage = \"Dead check\"\nseverity = \"warn\"\n",
    )
    .unwrap();

    // Stage a matching file; check fires (agent actor) and logs an event
    std::fs::write(temp.path().join("lib.rs"), "fn main() {}\n").unwrap();
    std::process::Command::new("git")
        .args(["add", "lib.rs"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    noslop()
        .arg("check")
        .env("NOSLOP_ACTOR", "claude-code")
        .current_dir(temp.path())
        .assert()
        .failure();
    assert!(temp.path().join(".noslop/events.jsonl").exists());

    // Ack WITHOUT changing anything: a rubber stamp
    noslop()
        .args(["ack", "TST-1", "-m", "looks fine"])
        .env("NOSLOP_ACTOR", "claude-code")
        .current_dir(temp.path())
        .assert()
        .success();

    // Stats: 1 fire, 1 ack, 1 stamp, 0 self-fix; TST-2 flagged dead
    let out = noslop()
        .arg("stats")
        .current_dir(temp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("ALWAYS-STAMPED"))
        .stdout(predicate::str::contains("DEAD-TARGET"));
    let stdout = String::from_utf8_lossy(&out.get_output().stdout).to_string();
    let tst1 = stdout.lines().find(|l| l.starts_with("TST-1")).unwrap();
    assert!(tst1.contains(" 1"), "expected fire/ack/stamp counts in: {tst1}");

    // JSON mode carries the same data
    noslop()
        .args(["stats", "--json"])
        .current_dir(temp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("\"rubber_stamps\":1"));
}
