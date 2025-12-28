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
        .stdout(predicate::str::contains("Assertions declare what must be reviewed"));
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
fn test_assert_add_creates_assertion() {
    let temp = TempDir::new().unwrap();

    // Initialize git repo
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    // Initialize noslop
    noslop().arg("init").current_dir(temp.path()).assert().success();

    // Add assertion
    noslop()
        .args(["assert", "add", "*.rs", "-m", "Test assertion"])
        .current_dir(temp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Added assertion"));

    // Verify .noslop.toml contains the assertion
    let content = std::fs::read_to_string(temp.path().join(".noslop.toml")).unwrap();
    assert!(content.contains("Test assertion"));
}

#[test]
fn test_assert_list_shows_assertions() {
    let temp = TempDir::new().unwrap();

    // Initialize git repo
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    // Create .noslop.toml with an assertion
    std::fs::write(
        temp.path().join(".noslop.toml"),
        r#"
[[assert]]
target = "*.rs"
message = "Check this thing"
severity = "block"
"#,
    )
    .unwrap();

    noslop()
        .args(["assert", "list"])
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
fn test_attest_stages_attestation() {
    let temp = TempDir::new().unwrap();

    // Initialize git repo
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    // Create .noslop directory
    std::fs::create_dir_all(temp.path().join(".noslop")).unwrap();

    noslop()
        .args(["attest", "test-assertion", "-m", "I checked it"])
        .current_dir(temp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Staged attestation"));

    // Verify staged attestations file exists
    assert!(temp.path().join(".noslop/staged-attestations.json").exists());
}

#[test]
fn test_assert_list_with_no_assertions() {
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
        .args(["assert", "list"])
        .current_dir(temp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("No assertions defined"));
}

#[test]
fn test_assert_list_with_no_noslop_files() {
    let temp = TempDir::new().unwrap();

    noslop()
        .args(["assert", "list"])
        .current_dir(temp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("No .noslop.toml files found"));
}

#[test]
fn test_assert_remove() {
    let temp = TempDir::new().unwrap();

    // Initialize git repo
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    // Initialize noslop
    noslop().arg("init").current_dir(temp.path()).assert().success();

    // Add two assertions
    noslop()
        .args(["assert", "add", "*.rs", "-m", "First assertion"])
        .current_dir(temp.path())
        .assert()
        .success();

    noslop()
        .args(["assert", "add", "*.ts", "-m", "Second assertion"])
        .current_dir(temp.path())
        .assert()
        .success();

    // List should show both
    noslop()
        .args(["assert", "list"])
        .current_dir(temp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("2 assertion(s) found"));

    // Remove the first assertion - need to use the correct ID format
    let content = std::fs::read_to_string(temp.path().join(".noslop.toml")).unwrap();
    assert!(content.contains("First assertion"));

    noslop()
        .args(["assert", "remove", ".noslop.toml:0"])
        .current_dir(temp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Removed assertion"));

    // Verify first assertion is gone
    let content = std::fs::read_to_string(temp.path().join(".noslop.toml")).unwrap();
    assert!(!content.contains("First assertion"));
    assert!(content.contains("Second assertion"));
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
fn test_assert_add_with_severity() {
    let temp = TempDir::new().unwrap();

    // Initialize git repo
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    // Initialize noslop
    noslop().arg("init").current_dir(temp.path()).assert().success();

    // Add assertion with block severity
    noslop()
        .args(["assert", "add", "*.rs", "-m", "Test assertion", "--severity", "block"])
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
        .stdout(predicate::str::contains("noslop assert import"));
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
