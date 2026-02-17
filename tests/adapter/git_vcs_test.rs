//! Tests for the extended VCS methods on GitVersionControl
//!
//! Uses `tempfile::TempDir` + `git2::Repository::init` to create
//! isolated git repositories for each test.

use std::fs;
use std::path::Path;

use git2::{Repository, Signature};

use noslop::adapters::GitVersionControl;
use noslop::core::ports::VersionControl;
use noslop::core::ports::vcs_types::{DiffLineKind, DiffStatus};

/// Create a temporary git repository with an initial commit on branch "main".
fn setup_repo(dir: &Path) -> Repository {
    let repo = Repository::init(dir).expect("init repo");
    let sig = Signature::now("test", "test@test.com").unwrap();

    // Create initial file and commit
    fs::write(dir.join("README.md"), "# Test\n").unwrap();
    {
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("README.md")).unwrap();
        index.write().unwrap();
        let tree_oid = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_oid).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "initial commit", &tree, &[]).unwrap();
    }

    repo
}

/// Commit a set of file changes with the given message.
fn commit_changes(repo: &Repository, _dir: &Path, message: &str) -> git2::Oid {
    let sig = Signature::now("test", "test@test.com").unwrap();
    let mut index = repo.index().unwrap();
    index.add_all(["*"], git2::IndexAddOption::DEFAULT, None).unwrap();
    index.update_all(["*"], None).unwrap();
    index.write().unwrap();
    let tree_oid = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_oid).unwrap();
    let parent = repo.head().unwrap().peel_to_commit().unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &[&parent]).unwrap()
}

fn new_vcs(dir: &Path) -> GitVersionControl {
    GitVersionControl::new(dir.to_path_buf())
}

// =========================================================================
// is_clean / commit_all via VersionControl trait
// =========================================================================

#[test]
fn is_clean_trait_on_fresh_repo() {
    let dir = tempfile::tempdir().unwrap();
    let _repo = setup_repo(dir.path());
    let vcs = new_vcs(dir.path());
    assert!(vcs.is_clean().unwrap());
}

#[test]
fn commit_all_trait_with_changes() {
    let dir = tempfile::tempdir().unwrap();
    let _repo = setup_repo(dir.path());
    let vcs = new_vcs(dir.path());

    fs::write(dir.path().join("new.txt"), "content").unwrap();
    let result = vcs.commit_all("trait checkpoint").unwrap();
    assert!(result.is_some());
    assert!(vcs.is_clean().unwrap());
}

// =========================================================================
// diff_between
// =========================================================================

#[test]
fn diff_between_detects_added_file() {
    let dir = tempfile::tempdir().unwrap();
    let repo = setup_repo(dir.path());
    let vcs = new_vcs(dir.path());

    let base = repo.head().unwrap().peel_to_commit().unwrap().id();

    fs::write(dir.path().join("new_file.rs"), "fn main() {}\n").unwrap();
    let head = commit_changes(&repo, dir.path(), "add new file");

    let diffs = vcs.diff_between(&base.to_string(), &head.to_string()).unwrap();
    assert_eq!(diffs.len(), 1);
    assert_eq!(diffs[0].status, DiffStatus::Added);
    assert_eq!(diffs[0].path.to_str().unwrap(), "new_file.rs");
    assert!(!diffs[0].hunks.is_empty());
    assert!(diffs[0].hunks[0].lines.iter().any(|l| l.kind == DiffLineKind::Addition));
}

#[test]
fn diff_between_detects_modified_file() {
    let dir = tempfile::tempdir().unwrap();
    let repo = setup_repo(dir.path());
    let vcs = new_vcs(dir.path());

    let base = repo.head().unwrap().peel_to_commit().unwrap().id();

    // Modify README.md
    fs::write(dir.path().join("README.md"), "# Test\nModified\n").unwrap();
    let head = commit_changes(&repo, dir.path(), "modify readme");

    let diffs = vcs.diff_between(&base.to_string(), &head.to_string()).unwrap();
    assert_eq!(diffs.len(), 1);
    assert_eq!(diffs[0].status, DiffStatus::Modified);
    assert_eq!(diffs[0].path.to_str().unwrap(), "README.md");
}

#[test]
fn diff_between_detects_deleted_file() {
    let dir = tempfile::tempdir().unwrap();
    let repo = setup_repo(dir.path());
    let vcs = new_vcs(dir.path());

    let base = repo.head().unwrap().peel_to_commit().unwrap().id();

    fs::remove_file(dir.path().join("README.md")).unwrap();
    let head = commit_changes(&repo, dir.path(), "delete readme");

    let diffs = vcs.diff_between(&base.to_string(), &head.to_string()).unwrap();
    assert_eq!(diffs.len(), 1);
    assert_eq!(diffs[0].status, DiffStatus::Deleted);
}

#[test]
fn diff_between_no_changes_returns_empty() {
    let dir = tempfile::tempdir().unwrap();
    let repo = setup_repo(dir.path());
    let vcs = new_vcs(dir.path());

    let sha = repo.head().unwrap().peel_to_commit().unwrap().id().to_string();
    let diffs = vcs.diff_between(&sha, &sha).unwrap();
    assert!(diffs.is_empty());
}

// =========================================================================
// commits_between
// =========================================================================

#[test]
fn commits_between_returns_correct_range() {
    let dir = tempfile::tempdir().unwrap();
    let repo = setup_repo(dir.path());
    let vcs = new_vcs(dir.path());

    let base = repo.head().unwrap().peel_to_commit().unwrap().id();

    fs::write(dir.path().join("a.txt"), "a").unwrap();
    commit_changes(&repo, dir.path(), "first change");

    fs::write(dir.path().join("b.txt"), "b").unwrap();
    let head = commit_changes(&repo, dir.path(), "second change");

    let commits = vcs.commits_between(&base.to_string(), &head.to_string()).unwrap();
    assert_eq!(commits.len(), 2);
    // Most recent first (topological + time)
    assert_eq!(commits[0].summary, "second change");
    assert_eq!(commits[1].summary, "first change");
}

#[test]
fn commits_between_same_ref_returns_empty() {
    let dir = tempfile::tempdir().unwrap();
    let repo = setup_repo(dir.path());
    let vcs = new_vcs(dir.path());

    let sha = repo.head().unwrap().peel_to_commit().unwrap().id().to_string();
    let commits = vcs.commits_between(&sha, &sha).unwrap();
    assert!(commits.is_empty());
}

#[test]
fn commits_between_populates_all_fields() {
    let dir = tempfile::tempdir().unwrap();
    let repo = setup_repo(dir.path());
    let vcs = new_vcs(dir.path());

    let base = repo.head().unwrap().peel_to_commit().unwrap().id();

    fs::write(dir.path().join("x.txt"), "x").unwrap();
    let head = commit_changes(&repo, dir.path(), "detailed commit message");

    let commits = vcs.commits_between(&base.to_string(), &head.to_string()).unwrap();
    assert_eq!(commits.len(), 1);
    let c = &commits[0];
    assert_eq!(c.short_sha.len(), 7);
    assert_eq!(c.sha.len(), 40);
    assert_eq!(c.summary, "detailed commit message");
    assert_eq!(c.author, "test");
    assert!(c.timestamp > 0);
}

// =========================================================================
// file_at_revision
// =========================================================================

#[test]
fn file_at_revision_reads_correct_content() {
    let dir = tempfile::tempdir().unwrap();
    let repo = setup_repo(dir.path());
    let vcs = new_vcs(dir.path());

    let rev1 = repo.head().unwrap().peel_to_commit().unwrap().id();

    // Modify the file
    fs::write(dir.path().join("README.md"), "# Updated\n").unwrap();
    let rev2 = commit_changes(&repo, dir.path(), "update readme");

    // Read at old revision
    let old_content = vcs.file_at_revision(Path::new("README.md"), &rev1.to_string()).unwrap();
    assert_eq!(old_content, "# Test\n");

    // Read at new revision
    let new_content = vcs.file_at_revision(Path::new("README.md"), &rev2.to_string()).unwrap();
    assert_eq!(new_content, "# Updated\n");
}

#[test]
fn file_at_revision_missing_file_errors() {
    let dir = tempfile::tempdir().unwrap();
    let repo = setup_repo(dir.path());
    let vcs = new_vcs(dir.path());

    let rev = repo.head().unwrap().peel_to_commit().unwrap().id().to_string();
    let result = vcs.file_at_revision(Path::new("nonexistent.rs"), &rev);
    assert!(result.is_err());
}

#[test]
fn file_at_revision_invalid_ref_errors() {
    let dir = tempfile::tempdir().unwrap();
    let _repo = setup_repo(dir.path());
    let vcs = new_vcs(dir.path());

    let result = vcs.file_at_revision(Path::new("README.md"), "nonexistent_ref");
    assert!(result.is_err());
}

// =========================================================================
// recent_commits
// =========================================================================

#[test]
fn recent_commits_returns_correct_count() {
    let dir = tempfile::tempdir().unwrap();
    let repo = setup_repo(dir.path());
    let vcs = new_vcs(dir.path());

    // Initial commit + 3 more = 4 total
    for i in 0..3 {
        fs::write(dir.path().join(format!("file{i}.txt")), format!("content {i}")).unwrap();
        commit_changes(&repo, dir.path(), &format!("commit {i}"));
    }

    let recent = vcs.recent_commits(2).unwrap();
    assert_eq!(recent.len(), 2);
    assert_eq!(recent[0].summary, "commit 2");
    assert_eq!(recent[1].summary, "commit 1");
}

#[test]
fn recent_commits_more_than_available() {
    let dir = tempfile::tempdir().unwrap();
    let _repo = setup_repo(dir.path());
    let vcs = new_vcs(dir.path());

    // Only 1 commit exists
    let recent = vcs.recent_commits(100).unwrap();
    assert_eq!(recent.len(), 1);
    assert_eq!(recent[0].summary, "initial commit");
}

#[test]
fn recent_commits_zero_returns_empty() {
    let dir = tempfile::tempdir().unwrap();
    let _repo = setup_repo(dir.path());
    let vcs = new_vcs(dir.path());

    let recent = vcs.recent_commits(0).unwrap();
    assert!(recent.is_empty());
}

// =========================================================================
// diff_stat_since
// =========================================================================

#[test]
fn diff_stat_since_counts_correctly() {
    let dir = tempfile::tempdir().unwrap();
    let repo = setup_repo(dir.path());
    let vcs = new_vcs(dir.path());

    let base = repo.head().unwrap().peel_to_commit().unwrap().id();

    // Add a file (1 file, some insertions)
    fs::write(dir.path().join("new.rs"), "line1\nline2\nline3\n").unwrap();
    commit_changes(&repo, dir.path(), "add file");

    let stat = vcs.diff_stat_since(&base.to_string()).unwrap();
    assert_eq!(stat.files_changed, 1);
    assert_eq!(stat.insertions, 3);
    assert_eq!(stat.deletions, 0);
}

#[test]
fn diff_stat_since_with_modifications() {
    let dir = tempfile::tempdir().unwrap();
    let repo = setup_repo(dir.path());
    let vcs = new_vcs(dir.path());

    let base = repo.head().unwrap().peel_to_commit().unwrap().id();

    // Modify existing file
    fs::write(dir.path().join("README.md"), "# Changed\nNew line\n").unwrap();
    commit_changes(&repo, dir.path(), "modify readme");

    let stat = vcs.diff_stat_since(&base.to_string()).unwrap();
    assert_eq!(stat.files_changed, 1);
    assert!(stat.insertions > 0);
    assert!(stat.deletions > 0);
}

#[test]
fn diff_stat_since_invalid_ref_errors() {
    let dir = tempfile::tempdir().unwrap();
    let _repo = setup_repo(dir.path());
    let vcs = new_vcs(dir.path());

    let result = vcs.diff_stat_since("nonexistent_ref");
    assert!(result.is_err());
}

// =========================================================================
// diff_between hunk detail
// =========================================================================

#[test]
fn diff_between_hunk_lines_classified_correctly() {
    let dir = tempfile::tempdir().unwrap();
    let repo = setup_repo(dir.path());
    let vcs = new_vcs(dir.path());

    // Write multi-line file
    fs::write(dir.path().join("code.rs"), "fn main() {\n    old_line();\n}\n").unwrap();
    let mid = commit_changes(&repo, dir.path(), "add code");

    // Modify it
    fs::write(dir.path().join("code.rs"), "fn main() {\n    new_line();\n}\n").unwrap();
    let head = commit_changes(&repo, dir.path(), "modify code");

    let diffs = vcs.diff_between(&mid.to_string(), &head.to_string()).unwrap();
    assert_eq!(diffs.len(), 1);
    assert_eq!(diffs[0].status, DiffStatus::Modified);

    let all_lines: Vec<_> = diffs[0].hunks.iter().flat_map(|h| &h.lines).collect();
    assert!(
        all_lines
            .iter()
            .any(|l| l.kind == DiffLineKind::Deletion && l.content.contains("old_line"))
    );
    assert!(
        all_lines
            .iter()
            .any(|l| l.kind == DiffLineKind::Addition && l.content.contains("new_line"))
    );
    assert!(all_lines.iter().any(|l| l.kind == DiffLineKind::Context));
}
