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
// CONCEPT TESTS
// =============================================================================

#[test]
fn test_config_create_concept() {
    let mut config = GlobalConfig::default();
    let workspace = Path::new("/test/workspace");

    let id1 = config.create_concept(workspace, "Concept One", None);
    assert_eq!(id1, "CON-1");

    let id2 = config.create_concept(workspace, "Concept Two", Some("A description"));
    assert_eq!(id2, "CON-2");

    // Verify concepts were added
    let concepts = config.list_concepts(workspace);
    assert_eq!(concepts.len(), 2);
    assert_eq!(concepts[0].name, "Concept One");
    assert!(concepts[0].description.is_none());
    assert_eq!(concepts[1].name, "Concept Two");
    assert_eq!(concepts[1].description, Some("A description".to_string()));
}

#[test]
fn test_config_list_concepts_empty() {
    let config = GlobalConfig::default();
    let workspace = Path::new("/test/workspace");

    let concepts = config.list_concepts(workspace);
    assert!(concepts.is_empty());
}

#[test]
fn test_config_get_concept() {
    let mut config = GlobalConfig::default();
    let workspace = Path::new("/test/workspace");

    let id = config.create_concept(workspace, "My Concept", None);

    let concept = config.get_concept(workspace, &id);
    assert!(concept.is_some());
    assert_eq!(concept.unwrap().name, "My Concept");

    // Non-existent concept
    let missing = config.get_concept(workspace, "CON-999");
    assert!(missing.is_none());
}

#[test]
fn test_config_delete_concept() {
    let mut config = GlobalConfig::default();
    let workspace = Path::new("/test/workspace");

    let id = config.create_concept(workspace, "Concept to delete", None);
    assert!(config.get_concept(workspace, &id).is_some());

    let deleted = config.delete_concept(workspace, &id);
    assert!(deleted);
    assert!(config.get_concept(workspace, &id).is_none());

    // Delete non-existent should return false
    let deleted_again = config.delete_concept(workspace, &id);
    assert!(!deleted_again);
}

#[test]
fn test_config_delete_concept_clears_current() {
    let mut config = GlobalConfig::default();
    let workspace = Path::new("/test/workspace");

    let id = config.create_concept(workspace, "Current concept", None);
    config.set_current_concept(workspace, Some(&id));
    assert_eq!(config.current_concept(workspace), Some(id.as_str()));

    config.delete_concept(workspace, &id);
    assert!(config.current_concept(workspace).is_none());
}

#[test]
fn test_config_current_concept() {
    let mut config = GlobalConfig::default();
    let workspace = Path::new("/test/workspace");

    // Initially no current concept
    assert!(config.current_concept(workspace).is_none());

    let id = config.create_concept(workspace, "Concept", None);
    config.set_current_concept(workspace, Some(&id));
    assert_eq!(config.current_concept(workspace), Some(id.as_str()));

    // Clear current concept
    config.set_current_concept(workspace, None);
    assert!(config.current_concept(workspace).is_none());
}

#[test]
fn test_config_update_concept_description() {
    let mut config = GlobalConfig::default();
    let workspace = Path::new("/test/workspace");

    let id = config.create_concept(workspace, "Concept", None);

    // Initially no description
    let concept = config.get_concept(workspace, &id).unwrap();
    assert!(concept.description.is_none());

    // Add description
    let updated = config.update_concept_description(workspace, &id, Some("New description"));
    assert!(updated);
    let concept = config.get_concept(workspace, &id).unwrap();
    assert_eq!(concept.description, Some("New description".to_string()));

    // Clear description
    let updated = config.update_concept_description(workspace, &id, None);
    assert!(updated);
    let concept = config.get_concept(workspace, &id).unwrap();
    assert!(concept.description.is_none());

    // Non-existent concept returns false
    let updated = config.update_concept_description(workspace, "CON-999", Some("test"));
    assert!(!updated);
}
