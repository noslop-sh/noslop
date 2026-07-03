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
fn test_ack_from_subdirectory_lands_in_repo_root_state() {
    let temp = TempDir::new().unwrap();
    std::process::Command::new("git")
        .args(["init", "-b", "main"])
        .current_dir(temp.path())
        .output()
        .unwrap();
    std::fs::write(
        temp.path().join(".noslop.toml"),
        "[[check]]\nid = \"TST-1\"\ntarget = \"*.rs\"\nmessage = \"Check it\"\nseverity = \"block\"\n",
    )
    .unwrap();
    let subdir = temp.path().join("apps/web");
    std::fs::create_dir_all(&subdir).unwrap();
    std::fs::write(subdir.join("lib.rs"), "fn main() {}\n").unwrap();
    std::process::Command::new("git")
        .args(["add", "-A"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    // Ack from deep inside the tree: state must land at the repo root, not
    // grow a stray .noslop/ under the cwd
    noslop()
        .args(["ack", "TST-1", "-m", "verified from a subdirectory"])
        .current_dir(&subdir)
        .assert()
        .success();

    assert!(temp.path().join(".noslop/staged-acks.json").exists());
    assert!(!subdir.join(".noslop").exists(), "stray .noslop/ created in subdirectory");

    // The gate run from the root reads the same store
    let out = noslop()
        .args(["--json", "check"])
        .env("NOSLOP_ACTOR", "claude-code")
        .current_dir(temp.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    let result: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let acked: Vec<&str> = result["acknowledged"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["id"].as_str().unwrap())
        .collect();
    assert!(acked.contains(&"TST-1"));
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

    // Fake agent CLI: dumps stdin (so tests can inspect the prompt),
    // emits two decomposed checks
    let script = temp.path().join("fake-agent.sh");
    let dump = temp.path().join("prompt-dump.txt");
    std::fs::write(
        &script,
        format!(
            concat!(
                "#!/bin/sh\ncat > {}\n",
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
            dump.display()
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

    // NO-13: the rejected rule's full text is stored and fed into the next
    // scan's prompt as a do-not-re-propose guard (defeats paraphrasing)
    let rules = std::fs::read_to_string(temp.path().join(".noslop/rejected-rules.txt")).unwrap();
    assert!(rules.contains("Did make check pass before this commit?"));
    let prompt = std::fs::read_to_string(temp.path().join("prompt-dump.txt")).unwrap();
    assert!(prompt.contains("already REJECTED"));
    assert!(prompt.contains("Did make check pass before this commit?"));
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

#[test]
fn test_check_diff_base_reconciles_ledger_not_local_state() {
    let temp = TempDir::new().unwrap();
    let git = |args: &[&str]| {
        std::process::Command::new("git")
            .args(args)
            .current_dir(temp.path())
            .env("GIT_AUTHOR_NAME", "t")
            .env("GIT_AUTHOR_EMAIL", "t@t")
            .env("GIT_COMMITTER_NAME", "t")
            .env("GIT_COMMITTER_EMAIL", "t@t")
            .output()
            .unwrap()
    };
    git(&["init", "-b", "main"]);

    std::fs::write(
        temp.path().join(".noslop.toml"),
        "[project]\nprefix = \"TST\"\n\n[[check]]\nid = \"TST-1\"\ntarget = \"*.rs\"\nmessage = \"Reviewed?\"\nseverity = \"block\"\n",
    )
    .unwrap();
    git(&["add", "-A"]);
    git(&["commit", "-m", "base"]);

    // Branch commits a matching file with NO ledger record (--no-verify style)
    git(&["checkout", "-b", "feature"]);
    std::fs::write(temp.path().join("lib.rs"), "fn main() {}\n").unwrap();
    git(&["add", "lib.rs"]);
    git(&["commit", "-m", "change without ack"]);

    noslop()
        .args(["check", "--ci", "--diff-base", "main"])
        .current_dir(temp.path())
        .assert()
        .failure()
        .stdout(predicate::str::contains("TST-1"));

    // Ack writes a ledger record; commit it — now the branch carries proof
    noslop()
        .args(["ack", "TST-1", "-m", "reviewed the rust change"])
        .env("NOSLOP_ACTOR", "claude-code")
        .current_dir(temp.path())
        .assert()
        .success();
    git(&["add", "-A"]);
    git(&["commit", "-m", "ack"]);

    noslop()
        .args(["check", "--ci", "--diff-base", "main"])
        .current_dir(temp.path())
        .assert()
        .success();
}

#[test]
fn test_envelope_wraps_check_with_touched_ledger_records() {
    let temp = TempDir::new().unwrap();
    let git = |args: &[&str]| {
        std::process::Command::new("git")
            .args(args)
            .current_dir(temp.path())
            .env("GIT_AUTHOR_NAME", "t")
            .env("GIT_AUTHOR_EMAIL", "t@t")
            .env("GIT_COMMITTER_NAME", "t")
            .env("GIT_COMMITTER_EMAIL", "t@t")
            .output()
            .unwrap()
    };
    git(&["init", "-b", "main"]);

    std::fs::write(
        temp.path().join(".noslop.toml"),
        "[project]\nprefix = \"TST\"\n\n[[check]]\nid = \"TST-1\"\ntarget = \"*.rs\"\nmessage = \"Reviewed?\"\nseverity = \"block\"\n\n[[check]]\nid = \"TST-2\"\ntarget = \"*.md\"\nmessage = \"Docs current?\"\nseverity = \"block\"\n",
    )
    .unwrap();
    git(&["add", "-A"]);
    git(&["commit", "-m", "base"]);

    // Branch touches only the .rs check; ack both so the ledger holds a
    // record the run touched (TST-1) and one it did not (TST-2)
    git(&["checkout", "-b", "feature"]);
    std::fs::write(temp.path().join("lib.rs"), "fn main() {}\n").unwrap();
    git(&["add", "lib.rs"]);
    for (id, msg) in [("TST-1", "verified the rust change"), ("TST-2", "unrelated ack")] {
        noslop()
            .args(["ack", id, "-m", msg])
            .env("NOSLOP_ACTOR", "claude-code")
            .current_dir(temp.path())
            .assert()
            .success();
    }
    git(&["add", "-A"]);
    git(&["commit", "-m", "change with acks"]);

    // The Action flow: check --json to a file, then envelope wraps it
    let check_out = noslop()
        .args(["--json", "check", "--ci", "--diff-base", "main"])
        .current_dir(temp.path())
        .output()
        .unwrap();
    std::fs::write(temp.path().join("noslop-check.json"), &check_out.stdout).unwrap();

    let envelope_out = noslop()
        .args([
            "envelope",
            "--check",
            "noslop-check.json",
            "--repo",
            "acme/api",
            "--sha",
            "abc123",
            "--base",
            "main",
        ])
        .current_dir(temp.path())
        .output()
        .unwrap();
    assert!(envelope_out.status.success());

    let envelope: serde_json::Value = serde_json::from_slice(&envelope_out.stdout).unwrap();
    assert_eq!(envelope["schema"], 1);
    assert_eq!(envelope["repo"], "acme/api");
    assert_eq!(envelope["pr"], ""); // defaulted: non-PR run

    // check payload embedded verbatim, including the gate-time tree oid
    assert_eq!(envelope["check"]["passed"], true);
    assert!(envelope["check"]["tree_oid"].is_string());

    // Ledger carries the touched record with its justification and ack
    // tree oid; the untouched TST-2 record never leaves the repository
    let ledger = envelope["ledger"].as_array().unwrap();
    assert_eq!(ledger.len(), 1);
    assert_eq!(ledger[0]["check_id"], "TST-1");
    assert_eq!(ledger[0]["message"], "verified the rust change");
    assert_eq!(ledger[0]["acknowledged_by"], "claude-code");
    assert!(ledger[0]["tree_oid"].is_string());
    assert!(ledger[0]["created_at"].is_string());
}

#[test]
fn test_envelope_rejects_non_check_payload() {
    let temp = TempDir::new().unwrap();
    std::fs::write(temp.path().join("bogus.json"), "[1,2,3]").unwrap();

    noslop()
        .args(["envelope", "--check", "bogus.json", "--repo", "r", "--sha", "s", "--base", "m"])
        .current_dir(temp.path())
        .assert()
        .failure();
}

#[test]
fn test_ack_embeds_fire_evidence_from_local_events() {
    let temp = TempDir::new().unwrap();
    let git = |args: &[&str]| {
        std::process::Command::new("git")
            .args(args)
            .current_dir(temp.path())
            .env("GIT_AUTHOR_NAME", "t")
            .env("GIT_AUTHOR_EMAIL", "t@t")
            .env("GIT_COMMITTER_NAME", "t")
            .env("GIT_COMMITTER_EMAIL", "t@t")
            .output()
            .unwrap()
    };
    git(&["init", "-b", "main"]);

    std::fs::write(
        temp.path().join(".noslop.toml"),
        "[project]\nprefix = \"TST\"\n\n[[check]]\nid = \"TST-1\"\ntarget = \"*.rs\"\nmessage = \"Reviewed?\"\nseverity = \"block\"\n",
    )
    .unwrap();
    std::fs::write(temp.path().join("lib.rs"), "fn main() {}\n").unwrap();
    git(&["add", "-A"]);

    // The gate fires TST-1 and logs a local fire event (agent actor, local mode)
    noslop()
        .args(["check"])
        .env("NOSLOP_ACTOR", "claude-code")
        .current_dir(temp.path())
        .assert()
        .failure();
    assert!(temp.path().join(".noslop/events.jsonl").exists());

    // The ack copies the fire's tree oid + timestamp into the ledger record
    noslop()
        .args(["ack", "TST-1", "-m", "verified"])
        .env("NOSLOP_ACTOR", "claude-code")
        .current_dir(temp.path())
        .assert()
        .success();

    let acks_dir = temp.path().join(".noslop/acks");
    let record_path = std::fs::read_dir(&acks_dir).unwrap().next().unwrap().unwrap().path();
    let record: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(record_path).unwrap()).unwrap();
    assert!(record["fire_tree_oid"].is_string());
    assert!(record["fired_at"].is_string());
    // Nothing changed between fire and ack: the trees match (a rubber stamp)
    assert_eq!(record["fire_tree_oid"], record["tree_oid"]);
}

#[test]
fn test_ack_without_prior_fire_omits_fire_fields() {
    let temp = TempDir::new().unwrap();
    std::process::Command::new("git")
        .args(["init", "-b", "main"])
        .current_dir(temp.path())
        .output()
        .unwrap();
    std::fs::write(
        temp.path().join(".noslop.toml"),
        "[project]\nprefix = \"TST\"\n\n[[check]]\nid = \"TST-1\"\ntarget = \"*.rs\"\nmessage = \"Reviewed?\"\nseverity = \"block\"\n",
    )
    .unwrap();

    noslop()
        .args(["ack", "TST-1", "-m", "acked before any check ran"])
        .env("NOSLOP_ACTOR", "claude-code")
        .current_dir(temp.path())
        .assert()
        .success();

    let acks_dir = temp.path().join(".noslop/acks");
    let record_path = std::fs::read_dir(&acks_dir).unwrap().next().unwrap().unwrap().path();
    let record: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(record_path).unwrap()).unwrap();
    // Additive optional fields: absent, not null, when no fire event exists
    assert!(record.get("fire_tree_oid").is_none());
    assert!(record.get("fired_at").is_none());
}

/// Minimal one-shot HTTP server: answers every request with the given JSON.
fn serve_check_set(body: &'static str) -> String {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    std::thread::spawn(move || {
        // Serve a handful of requests, then let the thread die with the test
        for stream in listener.incoming().take(4) {
            let Ok(mut stream) = stream else { break };
            use std::io::{Read, Write};
            let mut buf = [0u8; 2048];
            let _ = stream.read(&mut buf);
            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            let _ = stream.write_all(response.as_bytes());
        }
    });
    format!("http://{addr}")
}

#[test]
fn test_remote_checks_gate_and_monitor_stays_silent() {
    let url = serve_check_set(
        r#"{"check_set_version":"v-test-1","checks":[
            {"id":"ORG-1","target":"*.rs","message":"org: reviewed?","severity":"block","state":"enforce","owner":"saumil","bypasses":[]},
            {"id":"ORG-2","target":"*.rs","message":"org: trial check","severity":"block","state":"monitor","owner":"saumil","bypasses":[]}
        ]}"#,
    );

    let temp = TempDir::new().unwrap();
    let git = |args: &[&str]| {
        std::process::Command::new("git")
            .args(args)
            .current_dir(temp.path())
            .env("GIT_AUTHOR_NAME", "t")
            .env("GIT_AUTHOR_EMAIL", "t@t")
            .env("GIT_COMMITTER_NAME", "t")
            .env("GIT_COMMITTER_EMAIL", "t@t")
            .output()
            .unwrap()
    };
    git(&["init", "-b", "main"]);
    std::fs::write(
        temp.path().join(".noslop.toml"),
        format!("[project]\nprefix = \"TST\"\n\n[remote]\nurl = \"{url}\"\n"),
    )
    .unwrap();
    std::fs::write(temp.path().join("lib.rs"), "fn main() {}\n").unwrap();
    git(&["add", "-A"]);

    let out = noslop()
        .args(["--json", "check"])
        .env("NOSLOP_ACTOR", "claude-code")
        .env("NOSLOP_CLOUD_TOKEN", "nslp_test")
        .current_dir(temp.path())
        .output()
        .unwrap();
    // The enforce-state org check blocks the agent
    assert!(!out.status.success());

    let result: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(result["check_set_version"], "v-test-1");
    // Fetched this run: the set is fresh
    assert_eq!(result["check_set_age_seconds"], 0);
    let blocking: Vec<&str> = result["blocking"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| b["id"].as_str().unwrap())
        .collect();
    assert!(blocking.contains(&"ORG-1"));
    // The monitor-state check never blocks and never joins blocking output;
    // it rides the monitor array for promotion telemetry
    assert!(!blocking.contains(&"ORG-2"));
    let monitor: Vec<&str> = result["monitor"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| b["id"].as_str().unwrap())
        .collect();
    assert_eq!(monitor, ["ORG-2"]);
}

#[test]
fn test_remote_fetch_failure_fails_open_to_local_checks() {
    let temp = TempDir::new().unwrap();
    std::process::Command::new("git")
        .args(["init", "-b", "main"])
        .current_dir(temp.path())
        .output()
        .unwrap();
    // Unroutable remote: the gate must degrade to local checks, not error
    std::fs::write(
        temp.path().join(".noslop.toml"),
        "[project]\nprefix = \"TST\"\n\n[remote]\nurl = \"http://127.0.0.1:1\"\n\n[[check]]\nid = \"TST-1\"\ntarget = \"*.rs\"\nmessage = \"local ask\"\nseverity = \"block\"\n",
    )
    .unwrap();
    std::fs::write(temp.path().join("lib.rs"), "fn main() {}\n").unwrap();
    std::process::Command::new("git")
        .args(["add", "-A"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    let out = noslop()
        .args(["--json", "check"])
        .env("NOSLOP_ACTOR", "claude-code")
        .env("NOSLOP_CLOUD_TOKEN", "nslp_test")
        .current_dir(temp.path())
        .output()
        .unwrap();
    // Local check still gates; the remote outage is a stderr warning only
    assert!(!out.status.success());
    let result: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(result["blocking"][0]["id"], "TST-1");
    assert!(result.get("check_set_version").is_none());
    assert!(result.get("check_set_age_seconds").is_none());
    assert!(String::from_utf8_lossy(&out.stderr).contains("using local checks only"));
}

#[test]
fn test_remote_fetch_failure_falls_back_to_stale_cache_with_age() {
    let temp = TempDir::new().unwrap();
    std::process::Command::new("git")
        .args(["init", "-b", "main"])
        .current_dir(temp.path())
        .output()
        .unwrap();
    std::fs::write(
        temp.path().join(".noslop.toml"),
        "[project]\nprefix = \"TST\"\n\n[remote]\nurl = \"http://127.0.0.1:1\"\n",
    )
    .unwrap();
    // A cache from an hour ago: the unroutable fetch must degrade to it and
    // report honestly how stale the rules were
    let hour_ago = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        - 3600;
    std::fs::create_dir_all(temp.path().join(".noslop")).unwrap();
    std::fs::write(
        temp.path().join(".noslop/remote-checks.json"),
        format!(
            r#"{{"fetched_at":{hour_ago},"set":{{"check_set_version":"v-stale","checks":[{{"id":"ORG-1","target":"*.rs","message":"org: reviewed?","severity":"block","state":"enforce","owner":"saumil","bypasses":[]}}]}}}}"#
        ),
    )
    .unwrap();
    std::fs::write(temp.path().join("lib.rs"), "fn main() {}\n").unwrap();
    std::process::Command::new("git")
        .args(["add", "-A"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    let out = noslop()
        .args(["--json", "check"])
        .env("NOSLOP_ACTOR", "claude-code")
        .env("NOSLOP_CLOUD_TOKEN", "nslp_test")
        .current_dir(temp.path())
        .output()
        .unwrap();
    // The cached enforce-state check still gates the agent
    assert!(!out.status.success());
    let result: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(result["check_set_version"], "v-stale");
    assert!(result["check_set_age_seconds"].as_u64().unwrap() >= 3600);
    assert!(String::from_utf8_lossy(&out.stderr).contains("using cached set"));
}
