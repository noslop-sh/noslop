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
                  Findings track review status before pushing."
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
        /// Force re-initialization
        #[arg(short, long)]
        force: bool,
    },

    /// Check for unresolved findings in staged changes, or manage checks
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
