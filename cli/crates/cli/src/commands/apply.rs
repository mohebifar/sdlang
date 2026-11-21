//! `spec apply`: applies plan.json transaction by transaction.

use std::path::PathBuf;

use intlang_apply::{apply, ApplyOptions, Outcome};

#[derive(clap::Args)]
pub struct Args {
    /// The project directory (the top of its git work tree).
    #[arg(default_value = ".")]
    pub dir: PathBuf,

    /// The plan to apply; defaults to plan.json in the project.
    #[arg(long)]
    pub plan: Option<PathBuf>,

    /// Shell command run in the candidate for each coding dispatch. It reads the work order
    /// on stdin (Markdown) or from $SPEC_WORK_ORDER and writes its result to $SPEC_RESULT.
    #[arg(long)]
    pub agent: Option<String>,

    /// Shell command run for each review. It reads $SPEC_REVIEW_ORDER and writes
    /// $SPEC_VERDICT; it must not have written any of the code it reviews.
    #[arg(long)]
    pub reviewer: Option<String>,

    /// The execution profile checks run under.
    #[arg(long, default_value = "local")]
    pub profile: String,

    /// Coding attempts per transaction.
    #[arg(long, default_value_t = 3)]
    pub max_attempts: u32,
}

pub fn run(args: Args) -> u8 {
    if !args.dir.is_dir() {
        eprintln!("error: no such directory: {}", args.dir.display());
        return 2;
    }
    let options = ApplyOptions {
        plan_path: args
            .plan
            .clone()
            .unwrap_or_else(|| args.dir.join("plan.json")),
        root: args.dir,
        agent: args.agent,
        reviewer: args.reviewer,
        profile: args.profile,
        max_attempts: args.max_attempts,
    };
    let report = match apply(&options) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: {e}");
            let refused = [
                "binding mismatch",
                "uncommitted",
                "unexpected edit",
                "not a plan",
                "-->",
            ]
            .iter()
            .any(|m| e.message.contains(m));
            return if refused { 1 } else { 2 };
        }
    };
    for (id, outcome) in &report.transactions {
        match outcome {
            Outcome::Published { commit } => println!("{id:<6} published  {commit}"),
            Outcome::AlreadyPublished { commit } => println!("{id:<6} already published  {commit}"),
            Outcome::Failed { reason } => println!("{id:<6} failed     {reason}"),
            Outcome::Skipped { because } => println!("{id:<6} skipped    {because}"),
        }
    }
    if report.all_published() {
        0
    } else {
        1
    }
}
