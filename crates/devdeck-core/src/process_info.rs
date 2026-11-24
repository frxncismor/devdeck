//! Process information contract. The system layer is behind `ProcSource` so logic is testable
//! with a simulated source (constitution II).

use crate::error::{Error, ErrorKind, Result};
use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcInfo {
    pub pid: u32,
    pub name: String,
    pub exe: Option<PathBuf>,
    /// `None` when unreadable (elevated/protected process, bitness mismatch) => "unknown".
    pub cwd: Option<PathBuf>,
    pub ram_bytes: u64,
    pub cpu_percent: f32,
    /// Seconds since the Unix epoch.
    pub start_time: u64,
    pub parent_pid: Option<u32>,
}

pub trait ProcSource {
    fn snapshot(&self) -> Vec<ProcInfo>;
    /// Terminates the process. Returns `false` when the OS refused or the process is gone.
    fn kill(&self, pid: u32) -> bool;
}

/// Real implementation over `sysinfo`.
#[derive(Debug, Default, Clone, Copy)]
pub struct SysinfoSource;

impl SysinfoSource {
    fn system() -> Option<sysinfo::System> {
        let mut sys = sysinfo::System::new().ok()?;
        sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
        Some(sys)
    }
}

impl ProcSource for SysinfoSource {
    fn snapshot(&self) -> Vec<ProcInfo> {
        let Some(sys) = Self::system() else {
            return Vec::new();
        };
        sys.processes()
            .values()
            .map(|p| ProcInfo {
                pid: p.pid().as_u32(),
                name: p.name().to_string_lossy().into_owned(),
                exe: p.exe().map(|e| e.to_path_buf()),
                cwd: p.cwd().map(|c| c.to_path_buf()),
                ram_bytes: p.memory(),
                cpu_percent: p.cpu_usage(),
                start_time: p.start_time(),
                parent_pid: p.parent().map(|pp| pp.as_u32()),
            })
            .collect()
    }

    fn kill(&self, pid: u32) -> bool {
        let Some(sys) = Self::system() else {
            return false;
        };
        sys.process(sysinfo::Pid::from_u32(pid))
            .is_some_and(|p| p.kill())
    }
}

/// Snapshot of every visible process, sorted by pid.
pub fn snapshot(source: &dyn ProcSource) -> Vec<ProcInfo> {
    let mut v = source.snapshot();
    v.sort_by_key(|p| p.pid);
    v
}

/// Kills `pid` only if its current name still equals `expected_name` (case-insensitive),
/// which protects against PID reuse.
pub fn kill(source: &dyn ProcSource, pid: u32, expected_name: &str) -> Result<()> {
    let current = source
        .snapshot()
        .into_iter()
        .find(|p| p.pid == pid)
        .ok_or_else(|| Error::not_found("process_not_found"))?;
    if !current.name.eq_ignore_ascii_case(expected_name) {
        return Err(Error::not_allowed("process_name_mismatch")
            .with_param("expected", expected_name)
            .with_param("actual", current.name));
    }
    if source.kill(pid) {
        Ok(())
    } else {
        Err(Error::new(ErrorKind::NotAllowed, "process_access_denied"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    struct Fake {
        procs: Vec<ProcInfo>,
        allow_kill: bool,
        killed: RefCell<Vec<u32>>,
    }

    impl ProcSource for Fake {
        fn snapshot(&self) -> Vec<ProcInfo> {
            self.procs.clone()
        }
        fn kill(&self, pid: u32) -> bool {
            self.killed.borrow_mut().push(pid);
            self.allow_kill
        }
    }

    fn proc(pid: u32, name: &str) -> ProcInfo {
        ProcInfo {
            pid,
            name: name.into(),
            exe: None,
            cwd: None,
            ram_bytes: 1,
            cpu_percent: 0.0,
            start_time: 0,
            parent_pid: None,
        }
    }

    fn fake(allow_kill: bool) -> Fake {
        Fake {
            procs: vec![proc(30, "node.exe"), proc(10, "vite.exe")],
            allow_kill,
            killed: RefCell::new(vec![]),
        }
    }

    #[test]
    fn snapshot_is_sorted_by_pid() {
        let pids: Vec<u32> = snapshot(&fake(true)).iter().map(|p| p.pid).collect();
        assert_eq!(pids, vec![10, 30]);
    }

    #[test]
    fn kill_succeeds_when_name_matches() {
        let f = fake(true);
        kill(&f, 30, "NODE.EXE").unwrap();
        assert_eq!(*f.killed.borrow(), vec![30]);
    }

    #[test]
    fn kill_rejects_name_mismatch_without_killing() {
        let f = fake(true);
        let e = kill(&f, 30, "vite.exe").unwrap_err();
        assert_eq!(e.code, "process_name_mismatch");
        assert!(f.killed.borrow().is_empty());
    }

    #[test]
    fn kill_unknown_pid_is_not_found() {
        let e = kill(&fake(true), 999, "node.exe").unwrap_err();
        assert_eq!(e.kind, ErrorKind::NotFound);
        assert_eq!(e.code, "process_not_found");
    }

    #[test]
    fn kill_refused_by_os_is_access_denied() {
        let e = kill(&fake(false), 30, "node.exe").unwrap_err();
        assert_eq!(e.code, "process_access_denied");
    }

    #[test]
    fn sysinfo_source_sees_own_process() {
        let me = std::process::id();
        let procs = SysinfoSource.snapshot();
        let mine = procs.iter().find(|p| p.pid == me).expect("own pid");
        assert!(!mine.name.is_empty());
        assert!(mine.ram_bytes > 0);
    }
}
