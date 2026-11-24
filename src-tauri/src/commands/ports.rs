//! Ports commands (phase 2B replaces the bodies).
use crate::error::{Error, Result};
use devdeck_core::scope::Scope;
use serde_json::Value;

#[tauri::command]
pub fn ports_list(_scope: Scope, _include_udp: bool, _query: Option<String>) -> Result<Value> {
    Err(Error::not_implemented())
}

/// Shared with Reclaim. Real implementation: `devdeck_core::process_info::kill`.
#[tauri::command]
pub fn process_kill(_pid: u32, _expected_name: String) -> Result<()> {
    Err(Error::not_implemented())
}
