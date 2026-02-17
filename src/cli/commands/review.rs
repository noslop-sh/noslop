//! Review management commands

use std::path::PathBuf;

use noslop::adapters::{
    ConventionAnalyzer, FileReviewStore, GitVersionControl, TomlCheckRepository,
};
use noslop::core::models::Review;
use noslop::core::ports::{
    CheckRepository, ReviewAnalyzer, ReviewContext, ReviewStore, VersionControl,
};
use noslop::core::services::ReviewPipeline;
use noslop::output::OutputMode;
use serde::Serialize;

use crate::cli::app::ReviewAction;

/// Handle review subcommands
pub fn review(action: ReviewAction, mode: OutputMode) -> anyhow::Result<()> {
    let store = FileReviewStore::new();

    match action {
        ReviewAction::Run { base, head, check } => run_review(&base, &head, check, mode),
        ReviewAction::Start { base, head } => start_review(&store, &base, &head, mode),
        ReviewAction::List { open } => list_reviews(&store, open, mode),
        ReviewAction::Show { id } => show_review(&store, &id, mode),
        ReviewAction::Close { id } => close_review(&store, &id, mode),
    }
}

/// Build a `ReviewContext` from VCS data
fn build_review_context(
    vcs: &GitVersionControl,
    base: &str,
    head: &str,
) -> anyhow::Result<ReviewContext> {
    let file_diffs = vcs.diff_between(base, head)?;
    let changed_files: Vec<String> =
        file_diffs.iter().map(|fd| fd.path.to_string_lossy().to_string()).collect();

    // Build a unified diff string from the file diffs
    let mut diff_text = String::new();
    for fd in &file_diffs {
        diff_text.push_str(&format!("--- a/{}\n+++ b/{}\n", fd.path.display(), fd.path.display()));
        for hunk in &fd.hunks {
            diff_text.push_str(&format!(
                "@@ -{},{} +{},{} @@\n",
                hunk.old_start, hunk.old_lines, hunk.new_start, hunk.new_lines,
            ));
            for line in &hunk.lines {
                let prefix = match line.kind {
                    noslop::DiffLineKind::Context => ' ',
                    noslop::DiffLineKind::Addition => '+',
                    noslop::DiffLineKind::Deletion => '-',
                };
                diff_text.push(prefix);
                diff_text.push_str(&line.content);
                if !line.content.ends_with('\n') {
                    diff_text.push('\n');
                }
            }
        }
    }

    let repo_root = vcs.repo_root().unwrap_or_else(|_| PathBuf::from("."));

    Ok(ReviewContext {
        diff: diff_text,
        changed_files,
        repo_root,
        commit_range: Some(format!("{base}..{head}")),
        base: base.to_string(),
        head: head.to_string(),
    })
}

fn run_review(base: &str, head: &str, check: bool, mode: OutputMode) -> anyhow::Result<()> {
    let vcs = GitVersionControl::current_dir()?;
    let store = FileReviewStore::new();
    let check_repo = TomlCheckRepository::current_dir()?;

    // Load checks and build convention analyzer
    let checks = check_repo.list()?;
    let convention_analyzer = ConventionAnalyzer::new(checks);

    // Build the analyzer list
    let analyzers: Vec<Box<dyn ReviewAnalyzer>> = vec![Box::new(convention_analyzer)];

    // Build context from VCS
    let context = build_review_context(&vcs, base, head)?;

    // Run pipeline
    let pipeline = ReviewPipeline::new(analyzers);
    let review = pipeline.run(&context)?;

    // Save
    let id = review.id.clone();
    let total_findings = review.findings.len();
    let blocking_count = review.blocking_findings().len();
    let is_blocked = review.is_blocked();
    store.save(&review)?;

    // Output
    if mode == OutputMode::Json {
        println!(
            "{}",
            serde_json::json!({
                "success": true,
                "review_id": id,
                "base": base,
                "head": head,
                "findings": total_findings,
                "blocking": blocking_count,
                "blocked": is_blocked,
            })
        );
    } else {
        println!("Review: {id}");
        println!("  Range: {base}..{head}");
        println!("  Findings: {total_findings}");
        println!("  Blocking: {blocking_count}");
        if is_blocked {
            println!("\n  BLOCKED: {blocking_count} unresolved blocking finding(s)");
        } else {
            println!("\n  No blocking findings.");
        }
    }

    if check && is_blocked {
        std::process::exit(1);
    }

    Ok(())
}

fn start_review(
    store: &FileReviewStore,
    base: &str,
    head: &str,
    mode: OutputMode,
) -> anyhow::Result<()> {
    let review = Review::new(base, head);
    let id = review.id.clone();
    store.save(&review)?;

    if mode == OutputMode::Json {
        println!(
            "{}",
            serde_json::json!({
                "success": true,
                "review_id": id,
                "base": base,
                "head": head
            })
        );
    } else {
        println!("Started review: {id}");
        println!("  Base: {base}");
        println!("  Head: {head}");
    }
    Ok(())
}

#[derive(Serialize)]
struct ReviewListItem {
    id: String,
    base: String,
    head: String,
    status: String,
    findings: usize,
    open_findings: usize,
}

fn list_reviews(store: &FileReviewStore, open_only: bool, mode: OutputMode) -> anyhow::Result<()> {
    let reviews = if open_only {
        store.list_open()?
    } else {
        store.list_all()?
    };

    if mode == OutputMode::Json {
        let items: Vec<ReviewListItem> = reviews
            .iter()
            .map(|r| ReviewListItem {
                id: r.id.clone(),
                base: r.base.clone(),
                head: r.head.clone(),
                status: format!("{:?}", r.status).to_lowercase(),
                findings: r.findings.len(),
                open_findings: r.open_findings().len(),
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&items)?);
    } else if reviews.is_empty() {
        println!("No reviews found.");
    } else {
        println!("Reviews:\n");
        for r in &reviews {
            let open = r.open_findings().len();
            let total = r.findings.len();
            println!(
                "  [{}] {} ({open} open / {total} total)",
                format!("{:?}", r.status).to_uppercase(),
                r.id,
            );
            println!(
                "       {}..{}",
                &r.base[..7.min(r.base.len())],
                &r.head[..7.min(r.head.len())]
            );
        }
    }
    Ok(())
}

fn show_review(store: &FileReviewStore, id: &str, mode: OutputMode) -> anyhow::Result<()> {
    let review = store.load(id)?.ok_or_else(|| anyhow::anyhow!("Review not found: {id}"))?;

    if mode == OutputMode::Json {
        println!("{}", serde_json::to_string_pretty(&review)?);
    } else {
        println!("Review: {}", review.id);
        println!("Status: {:?}", review.status);
        println!(
            "Range:  {}..{}",
            &review.base[..7.min(review.base.len())],
            &review.head[..7.min(review.head.len())]
        );
        println!();

        if review.findings.is_empty() {
            println!("No findings.");
        } else {
            println!("Findings:");
            for f in &review.findings {
                let status = if f.is_open() { "OPEN" } else { "RESOLVED" };
                println!("  [{status}] {} - {}", f.id, f.target);
                println!("        {}", f.message);
            }
        }
    }
    Ok(())
}

fn close_review(store: &FileReviewStore, id: &str, mode: OutputMode) -> anyhow::Result<()> {
    let mut review = store.load(id)?.ok_or_else(|| anyhow::anyhow!("Review not found: {id}"))?;

    // Check for unresolved blocking findings
    let blocking_count = review.blocking_findings().len();
    if blocking_count > 0 {
        anyhow::bail!(
            "Cannot close review with {blocking_count} blocking finding(s). Resolve all blocking findings first.",
        );
    }

    review.close();
    store.save(&review)?;

    if mode == OutputMode::Json {
        println!(
            "{}",
            serde_json::json!({
                "success": true,
                "review_id": id
            })
        );
    } else {
        println!("Closed review: {id}");
    }
    Ok(())
}
