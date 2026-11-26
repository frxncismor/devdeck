//! Dev processes (node, esbuild, vite, cargo, rustc, rust-analyzer, headless Chromium, Playwright,
//! Docker, WSL) that may be holding RAM. Pure classification over a `ProcInfo` snapshot, so it is
//! tested with a simulated `ProcSource`.

use crate::process_info::{ProcInfo, ProcSource};
use crate::project::{Project, ProjectRef};
use crate::scope::{self, Attributable};
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DevKind {
    Node,
    Esbuild,
    Vite,
    Cargo,
    Rustc,
    RustAnalyzer,
    /// Headless Chrome/Chromium (e.g. `chrome-headless-shell`, puppeteer's Chromium).
    Chromium,
    Playwright,
    Docker,
    /// WSL (`vmmem`, `vmmemWSL`, `wsl`, `wslhost`).
    Wsl,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DevProcess {
    #[serde(flatten)]
    pub info: ProcInfo,
    pub kind: DevKind,
    pub project: Option<ProjectRef>,
    /// The worktree (or main repo) folder that contains the process cwd, if any.
    pub worktree_path: Option<PathBuf>,
    /// The cwd is attributed to a worktree whose folder (or the cwd itself) no longer exists.
    pub orphan: bool,
    /// `None` when the OS would not tell when the process started.
    pub age_secs: Option<u64>,
}

impl Attributable for DevProcess {
    fn path(&self) -> Option<&Path> {
        self.info.cwd.as_deref()
    }
    fn ram(&self) -> u64 {
        self.info.ram_bytes
    }
}

fn lower_stem(name: &str) -> String {
    let n = name.to_lowercase();
    n.strip_suffix(".exe").unwrap_or(&n).to_owned()
}

/// Kind of a dev process, or `None` when it is not one we list. Name-based (case-insensitive,
/// `.exe` ignored); the exe path refines Chromium/Playwright, which we only list when they are
/// clearly automation browsers (never the user's everyday Chrome).
pub fn kind_of(p: &ProcInfo) -> Option<DevKind> {
    let name = lower_stem(&p.name);
    let exe = p
        .exe
        .as_ref()
        .map(|e| e.to_string_lossy().to_lowercase().replace('\\', "/"))
        .unwrap_or_default();
    if name == "playwright" || exe.contains("ms-playwright") {
        return Some(DevKind::Playwright);
    }
    match name.as_str() {
        "node" => Some(DevKind::Node),
        "esbuild" => Some(DevKind::Esbuild),
        "vite" => Some(DevKind::Vite),
        "cargo" => Some(DevKind::Cargo),
        "rustc" => Some(DevKind::Rustc),
        "rust-analyzer" => Some(DevKind::RustAnalyzer),
        "chrome-headless-shell" | "headless_shell" | "chromium" => Some(DevKind::Chromium),
        "chrome" | "msedge" if exe.contains("chromium") || exe.contains("puppeteer") => {
            Some(DevKind::Chromium)
        }
        "docker" | "dockerd" | "docker desktop" | "com.docker.backend" | "com.docker.build" => {
            Some(DevKind::Docker)
        }
        "vmmem" | "vmmemwsl" | "wsl" | "wslhost" => Some(DevKind::Wsl),
        _ => None,
    }
}

/// Classifies a snapshot into dev processes, biggest RAM first.
/// The known worktrees are the project paths (main repo + worktrees); `path_exists` is injected so
/// the orphan rule is testable without touching the disk.
pub fn classify(
    procs: &[ProcInfo],
    projects: &[Project],
    now_secs: u64,
    path_exists: &dyn Fn(&Path) -> bool,
) -> Vec<DevProcess> {
    let known_worktrees: Vec<(&Path, &Project)> = projects
        .iter()
        .flat_map(|p| p.paths().map(move |w| (w, p)))
        .collect();
    let mut out: Vec<DevProcess> = procs
        .iter()
        .filter_map(|p| {
            let kind = kind_of(p)?;
            let owner = p.cwd.as_deref().and_then(|cwd| {
                known_worktrees
                    .iter()
                    .filter(|(w, _)| super::is_within(w, cwd))
                    .max_by_key(|(w, _)| scope::normalize(w).len())
            });
            let orphan = match (owner, p.cwd.as_deref()) {
                (Some((w, _)), Some(cwd)) => !path_exists(w) || !path_exists(cwd),
                _ => false,
            };
            Some(DevProcess {
                kind,
                project: owner.map(|(_, pr)| pr.to_ref()),
                worktree_path: owner.map(|(w, _)| w.to_path_buf()),
                orphan,
                age_secs: (p.start_time > 0).then(|| now_secs.saturating_sub(p.start_time)),
                info: p.clone(),
            })
        })
        .collect();
    out.sort_by(|a, b| {
        b.info
            .ram_bytes
            .cmp(&a.info.ram_bytes)
            .then(a.info.pid.cmp(&b.info.pid))
    });
    out
}

/// Live snapshot from a `ProcSource`, classified into dev processes (checks the real disk).
pub fn list(source: &dyn ProcSource, projects: &[Project], now_secs: u64) -> Vec<DevProcess> {
    let snap = crate::process_info::snapshot(source);
    classify(&snap, projects, now_secs, &|p| p.exists())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::ProjectId;

    struct Fake(Vec<ProcInfo>);
    impl ProcSource for Fake {
        fn snapshot(&self) -> Vec<ProcInfo> {
            self.0.clone()
        }
        fn kill(&self, _pid: u32) -> bool {
            false
        }
    }

    fn proc(pid: u32, name: &str, ram: u64, cwd: Option<&str>) -> ProcInfo {
        ProcInfo {
            pid,
            name: name.into(),
            exe: None,
            cwd: cwd.map(PathBuf::from),
            ram_bytes: ram,
            cpu_percent: 1.5,
            start_time: 1_000,
            parent_pid: None,
        }
    }

    fn project(name: &str, main: &str, wts: &[&str]) -> Project {
        Project {
            id: ProjectId::from_path(Path::new(main)),
            name: name.into(),
            main_path: main.into(),
            worktree_paths: wts.iter().map(PathBuf::from).collect(),
        }
    }

    fn all_exist(_: &Path) -> bool {
        true
    }

    #[test]
    fn recognizes_dev_names_case_insensitively_and_ignores_the_rest() {
        let names = [
            ("Node.exe", Some(DevKind::Node)),
            ("esbuild.exe", Some(DevKind::Esbuild)),
            ("vite", Some(DevKind::Vite)),
            ("cargo.exe", Some(DevKind::Cargo)),
            ("RUSTC.EXE", Some(DevKind::Rustc)),
            ("rust-analyzer.exe", Some(DevKind::RustAnalyzer)),
            ("chrome-headless-shell.exe", Some(DevKind::Chromium)),
            ("com.docker.backend.exe", Some(DevKind::Docker)),
            ("vmmem", Some(DevKind::Wsl)),
            ("vmmemWSL", Some(DevKind::Wsl)),
            ("wslhost.exe", Some(DevKind::Wsl)),
            ("explorer.exe", None),
            ("notepad.exe", None),
            ("chrome.exe", None),
        ];
        for (name, want) in names {
            assert_eq!(kind_of(&proc(1, name, 1, None)), want, "{name}");
        }
    }

    #[test]
    fn everyday_chrome_is_only_listed_when_it_is_an_automation_browser() {
        let mut p = proc(1, "chrome.exe", 1, None);
        p.exe = Some(PathBuf::from(
            r"C:\Users\x\AppData\Local\ms-playwright\chromium-1100\chrome-win\chrome.exe",
        ));
        assert_eq!(kind_of(&p), Some(DevKind::Playwright));
        p.exe = Some(PathBuf::from(
            r"C:\Users\x\.cache\puppeteer\chrome\chrome.exe",
        ));
        assert_eq!(kind_of(&p), Some(DevKind::Chromium));
        p.exe = Some(PathBuf::from(
            r"C:\Program Files\Google\Chrome\Application\chrome.exe",
        ));
        assert_eq!(kind_of(&p), None);
    }

    #[test]
    fn attributes_to_project_and_worktree_with_age_and_ram_order() {
        let ps = [project("app", "/r/app", &["/w/app-feat"])];
        let procs = [
            proc(1, "node.exe", 10, Some("/r/app/web")),
            proc(2, "esbuild.exe", 50, Some("/w/app-feat")),
            proc(3, "vite.exe", 5, Some("/somewhere/else")),
            proc(4, "explorer.exe", 99, None),
            proc(5, "rustc.exe", 1, None),
        ];
        let l = classify(&procs, &ps, 1_600, &all_exist);
        assert_eq!(
            l.iter().map(|p| p.info.pid).collect::<Vec<_>>(),
            [2, 1, 3, 5]
        );
        assert_eq!(l[0].project.as_ref().unwrap().name, "app");
        assert_eq!(
            l[0].worktree_path.as_deref(),
            Some(Path::new("/w/app-feat"))
        );
        assert_eq!(l[1].worktree_path.as_deref(), Some(Path::new("/r/app")));
        assert_eq!(l[0].age_secs, Some(600));
        assert!(l[2].project.is_none() && l[3].project.is_none());
        assert!(l.iter().all(|p| !p.orphan));
    }

    #[test]
    fn orphan_when_cwd_is_in_a_worktree_that_no_longer_exists() {
        let ps = [project("app", "/r/app", &["/w/app-gone"])];
        let procs = [
            proc(1, "node.exe", 1, Some("/w/app-gone/web")),
            proc(2, "node.exe", 1, Some("/r/app")),
            proc(3, "node.exe", 1, None),
            proc(4, "node.exe", 1, Some("/r/app/deleted-sub")),
        ];
        let exists = |p: &Path| !p.starts_with("/w/app-gone") && !p.ends_with("deleted-sub");
        let l = classify(&procs, &ps, 2_000, &exists);
        let by = |pid| l.iter().find(|p| p.info.pid == pid).unwrap();
        assert!(by(1).orphan);
        assert!(!by(2).orphan);
        assert!(!by(3).orphan, "unknown cwd is never an orphan");
        assert!(
            by(4).orphan,
            "cwd removed from disk inside a known worktree"
        );
    }

    #[test]
    fn longest_worktree_prefix_wins_and_future_start_is_age_zero() {
        let ps = [
            project("outer", "/r/outer", &[]),
            project("inner", "/r/outer/nested", &[]),
        ];
        let mut p = proc(1, "node", 1, Some("/r/outer/nested/x"));
        p.start_time = 9_999;
        let l = classify(&[p], &ps, 10, &all_exist);
        assert_eq!(l[0].project.as_ref().unwrap().name, "inner");
        assert_eq!(l[0].age_secs, Some(0));
    }

    #[test]
    fn unknown_start_time_means_unknown_age_not_the_unix_epoch() {
        let mut p = proc(1, "node", 0, None);
        p.start_time = 0;
        let l = classify(&[p], &[], 1_795_000_000, &all_exist);
        assert_eq!(l[0].age_secs, None);
        let v = serde_json::to_value(&l[0]).unwrap();
        assert!(v.get("ageSecs").is_some_and(|a| a.is_null()));
        assert_eq!(
            v["ramBytes"], 0,
            "0 stays 0: no running process uses 0 bytes"
        );
    }

    #[test]
    fn list_uses_the_process_source_and_serializes_flat_camel_case() {
        let f = Fake(vec![proc(7, "node.exe", 42, Some("/nowhere"))]);
        let l = list(&f, &[], 1_100);
        assert_eq!(l.len(), 1);
        let v = serde_json::to_value(&l[0]).unwrap();
        assert_eq!(v["pid"], 7);
        assert_eq!(v["ramBytes"], 42);
        assert_eq!(v["kind"], "node");
        assert_eq!(v["orphan"], false);
        assert_eq!(v["ageSecs"], 100);
        assert!(v["project"].is_null());
        assert_eq!(
            serde_json::to_value(DevKind::RustAnalyzer).unwrap(),
            "rustAnalyzer"
        );
    }
}
