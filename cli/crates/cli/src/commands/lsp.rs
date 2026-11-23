//! `sdc lsp`: the language server, on stdio.

use clap::Args as ClapArgs;

#[derive(ClapArgs)]
pub struct Args {
    /// Accepted for clients that pass it; stdio is the only transport.
    #[arg(long)]
    stdio: bool,
}

pub fn run(args: Args) -> u8 {
    let _ = args.stdio;
    match sdlang_lsp::run_stdio() {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("error: {e}");
            2
        }
    }
}
