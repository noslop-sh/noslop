//! Review pipeline service
//!
//! The central orchestrator for `noslop review`. Accepts a list of
//! `ReviewAnalyzer` trait objects, sorts them by tier, runs each in
//! order with fold semantics, and collects feedbacks into a `Review`.

use crate::core::models::Review;
use crate::core::ports::{ReviewAnalyzer, ReviewContext};

/// Builds and executes the review pipeline.
///
/// The pipeline holds a sequence of analyzers. On [`run`](Self::run), it:
///
/// 1. Sorts analyzers by tier (Static < Computed < Agent)
/// 2. Runs each analyzer in order
/// 3. Passes all prior feedbacks to each subsequent analyzer (fold semantics)
/// 4. Collects all feedbacks into a new [`Review`]
#[derive(Debug)]
pub struct ReviewPipeline {
    analyzers: Vec<Box<dyn ReviewAnalyzer>>,
}

impl ReviewPipeline {
    /// Create a new pipeline with the given analyzers.
    ///
    /// Analyzers will be sorted by tier before execution.
    #[must_use]
    pub fn new(analyzers: Vec<Box<dyn ReviewAnalyzer>>) -> Self {
        Self { analyzers }
    }

    /// Execute the full review pipeline.
    ///
    /// Creates a `Review` from the context's base/head, runs all analyzers
    /// in tier order with fold semantics, adds feedbacks to the review, and
    /// returns it.
    pub fn run(&self, context: &ReviewContext) -> anyhow::Result<Review> {
        // Build execution order: sort by tier, preserving insertion order within a tier
        let mut indices: Vec<usize> = (0..self.analyzers.len()).collect();
        indices.sort_by_key(|&i| self.analyzers[i].tier());

        let mut review = Review::new("0", &context.base, &context.head);
        let mut all_feedbacks = Vec::new();

        for &idx in &indices {
            let analyzer = &self.analyzers[idx];
            log::info!("Running analyzer: {}", analyzer.name());

            let feedbacks = analyzer.analyze(context, &all_feedbacks)?;
            log::info!("  {} produced {} feedback(s)", analyzer.name(), feedbacks.len());

            all_feedbacks.extend(feedbacks);
        }

        review.add_feedbacks(all_feedbacks);
        Ok(review)
    }
}

// Manual Debug impl is needed because Box<dyn ReviewAnalyzer> doesn't implement Debug.
// We already derive Debug on ReviewPipeline above, so we need to ensure the inner
// Vec can be debug-formatted. Let's provide a custom impl instead.
impl std::fmt::Debug for dyn ReviewAnalyzer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ReviewAnalyzer({})", self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::models::{Feedback, FeedbackSource, Severity, Target};
    use crate::core::ports::{AnalyzerTier, ContextKind};
    use std::path::PathBuf;
    use std::sync::Arc;

    /// A simple mock analyzer for testing.
    struct MockAnalyzer {
        analyzer_name: String,
        tier: AnalyzerTier,
        feedbacks: Vec<Feedback>,
    }

    impl MockAnalyzer {
        fn new(name: &str, tier: AnalyzerTier, feedbacks: Vec<Feedback>) -> Self {
            Self {
                analyzer_name: name.to_string(),
                tier,
                feedbacks,
            }
        }
    }

    impl ReviewAnalyzer for MockAnalyzer {
        fn required_context(&self) -> Vec<ContextKind> {
            vec![ContextKind::ChangedFiles]
        }

        fn analyze(
            &self,
            _context: &ReviewContext,
            _prior_feedbacks: &[Feedback],
        ) -> anyhow::Result<Vec<Feedback>> {
            Ok(self.feedbacks.clone())
        }

        fn name(&self) -> &str {
            &self.analyzer_name
        }

        fn tier(&self) -> AnalyzerTier {
            self.tier
        }
    }

    /// An analyzer that captures the count of prior feedbacks it received.
    struct CapturingAnalyzer {
        analyzer_name: String,
        tier: AnalyzerTier,
        feedbacks: Vec<Feedback>,
        prior_counts: Arc<std::sync::Mutex<Vec<(String, usize)>>>,
    }

    impl ReviewAnalyzer for CapturingAnalyzer {
        fn required_context(&self) -> Vec<ContextKind> {
            vec![ContextKind::ChangedFiles]
        }

        fn analyze(
            &self,
            _context: &ReviewContext,
            prior_feedbacks: &[Feedback],
        ) -> anyhow::Result<Vec<Feedback>> {
            self.prior_counts
                .lock()
                .unwrap()
                .push((self.analyzer_name.clone(), prior_feedbacks.len()));
            Ok(self.feedbacks.clone())
        }

        fn name(&self) -> &str {
            &self.analyzer_name
        }

        fn tier(&self) -> AnalyzerTier {
            self.tier
        }
    }

    /// An analyzer that records its execution order via shared state.
    struct OrderTracker {
        analyzer_name: String,
        tier: AnalyzerTier,
        order: Arc<std::sync::Mutex<Vec<String>>>,
    }

    impl ReviewAnalyzer for OrderTracker {
        fn required_context(&self) -> Vec<ContextKind> {
            vec![]
        }

        fn analyze(
            &self,
            _context: &ReviewContext,
            _prior: &[Feedback],
        ) -> anyhow::Result<Vec<Feedback>> {
            self.order.lock().unwrap().push(self.analyzer_name.clone());
            Ok(vec![])
        }

        fn name(&self) -> &str {
            &self.analyzer_name
        }

        fn tier(&self) -> AnalyzerTier {
            self.tier
        }
    }

    /// An analyzer that always fails.
    struct FailingAnalyzer;

    impl ReviewAnalyzer for FailingAnalyzer {
        fn required_context(&self) -> Vec<ContextKind> {
            vec![]
        }

        fn analyze(
            &self,
            _context: &ReviewContext,
            _prior: &[Feedback],
        ) -> anyhow::Result<Vec<Feedback>> {
            anyhow::bail!("analyzer failed")
        }

        fn name(&self) -> &'static str {
            "failing"
        }

        fn tier(&self) -> AnalyzerTier {
            AnalyzerTier::Static
        }
    }

    fn test_context() -> ReviewContext {
        ReviewContext {
            diff: String::new(),
            changed_files: vec!["src/main.rs".to_string()],
            repo_root: PathBuf::from("/tmp/test"),
            commit_range: None,
            base: "main".to_string(),
            head: "HEAD".to_string(),
        }
    }

    fn make_feedback(msg: &str) -> Feedback {
        Feedback::new(
            Target::file("src/main.rs"),
            Severity::Warn,
            msg,
            FeedbackSource::Check("TEST-1".into()),
        )
    }

    #[test]
    fn empty_pipeline_produces_empty_review() {
        let pipeline = ReviewPipeline::new(vec![]);
        let ctx = test_context();
        let review = pipeline.run(&ctx).unwrap();
        assert!(review.feedbacks.is_empty());
        assert_eq!(review.base, "main");
        assert_eq!(review.head, "HEAD");
    }

    #[test]
    fn single_analyzer_feedbacks_in_review() {
        let feedbacks = vec![make_feedback("found something")];
        let analyzer = MockAnalyzer::new("test", AnalyzerTier::Static, feedbacks);
        let pipeline = ReviewPipeline::new(vec![Box::new(analyzer)]);
        let ctx = test_context();
        let review = pipeline.run(&ctx).unwrap();
        assert_eq!(review.feedbacks.len(), 1);
        assert_eq!(review.feedbacks[0].message, "found something");
    }

    #[test]
    fn fold_semantics_prior_feedbacks_passed() {
        let f1 = make_feedback("static feedback");
        let f2 = make_feedback("computed feedback");

        let static_analyzer = MockAnalyzer::new("static", AnalyzerTier::Static, vec![f1]);
        let computed_analyzer = MockAnalyzer::new("computed", AnalyzerTier::Computed, vec![f2]);
        let agent_analyzer = MockAnalyzer::new("agent", AnalyzerTier::Agent, vec![]);

        let pipeline = ReviewPipeline::new(vec![
            Box::new(static_analyzer),
            Box::new(computed_analyzer),
            Box::new(agent_analyzer),
        ]);

        let ctx = test_context();
        let review = pipeline.run(&ctx).unwrap();

        // Review should have 2 feedbacks total (static + computed)
        assert_eq!(review.feedbacks.len(), 2);

        // Check fold semantics via captured prior counts
        // We can't access the mock analyzers after moving them into the pipeline,
        // so we use a different approach below.
    }

    #[test]
    fn fold_semantics_verified_with_shared_state() {
        let prior_counts: Arc<std::sync::Mutex<Vec<(String, usize)>>> =
            Arc::new(std::sync::Mutex::new(Vec::new()));

        let f1 = make_feedback("s1");
        let f2 = make_feedback("s2");
        let f3 = make_feedback("c1");

        let analyzers: Vec<Box<dyn ReviewAnalyzer>> = vec![
            Box::new(CapturingAnalyzer {
                analyzer_name: "static".into(),
                tier: AnalyzerTier::Static,
                feedbacks: vec![f1, f2],
                prior_counts: Arc::clone(&prior_counts),
            }),
            Box::new(CapturingAnalyzer {
                analyzer_name: "computed".into(),
                tier: AnalyzerTier::Computed,
                feedbacks: vec![f3],
                prior_counts: Arc::clone(&prior_counts),
            }),
            Box::new(CapturingAnalyzer {
                analyzer_name: "agent".into(),
                tier: AnalyzerTier::Agent,
                feedbacks: vec![],
                prior_counts: Arc::clone(&prior_counts),
            }),
        ];

        let pipeline = ReviewPipeline::new(analyzers);
        let ctx = test_context();
        let review = pipeline.run(&ctx).unwrap();

        assert_eq!(review.feedbacks.len(), 3);

        let counts = prior_counts.lock().unwrap();
        // static runs first: sees 0 prior feedbacks
        assert_eq!(counts[0], ("static".to_string(), 0));
        // computed runs second: sees 2 prior feedbacks from static
        assert_eq!(counts[1], ("computed".to_string(), 2));
        // agent runs third: sees 3 prior feedbacks (2 from static + 1 from computed)
        assert_eq!(counts[2], ("agent".to_string(), 3));
        drop(counts);
    }

    #[test]
    fn tier_ordering_respected() {
        let execution_order: Arc<std::sync::Mutex<Vec<String>>> =
            Arc::new(std::sync::Mutex::new(Vec::new()));

        // Insert in reverse tier order to verify sorting
        let analyzers: Vec<Box<dyn ReviewAnalyzer>> = vec![
            Box::new(OrderTracker {
                analyzer_name: "agent".into(),
                tier: AnalyzerTier::Agent,
                order: Arc::clone(&execution_order),
            }),
            Box::new(OrderTracker {
                analyzer_name: "computed".into(),
                tier: AnalyzerTier::Computed,
                order: Arc::clone(&execution_order),
            }),
            Box::new(OrderTracker {
                analyzer_name: "static".into(),
                tier: AnalyzerTier::Static,
                order: Arc::clone(&execution_order),
            }),
        ];

        let pipeline = ReviewPipeline::new(analyzers);
        let ctx = test_context();
        pipeline.run(&ctx).unwrap();

        let order = execution_order.lock().unwrap();
        assert_eq!(*order, vec!["static", "computed", "agent"]);
        drop(order);
    }

    #[test]
    fn insertion_order_preserved_within_tier() {
        let execution_order: Arc<std::sync::Mutex<Vec<String>>> =
            Arc::new(std::sync::Mutex::new(Vec::new()));

        // Two static analyzers, should keep insertion order
        let analyzers: Vec<Box<dyn ReviewAnalyzer>> = vec![
            Box::new(OrderTracker {
                analyzer_name: "conventions".into(),
                tier: AnalyzerTier::Static,
                order: Arc::clone(&execution_order),
            }),
            Box::new(OrderTracker {
                analyzer_name: "extra-checks".into(),
                tier: AnalyzerTier::Static,
                order: Arc::clone(&execution_order),
            }),
            Box::new(OrderTracker {
                analyzer_name: "agent:security".into(),
                tier: AnalyzerTier::Agent,
                order: Arc::clone(&execution_order),
            }),
        ];

        let pipeline = ReviewPipeline::new(analyzers);
        let ctx = test_context();
        pipeline.run(&ctx).unwrap();

        let order = execution_order.lock().unwrap();
        assert_eq!(*order, vec!["conventions", "extra-checks", "agent:security"]);
        drop(order);
    }

    #[test]
    fn analyzer_error_propagates() {
        let pipeline = ReviewPipeline::new(vec![Box::new(FailingAnalyzer)]);
        let ctx = test_context();
        let result = pipeline.run(&ctx);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("analyzer failed"));
    }

    #[test]
    fn review_base_head_from_context() {
        let pipeline = ReviewPipeline::new(vec![]);
        let mut ctx = test_context();
        ctx.base = "feature-branch".to_string();
        ctx.head = "abc1234".to_string();
        let review = pipeline.run(&ctx).unwrap();
        assert_eq!(review.base, "feature-branch");
        assert_eq!(review.head, "abc1234");
    }
}
