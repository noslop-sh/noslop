//! Review management commands

use noslop::adapters::FileReviewStore;
use noslop::core::models::Review;
use noslop::core::ports::ReviewStore;
use noslop::output::OutputMode;
use serde::Serialize;

use crate::cli::app::ReviewAction;

/// Handle review subcommands
pub fn review(action: ReviewAction, mode: OutputMode) -> anyhow::Result<()> {
    let store = FileReviewStore::new();

    match action {
        ReviewAction::Start { base, head } => start_review(&store, &base, &head, mode),
        ReviewAction::List { open } => list_reviews(&store, open, mode),
        ReviewAction::Show { id } => show_review(&store, &id, mode),
        ReviewAction::Close { id } => close_review(&store, &id, mode),
    }
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
