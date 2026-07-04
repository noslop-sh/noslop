//! Curate service - evidence-based rulebook recommendations
//!
//! Pure logic over [`CheckStats`]: which checks to prune, which to reword.
//! A no-action answer can be legitimate verification ("checked, nothing to
//! fix"), so rewording is only suggested once stamps repeat with zero
//! action rates — the signal is the rate, never a single ack.

use serde::Serialize;

use super::stats::CheckStats;

/// Minimum repeated stamps (with zero action rates) before a check is
/// flagged as reword-worthy.
const NO_ACTION_THRESHOLD: usize = 2;

/// What to do with a check
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CurateAction {
    /// Remove the check
    Prune,
    /// Rewrite the message or retarget the glob
    Reword,
}

/// A recommendation for one check
#[derive(Debug, Clone, Serialize)]
pub struct Recommendation {
    /// Check id
    pub check_id: String,
    /// Target glob (context for the reader)
    pub target: String,
    /// Suggested action
    pub action: CurateAction,
    /// Evidence for the suggestion
    pub reason: String,
}

/// Produce recommendations from per-check stats.
///
/// Only actionable findings are returned; healthy checks are omitted.
#[must_use]
pub fn recommend(stats: &[CheckStats]) -> Vec<Recommendation> {
    let mut recs = Vec::new();

    for s in stats {
        if s.dead_target {
            recs.push(Recommendation {
                check_id: s.id.clone(),
                target: s.target.clone(),
                action: CurateAction::Prune,
                reason: "target matches no tracked file — the code this check guarded is gone"
                    .to_string(),
            });
            continue;
        }

        if s.acted == 0 && s.no_action >= NO_ACTION_THRESHOLD {
            recs.push(Recommendation {
                check_id: s.id.clone(),
                target: s.target.clone(),
                action: CurateAction::Reword,
                reason: format!(
                    "{} ack(s), {} stamp(s), zero action rates — the guidance never \
                     changes behavior; reword it to be actionable, scope it tighter, or prune",
                    s.acks, s.no_action
                ),
            });
        }
    }

    recs
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stat(
        id: &str,
        fires: usize,
        acks: usize,
        acted: usize,
        no_action: usize,
        dead_target: bool,
    ) -> CheckStats {
        CheckStats {
            id: id.to_string(),
            target: "src/**/*.rs".to_string(),
            severity: "block".to_string(),
            fires,
            acks,
            acted,
            no_action,
            last_fired: None,
            dead_target,
        }
    }

    #[test]
    fn dead_target_is_pruned() {
        let recs = recommend(&[stat("C-1", 0, 0, 0, 0, true)]);
        assert_eq!(recs.len(), 1);
        assert_eq!(recs[0].action, CurateAction::Prune);
    }

    #[test]
    fn repeated_stamps_without_corrections_reword() {
        let recs = recommend(&[stat("C-1", 3, 3, 0, 2, false)]);
        assert_eq!(recs.len(), 1);
        assert_eq!(recs[0].action, CurateAction::Reword);
    }

    #[test]
    fn single_stamp_is_tolerated() {
        // One no-change ack can be honest verification
        let recs = recommend(&[stat("C-1", 1, 1, 0, 1, false)]);
        assert!(recs.is_empty());
    }

    #[test]
    fn acting_checks_are_healthy() {
        let recs = recommend(&[stat("C-1", 5, 5, 3, 2, false)]);
        assert!(recs.is_empty());
    }
}
