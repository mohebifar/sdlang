//! `spec plan`: plans a change from the spec, the recorded state and the files on disk,
//! writes the plan document and prints its summary.

use std::fmt::Write as _;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

#[derive(clap::Args)]
pub struct Args {
    /// The project directory (holding spec/, spec.lock and .spec/).
    #[arg(default_value = ".")]
    pub dir: PathBuf,

    /// The plan id; defaults to p-<UTC date>-01.
    #[arg(long)]
    pub id: Option<String>,

    /// Where to write the plan document; defaults to plan.json in the project.
    #[arg(long, conflicts_with = "json")]
    pub out: Option<PathBuf>,

    /// Print the plan document to stdout instead of writing it.
    #[arg(long)]
    pub json: bool,
}

/// `YYYY-MM-DD` for days since the Unix epoch (Howard Hinnant's civil-from-days).
fn civil(days: i64) -> String {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

fn today() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    civil(secs.div_euclid(86_400))
}

fn head_commit(dir: &std::path::Path) -> Option<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["rev-parse", "--verify", "HEAD"])
        .output()
        .ok()?;
    let sha = String::from_utf8(out.stdout).ok()?.trim().to_string();
    (out.status.success() && sha.len() == 40).then(|| format!("git:{sha}"))
}

pub fn run(args: Args) -> u8 {
    if !args.dir.is_dir() {
        eprintln!("error: no such directory: {}", args.dir.display());
        return 2;
    }
    let Some(baseline) = head_commit(&args.dir) else {
        eprintln!(
            "error: {} is not in a git work tree with a commit; the plan binds the baseline commit",
            args.dir.display()
        );
        return 2;
    };
    let project = match intlang_graph::loader::load(&args.dir) {
        Ok(p) => p,
        Err(e) => {
            eprint!("{}", e.render());
            return 1;
        }
    };
    let graph = match intlang_graph::lower::lower_project(&project) {
        Ok(g) => g,
        Err(e) => {
            eprint!("{}", e.render());
            return 1;
        }
    };
    let id = args.id.unwrap_or_else(|| format!("p-{}-01", today()));
    let plan = match intlang_plan::plan(&intlang_plan::PlanInputs {
        root: &args.dir,
        project: &project,
        graph: &graph,
        plan_id: &id,
        baseline_commit: &baseline,
        acquire: None,
    }) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error: {e}");
            return 1;
        }
    };
    let text = serde_json::to_string_pretty(&plan).expect("serialisable");
    if args.json {
        println!("{text}");
        return 0;
    }
    let out = args.out.unwrap_or_else(|| args.dir.join("plan.json"));
    if let Err(e) = fs::write(&out, text + "\n") {
        eprintln!("error: cannot write {}: {e}", out.display());
        return 2;
    }
    print!("{}", summary(&plan, &graph));
    let unspecified = intlang_plan::materialise::materialise(&project, &graph)
        .ok()
        .and_then(|m| intlang_gate::findings::unspecified_addresses(&graph, &m).ok())
        .unwrap_or_default();
    if !unspecified.is_empty() {
        println!(
            "Unspecified (no requirements, so nothing about their behaviour is tested or reviewed): {}",
            unspecified.join(", ")
        );
    }
    println!("Saved to {}.", out.display());
    0
}

/// A kind's short name: `core:action` -> `action`, `npm:@intlang/nextjs::page` -> `page`.
fn short(kind: &str) -> &str {
    kind.rsplit_once("::")
        .map(|(_, k)| k)
        .or_else(|| kind.strip_prefix("core:"))
        .unwrap_or(kind)
}

/// The human summary, in the order SPEC.md's worked example prints it.
pub fn summary(plan: &Value, graph: &Value) -> String {
    let kinds: std::collections::BTreeMap<&str, &str> = graph["declarations"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|d| Some((d["address"].as_str()?, d["kind"].as_str()?)))
        .collect();
    let kind_of = |a: &str| kinds.get(a).map(|k| short(k)).unwrap_or("");
    let entries = plan["entries"].as_array().cloned().unwrap_or_default();
    let blocked = plan["blocked"].as_array().cloned().unwrap_or_default();
    let mut s = String::new();
    let changes: Vec<&Value> = entries.iter().filter(|e| e["change"] != "none").collect();
    if !changes.is_empty() {
        s.push_str("Spec changes\n");
        for e in &changes {
            let sign = match e["change"].as_str() {
                Some("create") => '+',
                Some("delete") => '-',
                _ => '~',
            };
            let address = e["address"].as_str().unwrap_or_default();
            let causes: Vec<&str> = e["causes"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .collect();
            let _ = writeln!(
                s,
                "  {sign} {:<12} {:<28} {} by {}",
                kind_of(address),
                address,
                causes.join(", "),
                e["strategy"].as_str().unwrap_or("")
            );
        }
    }
    let obligated: Vec<&Value> = entries.iter().filter(|e| e["change"] == "none").collect();
    if !obligated.is_empty() {
        s.push_str("\nObligations\n");
        for e in obligated {
            let address = e["address"].as_str().unwrap_or_default();
            let reasons: Vec<String> = e["obligations"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|o| {
                    let compat = o["compatibility"]
                        .as_str()
                        .map(|c| format!(" ({c})"))
                        .unwrap_or_default();
                    format!(
                        "{} {}{compat}",
                        o["kind"].as_str().unwrap_or(""),
                        o["because"].as_str().unwrap_or("")
                    )
                })
                .collect();
            let _ = writeln!(
                s,
                "  ? {:<12} {:<28} {}",
                kind_of(address),
                address,
                reasons.join("; ")
            );
        }
    }
    if !blocked.is_empty() {
        s.push_str("\nBlocked\n");
        for b in &blocked {
            let address = b["address"].as_str().unwrap_or_default();
            let detail = match b["reason"].as_str() {
                Some("drift") => format!("{} edited by hand", b["path"].as_str().unwrap_or("")),
                Some(reason) => reason.to_string(),
                None => String::new(),
            };
            let _ = writeln!(s, "  ! {:<12} {:<28} {detail}", kind_of(address), address);
            if let Some(r) = b["resolutions"].as_array() {
                let r: Vec<&str> = r.iter().filter_map(Value::as_str).collect();
                let _ = writeln!(s, "      resolve with: {}", r.join(" | "));
            }
        }
    }
    let transactions = plan["transactions"].as_array().cloned().unwrap_or_default();
    if !transactions.is_empty() {
        s.push_str("\nTransactions (serial)\n");
        for t in &transactions {
            let members: Vec<&str> = t["entries"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .collect();
            let mut line = format!(
                "  {:<5} {}",
                t["id"].as_str().unwrap_or(""),
                members.join(", ")
            );
            let after: Vec<&str> = t["after"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .collect();
            if !after.is_empty() {
                let _ = write!(line, "  after {}", after.join(", "));
            }
            if let Some(o) = t["obligations"].as_object().filter(|o| !o.is_empty()) {
                let names: Vec<&String> = o.keys().collect();
                let names: Vec<&str> = names.iter().map(|n| n.as_str()).collect();
                let _ = write!(line, "  then check {}", names.join(", "));
            }
            s.push_str(line.trim_end());
            s.push('\n');
        }
    }
    let count = |c: &str| entries.iter().filter(|e| e["change"] == c).count();
    let by_generator = entries
        .iter()
        .filter(|e| e["strategy"] == "generator")
        .count();
    let dispatches: std::collections::BTreeSet<&str> = entries
        .iter()
        .filter(|e| e["strategy"] == "agent")
        .filter_map(|e| e["transaction"].as_str())
        .collect();
    let coverage = plan["reviews"]["coverage"].as_array().map_or(0, Vec::len);
    let _ = writeln!(
        s,
        "\nChanges: {} create, {} update ({by_generator} by generator), {} delete, {} obligation-only, {} blocked",
        count("create"),
        count("update"),
        count("delete"),
        count("none"),
        blocked.len()
    );
    let _ = writeln!(
        s,
        "Coding dispatches: {} ({})",
        dispatches.len(),
        dispatches.into_iter().collect::<Vec<_>>().join(", ")
    );
    let _ = writeln!(s, "Coverage reviews: {coverage}");
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn civil_dates() {
        assert_eq!(civil(0), "1970-01-01");
        assert_eq!(civil(20_775), "2026-11-18");
        assert_eq!(civil(11_016), "2000-02-29");
    }
}
