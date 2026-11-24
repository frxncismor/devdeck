//! Forgotten worktrees: linked worktrees with no commit for more than `forgotten_days`, or whose
//! branch is already merged into the base branch. Read-only; removing one reuses
//! `worktrees::remove` (and its `worktree_remove` command).

use crate::process_info::ProcInfo;
use crate::scope::Attributable;
use crate::worktrees::git::{GitRunner, SystemGit};
use crate::worktrees::{self, Worktree};
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Reason {
    /// No commit for more than `forgotten_days`.
    Stale,
    /// The branch is already merged into the base branch.
    Merged,
    /// No uncommitted changes (safe to remove).
    Clean,
    /// Processes are running with their cwd inside the worktree.
    LiveProcesses,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ForgottenWorktree {
    pub worktree: Worktree,
    /// `None` when the folder is missing or could not be sized.
    pub disk_bytes: Option<u64>,
    /// Whole days since the last commit; `None` when there is none (e.g. missing folder).
    pub last_commit_age_days: Option<u64>,
    pub merged: bool,
    /// No uncommitted changes.
    pub clean: bool,
    pub live_processes: u32,
    pub reasons: Vec<Reason>,
}

impl Attributable for ForgottenWorktree {
    fn path(&self) -> Option<&Path> {
        Some(&self.worktree.path)
    }
    fn disk(&self) -> u64 {
        self.disk_bytes.unwrap_or(0)
    }
}

#[derive(Debug, Clone)]
pub struct Options {
    pub forgotten_days: u32,
    pub base_branch: String,
    /// "Now" in Unix seconds (injected for tests).
    pub now_secs: i64,
}

/// `branch` is an ancestor of `base` **and** has moved away from it. A branch still pointing at the
/// very same commit as `base` (a just-created worktree, or a fast-forwarded one) is not reported
/// as merged: that would flag every fresh worktree. Any git failure means "not merged".
fn merged_into_base(git: &dyn GitRunner, w: &Worktree, base: &str) -> bool {
    let Some(branch) = w.branch.as_deref().filter(|b| *b != base) else {
        return false;
    };
    let full = format!("refs/heads/{branch}");
    let Ok(heads) = worktrees::git::run_ok(git, &w.repo_path, &["rev-parse", &full, base, "--"])
    else {
        return false;
    };
    let mut lines = heads.lines();
    if lines.next() == lines.next() {
        return false;
    }
    worktrees::is_merged_with(git, &w.repo_path, branch, base).unwrap_or(false)
}

fn live_processes(w: &Worktree, procs: &[ProcInfo]) -> u32 {
    procs
        .iter()
        .filter(|p| {
            p.cwd
                .as_deref()
                .is_some_and(|c| super::is_within(&w.path, c))
        })
        .count() as u32
}

/// Picks the forgotten worktrees out of `worktrees` (as returned by `worktrees::list`). The main
/// worktree is never reported. `size` is called once per existing worktree folder.
pub fn find_with(
    git: &dyn GitRunner,
    worktrees: Vec<Worktree>,
    procs: &[ProcInfo],
    opts: &Options,
    size: &mut dyn FnMut(&Path) -> Option<u64>,
) -> Vec<ForgottenWorktree> {
    let mut out = Vec::new();
    for w in worktrees.into_iter().filter(|w| !w.is_main) {
        let age_days = w
            .last_commit
            .as_ref()
            .map(|c| (opts.now_secs - c.timestamp).max(0) as u64 / 86_400);
        let stale = age_days.is_some_and(|d| d > u64::from(opts.forgotten_days));
        let merged = merged_into_base(git, &w, &opts.base_branch);
        if !stale && !merged {
            continue;
        }
        let clean = w.dirty_count == 0;
        let live = live_processes(&w, procs);
        let mut reasons = Vec::new();
        if stale {
            reasons.push(Reason::Stale);
        }
        if merged {
            reasons.push(Reason::Merged);
        }
        if clean {
            reasons.push(Reason::Clean);
        }
        if live > 0 {
            reasons.push(Reason::LiveProcesses);
        }
        let disk_bytes = if w.exists { size(&w.path) } else { None };
        out.push(ForgottenWorktree {
            worktree: w,
            disk_bytes,
            last_commit_age_days: age_days,
            merged,
            clean,
            live_processes: live,
            reasons,
        });
    }
    out
}

/// `find_with` over the system git.
pub fn find(
    worktrees: Vec<Worktree>,
    procs: &[ProcInfo],
    opts: &Options,
    size: &mut dyn FnMut(&Path) -> Option<u64>,
) -> Vec<ForgottenWorktree> {
    find_with(&SystemGit, worktrees, procs, opts, size)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::worktrees::git::git_available;
    use std::fs;
    use std::path::PathBuf;
    use std::process::Command;

    fn g(dir: &Path, args: &[&str]) {
        let out = Command::new("git")
            .arg("-C")
            .arg(dir)
            .args([
                "-c",
                "user.name=t",
                "-c",
                "user.email=t@t",
                "-c",
                "commit.gpgsign=false",
            ])
            .args(args)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }

    fn commit_file(dir: &Path, name: &str) {
        fs::write(dir.join(name), name).unwrap();
        g(dir, &["add", "."]);
        g(dir, &["commit", "-m", name]);
    }

    macro_rules! need_git {
        () => {
            if !git_available() {
                eprintln!("git not available: skipping");
                return;
            }
        };
    }

    struct Repo {
        _t: tempfile::TempDir,
        base: PathBuf,
        repo: PathBuf,
    }

    fn repo() -> Repo {
        let t = tempfile::tempdir().unwrap();
        // Real (long-name) path: git reports worktrees that way, so tests compare like with like
        // even when TEMP is an 8.3 short path (as on GitHub's Windows runners).
        let base = crate::fsutil::real_path(t.path()).unwrap();
        let repo = base.join("app");
        fs::create_dir_all(&repo).unwrap();
        g(&repo, &["init", "-b", "main"]);
        commit_file(&repo, "a.txt");
        Repo { _t: t, base, repo }
    }

    fn add_worktree(r: &Repo, branch: &str) -> PathBuf {
        let p = r.base.join(format!("app-{}", branch.replace('/', "-")));
        g(
            &r.repo,
            &["worktree", "add", "-b", branch, p.to_str().unwrap()],
        );
        p
    }

    fn opts(now_offset_days: i64) -> Options {
        Options {
            forgotten_days: 30,
            base_branch: "main".into(),
            now_secs: crate::reclaim::now_secs() as i64 + now_offset_days * 86_400,
        }
    }

    fn find_in(r: &Repo, procs: &[ProcInfo], o: &Options) -> Vec<ForgottenWorktree> {
        let wts = worktrees::list(&r.repo).unwrap();
        find(wts, procs, o, &mut |_| Some(1234))
    }

    fn proc_in(pid: u32, cwd: &Path) -> ProcInfo {
        ProcInfo {
            pid,
            name: "node.exe".into(),
            exe: None,
            cwd: Some(cwd.to_path_buf()),
            ram_bytes: 1,
            cpu_percent: 0.0,
            start_time: 0,
            parent_pid: None,
        }
    }

    #[test]
    fn a_fresh_worktree_is_not_forgotten_and_main_never_is() {
        need_git!();
        let r = repo();
        add_worktree(&r, "feat/new");
        assert!(find_in(&r, &[], &opts(0)).is_empty());
        // Even far in the future the main worktree is never reported, only the linked one.
        let l = find_in(&r, &[], &opts(400));
        assert_eq!(l.len(), 1);
        assert_eq!(l[0].worktree.branch.as_deref(), Some("feat/new"));
        assert!(!l[0].worktree.is_main);
    }

    #[test]
    fn stale_after_forgotten_days_with_age_size_and_reasons() {
        need_git!();
        let r = repo();
        add_worktree(&r, "old");
        assert!(find_in(&r, &[], &opts(29)).is_empty());
        let l = find_in(&r, &[], &opts(45));
        assert_eq!(l.len(), 1);
        assert_eq!(l[0].last_commit_age_days, Some(45));
        assert_eq!(l[0].disk_bytes, Some(1234));
        assert!(!l[0].merged && l[0].clean);
        assert_eq!(l[0].reasons, [Reason::Stale, Reason::Clean]);
        assert_eq!(l[0].live_processes, 0);
    }

    #[test]
    fn the_threshold_is_configurable() {
        need_git!();
        let r = repo();
        add_worktree(&r, "old");
        let mut o = opts(10);
        o.forgotten_days = 5;
        assert_eq!(find_in(&r, &[], &o).len(), 1);
        o.forgotten_days = 60;
        assert!(find_in(&r, &[], &o).is_empty());
    }

    #[test]
    fn merged_branch_is_forgotten_even_when_recent() {
        need_git!();
        let r = repo();
        let feat = add_worktree(&r, "done");
        commit_file(&feat, "feature.txt");
        let wip = add_worktree(&r, "wip");
        commit_file(&wip, "wip.txt");
        g(&r.repo, &["merge", "--no-ff", "-m", "merge done", "done"]);
        let l = find_in(&r, &[], &opts(0));
        assert_eq!(l.len(), 1, "{l:?}");
        assert_eq!(l[0].worktree.branch.as_deref(), Some("done"));
        assert!(l[0].merged);
        assert_eq!(l[0].reasons, [Reason::Merged, Reason::Clean]);
        assert_eq!(l[0].last_commit_age_days, Some(0));
    }

    #[test]
    fn a_branch_still_at_the_base_commit_is_not_reported_as_merged() {
        need_git!();
        let r = repo();
        add_worktree(&r, "same");
        assert!(find_in(&r, &[], &opts(0)).is_empty());
    }

    #[test]
    fn dirty_worktrees_are_flagged_not_clean_and_live_processes_counted() {
        need_git!();
        let r = repo();
        let old = add_worktree(&r, "old");
        fs::write(old.join("a.txt"), "changed").unwrap();
        let other = add_worktree(&r, "other");
        let procs = [
            proc_in(1, &old.join("web")),
            proc_in(2, &old),
            proc_in(3, &other),
            proc_in(4, &r.repo),
        ];
        let l = find_in(&r, &procs, &opts(60));
        let o = l.iter().find(|f| f.worktree.path == old).unwrap();
        assert!(!o.clean);
        assert_eq!(o.live_processes, 2);
        assert_eq!(
            o.reasons,
            [Reason::Stale, Reason::LiveProcesses],
            "dirty => no clean reason"
        );
        let other = l.iter().find(|f| f.worktree.path == other).unwrap();
        assert_eq!(other.live_processes, 1);
    }

    #[test]
    fn a_missing_folder_is_listed_when_merged_but_never_sized() {
        need_git!();
        let r = repo();
        let feat = add_worktree(&r, "gone");
        commit_file(&feat, "f.txt");
        g(&r.repo, &["merge", "--no-ff", "-m", "m", "gone"]);
        fs::remove_dir_all(&feat).unwrap();
        let mut sized = 0;
        let wts = worktrees::list(&r.repo).unwrap();
        let l = find(wts, &[], &opts(0), &mut |_| {
            sized += 1;
            Some(1)
        });
        assert_eq!(l.len(), 1);
        assert!(!l[0].worktree.exists);
        assert_eq!(l[0].disk_bytes, None);
        assert_eq!(sized, 0);
    }

    #[test]
    fn serializes_reasons_as_stable_snake_case_codes() {
        let v = serde_json::to_value([
            Reason::Stale,
            Reason::Merged,
            Reason::Clean,
            Reason::LiveProcesses,
        ])
        .unwrap();
        assert_eq!(
            v,
            serde_json::json!(["stale", "merged", "clean", "live_processes"])
        );
    }
}
