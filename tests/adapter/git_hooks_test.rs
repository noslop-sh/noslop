//! Tests for git hooks installation and removal

use std::fs;
use std::path::Path;
use std::sync::Mutex;
use tempfile::TempDir;

/// Mutex to serialize tests that change the current directory
static DIR_MUTEX: Mutex<()> = Mutex::new(());

/// Set up a temporary git repository for testing
fn setup_temp_git_repo() -> TempDir {
    let temp = TempDir::new().unwrap();
    let hooks_dir = temp.path().join(".git/hooks");
    fs::create_dir_all(&hooks_dir).unwrap();
    temp
}

/// Change to the temp directory for hook operations (serialized via mutex)
fn in_dir<F, R>(dir: &Path, f: F) -> R
where
    F: FnOnce() -> R,
{
    let _guard = DIR_MUTEX.lock().unwrap();
    let original = std::env::current_dir().expect("Failed to get current dir");
    std::env::set_current_dir(dir).expect("Failed to change to temp dir");
    let result = f();
    std::env::set_current_dir(&original).expect("Failed to restore original dir");
    result
}

mod remove_noslop_sections {
    use noslop::adapters::git::hooks::remove_noslop_sections;

    #[test]
    fn removes_noslop_only_content() {
        let content = r#"#!/bin/sh
# noslop pre-commit hook
# Checks that checks are acknowledged before allowing commit

noslop check
"#;
        let result = remove_noslop_sections(content);
        assert_eq!(result, "#!/bin/sh\n");
    }

    #[test]
    fn preserves_other_hook_content() {
        let content = r#"#!/bin/sh
# My custom pre-commit checks
echo "Running custom checks..."
./scripts/lint.sh

# noslop
noslop check
"#;
        let result = remove_noslop_sections(content);
        assert_eq!(
            result,
            r#"#!/bin/sh
# My custom pre-commit checks
echo "Running custom checks..."
./scripts/lint.sh
"#
        );
    }

    #[test]
    fn handles_noslop_in_middle() {
        let content = r#"#!/bin/sh
# First section
echo "first"

# noslop pre-commit hook
noslop check

# Second section
echo "second"
"#;
        let result = remove_noslop_sections(content);
        // Verify noslop content is removed and other content preserved
        assert!(!result.contains("noslop"), "noslop should be removed");
        assert!(result.contains("# First section"), "First section should remain");
        assert!(result.contains("# Second section"), "Second section should remain");
        assert!(result.contains("echo \"first\""), "First command should remain");
        assert!(result.contains("echo \"second\""), "Second command should remain");
    }

    #[test]
    fn handles_empty_content() {
        let result = remove_noslop_sections("");
        assert_eq!(result, "");
    }

    #[test]
    fn handles_shebang_only() {
        let content = "#!/bin/sh\n";
        let result = remove_noslop_sections(content);
        assert_eq!(result, "#!/bin/sh\n");
    }
}

mod remove_noslop_hooks {
    use super::*;
    use noslop::adapters::git::hooks::remove_noslop_hooks;

    #[test]
    fn removes_noslop_only_hook_file() {
        let temp = setup_temp_git_repo();
        let hook_path = temp.path().join(".git/hooks/pre-commit");

        fs::write(
            &hook_path,
            r#"#!/bin/sh
# noslop pre-commit hook
noslop check
"#,
        )
        .unwrap();

        in_dir(temp.path(), || {
            remove_noslop_hooks().unwrap();
        });

        assert!(!hook_path.exists(), "Hook file should be removed");
    }

    #[test]
    fn preserves_non_noslop_content() {
        let temp = setup_temp_git_repo();
        let hook_path = temp.path().join(".git/hooks/pre-commit");

        fs::write(
            &hook_path,
            r#"#!/bin/sh
# Custom checks
./lint.sh

# noslop
noslop check
"#,
        )
        .unwrap();

        in_dir(temp.path(), || {
            remove_noslop_hooks().unwrap();
        });

        assert!(hook_path.exists(), "Hook file should still exist");
        let content = fs::read_to_string(&hook_path).unwrap();
        assert!(!content.contains("noslop"), "noslop content should be removed");
        assert!(content.contains("./lint.sh"), "Custom content should be preserved");
    }

    #[test]
    fn handles_all_three_hooks() {
        let temp = setup_temp_git_repo();
        let hooks = ["pre-commit", "commit-msg", "post-commit"];

        for hook in hooks {
            let hook_path = temp.path().join(format!(".git/hooks/{}", hook));
            fs::write(
                &hook_path,
                format!(
                    r#"#!/bin/sh
# noslop {} hook
noslop check
"#,
                    hook
                ),
            )
            .unwrap();
        }

        in_dir(temp.path(), || {
            remove_noslop_hooks().unwrap();
        });

        for hook in hooks {
            let hook_path = temp.path().join(format!(".git/hooks/{}", hook));
            assert!(!hook_path.exists(), "{} should be removed", hook);
        }
    }

    #[test]
    fn handles_missing_hooks_dir() {
        let temp = TempDir::new().unwrap();
        // No .git/hooks directory

        in_dir(temp.path(), || {
            let result = remove_noslop_hooks();
            assert!(result.is_ok(), "Should succeed when no hooks dir exists");
        });
    }
}

mod install_hooks_force {
    use super::*;
    use noslop::adapters::git::GitVersionControl;
    use noslop::core::ports::VersionControl;

    #[test]
    fn force_reinstall_replaces_modified_hooks() {
        let temp = setup_temp_git_repo();
        let hook_path = temp.path().join(".git/hooks/pre-commit");

        // Write a modified hook (simulating user edits)
        fs::write(
            &hook_path,
            r#"#!/bin/sh
# noslop pre-commit hook (MODIFIED BY USER)
# Added extra stuff
noslop check --verbose
echo "extra stuff"
"#,
        )
        .unwrap();

        in_dir(temp.path(), || {
            let git = GitVersionControl::new(temp.path().to_path_buf());
            git.install_hooks(true).unwrap();
        });

        let content = fs::read_to_string(&hook_path).unwrap();
        assert!(content.contains("noslop check"), "Should have noslop check command");
        // The hook should be fresh, not have the user modifications
        assert!(
            !content.contains("MODIFIED BY USER"),
            "User modifications should be removed with --force"
        );
    }

    #[test]
    fn force_reinstall_preserves_non_noslop_content() {
        let temp = setup_temp_git_repo();
        let hook_path = temp.path().join(".git/hooks/pre-commit");

        // Write a hook with both custom and noslop content
        fs::write(
            &hook_path,
            r#"#!/bin/sh
# Custom pre-commit checks
./run-my-linter.sh

# noslop
noslop check
"#,
        )
        .unwrap();

        in_dir(temp.path(), || {
            let git = GitVersionControl::new(temp.path().to_path_buf());
            git.install_hooks(true).unwrap();
        });

        let content = fs::read_to_string(&hook_path).unwrap();
        assert!(content.contains("./run-my-linter.sh"), "Custom content should be preserved");
        assert!(content.contains("noslop"), "noslop should be reinstalled");
    }
}
