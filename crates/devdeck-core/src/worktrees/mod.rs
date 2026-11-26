//! Worktrees module (phase 2A): list, scan the configured root, remove, open, and helpers used by
//! Ports (2B) and Reclaim (2D).

pub mod git;
pub mod open;
pub mod scan;

use crate::error::{Error, ErrorKind, Result};
use crate::process_info::{ProcInfo, ProcSource, SysinfoSource};
use crate::project::{build_projects, Project, ProjectId, ProjectRef};
use crate::reclaim::is_within;
use crate::scope::{self, Attributable};
use git::{GitRunner, SystemGit};
use serde::Serialize;
use std::path::{Path, PathBuf};

pub use scan::scan;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LastCommit {
    pub hash: String,
    pub subject: String,
    pub date_iso: String,
    /// Committer time as Unix seconds (added in 2A so Reclaim can compute ages without parsing ISO dates).
    pub timestamp: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Worktree {
    pub path: PathBuf,
    pub repo_path: PathBuf,
    pub project: Option<ProjectRef>,
    pub branch: Option<String>,
    pub detached: bool,
    pub is_main: bool,
    pub dirty_count: u32,
    pub ahead: Option<u32>,
    pub behind: Option<u32>,
    pub last_commit: Option<LastCommit>,
    pub exists: bool,
}

impl Attributable for Worktree {
    fn path(&self) -> Option<&Path> {
        Some(&self.path)
    }
}

/// git prints `C:/a/b`; keep native separators so paths compare and display consistently.
fn native(p: &Path) -> PathBuf {
    if cfg!(windows) {
        PathBuf::from(p.to_string_lossy().replace('/', "\\"))
    } else {
        p.to_path_buf()
    }
}

/// Same location: equal after normalization or, when both exist, equal once canonicalized
/// (git reports long names while a caller may pass an 8.3 short name such as `RUNNER~1`).
fn same_path(a: &Path, b: &Path) -> bool {
    if scope::normalize(a) == scope::normalize(b) {
        return true;
    }
    match (crate::fsutil::real_path(a), crate::fsutil::real_path(b)) {
        (Some(x), Some(y)) => scope::normalize(&x) == scope::normalize(&y),
        _ => false,
    }
}

fn project_ref(main: &Path) -> ProjectRef {
    ProjectRef {
        id: ProjectId::from_path(main),
        name: main
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| main.to_string_lossy().into_owned()),
    }
}

/// Lists the worktrees of the repository at `repo` (main worktree first).
pub fn list(repo: &Path) -> Result<Vec<Worktree>> {
    list_with(&SystemGit, repo)
}

pub fn list_with(git: &dyn GitRunner, repo: &Path) -> Result<Vec<Worktree>> {
    let out = git::run_ok(git, repo, &["worktree", "list", "--porcelain"])?;
    let raw = git::parse_worktree_list(&out);
    let Some(main) = raw.first() else {
        return Ok(Vec::new());
    };
    let repo_path = native(&main.path);
    let project = project_ref(&repo_path);
    let entries: Vec<(usize, &git::RawWorktree)> =
        raw.iter().enumerate().filter(|(_, r)| !r.bare).collect();
    // Status and last commit are independent git processes per worktree: run a few at a time.
    Ok(par_map(&entries, GIT_PARALLELISM, |(i, r)| {
        let path = native(&r.path);
        let exists = path.is_dir();
        let mut w = Worktree {
            path: path.clone(),
            repo_path: repo_path.clone(),
            project: Some(project.clone()),
            branch: r.branch.clone(),
            detached: r.detached,
            is_main: *i == 0,
            dirty_count: 0,
            ahead: None,
            behind: None,
            last_commit: None,
            exists,
        };
        if !exists {
            // The folder is gone, but the repository still knows its HEAD and branch: report those
            // rather than "no commits" / "no remote branch".
            if let Some(head) = r.head.as_deref() {
                w.last_commit = last_commit_with(git, &repo_path, head);
            }
            if let Some((ahead, behind)) = r
                .branch
                .as_deref()
                .and_then(|b| upstream_counts(git, &repo_path, b))
            {
                w.ahead = Some(ahead);
                w.behind = Some(behind);
            }
        } else {
            if let Ok(s) = git.run(&path, &["status", "--porcelain=v2", "--branch"]) {
                if s.success() {
                    let st = git::parse_status(&s.stdout);
                    w.dirty_count = st.dirty_count;
                    w.ahead = st.ahead;
                    w.behind = st.behind;
                }
            }
            w.last_commit = last_commit_with(git, &path, "HEAD");
        }
        w
    }))
}

/// Maximum git processes per repository (and repositories) running at once.
const GIT_PARALLELISM: usize = 4;

/// `items.iter().map(f)` on up to `limit` threads, keeping the input order.
fn par_map<T: Sync, R: Send>(items: &[T], limit: usize, f: impl Fn(&T) -> R + Sync) -> Vec<R> {
    let n = items.len();
    if n <= 1 || limit <= 1 {
        return items.iter().map(f).collect();
    }
    let next = std::sync::atomic::AtomicUsize::new(0);
    let slots: std::sync::Mutex<Vec<Option<R>>> =
        std::sync::Mutex::new(std::iter::repeat_with(|| None).take(n).collect());
    std::thread::scope(|s| {
        for _ in 0..limit.min(n) {
            s.spawn(|| loop {
                let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                if i >= n {
                    break;
                }
                let r = f(&items[i]);
                slots.lock().unwrap()[i] = Some(r);
            });
        }
    });
    slots.into_inner().unwrap().into_iter().flatten().collect()
}

/// `(ahead, behind)` of a local branch against its upstream; `None` without one.
fn upstream_counts(git: &dyn GitRunner, repo: &Path, branch: &str) -> Option<(u32, u32)> {
    let range = format!("{branch}...{branch}@{{upstream}}");
    let out = git
        .run(repo, &["rev-list", "--left-right", "--count", &range, "--"])
        .ok()
        .filter(|o| o.success())?;
    let mut it = out.stdout.split_whitespace().map(|n| n.parse().ok());
    Some((it.next()??, it.next()??))
}

fn last_commit_with(git: &dyn GitRunner, dir: &Path, rev: &str) -> Option<LastCommit> {
    let out = git
        .run(dir, &["log", "-1", git::LOG_FORMAT, rev, "--"])
        .ok()?;
    if out.success() {
        git::parse_log(&out.stdout)
    } else {
        None
    }
}

/// Worktrees of every repository found under `root` (up to `depth`). A repo that fails to list is skipped.
pub fn list_root(git: &dyn GitRunner, root: &Path, depth: u8) -> Vec<Worktree> {
    let repos = scan(root, depth);
    par_map(&repos, GIT_PARALLELISM, |repo| list_with(git, repo).ok())
        .into_iter()
        .flatten()
        .flatten()
        .collect()
}

/// Everything the UI needs from one root scan: all worktrees and the derived projects.
pub fn scan_root(git: &dyn GitRunner, root: &Path, depth: u8) -> (Vec<Worktree>, Vec<Project>) {
    let worktrees = list_root(git, root, depth);
    let projects = build_projects(&worktrees);
    (worktrees, projects)
}

/// Projects found under the configured root (what `list_projects` / `AppState.projects` hold).
pub fn projects_from_root(root: &Path, depth: u8) -> Vec<Project> {
    scan_root(&SystemGit, root, depth).1
}

/// `git merge-base --is-ancestor <branch> <base>`: true when `branch` is fully merged into `base`.
/// Exit 1 means "not an ancestor"; any other failure (unknown ref, git error) is an error.
pub fn is_merged(repo: &Path, branch: &str, base: &str) -> Result<bool> {
    is_merged_with(&SystemGit, repo, branch, base)
}

pub fn is_merged_with(git: &dyn GitRunner, repo: &Path, branch: &str, base: &str) -> Result<bool> {
    let out = git.run(repo, &["merge-base", "--is-ancestor", branch, base])?;
    match out.code {
        0 => Ok(true),
        1 => Ok(false),
        _ => {
            Err(Error::new(ErrorKind::Git, "git_failed").with_detail(out.stderr.trim().to_owned()))
        }
    }
}

/// Committer time (Unix seconds) of `rev` (default `HEAD`) in `dir`; `None` if there are no commits.
pub fn last_commit_time(dir: &Path, rev: Option<&str>) -> Result<Option<i64>> {
    let rev = rev.unwrap_or("HEAD");
    let out = SystemGit.run(dir, &["log", "-1", "--format=%ct", rev, "--"])?;
    Ok(out
        .success()
        .then(|| out.stdout.trim().parse().ok())
        .flatten())
}

/// Removes the linked worktree at `path`.
/// - The main worktree is never removable (`worktree_main_not_removable`).
/// - A dirty worktree needs `force = true` (otherwise `worktree_dirty`, which the UI turns into a confirmation).
/// - A folder with processes running inside is left untouched (`worktree_in_use`, param `count`):
///   git would delete its files and unregister it before failing on the folder itself.
/// - `repo_path` is the project's main repository: git runs from there. A worktree whose folder no
///   longer exists is forgotten on its own (never `prune`, which forgets every missing one).
pub fn remove(repo_path: &Path, path: &Path, force: bool) -> Result<()> {
    remove_with(&SystemGit, &SysinfoSource, repo_path, path, force)
}

/// Processes whose cwd is the folder `root` or lives under it (an unreadable cwd cannot be counted).
/// Compared as written and canonicalized, since a cwd may use 8.3 names or go through a link.
fn processes_inside(root: &Path, procs: &[ProcInfo]) -> usize {
    let real_root = crate::fsutil::real_path(root);
    procs
        .iter()
        .filter_map(|p| p.cwd.as_deref())
        .filter(|cwd| {
            is_within(root, cwd)
                || real_root.as_deref().is_some_and(|r| {
                    crate::fsutil::real_path(cwd).is_some_and(|c| is_within(r, &c))
                })
        })
        .count()
}

pub fn remove_with(
    git: &dyn GitRunner,
    procs: &dyn ProcSource,
    repo_path: &Path,
    path: &Path,
    force: bool,
) -> Result<()> {
    let out = git::run_ok(git, repo_path, &["worktree", "list", "--porcelain"])?;
    let raw = git::parse_worktree_list(&out);
    let main = raw
        .first()
        .ok_or_else(|| Error::not_found("not_found"))?
        .path
        .clone();
    let target = raw
        .iter()
        .find(|r| same_path(&r.path, path))
        .ok_or_else(|| Error::not_found("not_found"))?;
    if same_path(&target.path, &main) {
        return Err(Error::not_allowed("worktree_main_not_removable"));
    }
    let target_path = native(&target.path);
    let target_str = target_path.to_string_lossy().into_owned();
    if !path.is_dir() {
        let args = ["worktree", "remove", "--", target_str.as_str()];
        git::run_ok(git, &native(&main), &args)?;
        return Ok(());
    }
    let running = processes_inside(&target_path, &procs.snapshot());
    if running > 0 {
        return Err(Error::not_allowed("worktree_in_use").with_param("count", running));
    }
    let s = git::run_ok(git, path, &["status", "--porcelain=v2"])?;
    let dirty = git::parse_status(&s).dirty_count > 0;
    if dirty && !force {
        return Err(Error::not_allowed("worktree_dirty"));
    }
    let mut args = vec!["worktree", "remove"];
    if dirty {
        args.push("--force");
    }
    args.push("--");
    args.push(&target_str);
    git::run_ok(git, &native(&main), &args)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use git::git_available;
    use std::fs;
    use std::process::Command;

    /// Runs git in `dir`, panicking on failure.
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

    fn init_repo(dir: &Path) {
        fs::create_dir_all(dir).unwrap();
        g(dir, &["init", "-b", "main"]);
        fs::write(dir.join("a.txt"), "a").unwrap();
        g(dir, &["add", "."]);
        g(dir, &["commit", "-m", "first"]);
    }

    macro_rules! need_git {
        () => {
            if !git_available() {
                eprintln!("git not available: skipping");
                return;
            }
        };
    }

    fn canon(p: &Path) -> String {
        scope::normalize(&fs::canonicalize(p).unwrap())
    }

    #[test]
    fn lists_main_linked_dirty_and_detached() {
        need_git!();
        let t = tempfile::tempdir().unwrap();
        let repo = t.path().join("app");
        init_repo(&repo);
        let feat = t.path().join("app-feat");
        g(
            &repo,
            &["worktree", "add", "-b", "feat/x", feat.to_str().unwrap()],
        );
        let det = t.path().join("app-det");
        g(
            &repo,
            &["worktree", "add", "--detach", det.to_str().unwrap()],
        );
        fs::write(feat.join("new.txt"), "x").unwrap();
        fs::write(feat.join("a.txt"), "changed").unwrap();

        let l = list(&repo).unwrap();
        assert_eq!(l.len(), 3);
        assert!(l[0].is_main && l.iter().filter(|w| w.is_main).count() == 1);
        assert_eq!(l[0].branch.as_deref(), Some("main"));
        assert_eq!(canon(&l[0].path), canon(&repo));
        assert!(l.iter().all(|w| w.repo_path == l[0].repo_path));
        assert_eq!(l[0].project.as_ref().unwrap().name, "app");
        assert_eq!(l[0].dirty_count, 0);

        let by_name = |n: &str| l.iter().find(|w| w.path.ends_with(n)).unwrap();
        let (f, d) = (by_name("app-feat"), by_name("app-det"));
        assert_eq!(f.branch.as_deref(), Some("feat/x"));
        assert_eq!(f.dirty_count, 2);
        assert!(f.exists);
        assert!(d.detached && d.branch.is_none());

        let c = l[0].last_commit.as_ref().unwrap();
        assert_eq!(c.subject, "first");
        assert!(c.timestamp > 1_600_000_000);
        assert!(c.date_iso.contains('T'));
    }

    #[test]
    fn no_upstream_means_no_ahead_behind_and_upstream_gives_counts() {
        need_git!();
        let t = tempfile::tempdir().unwrap();
        let repo = t.path().join("app");
        init_repo(&repo);
        assert_eq!(list(&repo).unwrap()[0].ahead, None);

        let remote = t.path().join("remote.git");
        fs::create_dir_all(&remote).unwrap();
        g(&remote, &["init", "--bare", "-b", "main"]);
        g(
            &repo,
            &["remote", "add", "origin", remote.to_str().unwrap()],
        );
        g(&repo, &["push", "-u", "origin", "main"]);
        fs::write(repo.join("b.txt"), "b").unwrap();
        g(&repo, &["add", "."]);
        g(&repo, &["commit", "-m", "second"]);
        let w = &list(&repo).unwrap()[0];
        assert_eq!((w.ahead, w.behind), (Some(1), Some(0)));
    }

    #[test]
    fn missing_worktree_dir_is_reported_as_not_existing() {
        need_git!();
        let t = tempfile::tempdir().unwrap();
        let repo = t.path().join("app");
        init_repo(&repo);
        let feat = t.path().join("app-feat");
        g(
            &repo,
            &["worktree", "add", "-b", "feat", feat.to_str().unwrap()],
        );
        fs::remove_dir_all(&feat).unwrap();
        let l = list(&repo).unwrap();
        assert_eq!(l.len(), 2);
        assert!(!l[1].exists);
        assert_eq!(l[1].dirty_count, 0);
    }

    #[test]
    fn a_missing_folder_keeps_the_last_commit_and_remote_state_of_its_branch() {
        need_git!();
        let t = tempfile::tempdir().unwrap();
        let repo = t.path().join("app");
        init_repo(&repo);
        let remote = t.path().join("remote.git");
        fs::create_dir_all(&remote).unwrap();
        g(&remote, &["init", "--bare", "-b", "main"]);
        g(
            &repo,
            &["remote", "add", "origin", remote.to_str().unwrap()],
        );
        let add = |branch: &str, dir: &Path| {
            g(
                &repo,
                &["worktree", "add", "-b", branch, dir.to_str().unwrap()],
            );
            fs::write(dir.join(branch), branch).unwrap();
            g(dir, &["add", "."]);
            g(dir, &["commit", "-m", branch]);
        };
        let pushed = t.path().join("app-pushed");
        add("pushed", &pushed);
        g(&pushed, &["push", "-u", "origin", "pushed"]);
        fs::write(pushed.join("more"), "more").unwrap();
        g(&pushed, &["add", "."]);
        g(&pushed, &["commit", "-m", "more"]);
        let local = t.path().join("app-local");
        add("local", &local);
        let det = t.path().join("app-det");
        g(
            &repo,
            &["worktree", "add", "--detach", det.to_str().unwrap()],
        );
        for d in [&pushed, &local, &det] {
            fs::remove_dir_all(d).unwrap();
        }

        let l = list(&repo).unwrap();
        let by_name = |n: &str| l.iter().find(|w| w.path.ends_with(n)).unwrap();
        let subject = |n: &str| by_name(n).last_commit.as_ref().unwrap().subject.clone();
        assert!(l.iter().skip(1).all(|w| !w.exists));
        assert_eq!(subject("app-pushed"), "more");
        assert_eq!(
            (by_name("app-pushed").ahead, by_name("app-pushed").behind),
            (Some(1), Some(0))
        );
        assert_eq!(subject("app-local"), "local");
        assert_eq!(by_name("app-local").ahead, None);
        assert_eq!(subject("app-det"), "first");
    }

    #[test]
    fn removing_a_missing_worktree_forgets_only_that_one() {
        need_git!();
        let t = tempfile::tempdir().unwrap();
        let repo = t.path().join("app");
        init_repo(&repo);
        let one = t.path().join("app-one");
        let two = t.path().join("app-two");
        for (branch, dir) in [("one", &one), ("two", &two)] {
            g(
                &repo,
                &["worktree", "add", "-b", branch, dir.to_str().unwrap()],
            );
            fs::remove_dir_all(dir).unwrap();
        }

        remove(&repo, &one, false).unwrap();

        let l = list(&repo).unwrap();
        assert_eq!(l.len(), 2, "{l:?}");
        assert!(l[1].path.ends_with("app-two"));
    }

    #[test]
    fn list_with_a_failing_git_is_a_git_error() {
        let e = list_with(&FakeGit(vec![]), Path::new(".")).unwrap_err();
        assert_eq!(e.kind, ErrorKind::Git);
        assert_eq!(e.code, "git_failed");
        assert_eq!(e.detail.as_deref(), Some("fatal"));
    }

    #[test]
    fn remove_rejects_main_and_requires_force_when_dirty() {
        need_git!();
        let t = tempfile::tempdir().unwrap();
        let repo = t.path().join("app");
        init_repo(&repo);
        let feat = t.path().join("app-feat");
        g(
            &repo,
            &["worktree", "add", "-b", "feat", feat.to_str().unwrap()],
        );

        let e = remove(&repo, &repo, true).unwrap_err();
        assert_eq!(e.code, "worktree_main_not_removable");
        assert!(repo.is_dir());

        fs::write(feat.join("dirty.txt"), "x").unwrap();
        let e = remove(&repo, &feat, false).unwrap_err();
        assert_eq!(e.code, "worktree_dirty");
        assert!(feat.is_dir());

        remove(&repo, &feat, true).unwrap();
        assert!(!feat.exists());
        assert_eq!(list(&repo).unwrap().len(), 1);
    }

    #[test]
    fn remove_clean_worktree_without_force_and_unknown_path_is_not_found() {
        need_git!();
        let t = tempfile::tempdir().unwrap();
        let repo = t.path().join("app");
        init_repo(&repo);
        let feat = t.path().join("app-feat");
        g(
            &repo,
            &["worktree", "add", "-b", "feat", feat.to_str().unwrap()],
        );
        remove(&repo, &feat, false).unwrap();
        assert!(!feat.exists());
        let e = remove(&repo, &repo.join("nope"), false).unwrap_err();
        assert!(matches!(e.kind, ErrorKind::Git | ErrorKind::NotFound));
    }

    #[test]
    fn is_merged_and_last_commit_time() {
        need_git!();
        let t = tempfile::tempdir().unwrap();
        let repo = t.path().join("app");
        init_repo(&repo);
        g(&repo, &["branch", "merged"]);
        g(&repo, &["checkout", "-b", "ahead"]);
        fs::write(repo.join("c.txt"), "c").unwrap();
        g(&repo, &["add", "."]);
        g(&repo, &["commit", "-m", "extra"]);
        assert!(is_merged(&repo, "merged", "main").unwrap());
        assert!(!is_merged(&repo, "ahead", "main").unwrap());
        assert_eq!(
            is_merged(&repo, "nope", "main").unwrap_err().kind,
            ErrorKind::Git
        );
        let ts = last_commit_time(&repo, Some("main")).unwrap().unwrap();
        assert!(ts > 1_600_000_000);
        assert!(last_commit_time(&repo, None).unwrap().is_some());
    }

    #[test]
    fn root_scan_builds_projects_including_external_worktrees() {
        need_git!();
        let t = tempfile::tempdir().unwrap();
        let root = t.path().join("root");
        init_repo(&root.join("alpha"));
        init_repo(&root.join("beta"));
        let ext = t.path().join("elsewhere-alpha-feat");
        g(
            &root.join("alpha"),
            &["worktree", "add", "-b", "f", ext.to_str().unwrap()],
        );

        let (wts, projects) = scan_root(&SystemGit, &root, 2);
        assert_eq!(wts.len(), 3);
        assert_eq!(projects.len(), 2);
        assert_eq!(projects[0].name, "alpha");
        assert_eq!(projects[0].worktree_paths.len(), 1);
        assert_eq!(projects_from_root(&root, 2).len(), 2);
    }

    #[test]
    fn worktree_serializes_camel_case_with_timestamp() {
        let w = Worktree {
            path: "/r/a".into(),
            repo_path: "/r/a".into(),
            project: None,
            branch: Some("main".into()),
            detached: false,
            is_main: true,
            dirty_count: 1,
            ahead: Some(1),
            behind: None,
            last_commit: Some(LastCommit {
                hash: "h".into(),
                subject: "s".into(),
                date_iso: "d".into(),
                timestamp: 5,
            }),
            exists: true,
        };
        let v = serde_json::to_value(&w).unwrap();
        assert_eq!(v["isMain"], true);
        assert_eq!(v["lastCommit"]["timestamp"], 5);
        assert_eq!(v["dirtyCount"], 1);
    }

    #[test]
    fn par_map_keeps_order_and_runs_everything() {
        let items: Vec<u32> = (0..50).collect();
        let out = par_map(&items, 4, |n| {
            std::thread::sleep(std::time::Duration::from_millis(u64::from(50 - n) % 3));
            n * 2
        });
        assert_eq!(out, items.iter().map(|n| n * 2).collect::<Vec<_>>());
        assert_eq!(par_map(&[7u32], 4, |n| n + 1), vec![8]);
        assert!(par_map::<u32, u32>(&[], 4, |n| *n).is_empty());
    }

    struct FakeGit(Vec<(&'static str, i32)>);
    impl GitRunner for FakeGit {
        fn run(&self, _cwd: &Path, args: &[&str]) -> Result<git::GitOutput> {
            let joined = args.join(" ");
            let code = self
                .0
                .iter()
                .find(|(k, _)| joined.starts_with(k))
                .map_or(128, |(_, c)| *c);
            Ok(git::GitOutput {
                code,
                stdout: String::new(),
                stderr: "fatal".into(),
            })
        }
    }

    /// Records every git call; `worktree list` reports a main repo and a linked worktree.
    struct RecordingGit {
        list: String,
        calls: std::sync::Mutex<Vec<String>>,
    }
    impl GitRunner for RecordingGit {
        fn run(&self, _cwd: &Path, args: &[&str]) -> Result<git::GitOutput> {
            self.calls.lock().unwrap().push(args.join(" "));
            let stdout = if args.starts_with(&["worktree", "list"]) {
                self.list.clone()
            } else {
                String::new()
            };
            Ok(git::GitOutput {
                code: 0,
                stdout,
                stderr: String::new(),
            })
        }
    }

    #[test]
    fn remove_never_prunes_when_the_worktree_folder_is_gone() {
        let t = tempfile::tempdir().unwrap();
        let main = t.path().join("app");
        let gone = t.path().join("app-gone");
        let list = format!(
            "worktree {}
HEAD aaaa
branch refs/heads/main

worktree {}
HEAD bbbb
branch refs/heads/gone
",
            main.display(),
            gone.display()
        );
        let git = RecordingGit {
            list,
            calls: Default::default(),
        };
        remove_with(&git, &FakeProcs(vec![]), &main, &gone, false).unwrap();
        let calls = git.calls.lock().unwrap();
        assert!(calls.contains(&format!("worktree remove -- {}", gone.display())));
        assert!(!calls.iter().any(|c| c.starts_with("worktree prune")));
    }

    struct FakeProcs(Vec<PathBuf>);
    impl ProcSource for FakeProcs {
        fn snapshot(&self) -> Vec<ProcInfo> {
            self.0.iter().map(|cwd| proc_at(Some(cwd))).collect()
        }
        fn kill(&self, _pid: u32) -> bool {
            false
        }
    }

    fn proc_at(cwd: Option<&Path>) -> ProcInfo {
        ProcInfo {
            pid: 1,
            name: "node.exe".into(),
            exe: None,
            cwd: cwd.map(Path::to_path_buf),
            ram_bytes: 1,
            cpu_percent: 0.0,
            start_time: 0,
            parent_pid: None,
        }
    }

    fn inside(root: &str, cwds: &[&str]) -> usize {
        let procs: Vec<ProcInfo> = cwds.iter().map(|c| proc_at(Some(Path::new(c)))).collect();
        processes_inside(Path::new(root), &procs)
    }

    #[test]
    fn processes_inside_counts_the_folder_and_below_but_not_siblings_sharing_a_prefix() {
        let cwds = [
            "/w/foo",
            "/w/foo/",
            "/w/foo/a/b",
            "/w/foo-bar",
            "/w/foobar/x",
            "/w",
        ];
        assert_eq!(inside("/w/foo", &cwds), 3);
        assert_eq!(inside("/w/foo/", &cwds), 3);
        assert_eq!(processes_inside(Path::new("/w/foo"), &[proc_at(None)]), 0);
    }

    #[cfg(windows)]
    #[test]
    fn processes_inside_ignores_separators_and_case_on_windows() {
        let cwds = [
            r"c:\w\FOO",
            r"C:/W/foo/web",
            r"\\?\C:\w\foo\x",
            r"C:\w\Foo-bar",
        ];
        assert_eq!(inside(r"C:\w\foo", &cwds), 3);
    }

    #[test]
    fn remove_refuses_while_processes_run_inside_and_touches_nothing() {
        need_git!();
        let t = tempfile::tempdir().unwrap();
        let repo = t.path().join("app");
        init_repo(&repo);
        let feat = t.path().join("app-feat");
        g(
            &repo,
            &["worktree", "add", "-b", "feat", feat.to_str().unwrap()],
        );
        fs::create_dir(feat.join("web")).unwrap();
        fs::write(feat.join("web").join("dirty.txt"), "x").unwrap();
        let procs = FakeProcs(vec![feat.clone(), feat.join("web")]);

        // Reported before the dirty check, so the user is never asked to discard changes in vain.
        for force in [false, true] {
            let e = remove_with(&SystemGit, &procs, &repo, &feat, force).unwrap_err();
            assert_eq!(e.code, "worktree_in_use");
            assert_eq!(e.kind, ErrorKind::NotAllowed);
            assert_eq!(e.params.unwrap()["count"], 2);
        }
        assert!(feat.join("a.txt").is_file());
        assert!(feat.join("web").join("dirty.txt").is_file());
        assert_eq!(list(&repo).unwrap().len(), 2);

        remove_with(
            &SystemGit,
            &FakeProcs(vec![repo.clone()]),
            &repo,
            &feat,
            true,
        )
        .unwrap();
        assert!(!feat.exists());
    }

    #[test]
    fn remove_sees_a_real_process_whose_cwd_is_inside_the_worktree() {
        need_git!();
        let t = tempfile::tempdir().unwrap();
        let repo = t.path().join("app");
        init_repo(&repo);
        let feat = t.path().join("app-feat");
        g(
            &repo,
            &["worktree", "add", "-b", "feat", feat.to_str().unwrap()],
        );
        // Stays alive until its stdin is closed.
        let mut child = Command::new("git")
            .args(["cat-file", "--batch"])
            .current_dir(&feat)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .spawn()
            .unwrap();

        let result = remove(&repo, &feat, false);
        let untouched = feat.join("a.txt").is_file();
        drop(child.stdin.take());
        child.wait().unwrap();

        assert_eq!(result.unwrap_err().code, "worktree_in_use");
        assert!(untouched);
    }

    #[test]
    fn is_merged_maps_exit_codes_with_a_fake_runner() {
        let p = Path::new(".");
        assert!(is_merged_with(&FakeGit(vec![("merge-base", 0)]), p, "a", "b").unwrap());
        assert!(!is_merged_with(&FakeGit(vec![("merge-base", 1)]), p, "a", "b").unwrap());
        assert!(is_merged_with(&FakeGit(vec![]), p, "a", "b").is_err());
    }
}
