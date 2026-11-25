//! `Project` contract: one project per main repository, with all its worktree paths.

use crate::worktrees::Worktree;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Stable id: FNV-1a hash of the normalized path of the main repo, as 16 hex digits.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProjectId(pub String);

impl ProjectId {
    pub fn from_path(main_path: &Path) -> Self {
        let norm = crate::scope::normalize(main_path);
        let mut h: u64 = 0xcbf29ce484222325;
        for b in norm.bytes() {
            h ^= u64::from(b);
            h = h.wrapping_mul(0x100000001b3);
        }
        ProjectId(format!("{h:016x}"))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectRef {
    pub id: ProjectId,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: ProjectId,
    pub name: String,
    pub main_path: PathBuf,
    pub worktree_paths: Vec<PathBuf>,
}

impl Project {
    pub fn to_ref(&self) -> ProjectRef {
        ProjectRef {
            id: self.id.clone(),
            name: self.name.clone(),
        }
    }

    /// All paths owned by the project (main repo first).
    pub fn paths(&self) -> impl Iterator<Item = &Path> {
        std::iter::once(self.main_path.as_path())
            .chain(self.worktree_paths.iter().map(|p| p.as_path()))
    }
}

/// Groups worktrees (as returned by `worktrees::list` over every repo of the root) into projects,
/// one per `repo_path`. Sorted by name (case-insensitive), then path.
pub fn build_projects(worktrees: &[Worktree]) -> Vec<Project> {
    let mut by_repo: BTreeMap<PathBuf, Vec<PathBuf>> = BTreeMap::new();
    for w in worktrees {
        let entry = by_repo.entry(w.repo_path.clone()).or_default();
        if w.path != w.repo_path && !entry.contains(&w.path) {
            entry.push(w.path.clone());
        }
    }
    let mut projects: Vec<Project> = by_repo
        .into_iter()
        .map(|(main_path, worktree_paths)| Project {
            id: ProjectId::from_path(&main_path),
            name: main_path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| main_path.to_string_lossy().into_owned()),
            main_path,
            worktree_paths,
        })
        .collect();
    projects.sort_by(|a, b| {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then_with(|| a.main_path.cmp(&b.main_path))
    });
    projects
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wt(path: &str, repo: &str) -> Worktree {
        Worktree {
            path: path.into(),
            repo_path: repo.into(),
            project: None,
            branch: None,
            detached: false,
            is_main: path == repo,
            dirty_count: 0,
            ahead: None,
            behind: None,
            last_commit: None,
            exists: true,
        }
    }

    #[test]
    fn groups_worktrees_by_repo() {
        let ws = vec![
            wt("/r/zeta", "/r/zeta"),
            wt("/r/alpha", "/r/alpha"),
            wt("/w/alpha-feat", "/r/alpha"),
        ];
        let ps = build_projects(&ws);
        assert_eq!(ps.len(), 2);
        assert_eq!(ps[0].name, "alpha");
        assert_eq!(ps[0].worktree_paths, vec![PathBuf::from("/w/alpha-feat")]);
        assert_eq!(ps[1].name, "zeta");
    }

    #[test]
    fn id_is_stable_and_path_based() {
        let a = ProjectId::from_path(Path::new("/r/alpha"));
        assert_eq!(a, ProjectId::from_path(Path::new("/r/alpha")));
        assert_ne!(a, ProjectId::from_path(Path::new("/r/beta")));
        assert_eq!(a.0.len(), 16);
    }

    #[test]
    fn serializes_camel_case() {
        let ps = build_projects(&[wt("/r/a", "/r/a")]);
        let v = serde_json::to_value(&ps[0]).unwrap();
        assert!(v.get("mainPath").is_some() && v.get("worktreePaths").is_some());
        assert!(v["id"].is_string());
    }
}
