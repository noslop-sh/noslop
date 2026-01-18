//! Global configuration management
//!
//! Provides persistent storage for user preferences and workspace state.
//! Config is stored at `~/.config/noslop/config.toml` (XDG standard).

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::paths;

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
    /// Concepts in this workspace
    #[serde(default)]
    pub concepts: Vec<ConceptConfig>,
    /// Currently selected concept (None = view all)
    #[serde(default)]
    pub current_concept: Option<String>,
}

/// Concept configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptConfig {
    /// Concept ID (e.g., "CON-1")
    pub id: String,
    /// Concept name
    pub name: String,
    /// Description providing context for LLMs
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Scope patterns (files/directories this concept applies to)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scope: Vec<String>,
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
        paths::global_config_dir()
    }

    /// Get the config file path
    #[must_use]
    pub fn config_path() -> PathBuf {
        paths::global_config()
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

    // === Concept operations ===

    /// Create a new concept, returns the concept ID
    pub fn create_concept(
        &mut self,
        workspace: &Path,
        name: &str,
        description: Option<&str>,
    ) -> String {
        let ws = self.workspace_mut(workspace);

        // Generate next ID
        let max_num = ws
            .concepts
            .iter()
            .filter_map(|c| c.id.strip_prefix("CON-").and_then(|n| n.parse::<u32>().ok()))
            .max()
            .unwrap_or(0);

        let id = format!("CON-{}", max_num + 1);

        ws.concepts.push(ConceptConfig {
            id: id.clone(),
            name: name.to_string(),
            description: description.map(String::from),
            scope: Vec::new(),
            created_at: chrono::Utc::now().to_rfc3339(),
        });

        id
    }

    /// Update a concept's description
    pub fn update_concept_description(
        &mut self,
        workspace: &Path,
        id: &str,
        description: Option<&str>,
    ) -> bool {
        let ws = self.workspace_mut(workspace);
        if let Some(concept) = ws.concepts.iter_mut().find(|c| c.id == id) {
            concept.description = description.map(String::from);
            true
        } else {
            false
        }
    }

    /// List all concepts in a workspace
    #[must_use]
    pub fn list_concepts(&self, workspace: &Path) -> Vec<&ConceptConfig> {
        self.workspace(workspace)
            .map(|ws| ws.concepts.iter().collect())
            .unwrap_or_default()
    }

    /// Get a concept by ID
    #[must_use]
    pub fn get_concept(&self, workspace: &Path, id: &str) -> Option<&ConceptConfig> {
        self.workspace(workspace).and_then(|ws| ws.concepts.iter().find(|c| c.id == id))
    }

    /// Delete a concept by ID, returns true if found and deleted
    pub fn delete_concept(&mut self, workspace: &Path, id: &str) -> bool {
        let ws = self.workspace_mut(workspace);
        let len_before = ws.concepts.len();
        ws.concepts.retain(|c| c.id != id);

        // Clear current_concept if it was the deleted concept
        if ws.current_concept.as_deref() == Some(id) {
            ws.current_concept = None;
        }

        ws.concepts.len() < len_before
    }

    /// Set the current concept (None = view all)
    pub fn set_current_concept(&mut self, workspace: &Path, id: Option<&str>) {
        let ws = self.workspace_mut(workspace);
        ws.current_concept = id.map(String::from);
    }

    /// Get the current concept ID
    #[must_use]
    pub fn current_concept(&self, workspace: &Path) -> Option<&str> {
        self.workspace(workspace).and_then(|ws| ws.current_concept.as_deref())
    }
}
