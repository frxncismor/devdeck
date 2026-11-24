//! Git access: the `GitRunner` trait, its `Command` implementation and parsers for the porcelain outputs.

use super::LastCommit;
use crate::error::{Error, ErrorKind, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Result of one git invocation. A non-zero exit is data, not an error (callers decide).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitOutput {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

impl GitOutput {
    pub fn success(&self) -> bool {
        self.code == 0
    }
}

/// Runs `git <args>` with `cwd` as working directory. `Err` only when git itself cannot be started.
pub trait GitRunner {
    fn run(&self, cwd: &Path, args: &[&str]) -> Result<GitOutput>;
}

/// Real implementation: spawns the `git` executable (no shell).
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemGit;

impl GitRunner for SystemGit {
    fn run(&self, cwd: &Path, args: &[&str]) -> Result<GitOutput> {
        let mut cmd = Command::new("git");
        cmd.arg("-C")
            .arg(cwd)
            .args(args)
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("LC_ALL", "C");
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
        }
        let out = cmd.output().map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                Error::new(ErrorKind::Git, "git_not_found").with_detail(e.to_string())
            } else {
                Error::from(e)
            }
        })?;
        Ok(GitOutput {
            code: out.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
        })
    }
}

/// Runs git and turns a non-zero exit into `git_failed` (stderr in `detail`).
pub fn run_ok(git: &dyn GitRunner, cwd: &Path, args: &[&str]) -> Result<String> {
    let out = git.run(cwd, args)?;
    if out.success() {
        Ok(out.stdout)
    } else {
        Err(Error::new(ErrorKind::Git, "git_failed").with_detail(out.stderr.trim().to_owned()))
    }
}

/// True if a `git` executable can be started.
pub fn git_available() -> bool {
    Command::new("git")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// One entry of `git worktree list --porcelain`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawWorktree {
    pub path: PathBuf,
    pub branch: Option<String>,
    pub detached: bool,
    pub bare: bool,
}

/// Parses `git worktree list --porcelain` (blocks separated by blank lines; the first is the main one).
pub fn parse_worktree_list(out: &str) -> Vec<RawWorktree> {
    let mut list = Vec::new();
    let mut cur: Option<RawWorktree> = None;
    for line in out.lines() {
        let line = line.trim_end_matches('\r');
        if line.is_empty() {
            if let Some(w) = cur.take() {
                list.push(w);
            }
        } else if let Some(p) = line.strip_prefix("worktree ") {
            if let Some(w) = cur.take() {
                list.push(w);
            }
            cur = Some(RawWorktree {
                path: PathBuf::from(p),
                branch: None,
                detached: false,
                bare: false,
            });
        } else if let Some(w) = cur.as_mut() {
            if let Some(b) = line.strip_prefix("branch ") {
                w.branch = Some(b.strip_prefix("refs/heads/").unwrap_or(b).to_owned());
            } else if line == "detached" {
                w.detached = true;
            } else if line == "bare" {
                w.bare = true;
            }
        }
    }
    if let Some(w) = cur {
        list.push(w);
    }
    list
}

/// Parsed `git status --porcelain=v2 --branch`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StatusInfo {
    pub dirty_count: u32,
    pub ahead: Option<u32>,
    pub behind: Option<u32>,
    pub branch: Option<String>,
    pub upstream: Option<String>,
}

/// Parses `git status --porcelain=v2 --branch`: each non-header line (changed, renamed, unmerged,
/// untracked) counts as dirty; `# branch.ab` gives ahead/behind (absent without upstream).
pub fn parse_status(out: &str) -> StatusInfo {
    let mut s = StatusInfo::default();
    for line in out.lines() {
        let line = line.trim_end_matches('\r');
        if let Some(h) = line.strip_prefix("# ") {
            if let Some(b) = h.strip_prefix("branch.head ") {
                if b != "(detached)" {
                    s.branch = Some(b.to_owned());
                }
            } else if let Some(u) = h.strip_prefix("branch.upstream ") {
                s.upstream = Some(u.to_owned());
            } else if let Some(ab) = h.strip_prefix("branch.ab ") {
                let mut it = ab.split_whitespace();
                s.ahead = it.next().and_then(|a| a.strip_prefix('+')?.parse().ok());
                s.behind = it.next().and_then(|b| b.strip_prefix('-')?.parse().ok());
            }
        } else if matches!(line.chars().next(), Some('1' | '2' | 'u' | '?')) {
            s.dirty_count += 1;
        }
    }
    s
}

/// Format for `git log -1`: hash, subject, committer date (strict ISO), committer timestamp.
pub const LOG_FORMAT: &str = "--format=%H%x1f%s%x1f%cI%x1f%ct";

/// Parses the output of `git log -1 LOG_FORMAT`. `None` for empty output (no commits).
pub fn parse_log(out: &str) -> Option<LastCommit> {
    let line = out.lines().next()?.trim_end_matches('\r');
    let mut it = line.split('\u{1f}');
    let hash = it.next().filter(|h| !h.is_empty())?;
    let subject = it.next()?;
    let date_iso = it.next()?;
    let timestamp = it.next()?.trim().parse().ok()?;
    Some(LastCommit {
        hash: hash.to_owned(),
        subject: subject.to_owned(),
        date_iso: date_iso.to_owned(),
        timestamp,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_worktree_list() {
        let out = "worktree C:/r/app\nHEAD aaa\nbranch refs/heads/main\n\n\
                   worktree C:/w/app-feat\nHEAD bbb\nbranch refs/heads/feat/x\n\n\
                   worktree C:/w/app-det\nHEAD ccc\ndetached\n\n\
                   worktree C:/w/gone\nHEAD ddd\nbranch refs/heads/gone\nprunable gitdir file points to non-existent location\n";
        let l = parse_worktree_list(out);
        assert_eq!(l.len(), 4);
        assert_eq!(l[0].path, PathBuf::from("C:/r/app"));
        assert_eq!(l[0].branch.as_deref(), Some("main"));
        assert_eq!(l[1].branch.as_deref(), Some("feat/x"));
        assert!(l[2].detached && l[2].branch.is_none());
        assert_eq!(l[3].branch.as_deref(), Some("gone"));
    }

    #[test]
    fn parses_worktree_list_with_crlf_and_bare() {
        let l = parse_worktree_list("worktree /r/bare.git\r\nbare\r\n\r\n");
        assert_eq!(l.len(), 1);
        assert!(l[0].bare);
    }

    #[test]
    fn parses_status_with_upstream_and_dirty() {
        let out = "# branch.oid abc\n# branch.head main\n# branch.upstream origin/main\n# branch.ab +2 -3\n\
                   1 .M N... 100644 100644 100644 a b src/a.rs\n\
                   2 R. N... 100644 100644 100644 a b R100 new.rs\told.rs\n\
                   u UU N... 1 2 3 4 a b c conflict.rs\n\
                   ? untracked.txt\n";
        let s = parse_status(out);
        assert_eq!(s.dirty_count, 4);
        assert_eq!((s.ahead, s.behind), (Some(2), Some(3)));
        assert_eq!(s.branch.as_deref(), Some("main"));
        assert_eq!(s.upstream.as_deref(), Some("origin/main"));
    }

    #[test]
    fn parses_status_without_upstream_and_detached() {
        let s = parse_status("# branch.oid abc\n# branch.head (detached)\n");
        assert_eq!(s.dirty_count, 0);
        assert_eq!((s.ahead, s.behind), (None, None));
        assert_eq!(s.branch, None);
    }

    #[test]
    fn parses_log() {
        let out =
            "abc123\u{1f}fix: thing, with commas\u{1f}2026-11-20T10:00:00+01:00\u{1f}1795000000\n";
        let c = parse_log(out).unwrap();
        assert_eq!(c.hash, "abc123");
        assert_eq!(c.subject, "fix: thing, with commas");
        assert_eq!(c.date_iso, "2026-11-20T10:00:00+01:00");
        assert_eq!(c.timestamp, 1_795_000_000);
        assert!(parse_log("").is_none());
    }
}
