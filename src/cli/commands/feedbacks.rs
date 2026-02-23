//! Feedbacks management commands

use noslop::DismissReason;
use noslop::adapters::FileReviewStore;
use noslop::core::ports::ReviewStore;
use noslop::output::OutputMode;

use crate::cli::app::FeedbacksAction;

/// Handle feedbacks subcommands
pub fn feedbacks(action: FeedbacksAction, mode: OutputMode) -> anyhow::Result<()> {
    let store = FileReviewStore::new();

    match action {
        FeedbacksAction::List { id } => list_feedbacks(&store, &id, mode),
        FeedbacksAction::Resolve {
            review_id,
            feedback_id,
        } => resolve_feedback(&store, &review_id, &feedback_id, mode),
        FeedbacksAction::Dismiss {
            review_id,
            feedback_id,
            reason,
        } => dismiss_feedback(&store, &review_id, &feedback_id, reason.as_deref(), mode),
    }
}

fn list_feedbacks(store: &FileReviewStore, id: &str, mode: OutputMode) -> anyhow::Result<()> {
    let review = store.load(id)?.ok_or_else(|| anyhow::anyhow!("Review not found: {id}"))?;

    if mode == OutputMode::Json {
        println!("{}", serde_json::to_string_pretty(&review.feedbacks)?);
    } else if review.feedbacks.is_empty() {
        println!("No feedbacks for review {id}.");
    } else {
        println!("Feedbacks for review {id}:\n");
        for f in &review.feedbacks {
            let status = if f.is_open() { "OPEN" } else { "RESOLVED" };
            println!("  [{status}] {} - {}", f.id, f.target);
            println!("        {}", f.message);
        }
    }
    Ok(())
}

fn resolve_feedback(
    store: &FileReviewStore,
    review_id: &str,
    feedback_id: &str,
    mode: OutputMode,
) -> anyhow::Result<()> {
    let mut review = store
        .load(review_id)?
        .ok_or_else(|| anyhow::anyhow!("Review not found: {review_id}"))?;

    let feedback = review
        .feedbacks
        .iter_mut()
        .find(|f| f.id == feedback_id)
        .ok_or_else(|| anyhow::anyhow!("Feedback not found: {feedback_id}"))?;

    feedback.resolve(None);
    store.save(&review)?;

    if mode == OutputMode::Json {
        println!(
            "{}",
            serde_json::json!({
                "success": true,
                "review_id": review_id,
                "feedback_id": feedback_id,
                "status": "resolved"
            })
        );
    } else {
        println!("Resolved feedback {feedback_id} in review {review_id}.");
    }
    Ok(())
}

fn dismiss_feedback(
    store: &FileReviewStore,
    review_id: &str,
    feedback_id: &str,
    reason: Option<&str>,
    mode: OutputMode,
) -> anyhow::Result<()> {
    let mut review = store
        .load(review_id)?
        .ok_or_else(|| anyhow::anyhow!("Review not found: {review_id}"))?;

    let feedback = review
        .feedbacks
        .iter_mut()
        .find(|f| f.id == feedback_id)
        .ok_or_else(|| anyhow::anyhow!("Feedback not found: {feedback_id}"))?;

    let dismiss_reason = match reason.unwrap_or("wont_fix") {
        "false_positive" => DismissReason::FalsePositive,
        "wont_fix" => DismissReason::WontFix,
        "not_applicable" => DismissReason::NotApplicable,
        "investigate_later" => DismissReason::InvestigateLater,
        other => anyhow::bail!(
            "Invalid dismiss reason: {other}. Expected: false_positive, wont_fix, not_applicable, investigate_later"
        ),
    };
    feedback.dismiss(dismiss_reason);
    store.save(&review)?;

    if mode == OutputMode::Json {
        println!(
            "{}",
            serde_json::json!({
                "success": true,
                "review_id": review_id,
                "feedback_id": feedback_id,
                "status": "dismissed",
                "reason": reason.unwrap_or("wont_fix")
            })
        );
    } else {
        let r = reason.unwrap_or("(no reason)");
        println!("Dismissed feedback {feedback_id} in review {review_id}: {r}");
    }
    Ok(())
}
