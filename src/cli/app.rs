//! CLI definitions and entry point

use clap::{Parser, Subcommand};

use super::commands;
use noslop::output::OutputMode;

/// noslop - Pre-commit checks with acknowledgment tracking
#[derive(Parser, Debug)]
#[command(
    name = "noslop",
    version,
    about = "Pre-commit checks with acknowledgment tracking",
    long_about = "Enforce code review considerations via pre-commit hooks.\n\n\
                  Checks declare what must be reviewed when files change.\n\
                  Acknowledgments prove the review happened before committing."
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

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Initialize noslop in the current repository
    Init {
        /// Force re-initialization
        #[arg(short, long)]
        force: bool,
    },

    /// Check for unacknowledged checks in staged changes, or manage checks
    Check {
        /// Run in CI mode (stricter, non-interactive)
        #[arg(long)]
        ci: bool,

        #[command(subcommand)]
        action: Option<CheckAction>,
    },

    /// Acknowledge a check (prove something was considered)
    Ack {
        /// Check ID to acknowledge
        id: String,

        /// Acknowledgment message
        #[arg(short, long)]
        message: String,
    },

    /// Add acknowledgment trailers to commit message (used by commit-msg hook)
    #[command(hide = true)]
    AddTrailers {
        /// Path to commit message file
        commit_msg_file: String,
    },

    /// Clear staged acknowledgments (used by post-commit hook)
    #[command(hide = true)]
    ClearStaged,

    /// Manage code reviews
    Review {
        #[command(subcommand)]
        action: ReviewAction,
    },

    /// Show version
    Version,
}

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

#[derive(Subcommand, Debug)]
pub enum ReviewAction {
    /// Start a new review for a commit range
    Start {
        /// Base commit SHA (start of diff)
        base: String,

        /// Head commit SHA (end of diff)
        head: String,
    },

    /// Add a comment to a review
    Comment {
        /// Review ID (e.g., REV-xxxx)
        review_id: String,

        /// Target file path
        target: String,

        /// Comment message
        #[arg(short, long)]
        message: String,

        /// Line number (optional)
        #[arg(short, long)]
        line: Option<u32>,
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

    /// Resolve a comment
    Resolve {
        /// Comment ID (e.g., REV-xxxx:1)
        comment_id: String,

        /// Resolution message
        #[arg(short, long)]
        message: Option<String>,
    },

    /// Close a review
    Close {
        /// Review ID
        id: String,
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
        Some(Command::Init { force }) => commands::init(force, output_mode),
        Some(Command::Check { action: None, ci }) => commands::check_validate(ci, output_mode),
        Some(Command::Check {
            action: Some(action),
            ..
        }) => commands::check_manage(action, output_mode),
        Some(Command::Ack { id, message }) => commands::ack(&id, &message, output_mode),
        Some(Command::AddTrailers { commit_msg_file }) => commands::add_trailers(&commit_msg_file),
        Some(Command::ClearStaged) => commands::clear_staged(),
        Some(Command::Review { action }) => commands::review(action, output_mode),
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
