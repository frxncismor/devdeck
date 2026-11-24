//! Ports module (phase 2B): listening ports enriched with process info and attributed to the
//! project / worktree that owns the process.

pub mod listing;

use crate::error::Result;
use crate::process_info::{self, ProcInfo, ProcSource};
use crate::project::{Project, ProjectRef};
use crate::scope::{self, group_by_project, Attributable, Grouped, Scope};
use listing::{PortSource, Protocol, RawListener};
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub use listing::SystemPorts;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListeningPort {
    pub port: u16,
    pub protocol: Protocol,
    pub pid: u32,
    pub process_name: String,
    pub exe_path: Option<PathBuf>,
    /// `None` => unknown (elevated/protected process); never an error.
    pub cwd: Option<PathBuf>,
    /// The project worktree (main or linked) that contains the process, when any.
    pub worktree_path: Option<PathBuf>,
    pub project: Option<ProjectRef>,
}

impl Attributable for ListeningPort {
    /// Attribution path: the working directory, else the executable path (e.g. a binary under
    /// `node_modules/.bin` or `target/debug` of a project).
    fn path(&self) -> Option<&Path> {
        self.cwd.as_deref().or(self.exe_path.as_deref())
    }
}

/// The project worktree (main repo or linked worktree) containing `path`; longest prefix wins.
fn owning_worktree<'a>(path: &Path, project: &'a Project) -> Option<&'a Path> {
    let target = scope::normalize(path);
    project
        .paths()
        .filter(|root| {
            let r = scope::normalize(root);
            target == r || target.starts_with(&format!("{}/", r.trim_end_matches('/')))
        })
        .max_by_key(|root| scope::normalize(root).len())
}

/// Lists listening ports enriched with process info and attributed to `projects`.
pub fn list(
    ports: &dyn PortSource,
    procs: &dyn ProcSource,
    projects: &[Project],
    include_udp: bool,
) -> Vec<ListeningPort> {
    let by_pid: HashMap<u32, ProcInfo> = process_info::snapshot(procs)
        .into_iter()
        .map(|p| (p.pid, p))
        .collect();
    ports
        .listeners(include_udp)
        .into_iter()
        .map(|raw: RawListener| {
            let info = by_pid.get(&raw.pid);
            let mut lp = ListeningPort {
                port: raw.port,
                protocol: raw.protocol,
                pid: raw.pid,
                process_name: info.map_or(raw.name, |i| i.name.clone()),
                exe_path: info.and_then(|i| i.exe.clone()),
                cwd: info.and_then(|i| i.cwd.clone()),
                worktree_path: None,
                project: None,
            };
            let owner = lp
                .path()
                .and_then(|p| scope::attribute(p, projects).map(|id| (p, id)));
            if let Some((path, id)) = owner {
                if let Some(project) = projects.iter().find(|p| p.id == id) {
                    let wt = owning_worktree(path, project).map(Path::to_path_buf);
                    lp.worktree_path = wt;
                    lp.project = Some(project.to_ref());
                }
            }
            lp
        })
        .collect()
}

/// Case-insensitive substring filter over port, pid, process name, exe/cwd/worktree path and
/// project name. A blank query keeps everything.
pub fn filter(items: Vec<ListeningPort>, query: &str) -> Vec<ListeningPort> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return items;
    }
    let lossy = |o: &Option<PathBuf>| {
        o.as_ref()
            .map(|x| x.to_string_lossy().to_lowercase())
            .unwrap_or_default()
    };
    items
        .into_iter()
        .filter(|p| {
            [
                p.port.to_string(),
                p.pid.to_string(),
                p.process_name.to_lowercase(),
                lossy(&p.exe_path),
                lossy(&p.cwd),
                lossy(&p.worktree_path),
                p.project
                    .as_ref()
                    .map(|r| r.name.to_lowercase())
                    .unwrap_or_default(),
            ]
            .iter()
            .any(|s| s.contains(&q))
        })
        .collect()
}

/// Full pipeline used by the `ports_list` command: list, filter by `query`, filter by scope and
/// group by project with totals.
pub fn list_grouped(
    ports: &dyn PortSource,
    procs: &dyn ProcSource,
    projects: &[Project],
    scope: &Scope,
    include_udp: bool,
    query: Option<&str>,
) -> Result<Grouped<ListeningPort>> {
    let items = filter(
        list(ports, procs, projects, include_udp),
        query.unwrap_or(""),
    );
    Ok(group_by_project(items, scope, projects))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::ProjectId;

    struct FakePorts(Vec<RawListener>);
    impl PortSource for FakePorts {
        fn listeners(&self, include_udp: bool) -> Vec<RawListener> {
            self.0
                .iter()
                .filter(|l| include_udp || l.protocol == Protocol::Tcp)
                .cloned()
                .collect()
        }
    }

    struct FakeProcs(Vec<ProcInfo>);
    impl ProcSource for FakeProcs {
        fn snapshot(&self) -> Vec<ProcInfo> {
            self.0.clone()
        }
        fn kill(&self, _pid: u32) -> bool {
            true
        }
    }

    fn raw(port: u16, protocol: Protocol, pid: u32) -> RawListener {
        RawListener {
            port,
            protocol,
            pid,
            name: format!("raw{pid}"),
        }
    }

    fn proc(pid: u32, name: &str, cwd: Option<&str>, exe: Option<&str>) -> ProcInfo {
        ProcInfo {
            pid,
            name: name.into(),
            exe: exe.map(PathBuf::from),
            cwd: cwd.map(PathBuf::from),
            ram_bytes: 10,
            cpu_percent: 0.0,
            start_time: 0,
            parent_pid: None,
        }
    }

    fn project(name: &str, main: &str, wts: &[&str]) -> Project {
        let main = PathBuf::from(main);
        Project {
            id: ProjectId::from_path(&main),
            name: name.into(),
            main_path: main,
            worktree_paths: wts.iter().map(PathBuf::from).collect(),
        }
    }

    fn fixture() -> (FakePorts, FakeProcs, Vec<Project>) {
        let ports = FakePorts(vec![
            raw(3000, Protocol::Tcp, 10),
            raw(3001, Protocol::Tcp, 10), // several ports per PID
            raw(5173, Protocol::Tcp, 20), // inside a linked worktree
            raw(8080, Protocol::Tcp, 30), // unknown cwd
            raw(9000, Protocol::Tcp, 40), // no project
            raw(5353, Protocol::Udp, 40),
            raw(7000, Protocol::Tcp, 99), // process not visible
        ]);
        let procs = FakeProcs(vec![
            proc(10, "node.exe", Some("/r/app/src"), None),
            proc(20, "vite.exe", Some("/w/app-feat/web"), None),
            proc(30, "system", None, None),
            proc(
                40,
                "python.exe",
                Some("/elsewhere"),
                Some("/usr/bin/python"),
            ),
        ]);
        let projects = vec![project("app", "/r/app", &["/w/app-feat"])];
        (ports, procs, projects)
    }

    #[test]
    fn enriches_and_attributes_to_project_and_worktree() {
        let (ports, procs, projects) = fixture();
        let v = list(&ports, &procs, &projects, false);
        let by_port = |p: u16| v.iter().find(|x| x.port == p).unwrap();
        let main = by_port(3000);
        assert_eq!(main.process_name, "node.exe");
        assert_eq!(main.project.as_ref().unwrap().name, "app");
        assert_eq!(main.worktree_path.as_deref(), Some(Path::new("/r/app")));
        let linked = by_port(5173);
        assert_eq!(linked.project.as_ref().unwrap().name, "app");
        assert_eq!(
            linked.worktree_path.as_deref(),
            Some(Path::new("/w/app-feat"))
        );
        let none = by_port(9000);
        assert!(none.project.is_none() && none.worktree_path.is_none());
    }

    #[test]
    fn several_ports_per_pid_are_kept() {
        let (ports, procs, projects) = fixture();
        let v = list(&ports, &procs, &projects, false);
        assert_eq!(v.iter().filter(|p| p.pid == 10).count(), 2);
    }

    #[test]
    fn unknown_cwd_is_none_not_an_error() {
        let (ports, procs, projects) = fixture();
        let v = list(&ports, &procs, &projects, false);
        let p = v.iter().find(|p| p.port == 8080).unwrap();
        assert!(p.cwd.is_none() && p.project.is_none());
        assert_eq!(p.process_name, "system");
    }

    #[test]
    fn invisible_process_falls_back_to_socket_table_name() {
        let (ports, procs, projects) = fixture();
        let v = list(&ports, &procs, &projects, false);
        let p = v.iter().find(|p| p.port == 7000).unwrap();
        assert_eq!(p.process_name, "raw99");
        assert!(p.exe_path.is_none());
    }

    #[test]
    fn exe_path_is_the_attribution_fallback() {
        let ports = FakePorts(vec![raw(4000, Protocol::Tcp, 1)]);
        let procs = FakeProcs(vec![proc(1, "app", None, Some("/r/app/target/debug/app"))]);
        let projects = vec![project("app", "/r/app", &[])];
        let v = list(&ports, &procs, &projects, false);
        assert_eq!(v[0].project.as_ref().unwrap().name, "app");
        assert_eq!(v[0].worktree_path.as_deref(), Some(Path::new("/r/app")));
    }

    #[test]
    fn cwd_wins_over_exe_for_attribution() {
        let ports = FakePorts(vec![raw(4000, Protocol::Tcp, 1)]);
        let procs = FakeProcs(vec![proc(1, "node", Some("/r/b"), Some("/r/a/bin/node"))]);
        let projects = vec![project("a", "/r/a", &[]), project("b", "/r/b", &[])];
        let v = list(&ports, &procs, &projects, false);
        assert_eq!(v[0].project.as_ref().unwrap().name, "b");
    }

    #[test]
    fn udp_only_when_requested() {
        let (ports, procs, projects) = fixture();
        assert!(!list(&ports, &procs, &projects, false)
            .iter()
            .any(|p| p.protocol == Protocol::Udp));
        assert!(list(&ports, &procs, &projects, true)
            .iter()
            .any(|p| p.protocol == Protocol::Udp && p.port == 5353));
    }

    #[test]
    fn filter_matches_port_pid_name_path_and_project() {
        let (ports, procs, projects) = fixture();
        let all = list(&ports, &procs, &projects, true);
        let ports_of = |q: &str| -> Vec<u16> {
            let mut v: Vec<u16> = filter(all.clone(), q).iter().map(|p| p.port).collect();
            v.sort();
            v
        };
        assert_eq!(ports_of("5173"), vec![5173]);
        assert_eq!(ports_of("VITE"), vec![5173]);
        assert_eq!(ports_of("app-feat"), vec![5173]);
        assert_eq!(ports_of("/usr/bin"), vec![5353, 9000]);
        assert_eq!(ports_of("  ").len(), all.len());
        assert_eq!(ports_of("app"), vec![3000, 3001, 5173]);
        assert_eq!(ports_of("30"), vec![3000, 3001, 8080]);
        assert!(ports_of("nomatch").is_empty());
    }

    #[test]
    fn grouped_by_project_with_scope_and_totals() {
        let (ports, procs, projects) = fixture();
        let g = list_grouped(&ports, &procs, &projects, &Scope::Global, false, None).unwrap();
        assert_eq!(g.total.count, 6);
        assert_eq!(g.groups[0].project.as_ref().unwrap().name, "app");
        assert_eq!(g.groups[0].items.len(), 3);
        assert!(g.groups[1].project.is_none());
        let scoped = list_grouped(
            &ports,
            &procs,
            &projects,
            &Scope::Project {
                id: projects[0].id.clone(),
            },
            false,
            Some("vite"),
        )
        .unwrap();
        assert_eq!(scoped.total.count, 1);
    }
}
