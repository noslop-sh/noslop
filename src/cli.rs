//! CLI definitions and entry point

use clap::{Parser, Subcommand};

use crate::commands;
use noslop::output::OutputMode;

/// noslop - Pre-commit assertions with attestation tracking
#[derive(Parser, Debug)]
#[command(
    name = "noslop",
    version,
    about = "Pre-commit assertions with attestation tracking",
    long_about = "Enforce code review considerations via pre-commit hooks.\n\n\
                  Assertions declare what must be reviewed when files change.\n\
                  Attestations prove the review happened before committing."
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

    /// Check assertions for staged changes (used by pre-commit hook)
    Check {
        /// Run in CI mode (stricter, non-interactive)
        #[arg(long)]
        ci: bool,
    },

    /// Manage assertions (declare what must be considered when code changes)
    Assert {
        #[command(subcommand)]
        action: AssertAction,
    },

    /// Attest to assertions (prove something was considered)
    Attest {
        /// Assertion ID to attest to
        id: String,

        /// Attestation message
        #[arg(short, long)]
        message: String,
    },

    /// Add attestation trailers to commit message (used by prepare-commit-msg hook)
    #[command(hide = true)]
    AddTrailers {
        /// Path to commit message file
        commit_msg_file: String,
    },

    /// Clear staged attestations (used by post-commit hook)
    #[command(hide = true)]
    ClearStaged,

    /// Manage tasks (track work to be done)
    Task {
        #[command(subcommand)]
        action: TaskAction,
    },

    /// Show version
    Version,
}

#[derive(Subcommand, Debug)]
pub enum AssertAction {
    /// Add an assertion
    Add {
        /// File or pattern this assertion applies to
        target: String,

        /// The assertion message
        #[arg(short, long)]
        message: String,

        /// Severity: info, warn, block
        #[arg(short, long, default_value = "block")]
        severity: String,
    },

    /// List assertions
    List {
        /// Filter by file
        #[arg(short, long)]
        target: Option<String>,
    },

    /// Remove an assertion
    Remove {
        /// Assertion ID
        id: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum TaskAction {
    /// Add a new task
    Add {
        /// Task title (what needs to be done)
        title: String,

        /// Priority: p0 (critical), p1 (high), p2 (medium), p3 (low)
        #[arg(short, long)]
        priority: Option<String>,

        /// Task IDs that block this one
        #[arg(long = "blocked-by", value_delimiter = ',')]
        blocked_by: Vec<String>,

        /// Optional note/context
        #[arg(short, long)]
        note: Option<String>,
    },

    /// List tasks
    List {
        /// Filter by status: pending, in_progress, done
        #[arg(short, long)]
        status: Option<String>,

        /// Show only ready tasks (pending with no blockers)
        #[arg(long)]
        ready: bool,
    },

    /// Show task details
    Show {
        /// Task ID
        id: String,
    },

    /// Start a task (mark as in_progress)
    Start {
        /// Task ID
        id: String,
    },

    /// Complete a task (mark as done)
    Done {
        /// Task ID
        id: String,
    },

    /// Add or update notes on a task
    Note {
        /// Task ID
        id: String,

        /// Note message
        #[arg(short, long)]
        message: String,
    },

    /// Add blockers to a task
    Block {
        /// Task ID to add blockers to
        id: String,

        /// Task IDs that block this one
        #[arg(required = true)]
        blocker_ids: Vec<String>,
    },

    /// Remove blockers from a task
    Unblock {
        /// Task ID to remove blockers from
        id: String,

        /// Task IDs to unblock
        #[arg(required = true)]
        blocker_ids: Vec<String>,
    },

    /// Remove a task
    Remove {
        /// Task ID
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
        Some(Command::Check { ci }) => commands::check(ci, output_mode),
        Some(Command::Assert { action }) => commands::assert_cmd(action, output_mode),
        Some(Command::Attest { id, message }) => commands::attest(&id, &message, output_mode),
        Some(Command::AddTrailers { commit_msg_file }) => commands::add_trailers(&commit_msg_file),
        Some(Command::ClearStaged) => commands::clear_staged(),
        Some(Command::Task { action }) => commands::task_cmd(action, output_mode),
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
