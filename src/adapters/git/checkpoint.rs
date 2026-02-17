//! Checkpoint operations for git repositories
//!
//! Provides `is_clean` and `commit_all` functions that operate on a
//! `git2::Repository`. These are used by the `VersionControl` trait
//! implementation and the `noslop checkpoint` CLI command.

use git2::{Repository, Signature, StatusOptions};

/// Returns true if no staged, unstaged, or untracked changes exist.
pub fn is_clean(repo: &Repository) -> anyhow::Result<bool> {
    let mut opts = StatusOptions::new();
    opts.include_untracked(true).recurse_untracked_dirs(true);
    Ok(repo.statuses(Some(&mut opts))?.is_empty())
}

/// Stage all changes and commit with the given message, bypassing hooks.
///
/// Returns the short SHA (7 chars) of the new commit, or `None` if the
/// working tree was already clean.
pub fn commit_all(repo: &Repository, message: &str) -> anyhow::Result<Option<String>> {
    if is_clean(repo)? {
        return Ok(None);
    }

    let mut index = repo.index()?;
    // Stage new and modified files
    index.add_all(["*"], git2::IndexAddOption::DEFAULT, None)?;
    // Stage deletions
    index.update_all(["*"], None)?;
    index.write()?;

    let tree_oid = index.write_tree()?;
    let tree = repo.find_tree(tree_oid)?;
    let parent = repo.head()?.peel_to_commit()?;
    let sig = repo.signature().or_else(|_| Signature::now("noslop", "noslop@localhost"))?;

    let oid = repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &[&parent])?;
    let sha = oid.to_string();
    let short = if sha.len() >= 7 { &sha[..7] } else { &sha };
    Ok(Some(short.to_string()))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    /// Create a temp repo with an initial commit so HEAD exists
    fn init_repo_with_commit(dir: &std::path::Path) -> Repository {
        let repo = Repository::init(dir).expect("init repo");
        // Need an initial commit for HEAD to exist
        let sig = Signature::now("test", "test@test.com").unwrap();
        let tree_oid = {
            let mut index = repo.index().unwrap();
            index.write_tree().unwrap()
        };
        {
            let tree = repo.find_tree(tree_oid).unwrap();
            repo.commit(Some("HEAD"), &sig, &sig, "initial commit", &tree, &[]).unwrap();
        }
        repo
    }

    #[test]
    fn is_clean_on_fresh_repo() {
        let dir = tempfile::tempdir().unwrap();
        let repo = init_repo_with_commit(dir.path());
        assert!(is_clean(&repo).unwrap());
    }

    #[test]
    fn is_clean_with_untracked_file() {
        let dir = tempfile::tempdir().unwrap();
        let repo = init_repo_with_commit(dir.path());
        fs::write(dir.path().join("new_file.txt"), "hello").unwrap();
        assert!(!is_clean(&repo).unwrap());
    }

    #[test]
    fn commit_all_stages_and_commits() {
        let dir = tempfile::tempdir().unwrap();
        let repo = init_repo_with_commit(dir.path());
        fs::write(dir.path().join("file.txt"), "content").unwrap();

        let result = commit_all(&repo, "test checkpoint").unwrap();
        assert!(result.is_some());
        let sha = result.unwrap();
        assert_eq!(sha.len(), 7);

        // Tree should be clean after commit
        assert!(is_clean(&repo).unwrap());
    }

    #[test]
    fn commit_all_returns_none_on_clean() {
        let dir = tempfile::tempdir().unwrap();
        let repo = init_repo_with_commit(dir.path());

        let result = commit_all(&repo, "nothing to commit").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn commit_all_handles_deletions() {
        let dir = tempfile::tempdir().unwrap();
        let repo = init_repo_with_commit(dir.path());

        // Create and commit a file first
        let file_path = dir.path().join("to_delete.txt");
        fs::write(&file_path, "will be deleted").unwrap();
        commit_all(&repo, "add file").unwrap();

        // Now delete it
        fs::remove_file(&file_path).unwrap();
        assert!(!is_clean(&repo).unwrap());

        let result = commit_all(&repo, "delete file").unwrap();
        assert!(result.is_some());
        assert!(is_clean(&repo).unwrap());
    }

    #[test]
    fn commit_all_handles_modifications() {
        let dir = tempfile::tempdir().unwrap();
        let repo = init_repo_with_commit(dir.path());

        // Create and commit a file
        let file_path = dir.path().join("modify_me.txt");
        fs::write(&file_path, "original").unwrap();
        commit_all(&repo, "add file").unwrap();

        // Modify it
        fs::write(&file_path, "modified").unwrap();
        assert!(!is_clean(&repo).unwrap());

        let result = commit_all(&repo, "modify file").unwrap();
        assert!(result.is_some());
        assert!(is_clean(&repo).unwrap());
    }
}
