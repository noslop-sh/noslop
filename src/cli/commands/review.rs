//! Review management commands

use noslop::adapters::FileReviewStore;
use noslop::core::models::{DiffPosition, Review};
use noslop::core::ports::ReviewStore;
use noslop::output::OutputMode;
use serde::Serialize;

use crate::cli::app::ReviewAction;

/// Handle review subcommands
pub fn review(action: ReviewAction, mode: OutputMode) -> anyhow::Result<()> {
    let store = FileReviewStore::new();

    match action {
        ReviewAction::Start { base, head } => start_review(&store, &base, &head, mode),
        ReviewAction::Comment {
            review_id,
            target,
            message,
            line,
        } => add_comment(&store, &review_id, &target, &message, line, mode),
        ReviewAction::List { open } => list_reviews(&store, open, mode),
        ReviewAction::Show { id } => show_review(&store, &id, mode),
        ReviewAction::Resolve {
            comment_id,
            message,
        } => resolve_comment(&store, &comment_id, message, mode),
        ReviewAction::Close { id } => close_review(&store, &id, mode),
    }
}

fn start_review(
    store: &FileReviewStore,
    base: &str,
    head: &str,
    mode: OutputMode,
) -> anyhow::Result<()> {
    let review = Review::new(base.to_string(), head.to_string());
    let id = review.id.clone();
    store.save(&review)?;

    if mode == OutputMode::Json {
        println!(
            "{}",
            serde_json::json!({
                "success": true,
                "review_id": id,
                "base_sha": base,
                "head_sha": head
            })
        );
    } else {
        println!("Started review: {}", id);
        println!("  Base: {}", base);
        println!("  Head: {}", head);
    }
    Ok(())
}

fn add_comment(
    store: &FileReviewStore,
    review_id: &str,
    target: &str,
    message: &str,
    line: Option<u32>,
    mode: OutputMode,
) -> anyhow::Result<()> {
    let mut review = store
        .load(review_id)?
        .ok_or_else(|| anyhow::anyhow!("Review not found: {}", review_id))?;

    let position = line.map_or(DiffPosition::default(), DiffPosition::new_line);
    review.add_comment(target.to_string(), message.to_string(), position);

    let comment_id = review.comments.last().map(|c| c.check.id.clone()).unwrap();
    store.save(&review)?;

    if mode == OutputMode::Json {
        println!(
            "{}",
            serde_json::json!({
                "success": true,
                "comment_id": comment_id,
                "review_id": review_id,
                "target": target
            })
        );
    } else {
        println!("Added comment: {}", comment_id);
        println!("  Target: {}", target);
        if let Some(l) = line {
            println!("  Line: {}", l);
        }
    }
    Ok(())
}

#[derive(Serialize)]
struct ReviewListItem {
    id: String,
    base_sha: String,
    head_sha: String,
    status: String,
    comments: usize,
    open_comments: usize,
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
                base_sha: r.base_sha.clone(),
                head_sha: r.head_sha.clone(),
                status: format!("{:?}", r.status).to_lowercase(),
                comments: r.comments.len(),
                open_comments: r.open_comments().len(),
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&items)?);
    } else if reviews.is_empty() {
        println!("No reviews found.");
    } else {
        println!("Reviews:\n");
        for r in &reviews {
            let open = r.open_comments().len();
            let total = r.comments.len();
            println!(
                "  [{}] {} ({} open / {} total)",
                format!("{:?}", r.status).to_uppercase(),
                r.id,
                open,
                total
            );
            println!(
                "       {}..{}",
                &r.base_sha[..7.min(r.base_sha.len())],
                &r.head_sha[..7.min(r.head_sha.len())]
            );
        }
    }
    Ok(())
}

fn show_review(store: &FileReviewStore, id: &str, mode: OutputMode) -> anyhow::Result<()> {
    let review = store.load(id)?.ok_or_else(|| anyhow::anyhow!("Review not found: {}", id))?;

    if mode == OutputMode::Json {
        println!("{}", serde_json::to_string_pretty(&review)?);
    } else {
        println!("Review: {}", review.id);
        println!("Status: {:?}", review.status);
        println!(
            "Range:  {}..{}",
            &review.base_sha[..7.min(review.base_sha.len())],
            &review.head_sha[..7.min(review.head_sha.len())]
        );
        println!();

        if review.comments.is_empty() {
            println!("No comments.");
        } else {
            println!("Comments:");
            for c in &review.comments {
                let status = if c.is_open() { "OPEN" } else { "RESOLVED" };
                println!("  [{}] {} - {}", status, c.check.id, c.check.target);
                println!("        {}", c.check.message);
                if !c.is_open()
                    && let Some(msg) = &c.resolution_message
                {
                    println!("        → {msg}");
                }
            }
        }
    }
    Ok(())
}

fn resolve_comment(
    store: &FileReviewStore,
    comment_id: &str,
    message: Option<String>,
    mode: OutputMode,
) -> anyhow::Result<()> {
    // Parse comment ID: "REV-xxxx:N"
    let parts: Vec<&str> = comment_id.rsplitn(2, ':').collect();
    if parts.len() != 2 {
        anyhow::bail!("Invalid comment ID format. Expected: REV-xxxx:N");
    }
    let review_id = parts[1];

    let mut review = store
        .load(review_id)?
        .ok_or_else(|| anyhow::anyhow!("Review not found: {}", review_id))?;

    let comment = review
        .comments
        .iter_mut()
        .find(|c| c.check.id == comment_id)
        .ok_or_else(|| anyhow::anyhow!("Comment not found: {}", comment_id))?;

    comment.resolve(message.clone());
    store.save(&review)?;

    if mode == OutputMode::Json {
        println!(
            "{}",
            serde_json::json!({
                "success": true,
                "comment_id": comment_id,
                "message": message
            })
        );
    } else {
        println!("Resolved: {}", comment_id);
        if let Some(msg) = message {
            println!("  Message: {}", msg);
        }
    }
    Ok(())
}

fn close_review(store: &FileReviewStore, id: &str, mode: OutputMode) -> anyhow::Result<()> {
    let mut review = store.load(id)?.ok_or_else(|| anyhow::anyhow!("Review not found: {}", id))?;

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
        println!("Closed review: {}", id);
    }
    Ok(())
}
