//! Worktrees commands (contracts/commands.md). Git work runs on the blocking pool so the UI never freezes.
use crate::commands::project::known_projects;
use crate::error::{Error, ErrorKind, Result};
use crate::state::AppState;
use devdeck_core::scope::{self, group_by_project, Grouped, Scope};
use devdeck_core::worktrees::{self, git::SystemGit, Worktree};
use std::path::PathBuf;
use tauri::State;

async fn blocking<T: Send + 'static>(f: impl FnOnce() -> Result<T> + Send + 'static) -> Result<T> {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| Error::new(ErrorKind::Io, "io_error").with_detail(e.to_string()))?
}

/// Scans the configured root, refreshes `AppState.projects` and returns the worktrees grouped by
/// project, filtered by `scope` in Rust (FR-005). Without a root the result is empty.
#[tauri::command]
pub async fn worktrees_scan(scope: Scope, state: State<'_, AppState>) -> Result<Grouped<Worktree>> {
    let (root, depth) = {
        let c = state.config.lock().unwrap();
        (c.root.clone(), c.scan_depth)
    };
    let Some(root) = root else {
        *state.projects.lock().unwrap() = Vec::new();
        return Ok(group_by_project(Vec::new(), &scope, &[]));
    };
    let (found, projects) =
        blocking(move || Ok(worktrees::scan_root(&SystemGit, &root, depth))).await?;
    *state.projects.lock().unwrap() = projects.clone();
    Ok(group_by_project(found, &scope, &projects))
}

/// A path received from the front end must be one of the known project paths (main repository or
/// linked worktree); anything else is rejected before a process is started or git is run.
async fn ensure_known_worktree(state: &AppState, path: &str) -> Result<PathBuf> {
    let path = PathBuf::from(path);
    let wanted = scope::normalize(&path);
    let projects = known_projects(state).await;
    if projects
        .iter()
        .any(|p| p.paths().any(|known| scope::normalize(known) == wanted))
    {
        Ok(path)
    } else {
        Err(Error::not_allowed("not_allowed"))
    }
}

#[tauri::command]
pub async fn worktree_open_vscode(path: String, state: State<'_, AppState>) -> Result<()> {
    let path = ensure_known_worktree(&state, &path).await?;
    blocking(move || worktrees::open::open_vscode(&path)).await
}

#[tauri::command]
pub async fn worktree_open_terminal(path: String, state: State<'_, AppState>) -> Result<()> {
    let path = ensure_known_worktree(&state, &path).await?;
    blocking(move || worktrees::open::open_terminal(&path)).await
}

/// Removes a linked worktree. The main worktree is rejected; a dirty one needs `force`.
#[tauri::command]
pub async fn worktree_remove(path: String, force: bool, state: State<'_, AppState>) -> Result<()> {
    let path = ensure_known_worktree(&state, &path).await?;
    blocking(move || worktrees::remove(&path, force)).await?;
    // The removed worktree no longer belongs to its project.
    state.projects.lock().unwrap().clear();
    Ok(())
}
