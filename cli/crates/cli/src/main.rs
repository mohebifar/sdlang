//! The `sdc` binary: argument parsing and dispatch to commands.

#![forbid(unsafe_code)]

use std::process::ExitCode;

use clap::{Parser, Subcommand};

/// One module per subcommand (`src/commands/<name>.rs`). Each exposes `Args` (clap) and
/// `run`, which returns the exit code: 0 success, 1 the input is wrong, 2 usage or
/// internal error.
mod commands {
    pub mod apply;
    pub mod check;
    pub mod graph;
    pub mod hash;
    pub mod import;
    pub mod lsp;
    pub mod mcp;
    pub mod parse;
    pub mod plan;
}

/// The command-line tool for sdlang.
#[derive(Parser)]
#[command(name = "sdc", version, about, propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Parse .sd files and report syntax errors.
    Parse(commands::parse::Args),
    /// Print the canonical JSON or digest of a JSON file, or a graph's projection hashes.
    Hash(commands::hash::Args),
    /// Lower a project's spec tree to the canonical graph and print it as JSON.
    Graph(commands::graph::Args),
    /// Plan a change from the spec, the recorded state and the files on disk.
    Plan(commands::plan::Args),
    /// Apply plan.json transaction by transaction with an agent and a reviewer.
    Apply(commands::apply::Args),
    /// Serve the MCP tools of one dispatch on stdio (started by an agent).
    Mcp(commands::mcp::Args),
    /// Report whether the branch is consistent: spec applied, no drift, sound ownership, current evidence.
    Check(commands::check::Args),
    /// Propose an ownership mapping for existing files; with --accept, write it for the next plan.
    Import(commands::import::Args),
    /// Serve the language server protocol on stdio, for editors.
    Lsp(commands::lsp::Args),
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
        Command::Apply(args) => commands::apply::run(args),
        Command::Mcp(args) => commands::mcp::run(args),
        Command::Check(args) => commands::check::run(args),
        Command::Import(args) => commands::import::run(args),
        Command::Lsp(args) => commands::lsp::run(args),
    };
    ExitCode::from(code)
}
