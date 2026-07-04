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

        /// Check the branch diff against this base ref instead of the index
        /// (CI source of truth; acks come from committed ledger records)
        #[arg(long, value_name = "REF")]
        diff_base: Option<String>,

        #[command(subcommand)]
        action: Option<CheckAction>,
    },

    /// Discover check proposals from rules files (CLAUDE.md, AGENTS.md, .cursor/rules)
    Discover {
        /// Review staged proposals: accept, edit, or reject each one
        #[arg(long)]
        review: bool,

        /// Mine PR review history via your agent CLI (needs gh, or --from-file)
        #[arg(long, conflicts_with = "review")]
        mine: bool,

        /// Mine from a JSONL export of review comments ({"path","body"} per line)
        #[arg(long, value_name = "PATH", conflicts_with = "review")]
        from_file: Option<String>,
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

    /// Fold pending ack records into .noslop/history.jsonl (run at merge time)
    Compact,

    /// Build the upload envelope for hosted ingestion (used by the Action)
    Envelope {
        /// Path to a `noslop check --json` payload
        #[arg(long, value_name = "PATH")]
        check: String,

        /// Repository slug (e.g. "noslop-sh/noslop")
        #[arg(long)]
        repo: String,

        /// Commit under test
        #[arg(long)]
        sha: String,

        /// Pull request number (empty for non-PR runs)
        #[arg(long, default_value = "")]
        pr: String,

        /// Base ref the diff was computed against
        #[arg(long)]
        base: String,
    },

    /// Per-check metrics: fires, acks, action rate vs no-action answers, dead targets
    Stats {
        /// Render as a markdown table (for CI summaries)
        #[arg(long)]
        markdown: bool,
    },

    /// Rulebook maintenance report: what to prune or reword, with evidence
    Curate {
        /// Render as a markdown table (for CI summaries)
        #[arg(long)]
        markdown: bool,
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
        Some(Command::Check {
            action: None,
            ci,
            diff_base,
        }) => commands::check_validate(ci, diff_base.as_deref(), output_mode),
        Some(Command::Check {
            action: Some(action),
            ..
        }) => commands::check_manage(action, output_mode),
        Some(Command::Discover {
            review,
            mine,
            from_file,
        }) => commands::discover(review, mine, from_file.as_deref(), output_mode),
        Some(Command::Ack { id, message }) => commands::ack(&id, &message, output_mode),
        Some(Command::AddTrailers { commit_msg_file }) => commands::add_trailers(&commit_msg_file),
        Some(Command::ClearStaged) => commands::clear_staged(),
        Some(Command::Compact) => commands::compact(),
        Some(Command::Envelope {
            check,
            repo,
            sha,
            pr,
            base,
        }) => commands::envelope(&check, &repo, &sha, &pr, &base),
        Some(Command::Stats { markdown }) => commands::stats(markdown, output_mode),
        Some(Command::Curate { markdown }) => commands::curate(markdown, output_mode),
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
