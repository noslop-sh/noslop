//! Curate command - evidence-based rulebook maintenance report
//!
//! Reads the same inputs as `noslop stats` and prints only what needs a
//! human decision: dead checks to prune, always-stamped checks to reword.

use crate::{git, noslop_file};
use noslop::adapters::{ledger, telemetry};
use noslop::core::services::curate::{CurateAction, recommend};
use noslop::core::services::stats::compute;
use noslop::output::OutputMode;

/// Render curation recommendations.
pub fn curate(markdown: bool, mode: OutputMode) -> anyhow::Result<()> {
    let checks = noslop_file::load_all_checks()?;
    if checks.is_empty() {
        println!("No checks defined. Run 'noslop discover' or 'noslop check add' first.");
        return Ok(());
    }

    let events = telemetry::load_events()?;
    let acks = ledger::load_all()?;
    let tracked = git::staged::tracked_files()?;
    let root = std::env::current_dir()?;

    let rows = compute(&checks, &events, &acks, &tracked, &root);
    let recs = recommend(&rows);

    if mode == OutputMode::Json {
        println!("{}", serde_json::json!({ "recommendations": recs }));
        return Ok(());
    }

    if recs.is_empty() {
        println!("Rulebook is healthy: no prune or reword candidates.");
        println!("({} check(s) reviewed against local telemetry and the ledger.)", rows.len());
        return Ok(());
    }

    if markdown {
        println!("| Check | Action | Why |");
        println!("|---|---|---|");
        for r in &recs {
            let action = match r.action {
                CurateAction::Prune => "prune",
                CurateAction::Reword => "reword",
            };
            println!("| {} (`{}`) | {} | {} |", r.check_id, r.target, action, r.reason);
        }
    } else {
        for r in &recs {
            let action = match r.action {
                CurateAction::Prune => "PRUNE ",
                CurateAction::Reword => "REWORD",
            };
            println!("{action} {:<10} {}", r.check_id, r.reason);
        }
        println!(
            "\n{} recommendation(s). Apply with 'noslop check remove <id>' or edit .noslop.toml.",
            recs.len()
        );
    }
    Ok(())
}
