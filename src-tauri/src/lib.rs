//! DevDeck Tauri shell: plugin setup, state and the closed command registry.
pub mod commands;
pub mod error;
pub mod state;

use commands::{config, json_viewer, ports, project, reclaim, worktrees};
use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .manage(state::AppState::default())
        .setup(|app| {
            let state = app.state::<state::AppState>();
            // A missing or unreadable store falls back to the default config.
            let _ = config::load(app.handle(), &state);
            // Warm the project list so the scope selector is populated before any tab is opened.
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let state = handle.state::<state::AppState>();
                project::known_projects(&state).await;
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            config::get_config,
            config::set_config,
            project::list_projects,
            worktrees::worktrees_scan,
            worktrees::worktree_open_vscode,
            worktrees::worktree_open_terminal,
            worktrees::worktree_remove,
            ports::ports_list,
            ports::process_kill,
            json_viewer::json_open,
            json_viewer::json_cancel,
            json_viewer::json_close,
            json_viewer::json_root,
            json_viewer::json_children,
            json_viewer::json_value,
            json_viewer::json_path,
            json_viewer::json_search,
            json_viewer::json_format_node,
            json_viewer::json_save_formatted,
            reclaim::reclaim_summary,
            reclaim::reclaim_forgotten_worktrees,
            reclaim::reclaim_dev_processes,
            reclaim::reclaim_folders,
            reclaim::reclaim_delete_folder,
            reclaim::reclaim_cancel,
        ])
        .run(tauri::generate_context!())
        .expect("error while running DevDeck");
}
