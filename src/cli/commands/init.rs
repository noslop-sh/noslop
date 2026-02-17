//! Initialize noslop in a repository

use std::fs;
use std::path::{Path, PathBuf};

use noslop::adapters::agents::get_agent_config;
use noslop::adapters::git;
use noslop::adapters::toml::generate_prefix_from_repo;
use noslop::core::models::AgentKind;
use noslop::core::ports::agent::AgentConfigBundle;
use noslop::output::OutputMode;

/// Initialize noslop in the current repository
pub fn init(agent: Option<&str>, force: bool, _mode: OutputMode) -> anyhow::Result<()> {
    let noslop_path = Path::new(".noslop.toml");

    if noslop_path.exists() && !force {
        println!("Already initialized (.noslop.toml exists).");
        println!("Use --force to reinitialize.");
        return Ok(());
    }

    // Phase 1: Parse agent type early (fail fast on bad input)
    let agent_kind = agent.map(parse_agent_type).transpose()?;

    println!("Initializing noslop...\n");

    // Generate project prefix from repo name
    let repo_name = git::get_repo_name();
    let prefix = generate_prefix_from_repo(&repo_name);
    println!("  Generated project prefix: {prefix}");

    // Create .noslop.toml with project config
    let noslop_toml = build_noslop_toml(&prefix, agent_kind);
    fs::write(noslop_path, noslop_toml)?;
    println!("  Created .noslop.toml");

    // Create .noslop/ for reviews
    fs::create_dir_all(".noslop")?;
    fs::write(".noslop/.gitkeep", "")?;
    println!("  Created .noslop/");

    // Install git hooks
    git::hooks::install_pre_commit()?;
    println!("  Installed pre-commit hook");
    git::hooks::install_commit_msg()?;
    println!("  Installed commit-msg hook");
    git::hooks::install_post_commit()?;
    println!("  Installed post-commit hook");

    // Install pre-push hook only when agent is specified
    if agent_kind.is_some() {
        install_pre_push()?;
        println!("  Installed pre-push hook");
    }

    // Phase 2: Agent setup (only if agent specified)
    if let Some(kind) = agent_kind {
        println!();
        setup_agent(kind)?;
    }

    // Check for existing docs to bootstrap from
    let docs = ["CLAUDE.md", "AGENTS.md", "ARCHITECTURE.md"];
    let found: Vec<_> = docs.iter().filter(|d| Path::new(d).exists()).collect();

    if !found.is_empty() {
        let found_str: Vec<&str> = found.into_iter().copied().collect();
        println!("\n  Found: {}", found_str.join(", "));
        println!("  Run 'noslop check import' to extract checks");
    }

    println!("\nnoslop initialized!");
    println!("\nNext steps:");
    println!("  noslop check add <target> -m \"message\"");
    println!("  git commit  # checks will be validated");

    Ok(())
}

/// Parse an agent type string, returning a helpful error for unknown agents.
fn parse_agent_type(agent_str: &str) -> anyhow::Result<AgentKind> {
    agent_str
        .parse::<AgentKind>()
        .map_err(|_| anyhow::anyhow!("Unknown agent type: {agent_str}. Supported: claude, codex"))
}

/// Build the `.noslop.toml` content, optionally including agent and review sections.
fn build_noslop_toml(prefix: &str, agent_kind: Option<AgentKind>) -> String {
    let mut toml = format!(
        r#"# noslop checks

[project]
prefix = "{prefix}"

# Checks are auto-assigned IDs like {prefix}-1, {prefix}-2, etc.
# You can also specify custom IDs:
#   id = "my-custom-id"
"#
    );

    if let Some(kind) = agent_kind {
        let agent_name = kind.command();
        toml.push_str(&format!(
            r#"
[agent]
type = "{agent_name}"

[review]
analyzers = ["conventions", "agent:security", "agent:quality"]

[review."agent:security"]
agent = "{agent_name}"

[review."agent:quality"]
agent = "{agent_name}"
"#
        ));
    }

    toml.push_str(
        r#"
# Example check (uncomment to use):
# [[check]]
# target = "*.rs"
# message = "Consider impact on public API"
# severity = "warn"
"#,
    );

    toml
}

/// Set up agent configuration (Phase 2 of init).
fn setup_agent(kind: AgentKind) -> anyhow::Result<()> {
    let config = get_agent_config(kind);

    // Validate agent CLI is available
    config.validate()?;

    // Generate config bundle
    let project_root = std::env::current_dir()?;
    let bundle = config.generate_config(&project_root)?;

    // Write bundle files
    let home = home_dir()?;
    write_config_bundle(&home, &bundle)?;

    println!("  {} agent configured.", kind.display_name());
    Ok(())
}

/// Get the user's home directory.
fn home_dir() -> anyhow::Result<PathBuf> {
    std::env::var("HOME")
        .map(PathBuf::from)
        .map_err(|_| anyhow::anyhow!("Could not determine home directory ($HOME not set)"))
}

/// Write all files from an [`AgentConfigBundle`] to disk.
///
/// - Global instructions and settings are written relative to `home`.
/// - Hook scripts are written to the agent config directory under `home`.
/// - Slash commands are written to the agent config directory under `home`.
pub fn write_config_bundle(home: &Path, bundle: &AgentConfigBundle) -> anyhow::Result<()> {
    // Write global instructions
    if let Some(gi) = &bundle.global_instructions {
        let path = home.join(&gi.relative_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, &gi.content)?;
        println!("  Wrote {}", gi.relative_path.display());
    }

    // Write settings file
    if let Some(sf) = &bundle.settings {
        let path = home.join(&sf.relative_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, &sf.content)?;
        println!("  Wrote {}", sf.relative_path.display());
    }

    // Write hook scripts
    for hook in &bundle.hooks {
        // Derive config dir from the global instructions path parent
        // e.g., `.claude/CLAUDE.md` -> `.claude/hooks/`
        let config_dir = bundle
            .global_instructions
            .as_ref()
            .and_then(|gi| gi.relative_path.parent().map(Path::to_path_buf))
            .unwrap_or_else(|| PathBuf::from(".config"));

        let script_name = format!(
            "{}-{}.sh",
            hook.event.to_lowercase(),
            hook.matcher.to_lowercase().replace('|', "-")
        );
        let hook_path = home.join(&config_dir).join("hooks").join(&script_name);
        if let Some(parent) = hook_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&hook_path, &hook.command)?;

        // Make executable on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&hook_path)?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&hook_path, perms)?;
        }

        println!("  Wrote hook: {script_name}");
    }

    // Write slash commands
    for cmd in &bundle.commands {
        let config_dir = bundle
            .global_instructions
            .as_ref()
            .and_then(|gi| gi.relative_path.parent().map(Path::to_path_buf))
            .unwrap_or_else(|| PathBuf::from(".config"));

        let cmd_path = home.join(&config_dir).join(&cmd.relative_path);
        if let Some(parent) = cmd_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&cmd_path, &cmd.content)?;
        println!("  Wrote command: /{}", cmd.name);
    }

    Ok(())
}

/// Install the pre-push hook (for review gating).
fn install_pre_push() -> anyhow::Result<()> {
    let hooks_dir = Path::new(".git/hooks");
    if !hooks_dir.exists() {
        anyhow::bail!("Not a git repository (.git/hooks not found)");
    }

    let hook_path = hooks_dir.join("pre-push");
    let hook_content = r"#!/usr/bin/env bash
# noslop pre-push hook
# Runs review check before pushing

noslop review run --check
exit $?
";

    if hook_path.exists() {
        let existing = fs::read_to_string(&hook_path)?;
        if existing.contains("noslop") {
            return Ok(()); // Already installed
        }
        // Append to existing hook
        let new_content = format!("{}\n\n# noslop\n{}", existing.trim(), hook_content);
        fs::write(&hook_path, new_content)?;
    } else {
        fs::write(&hook_path, hook_content)?;
    }

    // Make executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&hook_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&hook_path, perms)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_agent_type_claude() {
        let kind = parse_agent_type("claude").unwrap();
        assert_eq!(kind, AgentKind::Claude);
    }

    #[test]
    fn parse_agent_type_codex() {
        let kind = parse_agent_type("codex").unwrap();
        assert_eq!(kind, AgentKind::Codex);
    }

    #[test]
    fn parse_agent_type_unknown_fails() {
        let err = parse_agent_type("cursor").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("Unknown agent type: cursor"), "got: {msg}");
        assert!(msg.contains("claude, codex"), "got: {msg}");
    }

    #[test]
    fn build_toml_without_agent() {
        let toml = build_noslop_toml("NOS", None);
        assert!(toml.contains("[project]"));
        assert!(toml.contains("prefix = \"NOS\""));
        assert!(!toml.contains("[agent]"));
        assert!(!toml.contains("[review]"));
    }

    #[test]
    fn build_toml_with_claude() {
        let toml = build_noslop_toml("NOS", Some(AgentKind::Claude));
        assert!(toml.contains("[project]"));
        assert!(toml.contains("[agent]"));
        assert!(toml.contains("type = \"claude\""));
        assert!(toml.contains("[review]"));
        assert!(toml.contains("agent:security"));
        assert!(toml.contains("agent:quality"));
    }

    #[test]
    fn build_toml_with_codex() {
        let toml = build_noslop_toml("MYP", Some(AgentKind::Codex));
        assert!(toml.contains("type = \"codex\""));
        assert!(toml.contains("prefix = \"MYP\""));
    }

    #[test]
    fn write_config_bundle_creates_files() {
        let temp = tempfile::TempDir::new().unwrap();
        let home = temp.path();

        let bundle = AgentConfigBundle {
            global_instructions: Some(noslop::core::ports::agent::GlobalInstructions {
                relative_path: PathBuf::from(".testcli/INSTRUCTIONS.md"),
                content: "# Test Instructions".to_string(),
            }),
            settings: Some(noslop::core::ports::agent::SettingsFile {
                relative_path: PathBuf::from(".testcli/settings.json"),
                content: "{}".to_string(),
            }),
            hooks: vec![noslop::core::ports::agent::HookDefinition {
                event: "PreToolUse".to_string(),
                matcher: "Edit".to_string(),
                command: "#!/bin/bash\nexit 0\n".to_string(),
            }],
            commands: vec![noslop::core::ports::agent::SlashCommand {
                name: "test".to_string(),
                relative_path: PathBuf::from("commands/test.md"),
                content: "Test command".to_string(),
            }],
        };

        write_config_bundle(home, &bundle).unwrap();

        // Verify global instructions
        let gi_path = home.join(".testcli/INSTRUCTIONS.md");
        assert!(gi_path.exists(), "global instructions should exist");
        assert_eq!(fs::read_to_string(&gi_path).unwrap(), "# Test Instructions");

        // Verify settings
        let sf_path = home.join(".testcli/settings.json");
        assert!(sf_path.exists(), "settings should exist");
        assert_eq!(fs::read_to_string(&sf_path).unwrap(), "{}");

        // Verify hook
        let hook_path = home.join(".testcli/hooks/pretooluse-edit.sh");
        assert!(hook_path.exists(), "hook script should exist");

        // Verify command
        let cmd_path = home.join(".testcli/commands/test.md");
        assert!(cmd_path.exists(), "command should exist");
        assert_eq!(fs::read_to_string(&cmd_path).unwrap(), "Test command");
    }

    #[test]
    fn write_config_bundle_empty_bundle() {
        let temp = tempfile::TempDir::new().unwrap();
        let home = temp.path();

        let bundle = AgentConfigBundle::default();
        write_config_bundle(home, &bundle).unwrap();
        // Should succeed with nothing written
    }

    #[test]
    fn generated_toml_parses_correctly() {
        let toml_str = build_noslop_toml("TST", Some(AgentKind::Claude));
        let parsed: noslop::adapters::toml::NoslopFile = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.project.prefix, "TST");
        let agent = parsed.agent.unwrap();
        assert_eq!(agent.agent_type, "claude");
        let review = parsed.review.unwrap();
        assert_eq!(review.analyzers.len(), 3);
        assert!(review.analyzers.contains(&"conventions".to_string()));
    }
}
