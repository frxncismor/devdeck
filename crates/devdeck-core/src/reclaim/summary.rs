//! Recoverable-resources summary: RAM held by dev processes and disk held by forgotten worktrees
//! and regenerable folders, totalled and grouped by project with `scope::group_by_project`.

use super::folders::HeavyFolder;
use super::forgotten::ForgottenWorktree;
use super::processes::DevProcess;
use crate::project::{Project, ProjectId, ProjectRef};
use crate::scope::{group_by_project, Grouped, Scope};
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSummary {
    /// `None` = "No project" (label rendered by the frontend).
    pub project: Option<ProjectRef>,
    pub ram_bytes: u64,
    pub disk_bytes: u64,
    pub processes: u64,
    pub worktrees: u64,
    pub folders: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReclaimSummary {
    pub recoverable_ram_bytes: u64,
    pub recoverable_disk_bytes: u64,
    /// Projects with something recoverable, in project order, "No project" last.
    pub by_project: Vec<ProjectSummary>,
}

#[derive(Default)]
struct Acc {
    ram: u64,
    disk: u64,
    processes: u64,
    worktrees: u64,
    folders: u64,
}

fn add<T>(
    acc: &mut HashMap<Option<ProjectId>, Acc>,
    grouped: &Grouped<T>,
    f: impl Fn(&mut Acc, u64, u64, u64),
) {
    for g in &grouped.groups {
        if g.items.is_empty() {
            continue;
        }
        let key = g.project.as_ref().map(|p| p.id.clone());
        f(
            acc.entry(key).or_default(),
            g.totals.count,
            g.totals.ram_bytes,
            g.totals.disk_bytes,
        );
    }
}

/// Builds the summary for `scope`. Folders that live inside a forgotten worktree are not counted
/// again (removing the worktree already recovers them).
pub fn summarize(
    procs: Vec<DevProcess>,
    forgotten: Vec<ForgottenWorktree>,
    folders: Vec<HeavyFolder>,
    scope: &Scope,
    projects: &[Project],
) -> ReclaimSummary {
    let folders: Vec<HeavyFolder> = folders
        .into_iter()
        .filter(|f| {
            !forgotten
                .iter()
                .any(|w| super::is_within(&w.worktree.path, &f.path))
        })
        .collect();
    let mut acc: HashMap<Option<ProjectId>, Acc> = HashMap::new();
    add(
        &mut acc,
        &group_by_project(procs, scope, projects),
        |a, n, ram, _| {
            a.processes += n;
            a.ram += ram;
        },
    );
    add(
        &mut acc,
        &group_by_project(forgotten, scope, projects),
        |a, n, _, disk| {
            a.worktrees += n;
            a.disk += disk;
        },
    );
    add(
        &mut acc,
        &group_by_project(folders, scope, projects),
        |a, n, _, disk| {
            a.folders += n;
            a.disk += disk;
        },
    );
    let order = projects
        .iter()
        .map(|p| Some(p.to_ref()))
        .chain(std::iter::once(None));
    let by_project: Vec<ProjectSummary> = order
        .filter_map(|project| {
            let a = acc.remove(&project.as_ref().map(|p| p.id.clone()))?;
            Some(ProjectSummary {
                project,
                ram_bytes: a.ram,
                disk_bytes: a.disk,
                processes: a.processes,
                worktrees: a.worktrees,
                folders: a.folders,
            })
        })
        .collect();
    ReclaimSummary {
        recoverable_ram_bytes: by_project.iter().map(|p| p.ram_bytes).sum(),
        recoverable_disk_bytes: by_project.iter().map(|p| p.disk_bytes).sum(),
        by_project,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::process_info::ProcInfo;
    use crate::reclaim::folders::FolderKind;
    use crate::reclaim::forgotten::Reason;
    use crate::reclaim::processes::DevKind;
    use crate::worktrees::Worktree;
    use std::path::{Path, PathBuf};

    fn project(name: &str, main: &str) -> Project {
        Project {
            id: ProjectId::from_path(Path::new(main)),
            name: name.into(),
            main_path: main.into(),
            worktree_paths: vec![],
        }
    }

    fn dev(pid: u32, cwd: Option<&str>, ram: u64) -> DevProcess {
        DevProcess {
            info: ProcInfo {
                pid,
                name: "node.exe".into(),
                exe: None,
                cwd: cwd.map(PathBuf::from),
                ram_bytes: ram,
                cpu_percent: 0.0,
                start_time: 0,
                parent_pid: None,
            },
            kind: DevKind::Node,
            project: None,
            worktree_path: None,
            orphan: false,
            age_secs: Some(0),
        }
    }

    fn forgotten(path: &str, disk: Option<u64>) -> ForgottenWorktree {
        ForgottenWorktree {
            worktree: Worktree {
                path: path.into(),
                repo_path: "/r/a".into(),
                project: None,
                branch: Some("b".into()),
                detached: false,
                is_main: false,
                dirty_count: 0,
                ahead: None,
                behind: None,
                last_commit: None,
                exists: true,
            },
            disk_bytes: disk,
            last_commit_age_days: Some(99),
            merged: false,
            clean: true,
            live_processes: 0,
            reasons: vec![Reason::Stale],
        }
    }

    fn folder(path: &str, disk: Option<u64>) -> HeavyFolder {
        HeavyFolder {
            path: path.into(),
            kind: FolderKind::NodeModules,
            disk_bytes: disk,
            last_used_iso: None,
            project: None,
        }
    }

    fn sample() -> (
        Vec<Project>,
        Vec<DevProcess>,
        Vec<ForgottenWorktree>,
        Vec<HeavyFolder>,
    ) {
        let ps = vec![project("a", "/r/a"), project("b", "/r/b")];
        let procs = vec![
            dev(1, Some("/r/b/web"), 100),
            dev(2, Some("/r/a"), 10),
            dev(3, None, 7),
        ];
        let fw = vec![
            forgotten("/r/a/wt-old", Some(1_000)),
            forgotten("/r/b/wt", None),
        ];
        let folders = vec![
            folder("/r/b/node_modules", Some(500)),
            folder("/r/a/wt-old/node_modules", Some(900)),
            folder("/home/.npm", Some(40)),
        ];
        (ps, procs, fw, folders)
    }

    #[test]
    fn global_totals_and_per_project_breakdown() {
        let (ps, procs, fw, folders) = sample();
        let s = summarize(procs, fw, folders, &Scope::Global, &ps);
        assert_eq!(s.recoverable_ram_bytes, 117);
        // 1000 (a worktree) + 0 (b worktree, unsized) + 500 + 40; the 900 inside the forgotten
        // worktree is not counted twice.
        assert_eq!(s.recoverable_disk_bytes, 1_540);
        let names: Vec<_> = s
            .by_project
            .iter()
            .map(|p| p.project.as_ref().map(|r| r.name.as_str()))
            .collect();
        assert_eq!(names, [Some("a"), Some("b"), None]);
        let a = &s.by_project[0];
        assert_eq!(
            (
                a.ram_bytes,
                a.disk_bytes,
                a.processes,
                a.worktrees,
                a.folders
            ),
            (10, 1_000, 1, 1, 0)
        );
        let b = &s.by_project[1];
        assert_eq!(
            (
                b.ram_bytes,
                b.disk_bytes,
                b.processes,
                b.worktrees,
                b.folders
            ),
            (100, 500, 1, 1, 1)
        );
        let none = &s.by_project[2];
        assert_eq!(
            (
                none.ram_bytes,
                none.disk_bytes,
                none.processes,
                none.folders
            ),
            (7, 40, 1, 1)
        );
    }

    #[test]
    fn project_scope_is_filtered_to_that_project() {
        let (ps, procs, fw, folders) = sample();
        let scope = Scope::Project {
            id: ps[1].id.clone(),
        };
        let s = summarize(procs, fw, folders, &scope, &ps);
        assert_eq!(s.by_project.len(), 1);
        assert_eq!(s.recoverable_ram_bytes, 100);
        assert_eq!(s.recoverable_disk_bytes, 500);
        assert_eq!(s.by_project[0].project.as_ref().unwrap().name, "b");
    }

    #[test]
    fn nothing_recoverable_is_an_empty_summary() {
        let s = summarize(
            vec![],
            vec![],
            vec![],
            &Scope::Global,
            &[project("a", "/r/a")],
        );
        assert_eq!(s.recoverable_ram_bytes, 0);
        assert_eq!(s.recoverable_disk_bytes, 0);
        assert!(s.by_project.is_empty());
    }

    #[test]
    fn serializes_camel_case() {
        let s = summarize(vec![dev(1, None, 5)], vec![], vec![], &Scope::Global, &[]);
        let v = serde_json::to_value(&s).unwrap();
        assert_eq!(v["recoverableRamBytes"], 5);
        assert_eq!(v["byProject"][0]["processes"], 1);
        assert!(v["byProject"][0]["project"].is_null());
    }
}
