//! `spec check`: the read-only gate. Exit 0 when the branch is consistent, 1 with the
//! findings otherwise, 2 when the check cannot run.

use std::path::PathBuf;
use std::time::Duration;

use intlang_gate::findings::KINDS;
use intlang_gate::{check, CheckOptions};

#[derive(clap::Args)]
pub struct Args {
    /// The project directory (the top of its git work tree).
    #[arg(default_value = ".")]
    pub dir: PathBuf,

    /// Report findings of this kind without failing (repeatable).
    #[arg(long, value_name = "KIND")]
    pub allow: Vec<String>,

    /// Fail on unspecified declarations too.
    #[arg(long)]
    pub strict: bool,

    /// Also rerun every test run the evidence relies on, in a scratch worktree at HEAD.
    #[arg(long)]
    pub rerun: bool,

    /// The execution profile evidence was recorded under.
    #[arg(long, default_value = "local")]
    pub profile: String,

    /// Seconds a rerun check may run.
    #[arg(long, default_value_t = 900)]
    pub check_timeout: u64,

    /// Print the findings as a JSON array.
    #[arg(long)]
    pub json: bool,
}

pub fn run(args: Args) -> u8 {
    if !args.dir.is_dir() {
        eprintln!("error: no such directory: {}", args.dir.display());
        return 2;
    }
    if let Some(bad) = args.allow.iter().find(|a| !KINDS.contains(&a.as_str())) {
        eprintln!(
            "error: --allow {bad}: not a finding kind (one of {})",
            KINDS.join(", ")
        );
        return 2;
    }
    let mut options = CheckOptions::new(args.dir.clone());
    options.allow = args.allow;
    options.strict = args.strict;
    options.rerun = args.rerun;
    options.profile = args.profile;
    options.check_timeout = Duration::from_secs(args.check_timeout);
    let findings = match check(&options) {
        Ok(f) => f,
        Err(e) => {
            eprint!("{e}");
            if !e.message.ends_with('\n') {
                eprintln!();
            }
            // A spec that does not compile is wrong input; anything else stops the check.
            return if e.message.contains("-->") { 1 } else { 2 };
        }
    };
    let failed = findings.iter().any(|f| !f.warning);
    if args.json {
        let all: Vec<_> = findings.iter().map(|f| f.to_value()).collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&all).expect("serialisable")
        );
    } else {
        for f in &findings {
            let kind = if f.warning {
                format!("{} (warning)", f.kind)
            } else {
                f.kind.to_string()
            };
            println!("{kind:<22} {:<40} {}", f.subject, f.detail);
        }
        if !failed {
            println!("consistent");
        }
    }
    u8::from(failed)
}
