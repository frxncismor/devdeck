//! Worktrees module (phase 2A): list, scan the configured root, remove, open, and helpers used by
//! Ports (2B) and Reclaim (2D).

pub mod git;
pub mod open;
pub mod scan;

use crate::error::{Error, ErrorKind, Result};
use crate::project::{build_projects, Project, ProjectId, ProjectRef};
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

fn same_path(a: &Path, b: &Path) -> bool {
    scope::normalize(a) == scope::normalize(b)
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
    let mut result = Vec::with_capacity(raw.len());
    for (i, r) in raw.iter().enumerate() {
        if r.bare {
            continue;
        }
        let path = native(&r.path);
        let exists = path.is_dir();
        let mut w = Worktree {
            path: path.clone(),
            repo_path: repo_path.clone(),
            project: Some(project.clone()),
            branch: r.branch.clone(),
            detached: r.detached,
            is_main: i == 0,
            dirty_count: 0,
            ahead: None,
            behind: None,
            last_commit: None,
            exists,
        };
        if exists {
            if let Ok(s) = git.run(&path, &["status", "--porcelain=v2", "--branch"]) {
                if s.success() {
                    let st = git::parse_status(&s.stdout);
                    w.dirty_count = st.dirty_count;
                    w.ahead = st.ahead;
                    w.behind = st.behind;
                }
            }
            w.last_commit = last_commit_with(git, &path);
        }
        result.push(w);
    }
    Ok(result)
}

fn last_commit_with(git: &dyn GitRunner, dir: &Path) -> Option<LastCommit> {
    let out = git.run(dir, &["log", "-1", git::LOG_FORMAT]).ok()?;
    if out.success() {
        git::parse_log(&out.stdout)
    } else {
        None
    }
}

/// Worktrees of every repository found under `root` (up to `depth`). A repo that fails to list is skipped.
pub fn list_root(git: &dyn GitRunner, root: &Path, depth: u8) -> Vec<Worktree> {
    scan(root, depth)
        .iter()
        .filter_map(|repo| list_with(git, repo).ok())
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
pub fn remove(path: &Path, force: bool) -> Result<()> {
    remove_with(&SystemGit, path, force)
}

pub fn remove_with(git: &dyn GitRunner, path: &Path, force: bool) -> Result<()> {
    let out = git::run_ok(git, path, &["worktree", "list", "--porcelain"])?;
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
    let dirty = if path.is_dir() {
        let s = git::run_ok(git, path, &["status", "--porcelain=v2"])?;
        git::parse_status(&s).dirty_count > 0
    } else {
        false
    };
    if dirty && !force {
        return Err(Error::not_allowed("worktree_dirty"));
    }
    let target_path = native(&target.path);
    let target_str = target_path.to_string_lossy().into_owned();
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
        assert!(l[1].last_commit.is_none());
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

        let e = remove(&repo, true).unwrap_err();
        assert_eq!(e.code, "worktree_main_not_removable");
        assert!(repo.is_dir());

        fs::write(feat.join("dirty.txt"), "x").unwrap();
        let e = remove(&feat, false).unwrap_err();
        assert_eq!(e.code, "worktree_dirty");
        assert!(feat.is_dir());

        remove(&feat, true).unwrap();
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
        remove(&feat, false).unwrap();
        assert!(!feat.exists());
        let e = remove(&repo.join("nope"), false).unwrap_err();
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

    #[test]
    fn is_merged_maps_exit_codes_with_a_fake_runner() {
        let p = Path::new(".");
        assert!(is_merged_with(&FakeGit(vec![("merge-base", 0)]), p, "a", "b").unwrap());
        assert!(!is_merged_with(&FakeGit(vec![("merge-base", 1)]), p, "a", "b").unwrap());
        assert!(is_merged_with(&FakeGit(vec![]), p, "a", "b").is_err());
    }
}
