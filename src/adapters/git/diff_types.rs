//! VCS-internal data types for diff and commit information
//!
//! This module re-exports the VCS port types from `core::ports::vcs_types`
//! for use within the git adapter. The canonical definitions live in the
//! core ports layer since they are part of the `VersionControl` trait interface.

pub use crate::core::ports::vcs_types::*;

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn diff_status_display() {
        assert_eq!(DiffStatus::Added.to_string(), "added");
        assert_eq!(DiffStatus::Modified.to_string(), "modified");
        assert_eq!(DiffStatus::Deleted.to_string(), "deleted");
        assert_eq!(DiffStatus::Renamed.to_string(), "renamed");
    }

    #[test]
    fn diff_stat_display() {
        let stat = DiffStat {
            files_changed: 3,
            insertions: 42,
            deletions: 7,
        };
        assert_eq!(stat.to_string(), "3 file(s) changed, 42 insertion(s), 7 deletion(s)");
    }

    #[test]
    fn diff_stat_empty() {
        let stat = DiffStat::empty();
        assert_eq!(stat.files_changed, 0);
        assert_eq!(stat.insertions, 0);
        assert_eq!(stat.deletions, 0);
    }

    #[test]
    fn file_diff_construction() {
        let diff = FileDiff {
            path: PathBuf::from("src/main.rs"),
            old_path: None,
            status: DiffStatus::Modified,
            hunks: vec![DiffHunk {
                old_start: 1,
                old_lines: 3,
                new_start: 1,
                new_lines: 5,
                lines: vec![
                    DiffLine {
                        kind: DiffLineKind::Context,
                        content: "fn main() {".to_string(),
                    },
                    DiffLine {
                        kind: DiffLineKind::Deletion,
                        content: "    println!(\"old\");".to_string(),
                    },
                    DiffLine {
                        kind: DiffLineKind::Addition,
                        content: "    println!(\"new\");".to_string(),
                    },
                ],
            }],
        };
        assert_eq!(diff.status, DiffStatus::Modified);
        assert_eq!(diff.hunks.len(), 1);
        assert_eq!(diff.hunks[0].lines.len(), 3);
    }

    #[test]
    fn file_diff_renamed() {
        let diff = FileDiff {
            path: PathBuf::from("src/new_name.rs"),
            old_path: Some(PathBuf::from("src/old_name.rs")),
            status: DiffStatus::Renamed,
            hunks: vec![],
        };
        assert_eq!(diff.status, DiffStatus::Renamed);
        assert_eq!(diff.old_path.as_deref(), Some(std::path::Path::new("src/old_name.rs")));
    }

    #[test]
    fn commit_info_construction() {
        let info = CommitInfo {
            short_sha: "abc1234".to_string(),
            sha: "abc1234def5678".to_string(),
            summary: "Fix bug".to_string(),
            message: "Fix bug\n\nDetailed description".to_string(),
            author: "Test Author".to_string(),
            timestamp: 1_700_000_000,
        };
        assert_eq!(info.short_sha, "abc1234");
        assert_eq!(info.summary, "Fix bug");
    }

    #[test]
    fn diff_line_kind_equality() {
        assert_eq!(DiffLineKind::Context, DiffLineKind::Context);
        assert_ne!(DiffLineKind::Addition, DiffLineKind::Deletion);
    }

    #[test]
    fn diff_status_copy_semantics() {
        let status = DiffStatus::Added;
        let copied = status;
        assert_eq!(status, copied);
    }
}
