use crate::error::Result;
use crate::state::AppState;
use devdeck_core::project::Project;
use tauri::State;

/// Projects found under the configured root. Empty until the worktrees scan (phase 2A) fills it.
#[tauri::command]
pub fn list_projects(state: State<'_, AppState>) -> Result<Vec<Project>> {
    Ok(state.projects.lock().unwrap().clone())
}
