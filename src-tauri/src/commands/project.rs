use crate::error::Result;
use crate::state::AppState;
use devdeck_core::project::Project;
use devdeck_core::worktrees;
use tauri::State;

/// Projects scanned from the configured root (blocking: runs git). Empty without a root.
pub fn scan_projects(root: Option<std::path::PathBuf>, depth: u8) -> Vec<Project> {
    root.map(|r| worktrees::projects_from_root(&r, depth))
        .unwrap_or_default()
}

/// Known projects; computed from `Config.root` when none are cached yet (startup, root change).
/// Runs git on the blocking pool, never on the async/main thread, and caches the result.
pub async fn known_projects(state: &AppState) -> Vec<Project> {
    {
        let known = state.projects.lock().unwrap();
        if !known.is_empty() {
            return known.clone();
        }
    }
    let (root, depth) = {
        let c = state.config.lock().unwrap();
        (c.root.clone(), c.scan_depth)
    };
    if root.is_none() {
        return Vec::new();
    }
    let projects = tauri::async_runtime::spawn_blocking(move || scan_projects(root, depth))
        .await
        .unwrap_or_default();
    *state.projects.lock().unwrap() = projects.clone();
    projects
}

/// Projects found under the configured root (filled at startup, after a scan and on a root change).
#[tauri::command]
pub async fn list_projects(state: State<'_, AppState>) -> Result<Vec<Project>> {
    Ok(known_projects(&state).await)
}

/// Blocking variant of [`known_projects`] for code already running on the blocking pool.
pub fn known_projects_blocking(state: &AppState) -> Vec<Project> {
    {
        let known = state.projects.lock().unwrap();
        if !known.is_empty() {
            return known.clone();
        }
    }
    let (root, depth) = {
        let c = state.config.lock().unwrap();
        (c.root.clone(), c.scan_depth)
    };
    let projects = scan_projects(root, depth);
    *state.projects.lock().unwrap() = projects.clone();
    projects
}
