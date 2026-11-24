//! Worktrees commands (phase 2A replaces the bodies; signatures follow contracts/commands.md).
use crate::error::{Error, Result};
use devdeck_core::scope::Scope;
use serde_json::Value;

#[tauri::command]
pub fn worktrees_scan(_scope: Scope) -> Result<Value> {
    Err(Error::not_implemented())
}

#[tauri::command]
pub fn worktree_open_vscode(_path: String) -> Result<()> {
    Err(Error::not_implemented())
}

#[tauri::command]
pub fn worktree_open_terminal(_path: String) -> Result<()> {
    Err(Error::not_implemented())
}

#[tauri::command]
pub fn worktree_remove(_path: String, _force: bool) -> Result<()> {
    Err(Error::not_implemented())
}
