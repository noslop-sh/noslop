//! noslop - Pre-commit assertions with attestation tracking

mod cli;
mod commands;
mod git;
mod noslop_file;

fn main() {
    if let Err(e) = cli::run() {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
