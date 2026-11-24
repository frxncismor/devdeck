//! Root scan: finds repositories under a folder up to a depth, resolving linked worktrees to their main repo.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

const SKIP_DIRS: &[&str] = &["node_modules", "target", ".git", "dist", "build"];

/// Resolves a `.git` *file* of a linked worktree (`gitdir: <main>/.git/worktrees/<name>`) to the main repo path.
pub fn resolve_main_repo(git_file_content: &str) -> Option<PathBuf> {
    let gitdir = git_file_content.trim().strip_prefix("gitdir:")?.trim();
    let norm = gitdir.replace('\\', "/");
    let idx = norm.rfind("/.git/worktrees/")?;
    let main = &norm[..idx];
    if main.is_empty() {
        return None;
    }
    Some(PathBuf::from(main))
}

enum Kind {
    /// Main repository (has a `.git` directory).
    Repo,
    /// Linked worktree (`.git` file) pointing to this main repo, if resolvable.
    Linked(Option<PathBuf>),
    Plain,
}

fn classify(dir: &Path) -> Kind {
    let dot_git = dir.join(".git");
    match fs::symlink_metadata(&dot_git) {
        Ok(m) if m.is_dir() => Kind::Repo,
        Ok(m) if m.is_file() => {
            let main = fs::read_to_string(&dot_git)
                .ok()
                .and_then(|c| resolve_main_repo(&c));
            Kind::Linked(main)
        }
        _ => Kind::Plain,
    }
}

/// Scans `root` (depth 0 = the root itself, 1 = its children, …) and returns the main repository
/// paths found, sorted and de-duplicated. Linked worktrees resolve to their main repo (if it exists).
/// Never descends into repos, links/junctions or heavy folders.
pub fn scan(root: &Path, depth: u8) -> Vec<PathBuf> {
    let mut found = BTreeSet::new();
    walk(root, depth, &mut found);
    found.into_iter().collect()
}

fn walk(dir: &Path, depth: u8, found: &mut BTreeSet<PathBuf>) {
    match classify(dir) {
        Kind::Repo => {
            found.insert(dir.to_path_buf());
            return;
        }
        Kind::Linked(main) => {
            if let Some(m) = main.filter(|m| m.join(".git").is_dir()) {
                found.insert(m);
            }
            return;
        }
        Kind::Plain => {}
    }
    if depth == 0 {
        return;
    }
    let Ok(rd) = fs::read_dir(dir) else { return };
    for entry in rd.flatten() {
        let Ok(ft) = entry.file_type() else { continue };
        if !ft.is_dir() || ft.is_symlink() {
            continue;
        }
        let name = entry.file_name();
        if SKIP_DIRS.iter().any(|s| name.to_string_lossy() == *s) {
            continue;
        }
        let path = entry.path();
        if crate::fsutil::is_link_or_reparse(&path) {
            continue;
        }
        walk(&path, depth - 1, found);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mkrepo(p: &Path) {
        fs::create_dir_all(p.join(".git")).unwrap();
    }

    #[test]
    fn resolves_linked_worktree_gitdir() {
        assert_eq!(
            resolve_main_repo("gitdir: C:/r/app/.git/worktrees/feat\n"),
            Some(PathBuf::from("C:/r/app"))
        );
        assert_eq!(
            resolve_main_repo(r"gitdir: C:\r\app\.git\worktrees\feat"),
            Some(PathBuf::from("C:/r/app"))
        );
        assert_eq!(resolve_main_repo("gitdir: /x/.git/modules/sub"), None);
        assert_eq!(resolve_main_repo("garbage"), None);
    }

    #[test]
    fn finds_repos_within_depth_and_skips_heavy_dirs() {
        let t = tempfile::tempdir().unwrap();
        let r = t.path();
        mkrepo(&r.join("a"));
        mkrepo(&r.join("group/b"));
        mkrepo(&r.join("group/deep/c"));
        mkrepo(&r.join("node_modules/x"));
        fs::create_dir_all(r.join("plain")).unwrap();

        assert_eq!(scan(r, 1), vec![r.join("a")]);
        assert_eq!(scan(r, 2), vec![r.join("a"), r.join("group/b")]);
        let d3 = scan(r, 3);
        assert_eq!(d3.len(), 3);
        assert!(!d3.iter().any(|p| p.starts_with(r.join("node_modules"))));
    }

    #[test]
    fn does_not_descend_into_repos_and_root_may_be_a_repo() {
        let t = tempfile::tempdir().unwrap();
        mkrepo(t.path());
        mkrepo(&t.path().join("inner"));
        assert_eq!(scan(t.path(), 3), vec![t.path().to_path_buf()]);
    }

    #[test]
    fn linked_worktree_resolves_to_main_repo_without_duplicates() {
        let t = tempfile::tempdir().unwrap();
        let main = t.path().join("main-repo");
        mkrepo(&main);
        let wt = t.path().join("wt");
        fs::create_dir_all(&wt).unwrap();
        let gitdir = format!("gitdir: {}/.git/worktrees/wt\n", main.display());
        fs::write(wt.join(".git"), gitdir).unwrap();
        // A linked worktree whose main repo is gone is ignored.
        let orphan = t.path().join("orphan");
        fs::create_dir_all(&orphan).unwrap();
        fs::write(
            orphan.join(".git"),
            "gitdir: /nonexistent/zz/.git/worktrees/o\n",
        )
        .unwrap();
        assert_eq!(scan(t.path(), 1), vec![main]);
    }

    #[test]
    fn missing_root_is_empty() {
        assert!(scan(Path::new("/definitely/not/here"), 2).is_empty());
    }
}
