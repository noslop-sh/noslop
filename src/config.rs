//! Global configuration management
//!
//! Provides persistent storage for user preferences and workspace state.
//! Config is stored at `~/.config/noslop/config.toml` (XDG standard).

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Default config directory name under XDG config
const CONFIG_DIR: &str = "noslop";
/// Config file name
const CONFIG_FILE: &str = "config.toml";

/// Global noslop configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GlobalConfig {
    /// UI preferences
    #[serde(default)]
    pub ui: UiConfig,
    /// Per-workspace configurations (keyed by workspace path)
    #[serde(default)]
    pub workspaces: HashMap<String, WorkspaceConfig>,
}

/// UI preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    /// Theme preference
    #[serde(default = "default_theme")]
    pub theme: String,
}

fn default_theme() -> String {
    "dark".to_string()
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            theme: default_theme(),
        }
    }
}

/// Per-workspace configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    /// Repos discovered/added in this workspace
    #[serde(default)]
    pub repos: Vec<String>,
    /// Branch settings per repo (keyed by repo path)
    #[serde(default)]
    pub branches: HashMap<String, BranchSettings>,
    /// Color assignments (keyed by "repo/branch")
    #[serde(default)]
    pub colors: HashMap<String, usize>,
    /// Projects in this workspace
    #[serde(default)]
    pub projects: Vec<ProjectConfig>,
    /// Currently selected project (None = view all)
    #[serde(default)]
    pub current_project: Option<String>,
}

/// Project configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    /// Project ID (e.g., "PROJ-1")
    pub id: String,
    /// Project name
    pub name: String,
    /// When created (RFC3339)
    pub created_at: String,
}

/// Branch visibility settings for a repo
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BranchSettings {
    /// Branches currently selected (shown in kanban)
    #[serde(default)]
    pub selected: Vec<String>,
    /// Branches explicitly hidden by user
    #[serde(default)]
    pub hidden: Vec<String>,
}

impl GlobalConfig {
    /// Get the config directory path
    #[must_use]
    pub fn config_dir() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("~/.config"))
            .join(CONFIG_DIR)
    }

    /// Get the config file path
    #[must_use]
    pub fn config_path() -> PathBuf {
        Self::config_dir().join(CONFIG_FILE)
    }

    /// Load config from disk, or create default if not exists
    #[must_use]
    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            fs::read_to_string(&path)
                .ok()
                .and_then(|content| toml::from_str(&content).ok())
                .unwrap_or_default()
        } else {
            Self::default()
        }
    }

    /// Save config to disk
    pub fn save(&self) -> anyhow::Result<()> {
        let dir = Self::config_dir();
        fs::create_dir_all(&dir)?;

        let path = Self::config_path();
        let content = toml::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    /// Get workspace config for a path, creating if needed
    pub fn workspace_mut(&mut self, path: &Path) -> &mut WorkspaceConfig {
        let key = path.to_string_lossy().to_string();
        self.workspaces.entry(key).or_default()
    }

    /// Get workspace config for a path (read-only)
    #[must_use]
    pub fn workspace(&self, path: &Path) -> Option<&WorkspaceConfig> {
        let key = path.to_string_lossy().to_string();
        self.workspaces.get(&key)
    }

    /// Add a repo to a workspace
    pub fn add_repo(&mut self, workspace: &Path, repo_path: &Path) {
        let ws = self.workspace_mut(workspace);
        let repo_str = repo_path.to_string_lossy().to_string();
        if !ws.repos.contains(&repo_str) {
            ws.repos.push(repo_str);
        }
    }

    /// Set branch selected state
    pub fn set_branch_selected(
        &mut self,
        workspace: &Path,
        repo: &str,
        branch: &str,
        selected: bool,
    ) {
        let ws = self.workspace_mut(workspace);
        let settings = ws.branches.entry(repo.to_string()).or_default();

        if selected {
            if !settings.selected.contains(&branch.to_string()) {
                settings.selected.push(branch.to_string());
            }
        } else {
            settings.selected.retain(|b| b != branch);
        }
    }

    /// Set branch hidden state
    pub fn set_branch_hidden(&mut self, workspace: &Path, repo: &str, branch: &str, hidden: bool) {
        let ws = self.workspace_mut(workspace);
        let settings = ws.branches.entry(repo.to_string()).or_default();

        if hidden {
            if !settings.hidden.contains(&branch.to_string()) {
                settings.hidden.push(branch.to_string());
            }
            // Also deselect if hidden
            settings.selected.retain(|b| b != branch);
        } else {
            settings.hidden.retain(|b| b != branch);
        }
    }

    /// Get or assign a color index for a branch
    pub fn get_branch_color(&mut self, workspace: &Path, repo: &str, branch: &str) -> usize {
        let key = format!("{repo}/{branch}");
        let ws = self.workspace_mut(workspace);

        if let Some(&color) = ws.colors.get(&key) {
            return color;
        }

        // Assign next available color
        let next_color = ws.colors.values().max().map_or(0, |m| m + 1);
        ws.colors.insert(key, next_color);
        next_color
    }

    /// Check if a branch is selected
    #[must_use]
    pub fn is_branch_selected(&self, workspace: &Path, repo: &str, branch: &str) -> bool {
        self.workspace(workspace)
            .and_then(|ws| ws.branches.get(repo))
            .is_some_and(|settings| settings.selected.contains(&branch.to_string()))
    }

    /// Check if a branch is hidden
    #[must_use]
    pub fn is_branch_hidden(&self, workspace: &Path, repo: &str, branch: &str) -> bool {
        self.workspace(workspace)
            .and_then(|ws| ws.branches.get(repo))
            .is_some_and(|settings| settings.hidden.contains(&branch.to_string()))
    }

    // === Project operations ===

    /// Create a new project, returns the project ID
    pub fn create_project(&mut self, workspace: &Path, name: &str) -> String {
        let ws = self.workspace_mut(workspace);

        // Generate next ID
        let max_num = ws
            .projects
            .iter()
            .filter_map(|p| p.id.strip_prefix("PROJ-").and_then(|n| n.parse::<u32>().ok()))
            .max()
            .unwrap_or(0);

        let id = format!("PROJ-{}", max_num + 1);

        ws.projects.push(ProjectConfig {
            id: id.clone(),
            name: name.to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
        });

        id
    }

    /// List all projects in a workspace
    #[must_use]
    pub fn list_projects(&self, workspace: &Path) -> Vec<&ProjectConfig> {
        self.workspace(workspace)
            .map(|ws| ws.projects.iter().collect())
            .unwrap_or_default()
    }

    /// Get a project by ID
    #[must_use]
    pub fn get_project(&self, workspace: &Path, id: &str) -> Option<&ProjectConfig> {
        self.workspace(workspace).and_then(|ws| ws.projects.iter().find(|p| p.id == id))
    }

    /// Delete a project by ID, returns true if found and deleted
    pub fn delete_project(&mut self, workspace: &Path, id: &str) -> bool {
        let ws = self.workspace_mut(workspace);
        let len_before = ws.projects.len();
        ws.projects.retain(|p| p.id != id);

        // Clear current_project if it was the deleted project
        if ws.current_project.as_deref() == Some(id) {
            ws.current_project = None;
        }

        ws.projects.len() < len_before
    }

    /// Set the current project (None = view all)
    pub fn set_current_project(&mut self, workspace: &Path, id: Option<&str>) {
        let ws = self.workspace_mut(workspace);
        ws.current_project = id.map(String::from);
    }

    /// Get the current project ID
    #[must_use]
    pub fn current_project(&self, workspace: &Path) -> Option<&str> {
        self.workspace(workspace).and_then(|ws| ws.current_project.as_deref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_default_config() {
        let config = GlobalConfig::default();
        assert_eq!(config.ui.theme, "dark");
        assert!(config.workspaces.is_empty());
    }

    #[test]
    fn test_workspace_creation() {
        let mut config = GlobalConfig::default();
        let workspace = Path::new("/test/workspace");

        config.workspace_mut(workspace);
        assert!(config.workspaces.contains_key("/test/workspace"));
    }

    #[test]
    fn test_add_repo() {
        let mut config = GlobalConfig::default();
        let workspace = Path::new("/test/workspace");
        let repo = Path::new("/test/workspace/repo");

        config.add_repo(workspace, repo);

        let ws = config.workspace(workspace).unwrap();
        assert!(ws.repos.contains(&"/test/workspace/repo".to_string()));
    }

    #[test]
    fn test_branch_selection() {
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
    fn test_branch_hidden() {
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
    fn test_color_assignment() {
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
    fn test_save_and_load() {
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
    // PROJECT TESTS
    // =============================================================================

    #[test]
    fn test_create_project() {
        let mut config = GlobalConfig::default();
        let workspace = Path::new("/test/workspace");

        let id1 = config.create_project(workspace, "Project One");
        assert_eq!(id1, "PROJ-1");

        let id2 = config.create_project(workspace, "Project Two");
        assert_eq!(id2, "PROJ-2");

        // Verify projects were added
        let projects = config.list_projects(workspace);
        assert_eq!(projects.len(), 2);
        assert_eq!(projects[0].name, "Project One");
        assert_eq!(projects[1].name, "Project Two");
    }

    #[test]
    fn test_list_projects_empty() {
        let config = GlobalConfig::default();
        let workspace = Path::new("/test/workspace");

        let projects = config.list_projects(workspace);
        assert!(projects.is_empty());
    }

    #[test]
    fn test_get_project() {
        let mut config = GlobalConfig::default();
        let workspace = Path::new("/test/workspace");

        let id = config.create_project(workspace, "My Project");

        let project = config.get_project(workspace, &id);
        assert!(project.is_some());
        assert_eq!(project.unwrap().name, "My Project");

        // Non-existent project
        let missing = config.get_project(workspace, "PROJ-999");
        assert!(missing.is_none());
    }

    #[test]
    fn test_delete_project() {
        let mut config = GlobalConfig::default();
        let workspace = Path::new("/test/workspace");

        let id = config.create_project(workspace, "Project to delete");
        assert!(config.get_project(workspace, &id).is_some());

        let deleted = config.delete_project(workspace, &id);
        assert!(deleted);
        assert!(config.get_project(workspace, &id).is_none());

        // Delete non-existent should return false
        let deleted_again = config.delete_project(workspace, &id);
        assert!(!deleted_again);
    }

    #[test]
    fn test_delete_project_clears_current() {
        let mut config = GlobalConfig::default();
        let workspace = Path::new("/test/workspace");

        let id = config.create_project(workspace, "Current project");
        config.set_current_project(workspace, Some(&id));
        assert_eq!(config.current_project(workspace), Some(id.as_str()));

        config.delete_project(workspace, &id);
        assert!(config.current_project(workspace).is_none());
    }

    #[test]
    fn test_current_project() {
        let mut config = GlobalConfig::default();
        let workspace = Path::new("/test/workspace");

        // Initially no current project
        assert!(config.current_project(workspace).is_none());

        let id = config.create_project(workspace, "Project");
        config.set_current_project(workspace, Some(&id));
        assert_eq!(config.current_project(workspace), Some(id.as_str()));

        // Clear current project
        config.set_current_project(workspace, None);
        assert!(config.current_project(workspace).is_none());
    }
}
