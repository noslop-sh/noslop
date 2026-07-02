//! Discover command - turn existing rules files into check proposals
//!
//! `noslop discover` scans CLAUDE.md / AGENTS.md / `.cursor/rules` and
//! stages atomic check proposals in `.noslop/proposals.toml`; `--mine`
//! additionally mines PR review history through the developer's own agent
//! CLI. Nothing is enforced until a human accepts a proposal via
//! `noslop discover --review`, which promotes it to a check in `.noslop.toml`.

use std::io::{BufRead, Write};

use crate::noslop_file;
use noslop::adapters::runner::Runner;
use noslop::adapters::{gh, proposals, rules};
use noslop::core::models::Proposal;
use noslop::core::services::{discovery, mining};
use noslop::output::OutputMode;

/// Where raw runner output is saved when it cannot be parsed.
const RAW_OUTPUT_PATH: &str = ".noslop/proposals-raw.txt";

/// Comment chunk budget per runner invocation (bytes).
const CHUNK_BYTES: usize = 150_000;

/// Run discovery (scan/mine) or the review flow.
pub fn discover(
    review: bool,
    mine: bool,
    from_file: Option<&str>,
    mode: OutputMode,
) -> anyhow::Result<()> {
    if review {
        review_proposals()
    } else if mine || from_file.is_some() {
        mine_history(from_file, mode)
    } else {
        scan(mode)
    }
}

/// Mine PR review history into proposals via the local agent CLI.
fn mine_history(from_file: Option<&str>, mode: OutputMode) -> anyhow::Result<()> {
    let root = std::env::current_dir()?;

    // Runner: [discover] runner config wins, then PATH detection
    let config_runner = load_runner_config(&root);
    let Some(runner) = Runner::detect(config_runner.as_deref()) else {
        anyhow::bail!(
            "no agent CLI found. Install one (e.g. claude) or set it in .noslop.toml:\n\n  \
             [discover]\n  runner = \"claude -p\"\n\nThe command must read the prompt on stdin \
             and print TOML on stdout."
        );
    };

    // Comments: JSONL export or live GitHub history
    let (comments, source) = if let Some(path) = from_file {
        (read_comments_jsonl(path)?, format!("mining:{path}"))
    } else {
        let slug = gh::repo_slug()?;
        println!("Fetching review comments for {slug}...");
        let comments = gh::fetch_review_comments(&slug, 6)?;
        (comments, format!("mining:{slug}"))
    };

    if comments.is_empty() {
        println!("No human review comments found to mine.");
        return Ok(());
    }

    let repo_label = source.trim_start_matches("mining:").to_string();
    let chunks = mining::chunk_comments(comments, CHUNK_BYTES);
    println!(
        "Mining {} comment chunk(s) via '{}' (this can take a few minutes)...",
        chunks.len(),
        runner.describe()
    );

    let mut chunk_outputs = Vec::new();
    for (i, chunk) in chunks.iter().enumerate() {
        let prompt = mining::mining_prompt(&repo_label, chunk);
        let output = run_with_retry(&runner, &prompt, &source)?;
        println!("  chunk {}/{}: ok", i + 1, chunks.len());
        chunk_outputs.push(output);
    }

    // Single chunk: use directly. Multiple: consolidate with a merge pass.
    let mined = if chunk_outputs.len() == 1 {
        chunk_outputs.pop().expect("one chunk output")
    } else {
        let prompt = mining::merge_prompt(&repo_label, &chunk_outputs);
        run_with_retry(&runner, &prompt, &source)?
    };

    stage_fresh(&mined, &source, &root, mode)
}

/// Run a prompt and return its raw output, retrying once with error
/// feedback when the output fails to parse. Raw text is returned (not
/// proposals) so multi-chunk outputs can be fed to the merge pass verbatim.
fn run_with_retry(runner: &Runner, prompt: &str, source: &str) -> anyhow::Result<String> {
    let output = runner.run(prompt)?;
    match mining::parse_proposals(&output, source) {
        Ok(_) => Ok(output),
        Err(first_err) => {
            println!("  runner output unusable ({first_err}); retrying once...");
            let retry = mining::retry_prompt(prompt, &first_err);
            let output2 = runner.run(&retry)?;
            match mining::parse_proposals(&output2, source) {
                Ok(_) => Ok(output2),
                Err(second_err) => {
                    std::fs::write(
                        RAW_OUTPUT_PATH,
                        format!("{output}\n\n---RETRY---\n\n{output2}"),
                    )?;
                    anyhow::bail!(
                        "mining failed after retry ({second_err}); raw output saved to {RAW_OUTPUT_PATH}"
                    )
                },
            }
        },
    }
}

/// Parse mined output and stage the deduped fresh proposals.
fn stage_fresh(
    mined_output: &str,
    source: &str,
    root: &std::path::Path,
    mode: OutputMode,
) -> anyhow::Result<()> {
    let mined = mining::parse_proposals(mined_output, source)?;

    let mut known: Vec<String> = existing_check_keys(root);
    known.extend(proposals::load_rejected_keys()?);
    let staged = proposals::load()?;
    known.extend(staged.iter().map(Proposal::dedupe_key));

    let fresh = discovery::dedupe(mined, &known);
    let fresh_count = fresh.len();

    let mut all = staged;
    all.extend(fresh);
    proposals::save(&all)?;

    if mode == OutputMode::Json {
        println!(
            "{}",
            serde_json::json!({
                "new_proposals": fresh_count,
                "pending_review": all.len(),
                "proposals": all,
            })
        );
        return Ok(());
    }

    println!("\n{fresh_count} new proposal(s) mined, {} pending review.", all.len());
    if !all.is_empty() {
        println!("\nRun 'noslop discover --review' to accept, edit, or reject them.");
    }
    Ok(())
}

/// Read review comments from a JSONL export (objects with `path` and `body`).
fn read_comments_jsonl(path: &str) -> anyhow::Result<Vec<mining::ReviewComment>> {
    let content =
        std::fs::read_to_string(path).map_err(|e| anyhow::anyhow!("cannot read {path}: {e}"))?;
    let mut comments = Vec::new();
    for line in content.lines().filter(|l| !l.trim().is_empty()) {
        match serde_json::from_str::<mining::ReviewComment>(line) {
            Ok(c) => comments.push(c),
            Err(_) => continue, // tolerate malformed lines in exports
        }
    }
    Ok(comments)
}

/// The `[discover] runner` value from the repo root .noslop.toml, if set.
fn load_runner_config(root: &std::path::Path) -> Option<String> {
    let path = root.join(".noslop.toml");
    if !path.exists() {
        return None;
    }
    noslop_file::load_file(&path).ok().and_then(|f| f.discover.runner)
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
