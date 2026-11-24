//! Worktrees commands (contracts/commands.md). Git work runs on the blocking pool so the UI never freezes.
use crate::error::{Error, ErrorKind, Result};
use crate::state::AppState;
use devdeck_core::scope::{group_by_project, Grouped, Scope};
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

#[tauri::command]
pub async fn worktree_open_vscode(path: String) -> Result<()> {
    blocking(move || worktrees::open::open_vscode(&PathBuf::from(path))).await
}

#[tauri::command]
pub async fn worktree_open_terminal(path: String) -> Result<()> {
    blocking(move || worktrees::open::open_terminal(&PathBuf::from(path))).await
}

/// Removes a linked worktree. The main worktree is rejected; a dirty one needs `force`.
#[tauri::command]
pub async fn worktree_remove(path: String, force: bool) -> Result<()> {
    blocking(move || worktrees::remove(&PathBuf::from(path), force)).await
}
