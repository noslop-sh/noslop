//! noslop - Pre-commit checks with verification tracking

mod cli;
mod commands;
#[cfg(feature = "ui")]
mod server;

fn main() {
    if let Err(e) = cli::run() {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
