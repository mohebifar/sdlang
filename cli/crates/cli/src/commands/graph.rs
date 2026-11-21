//! `spec graph`: lowers a project's spec tree to the canonical graph and prints it.

use std::path::PathBuf;

#[derive(clap::Args)]
pub struct Args {
    /// The project directory (holding spec/ and, optionally, spec.lock).
    #[arg(default_value = ".")]
    pub dir: PathBuf,
}

pub fn run(args: Args) -> u8 {
    if !args.dir.is_dir() {
        eprintln!("error: no such directory: {}", args.dir.display());
        return 2;
    }
    match intlang_graph::build(&args.dir) {
        Ok(graph) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&graph).expect("serialisable")
            );
            0
        }
        Err(e) => {
            eprint!("{}", e.render());
            1
        }
    }
}
