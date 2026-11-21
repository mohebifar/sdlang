//! Git operations for apply: HEAD, candidate worktrees, frozen snapshots, the frozen diff
//! and publishing by fast-forward. Everything goes through the `git` command.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::error::{AResult, ApplyError};

/// The user's repository (its main worktree).
pub struct Repo {
    pub root: PathBuf,
}

/// One changed path in a frozen diff.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Change {
    Added(String),
    Modified(String),
    Deleted(String),
}

impl Change {
    pub fn path(&self) -> &str {
        match self {
            Change::Added(p) | Change::Modified(p) | Change::Deleted(p) => p,
        }
    }
}

fn git(dir: &Path, args: &[&str]) -> AResult<String> {
    let out = Command::new("git")
        .current_dir(dir)
        .args([
            "-c",
            "user.name=spec",
            "-c",
            "user.email=spec@localhost",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .output()
        .map_err(|e| ApplyError::new(format!("cannot run git: {e}")))?;
    if !out.status.success() {
        return Err(ApplyError::new(format!(
            "git {} failed in {}: {}",
            args.join(" "),
            dir.display(),
            String::from_utf8_lossy(&out.stderr).trim()
        )));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Every file git sees in `dir` (tracked, or untracked and not ignored), sorted, except
/// `.spec/` and the plan document: the whole tree as a run input closure counts it.
pub fn tree_files(dir: &Path) -> AResult<Vec<String>> {
    let out = git(dir, &["ls-files", "-z", "-co", "--exclude-standard"])?;
    let mut files: Vec<String> = out
        .split('\0')
        .filter(|p| !p.is_empty() && !p.starts_with(".spec/") && *p != "plan.json")
        .map(String::from)
        .collect();
    files.sort();
    files.dedup();
    Ok(files)
}

/// `git:<sha>` for a 40-hex object id.
pub fn tagged(sha: &str) -> String {
    format!("git:{}", sha.trim())
}

pub fn untagged(commit: &str) -> &str {
    commit.strip_prefix("git:").unwrap_or(commit)
}

impl Repo {
    pub fn open(root: &Path) -> AResult<Repo> {
        let top = git(root, &["rev-parse", "--show-toplevel"])?;
        let top = PathBuf::from(top.trim());
        let root = fs::canonicalize(root)
            .map_err(|e| ApplyError::new(format!("{}: {e}", root.display())))?;
        if fs::canonicalize(&top).ok() != Some(root.clone()) {
            return Err(ApplyError::new(format!(
                "{} must be the top of its git work tree ({} is)",
                root.display(),
                top.display()
            )));
        }
        let repo = Repo { root };
        repo.exclude_protocol_area()?;
        Ok(repo)
    }

    /// `.spec/run/` and `.spec/journal/` are never committed.
    fn exclude_protocol_area(&self) -> AResult<()> {
        let common = git(&self.root, &["rev-parse", "--git-common-dir"])?;
        let common = self.root.join(common.trim());
        let path = common.join("info/exclude");
        let current = fs::read_to_string(&path).unwrap_or_default();
        let mut text = current.clone();
        for line in ["/.spec/run/", "/.spec/journal/"] {
            if !current.lines().any(|l| l == line) {
                if !text.is_empty() && !text.ends_with('\n') {
                    text.push('\n');
                }
                text.push_str(line);
                text.push('\n');
            }
        }
        if text != current {
            fs::create_dir_all(path.parent().unwrap())
                .and_then(|_| fs::write(&path, text))
                .map_err(|e| ApplyError::new(format!("{}: {e}", path.display())))?;
        }
        Ok(())
    }

    pub fn head(&self) -> AResult<String> {
        Ok(tagged(&git(
            &self.root,
            &["rev-parse", "--verify", "HEAD"],
        )?))
    }

    /// Whether the user's worktree has uncommitted changes to tracked or untracked files
    /// (ignored files aside), other than `plan.json` itself.
    pub fn dirty(&self) -> AResult<Vec<String>> {
        let out = git(
            &self.root,
            &["status", "--porcelain", "--untracked-files=all"],
        )?;
        Ok(out
            .lines()
            .map(|l| l[3..].to_string())
            .filter(|p| p != "plan.json")
            .collect())
    }

    /// A fresh candidate worktree at `base`, under `.spec/run/`.
    pub fn candidate(&self, name: &str, base: &str) -> AResult<Candidate> {
        let dir = self.root.join(".spec/run/candidates").join(name);
        if dir.exists() {
            let _ = git(
                &self.root,
                &["worktree", "remove", "--force", dir.to_str().unwrap()],
            );
            let _ = fs::remove_dir_all(&dir);
        }
        let _ = git(&self.root, &["worktree", "prune"]);
        fs::create_dir_all(dir.parent().unwrap()).map_err(|e| ApplyError::new(e.to_string()))?;
        git(
            &self.root,
            &[
                "worktree",
                "add",
                "--detach",
                "--quiet",
                dir.to_str().unwrap(),
                untagged(base),
            ],
        )?;
        self.link_dependencies(&dir)?;
        Ok(Candidate {
            dir,
            base: base.to_string(),
        })
    }

    /// The installed dependency tree is not in git, so a candidate links the project's
    /// `node_modules` (TypeScript is the only v0 stack) and git never sees the link.
    /// Checks then run with exactly the dependencies the lockfile installed.
    fn link_dependencies(&self, dir: &Path) -> AResult<()> {
        let installed = self.root.join("node_modules");
        if !installed.is_dir() || dir.join("node_modules").exists() {
            return Ok(());
        }
        let common = git(&self.root, &["rev-parse", "--git-common-dir"])?;
        let exclude = self.root.join(common.trim()).join("info/exclude");
        let current = fs::read_to_string(&exclude).unwrap_or_default();
        if !current.lines().any(|l| l == "/node_modules") {
            let sep = if current.is_empty() || current.ends_with('\n') {
                ""
            } else {
                "\n"
            };
            fs::write(&exclude, format!("{current}{sep}/node_modules\n"))
                .map_err(|e| ApplyError::new(format!("{}: {e}", exclude.display())))?;
        }
        std::os::unix::fs::symlink(&installed, dir.join("node_modules")).map_err(|e| {
            ApplyError::new(format!("cannot link node_modules into the candidate: {e}"))
        })
    }

    /// Moves the current branch to `commit`, which must descend from HEAD, and updates the
    /// worktree.
    pub fn fast_forward(&self, commit: &str) -> AResult<()> {
        git(
            &self.root,
            &["merge", "--ff-only", "--quiet", untagged(commit)],
        )?;
        Ok(())
    }

    pub fn remove_candidate(&self, c: &Candidate) {
        let _ = git(
            &self.root,
            &["worktree", "remove", "--force", c.dir.to_str().unwrap()],
        );
    }

    pub fn is_ancestor(&self, ancestor: &str, of: &str) -> bool {
        git(
            &self.root,
            &[
                "merge-base",
                "--is-ancestor",
                untagged(ancestor),
                untagged(of),
            ],
        )
        .is_ok()
    }

    /// A file's text at a commit, if it exists there.
    pub fn show(&self, commit: &str, path: &str) -> Option<String> {
        git(
            &self.root,
            &["show", &format!("{}:{path}", untagged(commit))],
        )
        .ok()
    }
}

/// A candidate worktree.
pub struct Candidate {
    pub dir: PathBuf,
    /// The commit it started from.
    pub base: String,
}

impl Candidate {
    /// Commits the whole tree (ignored files aside) and returns the snapshot commit.
    pub fn freeze(&self, message: &str) -> AResult<String> {
        git(&self.dir, &["add", "-A"])?;
        git(
            &self.dir,
            &["commit", "--quiet", "--allow-empty", "-m", message],
        )?;
        Ok(tagged(&git(&self.dir, &["rev-parse", "HEAD"])?))
    }

    /// Paths changed from the base to the current HEAD of the candidate.
    pub fn diff_from_base(&self) -> AResult<Vec<Change>> {
        let out = git(
            &self.dir,
            &[
                "diff",
                "--name-status",
                "--no-renames",
                untagged(&self.base),
                "HEAD",
            ],
        )?;
        Ok(out
            .lines()
            .filter_map(|l| {
                let (status, path) = l.split_once('\t')?;
                Some(match status {
                    "A" => Change::Added(path.into()),
                    "D" => Change::Deleted(path.into()),
                    _ => Change::Modified(path.into()),
                })
            })
            .collect())
    }

    /// Discards everything not in the frozen HEAD (what checks wrote), ignored files aside.
    pub fn restore(&self) -> AResult<()> {
        git(&self.dir, &["reset", "--hard", "--quiet", "HEAD"])?;
        git(&self.dir, &["clean", "-fdq"])?;
        Ok(())
    }

    /// Whether the working tree differs from the frozen HEAD.
    pub fn changed_since_freeze(&self) -> AResult<bool> {
        Ok(!git(
            &self.dir,
            &["status", "--porcelain", "--untracked-files=all"],
        )?
        .trim()
        .is_empty())
    }

    /// Replaces the snapshot commits made since `base` with one commit carrying `message`
    /// (the transaction commit), and returns it.
    pub fn squash(&self, message: &str) -> AResult<String> {
        git(
            &self.dir,
            &["reset", "--soft", "--quiet", untagged(&self.base)],
        )?;
        git(&self.dir, &["add", "-A"])?;
        git(
            &self.dir,
            &["commit", "--quiet", "--allow-empty", "-m", message],
        )?;
        Ok(tagged(&git(&self.dir, &["rev-parse", "HEAD"])?))
    }

    /// The patch of the candidate against its base, kept when a transaction fails.
    pub fn patch(&self) -> String {
        let _ = git(&self.dir, &["add", "-A"]);
        git(&self.dir, &["diff", "--binary", untagged(&self.base)]).unwrap_or_default()
    }
}
