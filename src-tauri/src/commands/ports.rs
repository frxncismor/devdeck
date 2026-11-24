//! Ports commands (contracts/commands.md). System queries run on the blocking pool.
use crate::error::{Error, ErrorKind, Result};
use crate::state::AppState;
use devdeck_core::ports::{self, ListeningPort, SystemPorts};
use devdeck_core::process_info::{self, SysinfoSource};
use devdeck_core::project::Project;
use devdeck_core::scope::{Grouped, Scope};
use devdeck_core::worktrees;
use tauri::State;

async fn blocking<T: Send + 'static>(f: impl FnOnce() -> Result<T> + Send + 'static) -> Result<T> {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| Error::new(ErrorKind::Io, "io_error").with_detail(e.to_string()))?
}

/// Listening ports grouped by project, filtered by `scope` and `query` in Rust.
/// Uses `AppState.projects`; when empty (Worktrees not opened yet) they are computed from the
/// configured root and cached.
#[tauri::command]
pub async fn ports_list(
    scope: Scope,
    include_udp: bool,
    query: Option<String>,
    state: State<'_, AppState>,
) -> Result<Grouped<ListeningPort>> {
    let mut projects: Vec<Project> = state.projects.lock().unwrap().clone();
    if projects.is_empty() {
        let (root, depth) = {
            let c = state.config.lock().unwrap();
            (c.root.clone(), c.scan_depth)
        };
        if let Some(root) = root {
            projects = blocking(move || Ok(worktrees::projects_from_root(&root, depth))).await?;
            *state.projects.lock().unwrap() = projects.clone();
        }
    }
    blocking(move || {
        ports::list_grouped(
            &SystemPorts,
            &SysinfoSource,
            &projects,
            &scope,
            include_udp,
            query.as_deref(),
        )
    })
    .await
}

/// Shared with Reclaim. Kills `pid` only if its current name still equals `expected_name`.
#[tauri::command]
pub async fn process_kill(pid: u32, expected_name: String) -> Result<()> {
    blocking(move || process_info::kill(&SysinfoSource, pid, &expected_name)).await
}
