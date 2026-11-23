//! `sdc parse`: parses .sd files and reports syntax errors.

use std::fs;
use std::path::{Path, PathBuf};

use sdlang_syntax::{diagnostics, lexer, parse};
use serde_json::json;
use walkdir::WalkDir;

#[derive(clap::Args)]
pub struct Args {
    /// Files or directories; directories are searched for *.sd files.
    #[arg(required = true)]
    pub paths: Vec<PathBuf>,

    /// Print one JSON object per file (its path and tree, or its error) instead of text.
    #[arg(long)]
    pub json: bool,
}

/// Expands directories into their *.sd files, sorted; fails on a missing path.
fn collect(paths: &[PathBuf]) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    for path in paths {
        if path.is_dir() {
            let mut found: Vec<PathBuf> = WalkDir::new(path)
                .into_iter()
                .filter_entry(|e| e.file_name() != "target" && e.file_name() != ".git")
                .filter_map(Result::ok)
                .filter(|e| e.file_type().is_file())
                .map(|e| e.into_path())
                .filter(|p| p.extension().is_some_and(|ext| ext == "sd"))
                .collect();
            found.sort();
            files.extend(found);
        } else if path.is_file() {
            files.push(path.clone());
        } else {
            return Err(format!("no such file or directory: {}", path.display()));
        }
    }
    Ok(files)
}

fn display(path: &Path) -> String {
    path.display().to_string()
}

pub fn run(args: Args) -> u8 {
    let files = match collect(&args.paths) {
        Ok(files) => files,
        Err(message) => {
            eprintln!("error: {message}");
            return 2;
        }
    };
    let mut failed = 0;
    for path in &files {
        let raw = match fs::read_to_string(path) {
            Ok(raw) => raw,
            Err(e) => {
                eprintln!("error: cannot read {}: {e}", display(path));
                return 2;
            }
        };
        let source = lexer::normalize(&raw);
        match parse(&source) {
            Ok(tree) => {
                if args.json {
                    println!("{}", json!({ "path": display(path), "tree": tree }));
                }
            }
            Err(error) => {
                failed += 1;
                if args.json {
                    let (line, column) = diagnostics::line_col(&source, error.span.start);
                    let report = json!({
                        "path": display(path),
                        "error": {
                            "message": error.message,
                            "line": line,
                            "column": column,
                            "span": { "start": error.span.start, "end": error.span.end },
                        },
                    });
                    println!("{report}");
                } else {
                    eprint!("{}", diagnostics::render(&display(path), &source, &error));
                }
            }
        }
    }
    if !args.json {
        if failed == 0 {
            println!("parsed {} files", files.len());
        } else {
            eprintln!("{failed} of {} files failed to parse", files.len());
        }
    }
    u8::from(failed > 0)
}
