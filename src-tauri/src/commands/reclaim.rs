//! Reclaim commands (phase 2D). Read-only listings plus one guarded delete. Removing a worktree is
//! `worktree_remove` and terminating a process is `process_kill`. The folder search runs as a
//! cancelable job (`job://progress`, `job://done`); the other listings are plain async commands on
//! the blocking pool.
use crate::commands::project::{known_projects, known_projects_blocking};
use crate::error::{Error, ErrorKind, Result};
use crate::state::AppState;
use devdeck_core::process_info::{ProcInfo, ProcSource, SysinfoSource};
use devdeck_core::project::Project;
use devdeck_core::reclaim::folders::{self, GlobalCaches, HeavyFolder};
use devdeck_core::reclaim::forgotten::{self, ForgottenWorktree};
use devdeck_core::reclaim::now_secs;
use devdeck_core::reclaim::processes::{self, DevProcess};
use devdeck_core::reclaim::summary::{summarize, ReclaimSummary};
use devdeck_core::scope::{group_by_project, Grouped, Scope};
use devdeck_core::worktrees::{self, git::SystemGit};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager, State};

/// Last full (unfiltered) forgotten-worktree and folder results, so the summary does not have to
/// re-size gigabytes on every refresh. The summary lists processes fresh.
struct Cache {
    forgotten: Vec<ForgottenWorktree>,
    folders: Vec<HeavyFolder>,
}

static CACHE: Mutex<Cache> = Mutex::new(Cache {
    forgotten: Vec::new(),
    folders: Vec::new(),
});

static NEXT_JOB: AtomicU64 = AtomicU64::new(1);

async fn blocking<T: Send + 'static>(f: impl FnOnce() -> Result<T> + Send + 'static) -> Result<T> {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| Error::new(ErrorKind::Io, "io_error").with_detail(e.to_string()))?
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct JobRef {
    job_id: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Freed {
    freed_bytes: u64,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct Progress<'a> {
    job_id: &'a str,
    done: u64,
    total: Option<u64>,
}

/// `job://done`: exactly one per job. `folders` carries the final `Grouped<HeavyFolder>`.
#[derive(Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
struct Done<'a> {
    job_id: &'a str,
    cancelled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<Error>,
    #[serde(skip_serializing_if = "Option::is_none")]
    folders: Option<Grouped<HeavyFolder>>,
}

struct Config {
    root: Option<PathBuf>,
    scan_depth: u8,
    base_branch: String,
    forgotten_days: u32,
}

fn config(state: &AppState) -> Config {
    let c = state.config.lock().unwrap();
    Config {
        root: c.root.clone(),
        scan_depth: c.scan_depth,
        base_branch: c.base_branch.clone(),
        forgotten_days: c.forgotten_days,
    }
}

fn snapshot() -> Vec<ProcInfo> {
    SysinfoSource.snapshot()
}

#[tauri::command]
pub async fn reclaim_dev_processes(
    scope: Scope,
    state: State<'_, AppState>,
) -> Result<Grouped<DevProcess>> {
    let projects = known_projects(&state).await;
    let list = {
        let projects = projects.clone();
        blocking(move || Ok(processes::list(&SysinfoSource, &projects, now_secs()))).await?
    };
    Ok(group_by_project(list, &scope, &projects))
}

/// Lists the forgotten worktrees of the configured root (also refreshes `AppState.projects`).
#[tauri::command]
pub async fn reclaim_forgotten_worktrees(
    scope: Scope,
    state: State<'_, AppState>,
) -> Result<Grouped<ForgottenWorktree>> {
    let cfg = config(&state);
    let Some(root) = cfg.root.clone() else {
        CACHE.lock().unwrap().forgotten.clear();
        return Ok(group_by_project(Vec::new(), &scope, &[]));
    };
    let opts = forgotten::Options {
        forgotten_days: cfg.forgotten_days,
        base_branch: cfg.base_branch.clone(),
        now_secs: now_secs() as i64,
    };
    let depth = cfg.scan_depth;
    let (found, projects) = blocking(move || {
        let (wts, projects) = worktrees::scan_root(&SystemGit, &root, depth);
        let procs = snapshot();
        let never = AtomicBool::new(false);
        let found = forgotten::find(wts, &procs, &opts, &mut |p| {
            devdeck_core::fsutil::dir_size(p, &never, &mut |_| {}).ok()
        });
        Ok((found, projects))
    })
    .await?;
    *state.projects.lock().unwrap() = projects.clone();
    CACHE.lock().unwrap().forgotten = found.clone();
    Ok(group_by_project(found, &scope, &projects))
}

#[tauri::command]
pub async fn reclaim_summary(scope: Scope, state: State<'_, AppState>) -> Result<ReclaimSummary> {
    let projects = known_projects(&state).await;
    let (forgotten, folders) = {
        let c = CACHE.lock().unwrap();
        (c.forgotten.clone(), c.folders.clone())
    };
    blocking(move || {
        let procs = processes::list(&SysinfoSource, &projects, now_secs());
        Ok(summarize(procs, forgotten, folders, &scope, &projects))
    })
    .await
}

fn register_job(state: &AppState) -> (String, Arc<AtomicBool>) {
    let id = format!("reclaim-{}", NEXT_JOB.fetch_add(1, Ordering::Relaxed));
    let flag = Arc::new(AtomicBool::new(false));
    state
        .jobs
        .lock()
        .unwrap()
        .insert(id.clone(), Arc::clone(&flag));
    (id, flag)
}

fn finish_job(app: &AppHandle, job_id: &str, done: Done) {
    app.state::<AppState>().jobs.lock().unwrap().remove(job_id);
    let _ = app.emit("job://done", done);
}

/// Search + sizing of the regenerable folders. Pure of Tauri so it is easy to follow.
fn scan_folders(
    root: Option<&Path>,
    cancel: &AtomicBool,
    projects: &[Project],
    emit: &mut dyn FnMut(u64, Option<u64>),
) -> devdeck_core::Result<Vec<HeavyFolder>> {
    let caches = GlobalCaches::from_env();
    let mut found = match root {
        Some(r) => folders::find_folders(r, &caches, cancel, &mut |n| emit(n, None))?,
        None => folders::find_caches(&caches, &mut |n| emit(n, None)),
    };
    folders::attach_projects(&mut found, projects);
    folders::size_folders(&mut found, cancel, &mut |d, t| emit(d, Some(t)))?;
    Ok(found)
}

/// Starts the folder search; `job://progress` first reports folders found (no total) and then the
/// sizing progress (`done`/`total` folders). The final `job://done` carries `folders`.
#[tauri::command]
pub fn reclaim_folders(app: AppHandle, state: State<'_, AppState>, scope: Scope) -> Result<JobRef> {
    let cfg = config(&state);
    let (job_id, cancel) = register_job(&state);
    let out = JobRef {
        job_id: job_id.clone(),
    };
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let projects = known_projects_blocking(&state);
        let emit_app = app.clone();
        let jid = job_id.clone();
        let mut emit = move |done: u64, total: Option<u64>| {
            let _ = emit_app.emit(
                "job://progress",
                Progress {
                    job_id: &jid,
                    done,
                    total,
                },
            );
        };
        let done = match scan_folders(cfg.root.as_deref(), &cancel, &projects, &mut emit) {
            Ok(found) => {
                CACHE.lock().unwrap().folders = found.clone();
                Done {
                    job_id: &job_id,
                    folders: Some(group_by_project(found, &scope, &projects)),
                    ..Done::default()
                }
            }
            Err(e) if e.kind == ErrorKind::Cancelled => Done {
                job_id: &job_id,
                cancelled: true,
                ..Done::default()
            },
            Err(e) => Done {
                job_id: &job_id,
                error: Some(e),
                ..Done::default()
            },
        };
        finish_job(&app, &job_id, done);
    });
    Ok(out)
}

/// Permanently deletes an allow-listed regenerable folder (see `folders::validate_deletable`).
#[tauri::command]
pub async fn reclaim_delete_folder(path: String, state: State<'_, AppState>) -> Result<Freed> {
    let root = config(&state).root;
    let target = PathBuf::from(path);
    let freed = {
        let target = target.clone();
        blocking(move || {
            folders::delete_folder(
                &target,
                root.as_deref(),
                &GlobalCaches::from_env(),
                folders::home_dir().as_deref(),
            )
        })
        .await?
    };
    CACHE.lock().unwrap().folders.retain(|f| f.path != target);
    Ok(Freed { freed_bytes: freed })
}

#[tauri::command]
pub fn reclaim_cancel(state: State<'_, AppState>, job_id: String) -> Result<()> {
    if let Some(flag) = state.jobs.lock().unwrap().get(&job_id) {
        flag.store(true, Ordering::Relaxed);
    }
    Ok(())
}
