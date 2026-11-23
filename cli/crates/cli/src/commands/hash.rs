//! `sdc hash`: the canonical JSON or digest of a JSON file, or a graph's projection hashes.

use std::fs;
use std::path::PathBuf;

use serde_json::{Map, Value};

#[derive(clap::Args)]
pub struct Args {
    /// A JSON file.
    pub file: PathBuf,

    /// Print the canonical JSON text instead of its digest.
    #[arg(long, conflicts_with = "graph")]
    pub canonical: bool,

    /// Treat the file as a canonical graph and print each declaration's projection hashes.
    #[arg(long)]
    pub graph: bool,
}

pub fn run(args: Args) -> u8 {
    let text = match fs::read_to_string(&args.file) {
        Ok(text) => text,
        Err(e) => {
            eprintln!("error: cannot read {}: {e}", args.file.display());
            return 2;
        }
    };
    let value: Value = match serde_json::from_str(&text) {
        Ok(value) => value,
        Err(e) => {
            eprintln!("error: {} is not valid JSON: {e}", args.file.display());
            return 1;
        }
    };
    let result = if args.graph {
        sdlang_canon::projections(&value)
            .map(|all| {
                let map: Map<String, Value> =
                    all.iter().map(|(a, p)| (a.clone(), p.to_value())).collect();
                serde_json::to_string_pretty(&Value::Object(map)).expect("serialisable")
            })
            .map_err(|e| e.to_string())
    } else if args.canonical {
        sdlang_canon::canonical(&value).map_err(|e| e.to_string())
    } else {
        sdlang_canon::digest(&value).map_err(|e| e.to_string())
    };
    match result {
        Ok(out) => {
            println!("{out}");
            0
        }
        Err(message) => {
            eprintln!("error: {}: {message}", args.file.display());
            1
        }
    }
}
