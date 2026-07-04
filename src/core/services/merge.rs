//! Merging cloud-distributed checks with repo-local ones
//!
//! The org set is a floor: local config may add checks or raise severity
//! but never remove or weaken an org check (GitHub rulesets / Semgrep
//! Global Policy shape). Duplicate ids dedupe strictest-wins.

use crate::core::models::{Check, Severity};

const fn rank(severity: Severity) -> u8 {
    match severity {
        Severity::Info => 0,
        Severity::Warn => 1,
        Severity::Block => 2,
    }
}

/// Merge remote (org) checks into local ones, per matched file.
///
/// Both inputs are `(check, file)` pairs as produced by target matching.
/// For the same `(id, file)` the stricter severity survives — a local
/// entry can tighten an org check but never soften it.
#[must_use]
pub fn merge_checks(
    local: Vec<(Check, String)>,
    remote: Vec<(Check, String)>,
) -> Vec<(Check, String)> {
    let mut merged: Vec<(Check, String)> = remote;

    for (check, file) in local {
        if let Some(existing) = merged
            .iter_mut()
            .find(|(existing, existing_file)| existing.id == check.id && *existing_file == file)
        {
            if rank(check.severity) > rank(existing.0.severity) {
                existing.0 = check;
            }
        } else {
            merged.push((check, file));
        }
    }

    merged.sort_by(|a, b| (&a.0.id, &a.1).cmp(&(&b.0.id, &b.1)));
    merged
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(id: &str, severity: Severity) -> Check {
        Check::new(Some(id.into()), "src/**".into(), "ask".into(), severity)
    }

    #[test]
    fn local_checks_add_alongside_org_checks() {
        let merged = merge_checks(
            vec![(check("LOC-1", Severity::Block), "a.rs".into())],
            vec![(check("ORG-1", Severity::Block), "a.rs".into())],
        );
        assert_eq!(merged.len(), 2);
    }

    #[test]
    fn duplicate_ids_dedupe_strictest_wins() {
        // Local tightens: warn -> block survives as block
        let merged = merge_checks(
            vec![(check("ORG-1", Severity::Block), "a.rs".into())],
            vec![(check("ORG-1", Severity::Warn), "a.rs".into())],
        );
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].0.severity, Severity::Block);

        // Local cannot weaken: block stays block even when local says info
        let merged = merge_checks(
            vec![(check("ORG-1", Severity::Info), "a.rs".into())],
            vec![(check("ORG-1", Severity::Block), "a.rs".into())],
        );
        assert_eq!(merged[0].0.severity, Severity::Block);
    }

    #[test]
    fn same_id_on_different_files_stays_distinct() {
        let merged = merge_checks(
            vec![(check("ORG-1", Severity::Block), "b.rs".into())],
            vec![(check("ORG-1", Severity::Block), "a.rs".into())],
        );
        assert_eq!(merged.len(), 2);
    }
}
