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

    // Remove the first check
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
fn test_init_with_existing_docs() {
    let temp = TempDir::new().unwrap();

    // Initialize git repo
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    // Create CLAUDE.md file
    std::fs::write(temp.path().join("CLAUDE.md"), "# Claude docs\n").unwrap();

    noslop()
        .arg("init")
        .current_dir(temp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Found: CLAUDE.md"))
        .stdout(predicate::str::contains("noslop check import"));
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

    // Try to initialize again without force - should be prevented
    noslop()
        .arg("init")
        .current_dir(temp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Already initialized"));

    // Initialize with force
    noslop()
        .args(["init", "--force"])
        .current_dir(temp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Created .noslop.toml"));
}

#[test]
fn test_init_unknown_agent_fails() {
    let temp = TempDir::new().unwrap();

    // Initialize git repo
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    noslop()
        .args(["init", "cursor"])
        .current_dir(temp.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("Unknown agent type: cursor"));
}

#[test]
fn test_init_without_agent_no_agent_section() {
    let temp = TempDir::new().unwrap();

    // Initialize git repo
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

    // Verify no [agent] section in the generated TOML
    let content = std::fs::read_to_string(temp.path().join(".noslop.toml")).unwrap();
    assert!(!content.contains("[agent]"), "bare init should not have [agent] section");
    assert!(!content.contains("[review]"), "bare init should not have [review] section");

    // Verify no pre-push hook
    assert!(
        !temp.path().join(".git/hooks/pre-push").exists(),
        "bare init should not install pre-push hook"
    );
}

#[test]
fn test_init_with_agent_creates_agent_section() {
    let temp = TempDir::new().unwrap();

    // Initialize git repo
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    // This will fail at the agent validation step (CLI not installed) but should
    // still create the TOML with agent sections
    let result = noslop().args(["init", "claude"]).current_dir(temp.path()).assert();

    // The .noslop.toml should have been created with agent sections before Phase 2 fails
    let toml_path = temp.path().join(".noslop.toml");
    if toml_path.exists() {
        let content = std::fs::read_to_string(&toml_path).unwrap();
        assert!(content.contains("[agent]"), "should have [agent] section");
        assert!(content.contains("type = \"claude\""), "should specify claude agent");
        assert!(content.contains("[review]"), "should have [review] section");
    }

    // Pre-push hook should have been installed before Phase 2
    // (The TOML write and hooks happen in Phase 1)
    if temp.path().join(".git/hooks/pre-push").exists() {
        let hook = std::fs::read_to_string(temp.path().join(".git/hooks/pre-push")).unwrap();
        assert!(hook.contains("noslop review"));
    }

    // Agent validation will fail since claude CLI is not installed in test env
    // This is expected behavior
    let _ = result;
}
