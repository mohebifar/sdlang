//! `spec import`: proposes an ownership mapping for existing files; with `--accept`,
//! writes the proposed libs and the pending acquisitions for the next `spec plan`.

use std::path::PathBuf;

use intlang_adopt::proposal::render;
use intlang_adopt::{accept, propose};

#[derive(clap::Args)]
pub struct Args {
    /// Files or directories to adopt, relative to the project.
    #[arg(required = true)]
    pub paths: Vec<String>,

    /// The project directory (holding spec/ and .spec/).
    #[arg(long, default_value = ".")]
    pub dir: PathBuf,

    /// Write the proposed libs and .spec/import.json instead of only printing the proposal.
    #[arg(long)]
    pub accept: bool,
}

fn fail(message: &str) -> u8 {
    eprint!("{message}");
    if !message.ends_with('\n') {
        eprintln!();
    }
    // A spec that does not compile is wrong input; so is a path that names nothing.
    if message.contains("-->")
        || message.contains("no such file")
        || message.contains("never claims")
    {
        1
    } else {
        2
    }
}

pub fn run(args: Args) -> u8 {
    if !args.dir.is_dir() {
        eprintln!("error: no such directory: {}", args.dir.display());
        return 2;
    }
    let proposal = match propose(&args.dir, &args.paths) {
        Ok(p) => p,
        Err(e) => return fail(&e.message),
    };
    print!("{}", render(&proposal));
    if !args.accept {
        return 0;
    }
    if !proposal.acceptable() || proposal.is_empty() {
        eprintln!(
            "error: {}; nothing was written",
            if proposal.is_empty() {
                "nothing to claim"
            } else {
                "the proposal has collisions"
            }
        );
        return 1;
    }
    match accept(&args.dir, &proposal) {
        Ok(accepted) => {
            for f in &accepted.written {
                println!("wrote {f}");
            }
            println!(
                "pending acquisition: {} ({})",
                accepted.pending.join(", "),
                intlang_plan::PENDING_IMPORTS
            );
            println!(
                "next: review and commit these files, then run spec plan and spec apply; the plan acquires them with no agent"
            );
            0
        }
        Err(e) => fail(&format!("error: {}", e.message)),
    }
}
