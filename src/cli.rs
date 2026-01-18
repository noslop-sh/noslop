//! CLI definitions and entry point

use clap::{Parser, Subcommand};

use crate::commands;
use noslop::output::OutputMode;

/// noslop - Task management and pre-commit verification for AI agents and humans
#[derive(Parser, Debug)]
#[command(
    name = "noslop",
    version,
    about = "Task management and pre-commit verification tracking",
    long_about = "noslop tracks tasks and enforces code review considerations.\n\n\
QUICK START (for AI agents):\n\
  noslop status              Show current branch, tasks, and checks\n\
  noslop task list           List all tasks\n\
  noslop task next --start   Start the next prioritized task\n\
  noslop task done           Complete current task\n\n\
TASK WORKFLOW:\n\
  1. Check status:    noslop status\n\
  2. Pick a task:     noslop task next --start\n\
  3. Do the work\n\
  4. Mark complete:   noslop task done\n\n\
ADDING TASKS:\n\
  noslop task add \"Description\" --priority p1\n\
  Priorities: p0 (critical), p1 (high), p2 (medium), p3 (low)\n\n\
JSON OUTPUT (for parsing):\n\
  noslop --json task list    or    NOSLOP_JSON=1 noslop task list\n\n\
CHECK SYSTEM:\n\
  Checks are review requirements tied to file patterns.\n\
  When files matching a check's pattern change, verify before committing:\n\
    noslop check list                    List checks\n\
    noslop verify CHK-N -m \"Reviewed\"    Verify a check\n\n\
Run 'noslop <command> --help' for detailed command help."
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

    /// Manage checks (run, add, list, remove)
    Check {
        #[command(subcommand)]
        action: CheckAction,
    },

    /// Verify a check (prove something was considered)
    Verify {
        /// Check ID to verify
        id: String,

        /// Verification message
        #[arg(short, long)]
        message: String,
    },

    /// Add verification trailers to commit message (used by prepare-commit-msg hook)
    #[command(hide = true)]
    AddTrailers {
        /// Path to commit message file
        commit_msg_file: String,
    },

    /// Clear staged verifications (used by post-commit hook)
    #[command(hide = true)]
    ClearStaged,

    /// Prompt for task status at commit time (used by pre-commit hook)
    #[command(hide = true)]
    TaskPrompt,

    /// Manage tasks (track work to be done)
    ///
    /// Tasks have: ID (TSK-N), title, status (backlog/pending/in_progress/done),
    /// priority (p0-p3), and optional blockers. Use 'task next --start' to
    /// pick the highest priority unblocked task.
    Task {
        #[command(subcommand)]
        action: TaskAction,
    },

    /// Show current status (branch, tasks, checks)
    Status,

    /// Start local web UI for task and check management
    #[cfg(feature = "ui")]
    Ui {
        /// Port to listen on
        #[arg(short, long, default_value = "9999")]
        port: u16,

        /// Open browser automatically
        #[arg(long)]
        open: bool,
    },

    /// Show version
    Version,
}

#[derive(Subcommand, Debug)]
pub enum CheckAction {
    /// Run checks for staged changes (used by pre-commit hook)
    Run {
        /// Run in CI mode (stricter, non-interactive)
        #[arg(long)]
        ci: bool,
    },

    /// Add a check
    Add {
        /// File or pattern this check applies to
        scope: String,

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
        scope: Option<String>,
    },

    /// Remove a check
    Remove {
        /// Check ID
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

        /// Block this task by another task ID
        #[arg(short, long)]
        blocked_by: Option<Vec<String>>,
    },

    /// List all tasks
    List {
        /// Filter by status: pending, in_progress, done
        #[arg(short, long)]
        status: Option<String>,

        /// Show only unblocked tasks (pending and not blocked)
        #[arg(long)]
        unblocked: bool,
    },

    /// Show task details
    Show {
        /// Task ID
        id: String,
    },

    /// Show current active task (HEAD)
    Current,

    /// Get the next pending unblocked task
    Next {
        /// Also start the task (set as current)
        #[arg(long)]
        start: bool,
    },

    /// Start working on a task (sets HEAD)
    Start {
        /// Task ID
        id: String,
    },

    /// Mark current task as done
    Done,

    /// Block a task by another task
    Block {
        /// Task ID to block
        id: String,

        /// ID of blocking task
        #[arg(short, long)]
        by: String,
    },

    /// Unblock a task
    Unblock {
        /// Task ID to unblock
        id: String,

        /// ID of blocker to remove
        #[arg(short, long)]
        by: String,
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

    // Support NOSLOP_JSON env var for JSON output (agent-friendly)
    let json_from_env = std::env::var("NOSLOP_JSON").is_ok_and(|v| v == "1" || v == "true");
    let output_mode = if cli.json || json_from_env {
        OutputMode::Json
    } else {
        OutputMode::Human
    };

    match cli.command {
        Some(Command::Init { force }) => commands::init(force, output_mode),
        Some(Command::Check { action }) => match action {
            CheckAction::Run { ci } => commands::check_run(ci, output_mode),
            _ => commands::check_cmd(action, output_mode),
        },
        Some(Command::Verify { id, message }) => commands::verify(&id, &message, output_mode),
        Some(Command::AddTrailers { commit_msg_file }) => commands::add_trailers(&commit_msg_file),
        Some(Command::ClearStaged) => commands::clear_staged(),
        Some(Command::TaskPrompt) => {
            commands::task_prompt()?;
            Ok(())
        },
        Some(Command::Task { action }) => commands::task_cmd(action, output_mode),
        Some(Command::Status) => commands::status(output_mode),
        #[cfg(feature = "ui")]
        Some(Command::Ui { port, open }) => commands::ui(port, open),
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
