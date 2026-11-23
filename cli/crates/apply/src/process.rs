//! External commands (agents, reviewers, checks, generators) run in their own process
//! group, with stdin, captured output and a time limit. When a command exits or times out,
//! its whole group is stopped, so nothing it started keeps running. On Windows, which has
//! no process groups, the command's process tree is stopped with `taskkill /T` instead;
//! shell lines still run with `sh -c`, which Git for Windows provides.

use std::io::{Read, Write};
#[cfg(unix)]
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use crate::error::{AResult, ApplyError};

/// How a command finished.
#[derive(Clone, Debug, Default)]
pub struct Finished {
    /// The exit code; -1 when killed by a signal or timed out.
    pub code: i32,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub timed_out: bool,
    /// The command could not be started (code 127).
    pub not_started: bool,
}

/// What to run.
pub enum Program<'a> {
    /// An argv, executed without a shell.
    Argv(&'a [String]),
    /// A shell command line, run with `sh -c`.
    Shell(&'a str),
}

/// SIGKILL to the process group `pid` leads, by kill(2) rather than the kill command: its
/// handling of a negative pid differs between implementations, and procps-ng 4.0.2's, in
/// Debian bookworm, killed its caller when the group had already exited.
#[cfg(unix)]
fn kill_group(pid: u32) {
    let Some(pid) = i32::try_from(pid)
        .ok()
        .filter(|p| *p > 1)
        .and_then(rustix::process::Pid::from_raw)
    else {
        return;
    };
    let _ = rustix::process::kill_process_group(pid, rustix::process::Signal::KILL);
}

/// Stops the process tree `pid` leads. Windows finds the tree through parent ids, so
/// processes whose parent has already exited are not reached.
#[cfg(windows)]
fn kill_group(pid: u32) {
    let _ = Command::new("taskkill")
        .args(["/T", "/F", "/PID", &pid.to_string()])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

/// Runs `program` in `cwd`. `capture` false passes stdout and stderr through to ours.
pub fn run(
    program: Program,
    cwd: &Path,
    stdin: &str,
    env: &[(&str, String)],
    timeout: Duration,
    capture: bool,
) -> AResult<Finished> {
    let mut command = match program {
        Program::Argv(argv) => {
            if argv.is_empty() {
                return Ok(Finished {
                    code: 127,
                    not_started: true,
                    stderr: b"empty argv".to_vec(),
                    ..Finished::default()
                });
            }
            let mut c = Command::new(&argv[0]);
            c.args(&argv[1..]);
            c
        }
        Program::Shell(line) => {
            let mut c = Command::new("sh");
            c.arg("-c").arg(line);
            c
        }
    };
    command
        .current_dir(cwd)
        .envs(env.iter().map(|(k, v)| (*k, v.as_str())))
        .stdin(Stdio::piped());
    #[cfg(unix)]
    command.process_group(0);
    if capture {
        command.stdout(Stdio::piped()).stderr(Stdio::piped());
    } else {
        command.stdout(Stdio::null()).stderr(Stdio::inherit());
    }
    let mut child = match command.spawn() {
        Ok(c) => c,
        Err(e) => {
            return Ok(Finished {
                code: 127,
                not_started: true,
                stderr: format!("cannot start: {e}").into_bytes(),
                ..Finished::default()
            })
        }
    };
    let pid = child.id();
    let input = stdin.as_bytes().to_vec();
    let mut pipe_in = child.stdin.take();
    let writer = thread::spawn(move || {
        if let Some(p) = pipe_in.as_mut() {
            let _ = p.write_all(&input);
        }
    });
    let reader = |pipe: Option<Box<dyn Read + Send>>| {
        thread::spawn(move || {
            let mut buf = Vec::new();
            if let Some(mut p) = pipe {
                let _ = p.read_to_end(&mut buf);
            }
            buf
        })
    };
    let out = reader(
        child
            .stdout
            .take()
            .map(|p| Box::new(p) as Box<dyn Read + Send>),
    );
    let err = reader(
        child
            .stderr
            .take()
            .map(|p| Box::new(p) as Box<dyn Read + Send>),
    );
    let start = Instant::now();
    let mut timed_out = false;
    let status = loop {
        match child
            .try_wait()
            .map_err(|e| ApplyError::new(e.to_string()))?
        {
            Some(status) => break Some(status),
            None if start.elapsed() >= timeout => {
                timed_out = true;
                kill_group(pid);
                let _ = child.wait();
                break None;
            }
            None => thread::sleep(Duration::from_millis(20)),
        }
    };
    // Stop anything the command left behind in its group.
    #[cfg(unix)]
    kill_group(pid);
    let _ = writer.join();
    Ok(Finished {
        code: status.and_then(|s| s.code()).unwrap_or(-1),
        stdout: out.join().unwrap_or_default(),
        stderr: err.join().unwrap_or_default(),
        timed_out,
        not_started: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn captures_and_feeds_stdin() {
        let f = run(
            Program::Shell("cat; echo err >&2; exit 3"),
            Path::new("."),
            "hi",
            &[],
            Duration::from_secs(10),
            true,
        )
        .unwrap();
        assert_eq!(f.code, 3);
        assert_eq!(f.stdout, b"hi");
        assert_eq!(f.stderr, b"err\n");
        assert!(!f.timed_out);
    }

    #[test]
    fn times_out_and_stops_the_group() {
        let start = Instant::now();
        let f = run(
            Program::Shell("sleep 30 & sleep 30"),
            Path::new("."),
            "",
            &[],
            Duration::from_millis(300),
            true,
        )
        .unwrap();
        assert!(f.timed_out);
        assert!(start.elapsed() < Duration::from_secs(5));
    }

    #[test]
    fn missing_program() {
        let argv = vec!["/nonexistent/program".to_string()];
        let f = run(
            Program::Argv(&argv),
            Path::new("."),
            "",
            &[],
            Duration::from_secs(1),
            true,
        )
        .unwrap();
        assert!(f.not_started);
        assert_eq!(f.code, 127);
    }
}
