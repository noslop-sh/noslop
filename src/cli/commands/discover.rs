//! Discover command - turn existing rules files into check proposals
//!
//! `noslop discover` scans CLAUDE.md / AGENTS.md / `.cursor/rules` and
//! stages atomic check proposals in `.noslop/proposals.toml`. Nothing is
//! enforced until a human accepts a proposal via `noslop discover --review`,
//! which promotes it to a check in `.noslop.toml`.

use std::io::{BufRead, Write};

use crate::noslop_file;
use noslop::adapters::{proposals, rules};
use noslop::core::models::Proposal;
use noslop::core::services::discovery;
use noslop::output::OutputMode;

/// Run discovery (scan) or the review flow.
pub fn discover(review: bool, mode: OutputMode) -> anyhow::Result<()> {
    if review {
        review_proposals()
    } else {
        scan(mode)
    }
}

/// Scan rules files, dedupe against known checks and staged proposals.
fn scan(mode: OutputMode) -> anyhow::Result<()> {
    let root = std::env::current_dir()?;
    let files = rules::find_rules_files(&root)?;

    if files.is_empty() {
        println!(
            "No rules files found (looked for CLAUDE.md, AGENTS.md, .cursor/rules/, .claude/rules/)."
        );
        return Ok(());
    }

    let mut extracted = Vec::new();
    for f in &files {
        let props = match f.kind {
            rules::RulesKind::Markdown => discovery::extract_from_markdown(&f.content, &f.name),
            rules::RulesKind::Mdc => discovery::extract_from_mdc(&f.content, &f.name),
        };
        extracted.extend(props);
    }

    // Known keys: existing checks + staged proposals + past rejections
    let mut known: Vec<String> = existing_check_keys(&root);
    known.extend(proposals::load_rejected_keys()?);
    let staged = proposals::load()?;
    known.extend(staged.iter().map(Proposal::dedupe_key));

    let fresh = discovery::dedupe(extracted, &known);
    let fresh_count = fresh.len();

    let mut all = staged;
    all.extend(fresh);
    proposals::save(&all)?;

    if mode == OutputMode::Json {
        println!(
            "{}",
            serde_json::json!({
                "scanned": files.iter().map(|f| f.name.clone()).collect::<Vec<_>>(),
                "new_proposals": fresh_count,
                "pending_review": all.len(),
                "proposals": all,
            })
        );
        return Ok(());
    }

    println!("Scanned {} rules file(s):", files.len());
    for f in &files {
        println!("  {}", f.name);
    }
    println!("\n{fresh_count} new proposal(s), {} pending review.", all.len());
    if !all.is_empty() {
        println!("\nRun 'noslop discover --review' to accept, edit, or reject them.");
    }

    Ok(())
}

/// Interactive review: accept/edit/skip/reject each staged proposal.
fn review_proposals() -> anyhow::Result<()> {
    let staged = proposals::load()?;
    if staged.is_empty() {
        println!("No proposals to review. Run 'noslop discover' first.");
        return Ok(());
    }

    let stdin = std::io::stdin();
    let mut lines = stdin.lock().lines();
    let mut remaining: Vec<Proposal> = Vec::new();
    let mut accepted = 0_usize;
    let mut rejected = 0_usize;
    let total = staged.len();

    let mut queue = staged.into_iter().enumerate();
    while let Some((i, p)) = queue.next() {
        println!("\n[{}/{}] from {}", i + 1, total, p.source);
        println!("  target:   {}", p.target);
        println!("  message:  {}", p.message);
        println!("  severity: {}", p.severity);
        print!("Accept [a] / Edit [e] / Skip [s] / Reject [r] / Quit [q]? ");
        std::io::stdout().flush()?;

        let choice = lines.next().transpose()?.unwrap_or_else(|| "q".to_string());
        match choice.trim().to_lowercase().as_str() {
            "a" => {
                let id = noslop_file::add_check(&p.target, &p.message, &p.severity.to_string())?;
                println!("  Added as {id}");
                accepted += 1;
            },
            "e" => {
                let target = prompt_default(&mut lines, "target", &p.target)?;
                let message = prompt_default(&mut lines, "message", &p.message)?;
                let severity = prompt_default(&mut lines, "severity", &p.severity.to_string())?;
                let id = noslop_file::add_check(&target, &message, &severity)?;
                println!("  Added as {id}");
                accepted += 1;
            },
            "r" => {
                proposals::append_rejected_keys(&[p.dedupe_key()])?;
                rejected += 1;
            },
            "q" => {
                // Keep this one and everything not yet seen
                remaining.push(p);
                remaining.extend(queue.map(|(_, rest)| rest));
                break;
            },
            // Anything else (including "s") keeps it for later
            _ => remaining.push(p),
        }
    }

    proposals::save(&remaining)?;

    println!(
        "\nReview done: {accepted} accepted, {rejected} rejected, {} left pending.",
        remaining.len()
    );
    Ok(())
}

/// Prompt with a default value; empty input keeps the default.
fn prompt_default(
    lines: &mut std::io::Lines<std::io::StdinLock<'_>>,
    label: &str,
    default: &str,
) -> anyhow::Result<String> {
    print!("  {label} [{default}]: ");
    std::io::stdout().flush()?;
    let input = lines.next().transpose()?.unwrap_or_default();
    let trimmed = input.trim();
    if trimmed.is_empty() {
        Ok(default.to_string())
    } else {
        Ok(trimmed.to_string())
    }
}

/// Dedupe keys of every check already defined in reachable .noslop.toml files.
fn existing_check_keys(root: &std::path::Path) -> Vec<String> {
    let mut keys = Vec::new();
    for path in noslop_file::find_noslop_files(root) {
        if let Ok(file) = noslop_file::load_file(&path) {
            for entry in &file.checks {
                let p = Proposal {
                    target: entry.target.clone(),
                    message: entry.message.clone(),
                    severity: entry.severity.parse().unwrap_or_default(),
                    source: String::new(),
                };
                keys.push(p.dedupe_key());
            }
        }
    }
    keys
}
