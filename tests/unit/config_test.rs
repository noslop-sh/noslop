//! Tests for global configuration management

use std::fs;
use std::path::Path;

use noslop::config::GlobalConfig;
use tempfile::TempDir;

// =============================================================================
// BASIC CONFIG TESTS
// =============================================================================

#[test]
fn test_config_default() {
    let config = GlobalConfig::default();
    assert_eq!(config.ui.theme, "dark");
    assert!(config.workspaces.is_empty());
}

#[test]
fn test_config_workspace_creation() {
    let mut config = GlobalConfig::default();
    let workspace = Path::new("/test/workspace");

    config.workspace_mut(workspace);
    assert!(config.workspaces.contains_key("/test/workspace"));
}

#[test]
fn test_config_add_repo() {
    let mut config = GlobalConfig::default();
    let workspace = Path::new("/test/workspace");
    let repo = Path::new("/test/workspace/repo");

    config.add_repo(workspace, repo);

    let ws = config.workspace(workspace).unwrap();
    assert!(ws.repos.contains(&"/test/workspace/repo".to_string()));
}

// =============================================================================
// BRANCH TESTS
// =============================================================================

#[test]
fn test_config_branch_selection() {
    let mut config = GlobalConfig::default();
    let workspace = Path::new("/test/workspace");
    let repo = "/test/workspace/repo";

    // Initially not selected
    assert!(!config.is_branch_selected(workspace, repo, "main"));

    // Select branch
    config.set_branch_selected(workspace, repo, "main", true);
    assert!(config.is_branch_selected(workspace, repo, "main"));

    // Deselect branch
    config.set_branch_selected(workspace, repo, "main", false);
    assert!(!config.is_branch_selected(workspace, repo, "main"));
}

#[test]
fn test_config_branch_hidden() {
    let mut config = GlobalConfig::default();
    let workspace = Path::new("/test/workspace");
    let repo = "/test/workspace/repo";

    // Select branch first
    config.set_branch_selected(workspace, repo, "old-feature", true);
    assert!(config.is_branch_selected(workspace, repo, "old-feature"));

    // Hide branch (should also deselect)
    config.set_branch_hidden(workspace, repo, "old-feature", true);
    assert!(config.is_branch_hidden(workspace, repo, "old-feature"));
    assert!(!config.is_branch_selected(workspace, repo, "old-feature"));

    // Unhide
    config.set_branch_hidden(workspace, repo, "old-feature", false);
    assert!(!config.is_branch_hidden(workspace, repo, "old-feature"));
}

#[test]
fn test_config_color_assignment() {
    let mut config = GlobalConfig::default();
    let workspace = Path::new("/test/workspace");

    // First branch gets color 0
    let color1 = config.get_branch_color(workspace, "repo", "main");
    assert_eq!(color1, 0);

    // Same branch returns same color
    let color1_again = config.get_branch_color(workspace, "repo", "main");
    assert_eq!(color1_again, 0);

    // Second branch gets color 1
    let color2 = config.get_branch_color(workspace, "repo", "feature");
    assert_eq!(color2, 1);
}

#[test]
fn test_config_save_and_load() {
    let temp = TempDir::new().unwrap();
    let config_path = temp.path().join("config.toml");

    // Create config
    let mut config = GlobalConfig::default();
    config.set_branch_selected(Path::new("/ws"), "repo", "main", true);

    // Save to temp file
    let content = toml::to_string_pretty(&config).unwrap();
    fs::write(&config_path, &content).unwrap();

    // Load back
    let loaded_content = fs::read_to_string(&config_path).unwrap();
    let loaded: GlobalConfig = toml::from_str(&loaded_content).unwrap();

    assert!(loaded.is_branch_selected(Path::new("/ws"), "repo", "main"));
}

// =============================================================================
// CONCEPT TESTS (moved to tests/unit/noslop_file_test.rs)
// =============================================================================
// Concepts are now stored in .noslop.toml via noslop_file module.
// See tests/unit/noslop_file_test.rs for concept tests.
