//! Findings management commands

use noslop::DismissReason;
use noslop::adapters::FileReviewStore;
use noslop::core::ports::ReviewStore;
use noslop::output::OutputMode;

use crate::cli::app::FindingsAction;

/// Handle findings subcommands
pub fn findings(action: FindingsAction, mode: OutputMode) -> anyhow::Result<()> {
    let store = FileReviewStore::new();

    match action {
        FindingsAction::List { id } => list_findings(&store, &id, mode),
        FindingsAction::Resolve {
            review_id,
            finding_id,
        } => resolve_finding(&store, &review_id, &finding_id, mode),
        FindingsAction::Dismiss {
            review_id,
            finding_id,
            reason,
        } => dismiss_finding(&store, &review_id, &finding_id, reason.as_deref(), mode),
    }
}

fn list_findings(store: &FileReviewStore, id: &str, mode: OutputMode) -> anyhow::Result<()> {
    let review = store.load(id)?.ok_or_else(|| anyhow::anyhow!("Review not found: {id}"))?;

    if mode == OutputMode::Json {
        println!("{}", serde_json::to_string_pretty(&review.findings)?);
    } else if review.findings.is_empty() {
        println!("No findings for review {id}.");
    } else {
        println!("Findings for review {id}:\n");
        for f in &review.findings {
            let status = if f.is_open() { "OPEN" } else { "RESOLVED" };
            println!("  [{status}] {} - {}", f.id, f.target);
            println!("        {}", f.message);
        }
    }
    Ok(())
}

fn resolve_finding(
    store: &FileReviewStore,
    review_id: &str,
    finding_id: &str,
    mode: OutputMode,
) -> anyhow::Result<()> {
    let mut review = store
        .load(review_id)?
        .ok_or_else(|| anyhow::anyhow!("Review not found: {review_id}"))?;

    let finding = review
        .findings
        .iter_mut()
        .find(|f| f.id == finding_id)
        .ok_or_else(|| anyhow::anyhow!("Finding not found: {finding_id}"))?;

    finding.resolve(None);
    store.save(&review)?;

    if mode == OutputMode::Json {
        println!(
            "{}",
            serde_json::json!({
                "success": true,
                "review_id": review_id,
                "finding_id": finding_id,
                "status": "resolved"
            })
        );
    } else {
        println!("Resolved finding {finding_id} in review {review_id}.");
    }
    Ok(())
}

fn dismiss_finding(
    store: &FileReviewStore,
    review_id: &str,
    finding_id: &str,
    reason: Option<&str>,
    mode: OutputMode,
) -> anyhow::Result<()> {
    let mut review = store
        .load(review_id)?
        .ok_or_else(|| anyhow::anyhow!("Review not found: {review_id}"))?;

    let finding = review
        .findings
        .iter_mut()
        .find(|f| f.id == finding_id)
        .ok_or_else(|| anyhow::anyhow!("Finding not found: {finding_id}"))?;

    let dismiss_reason = match reason.unwrap_or("wont_fix") {
        "false_positive" => DismissReason::FalsePositive,
        "wont_fix" => DismissReason::WontFix,
        "not_applicable" => DismissReason::NotApplicable,
        "investigate_later" => DismissReason::InvestigateLater,
        other => anyhow::bail!(
            "Invalid dismiss reason: {other}. Expected: false_positive, wont_fix, not_applicable, investigate_later"
        ),
    };
    finding.dismiss(dismiss_reason);
    store.save(&review)?;

    if mode == OutputMode::Json {
        println!(
            "{}",
            serde_json::json!({
                "success": true,
                "review_id": review_id,
                "finding_id": finding_id,
                "status": "dismissed",
                "reason": reason.unwrap_or("wont_fix")
            })
        );
    } else {
        let r = reason.unwrap_or("(no reason)");
        println!("Dismissed finding {finding_id} in review {review_id}: {r}");
    }
    Ok(())
}
