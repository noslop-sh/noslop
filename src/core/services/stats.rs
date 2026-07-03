//! Stats service - per-check metrics from fire events and acknowledgments
//!
//! Pure join logic, no I/O. The rubber-stamp detector is the core idea:
//! a fire event and an acknowledgment both carry the staged-tree oid
//! (`git write-tree`), so an ack whose oid still matches a fire oid means
//! the check was acknowledged without changing anything — theater, not
//! verification. A differing oid means files changed between block and
//! ack: the guidance did its job.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::core::models::{Acknowledgment, Check, CheckFireEvent};
use crate::core::services::matcher::matches_target;

/// Per-check metrics
#[derive(Debug, Clone, serde::Serialize)]
pub struct CheckStats {
    /// Check id
    pub id: String,
    /// Target glob
    pub target: String,
    /// Severity as configured
    pub severity: String,
    /// Distinct staged states in which the check fired
    pub fires: usize,
    /// Total acknowledgments recorded
    pub acks: usize,
    /// Acks where the staged tree changed after the fire (guidance worked)
    pub self_corrected: usize,
    /// Acks where nothing changed between fire and ack (theater)
    pub rubber_stamps: usize,
    /// Most recent fire timestamp (RFC 3339)
    pub last_fired: Option<String>,
    /// No tracked file matches the target glob anymore
    pub dead_target: bool,
}

/// Compute per-check stats.
///
/// `tracked_files` is the repository file list used for dead-target
/// detection; `repo_root` anchors glob matching.
#[must_use]
pub fn compute(
    checks: &[Check],
    events: &[CheckFireEvent],
    acks: &[Acknowledgment],
    tracked_files: &[String],
    repo_root: &Path,
) -> Vec<CheckStats> {
    let mut events_by_check: BTreeMap<&str, Vec<&CheckFireEvent>> = BTreeMap::new();
    for e in events {
        events_by_check.entry(&e.check_id).or_default().push(e);
    }
    let mut acks_by_check: BTreeMap<&str, Vec<&Acknowledgment>> = BTreeMap::new();
    for a in acks {
        acks_by_check.entry(&a.check_id).or_default().push(a);
    }

    checks
        .iter()
        .map(|check| {
            let check_events = events_by_check.get(check.id.as_str());
            let check_acks = acks_by_check.get(check.id.as_str());

            // Distinct staged states, not raw invocations: re-running
            // `noslop check` on the same index must not inflate fires.
            let fire_oids: BTreeSet<&str> =
                check_events.into_iter().flatten().map(|e| e.tree_oid.as_str()).collect();

            let last_fired = check_events
                .into_iter()
                .flatten()
                .map(|e| e.created_at.as_str())
                .max()
                .map(str::to_string);

            let mut self_corrected = 0;
            let mut rubber_stamps = 0;
            for ack in check_acks.into_iter().flatten() {
                // Only categorizable when both sides carry a fingerprint
                if let Some(oid) = &ack.tree_oid {
                    if fire_oids.is_empty() {
                        continue; // ack without a recorded fire: no verdict
                    }
                    if fire_oids.contains(oid.as_str()) {
                        rubber_stamps += 1;
                    } else {
                        self_corrected += 1;
                    }
                }
            }

            let dead_target = !tracked_files
                .iter()
                .any(|f| matches_target(&check.target, f, repo_root, repo_root));

            CheckStats {
                id: check.id.clone(),
                target: check.target.clone(),
                severity: check.severity.to_string(),
                fires: fire_oids.len(),
                acks: check_acks.map_or(0, Vec::len),
                self_corrected,
                rubber_stamps,
                last_fired,
                dead_target,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::models::Severity;

    fn check(id: &str, target: &str) -> Check {
        Check::new(Some(id.to_string()), target.to_string(), format!("msg {id}"), Severity::Block)
    }

    fn event(check_id: &str, oid: &str, at: &str) -> CheckFireEvent {
        let mut e = CheckFireEvent::new(
            check_id.to_string(),
            "src/main.rs".to_string(),
            Severity::Block,
            "claude-code".to_string(),
            oid.to_string(),
        );
        e.created_at = at.to_string();
        e
    }

    fn ack(check_id: &str, oid: Option<&str>) -> Acknowledgment {
        Acknowledgment::new(check_id.to_string(), "did it".to_string(), "claude-code".to_string())
            .with_tree_oid(oid.map(str::to_string))
    }

    #[test]
    fn fires_dedupe_by_tree_oid() {
        let checks = vec![check("C-1", "src/**/*.rs")];
        // Same staged state checked 3 times, then a new state
        let events = vec![
            event("C-1", "oid-a", "2026-01-01T00:00:00Z"),
            event("C-1", "oid-a", "2026-01-01T00:01:00Z"),
            event("C-1", "oid-a", "2026-01-01T00:02:00Z"),
            event("C-1", "oid-b", "2026-01-02T00:00:00Z"),
        ];
        let stats =
            compute(&checks, &events, &[], &["src/main.rs".to_string()], Path::new("/repo"));
        assert_eq!(stats[0].fires, 2);
        assert_eq!(stats[0].last_fired.as_deref(), Some("2026-01-02T00:00:00Z"));
    }

    #[test]
    fn rubber_stamp_vs_self_correction() {
        let checks = vec![check("C-1", "src/**/*.rs")];
        let events = vec![event("C-1", "oid-a", "2026-01-01T00:00:00Z")];
        // First ack with the SAME oid as the fire: nothing changed = stamp.
        // Second ack with a DIFFERENT oid: files changed = self-corrected.
        let acks = vec![ack("C-1", Some("oid-a")), ack("C-1", Some("oid-b"))];
        let stats =
            compute(&checks, &events, &acks, &["src/main.rs".to_string()], Path::new("/repo"));
        assert_eq!(stats[0].rubber_stamps, 1);
        assert_eq!(stats[0].self_corrected, 1);
        assert_eq!(stats[0].acks, 2);
    }

    #[test]
    fn legacy_acks_without_oid_are_counted_but_not_judged() {
        let checks = vec![check("C-1", "src/**/*.rs")];
        let events = vec![event("C-1", "oid-a", "2026-01-01T00:00:00Z")];
        let acks = vec![ack("C-1", None)];
        let stats =
            compute(&checks, &events, &acks, &["src/main.rs".to_string()], Path::new("/repo"));
        assert_eq!(stats[0].acks, 1);
        assert_eq!(stats[0].rubber_stamps, 0);
        assert_eq!(stats[0].self_corrected, 0);
    }

    #[test]
    fn dead_target_detected() {
        let checks = vec![check("C-1", "migrations/**/*.py"), check("C-2", "src/**/*.rs")];
        let stats = compute(&checks, &[], &[], &["src/main.rs".to_string()], Path::new("/repo"));
        assert!(stats[0].dead_target);
        assert!(!stats[1].dead_target);
    }
}
