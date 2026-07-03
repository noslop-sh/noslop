//! Stats command - per-check metrics from local telemetry and the ledger
//!
//! Shows which checks fire, which get real self-correction, which get
//! rubber-stamped, and which are dead (target matches nothing). This is
//! the evidence `noslop curate` acts on.

use crate::{git, noslop_file};
use noslop::adapters::{ledger, telemetry};
use noslop::core::services::stats::{CheckStats, compute};
use noslop::output::OutputMode;

/// Render per-check stats.
pub fn stats(markdown: bool, mode: OutputMode) -> anyhow::Result<()> {
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

    if mode == OutputMode::Json {
        println!("{}", serde_json::json!({ "checks": rows }));
        return Ok(());
    }

    if markdown {
        render_markdown(&rows);
    } else {
        render_human(&rows);
    }
    Ok(())
}

fn render_human(rows: &[CheckStats]) {
    println!(
        "{:<10} {:<30} {:>5} {:>5} {:>9} {:>7}  FLAGS",
        "CHECK", "TARGET", "FIRES", "ACKS", "SELF-FIX", "STAMPS"
    );
    for r in rows {
        let mut flags = Vec::new();
        if r.dead_target {
            flags.push("DEAD-TARGET");
        }
        if r.rubber_stamps > 0 && r.self_corrected == 0 && r.acks > 0 {
            flags.push("ALWAYS-STAMPED");
        }
        let target = if r.target.len() > 28 {
            format!("{}…", &r.target[..27])
        } else {
            r.target.clone()
        };
        println!(
            "{:<10} {:<30} {:>5} {:>5} {:>9} {:>7}  {}",
            r.id,
            target,
            r.fires,
            r.acks,
            r.self_corrected,
            r.rubber_stamps,
            flags.join(", ")
        );
    }

    let dead = rows.iter().filter(|r| r.dead_target).count();
    let stamped = rows
        .iter()
        .filter(|r| r.rubber_stamps > 0 && r.self_corrected == 0 && r.acks > 0)
        .count();
    println!("\n{} check(s); {dead} dead target(s), {stamped} always-stamped.", rows.len());
    if dead + stamped > 0 {
        println!("Consider pruning or rewording flagged checks (noslop curate, coming next).");
    }
    println!(
        "\nNote: fires/stamps are local telemetry (this clone only); acks come from the tracked ledger."
    );
}

fn render_markdown(rows: &[CheckStats]) {
    println!("| Check | Target | Fires | Acks | Self-fix | Stamps | Flags |");
    println!("|---|---|---:|---:|---:|---:|---|");
    for r in rows {
        let mut flags = Vec::new();
        if r.dead_target {
            flags.push("dead-target");
        }
        if r.rubber_stamps > 0 && r.self_corrected == 0 && r.acks > 0 {
            flags.push("always-stamped");
        }
        println!(
            "| {} | `{}` | {} | {} | {} | {} | {} |",
            r.id,
            r.target,
            r.fires,
            r.acks,
            r.self_corrected,
            r.rubber_stamps,
            flags.join(", ")
        );
    }
}
