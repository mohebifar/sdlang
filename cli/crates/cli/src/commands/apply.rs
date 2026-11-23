//! `sdc apply`: applies plan.json transaction by transaction.

use std::path::PathBuf;
use std::time::Duration;

use sdlang_apply::{apply, ApplyOptions, Outcome};

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

    /// Seconds an agent or reviewer may run before its process group is stopped.
    #[arg(long, default_value_t = 1800)]
    pub agent_timeout: u64,

    /// Seconds a check or generator may run.
    #[arg(long, default_value_t = 900)]
    pub check_timeout: u64,
}

pub fn run(args: Args) -> u8 {
    if !args.dir.is_dir() {
        eprintln!("error: no such directory: {}", args.dir.display());
        return 2;
    }
    // Agents start the MCP server with this same binary.
    let spec = std::env::current_exe()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| "sdc".into());
    let plan = args
        .plan
        .clone()
        .unwrap_or_else(|| args.dir.join("plan.json"));
    let mut options = ApplyOptions::new(args.dir.clone(), plan);
    options.agent = args.agent;
    options.reviewer = args.reviewer;
    options.profile = args.profile;
    options.max_attempts = args.max_attempts;
    options.spec_command = vec![spec, "mcp".into(), "--dispatch".into()];
    options.agent_timeout = Duration::from_secs(args.agent_timeout);
    options.check_timeout = Duration::from_secs(args.check_timeout);
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
