//! CLI definitions and entry point

use clap::{Parser, Subcommand};

use super::commands;
use noslop::output::OutputMode;

/// noslop - Local code review for agent-generated code
#[derive(Parser, Debug)]
#[command(
    name = "noslop",
    version,
    about = "Local code review for agent-generated code",
    long_about = "Enforce code review considerations via pre-commit hooks.\n\n\
                  Checks declare what must be reviewed when files change.\n\
                  Feedbacks track review status before pushing."
)]
pub struct Cli {
    /// Enable verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Output in JSON format (machine-readable)
    #[arg(long, global = true)]
    pub json: bool,

    #[command(subcommand)]
    pub command: Option<Command>,
}

/// Top-level CLI commands
#[derive(Subcommand, Debug)]
pub enum Command {
    /// Initialize noslop in the current repository
    Init {
        /// Agent to configure (e.g., claude, codex)
        agent: Option<String>,

        /// Force re-initialization
        #[arg(short, long)]
        force: bool,
    },

    /// Check for unresolved feedbacks in staged changes, or manage checks
    Check {
        /// Run in CI mode (stricter, non-interactive)
        #[arg(long)]
        ci: bool,

        #[command(subcommand)]
        action: Option<CheckAction>,
    },

    /// Manage code reviews
    Review {
        #[command(subcommand)]
        action: ReviewAction,
    },

    /// Manage feedbacks within reviews
    #[command(name = "feedback", alias = "feedbacks")]
    Feedback {
        #[command(subcommand)]
        action: FeedbacksAction,
    },

    /// Safety commit: stage all changes and commit bypassing hooks
    Checkpoint {
        /// Commit message (default: timestamp-based)
        message: Option<String>,
    },

    /// Show version
    Version,
}

/// Check management subcommands
#[derive(Subcommand, Debug)]
pub enum CheckAction {
    /// Add a check
    Add {
        /// File or pattern this check applies to
        target: String,

        /// The check message
        #[arg(short, long)]
        message: String,

        /// Severity: info, warn, block
        #[arg(short, long, default_value = "block")]
        severity: String,
    },

    /// List checks
    List {
        /// Filter by file
        #[arg(short, long)]
        target: Option<String>,
    },

    /// Remove a check
    Remove {
        /// Check ID
        id: String,
    },
}

/// Review management subcommands
#[derive(Subcommand, Debug)]
pub enum ReviewAction {
    /// Run the review pipeline on a commit range
    Run {
        /// Base ref (default: main)
        #[arg(long, default_value = "main")]
        base: String,

        /// Head ref (default: HEAD)
        #[arg(long, default_value = "HEAD")]
        head: String,

        /// Exit with code 1 if unresolved blocking feedbacks exist
        #[arg(long)]
        check: bool,
    },

    /// Start a new review for a commit range
    Start {
        /// Base commit SHA (start of diff)
        base: String,

        /// Head commit SHA (end of diff)
        head: String,
    },

    /// List reviews
    List {
        /// Show only open reviews
        #[arg(long)]
        open: bool,
    },

    /// Show review details
    Show {
        /// Review ID
        id: String,
    },

    /// Close a review
    Close {
        /// Review ID
        id: String,
    },

    /// Set or update the review summary
    Summary {
        /// Review ID
        id: String,

        /// Summary text
        text: String,
    },
}

/// Feedback management subcommands
#[derive(Subcommand, Debug)]
pub enum FeedbacksAction {
    /// Add a feedback to a review
    Add {
        /// Review ID
        review_id: String,

        /// Feedback message
        message: String,

        /// Target file path
        #[arg(long)]
        file: String,

        /// Target line number
        #[arg(long)]
        line: Option<u32>,

        /// End line (for multi-line spans)
        #[arg(long)]
        end_line: Option<u32>,

        /// Severity: info, warn, block
        #[arg(long, default_value = "warn")]
        severity: String,

        /// Suggested fix
        #[arg(long)]
        suggestion: Option<String>,
    },

    /// List feedbacks for a review
    List {
        /// Review ID
        id: String,
    },

    /// Resolve a feedback
    Resolve {
        /// Review ID
        review_id: String,

        /// Feedback ID
        feedback_id: String,
    },

    /// Dismiss a feedback
    Dismiss {
        /// Review ID
        review_id: String,

        /// Feedback ID
        feedback_id: String,

        /// Reason for dismissal
        #[arg(long)]
        reason: Option<String>,
    },
}

/// Run the CLI
pub fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();

    if cli.verbose {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();
    } else {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    }

    let output_mode = if cli.json {
        OutputMode::Json
    } else {
        OutputMode::Human
    };

    match cli.command {
        Some(Command::Init { agent, force }) => {
            commands::init(agent.as_deref(), force, output_mode)
        },
        Some(Command::Check { action: None, ci }) => commands::check_validate(ci, output_mode),
        Some(Command::Check {
            action: Some(action),
            ..
        }) => commands::check_manage(action, output_mode),
        Some(Command::Review { action }) => commands::review(action, output_mode),
        Some(Command::Feedback { action }) => commands::feedbacks(action, output_mode),
        Some(Command::Checkpoint { message }) => {
            commands::checkpoint(message.as_deref(), output_mode)
        },
        Some(Command::Version) => {
            if output_mode == OutputMode::Json {
                println!(
                    "{}",
                    serde_json::json!({
                        "version": env!("CARGO_PKG_VERSION")
                    })
                );
            } else {
                println!("noslop v{}", env!("CARGO_PKG_VERSION"));
            }
            Ok(())
        },
        None => {
            if output_mode == OutputMode::Json {
                println!(
                    "{}",
                    serde_json::json!({
                        "version": env!("CARGO_PKG_VERSION"),
                        "hint": "Use --help for usage"
                    })
                );
            } else {
                println!("noslop v{}", env!("CARGO_PKG_VERSION"));
                println!("\nRun 'noslop --help' for usage");
                println!("Run 'noslop init' to get started");
            }
            Ok(())
        },
    }
}
