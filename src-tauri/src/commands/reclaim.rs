//! Reclaim commands (phase 2D replaces the bodies).
use crate::error::{Error, Result};
use devdeck_core::scope::Scope;
use serde_json::Value;

#[tauri::command]
pub fn reclaim_summary(_scope: Scope) -> Result<Value> {
    Err(Error::not_implemented())
}

#[tauri::command]
pub fn reclaim_forgotten_worktrees(_scope: Scope) -> Result<Value> {
    Err(Error::not_implemented())
}

#[tauri::command]
pub fn reclaim_dev_processes(_scope: Scope) -> Result<Value> {
    Err(Error::not_implemented())
}

#[tauri::command]
pub fn reclaim_folders(_scope: Scope) -> Result<Value> {
    Err(Error::not_implemented())
}

#[tauri::command]
pub fn reclaim_delete_folder(_path: String) -> Result<Value> {
    Err(Error::not_implemented())
}

#[tauri::command]
pub fn reclaim_cancel(_job_id: String) -> Result<()> {
    Err(Error::not_implemented())
}
