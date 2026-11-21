//! The `spec` binary: argument parsing and dispatch to commands.

#![forbid(unsafe_code)]

use std::process::ExitCode;

use clap::{Parser, Subcommand};

/// One module per subcommand (`src/commands/<name>.rs`). Each exposes `Args` (clap) and
/// `run`, which returns the exit code: 0 success, 1 the input is wrong, 2 usage or
/// internal error.
mod commands {
    pub mod graph;
    pub mod hash;
    pub mod parse;
    pub mod plan;
}

/// The command-line tool for int lang.
#[derive(Parser)]
#[command(name = "spec", version, about, propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Parse .int files and report syntax errors.
    Parse(commands::parse::Args),
    /// Print the canonical JSON or digest of a JSON file, or a graph's projection hashes.
    Hash(commands::hash::Args),
    /// Lower a project's spec tree to the canonical graph and print it as JSON.
    Graph(commands::graph::Args),
    /// Plan a change from the spec, the recorded state and the files on disk.
    Plan(commands::plan::Args),
}

fn main() -> ExitCode {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(e) => {
            let usage_error = e.use_stderr();
            let _ = e.print();
            // --help and --version are successes; anything else is a usage error.
            return if usage_error {
                ExitCode::from(2)
            } else {
                ExitCode::SUCCESS
            };
        }
    };
    let code = match cli.command {
        Command::Parse(args) => commands::parse::run(args),
        Command::Hash(args) => commands::hash::run(args),
        Command::Graph(args) => commands::graph::run(args),
        Command::Plan(args) => commands::plan::run(args),
    };
    ExitCode::from(code)
}
