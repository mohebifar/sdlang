//! `sdc mcp`: serves the MCP tools of one dispatch on stdio. An agent starts it from the
//! candidate worktree, as its work order's `tools.mcp.command` says.

use std::io;
use std::path::PathBuf;

#[derive(clap::Args)]
pub struct Args {
    /// The dispatch id from the work order.
    #[arg(long)]
    pub dispatch: String,

    /// The candidate worktree; defaults to the current directory.
    #[arg(long, default_value = ".")]
    pub dir: PathBuf,
}

pub fn run(args: Args) -> u8 {
    let stdin = io::stdin().lock();
    let stdout = io::stdout().lock();
    match sdlang_apply::mcp::serve(&args.dir, &args.dispatch, stdin, stdout) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("error: {e}");
            2
        }
    }
}
