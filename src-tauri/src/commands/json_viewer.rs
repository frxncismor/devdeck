//! JSON viewer commands (phase 2C replaces the bodies).
use crate::error::{Error, Result};
use serde_json::Value;

#[tauri::command]
pub fn json_open(_path: String) -> Result<Value> {
    Err(Error::not_implemented())
}

#[tauri::command]
pub fn json_cancel(_job_id: String) -> Result<()> {
    Err(Error::not_implemented())
}

#[tauri::command]
pub fn json_close(_session_id: String) -> Result<()> {
    Err(Error::not_implemented())
}

#[tauri::command]
pub fn json_root(_session_id: String) -> Result<Value> {
    Err(Error::not_implemented())
}

#[tauri::command]
pub fn json_children(
    _session_id: String,
    _node_id: u64,
    _offset: u64,
    _limit: u64,
) -> Result<Value> {
    Err(Error::not_implemented())
}

#[tauri::command]
pub fn json_value(_session_id: String, _node_id: u64, _max_bytes: u64) -> Result<String> {
    Err(Error::not_implemented())
}

#[tauri::command]
pub fn json_path(_session_id: String, _node_id: u64) -> Result<String> {
    Err(Error::not_implemented())
}

#[tauri::command]
pub fn json_search(
    _session_id: String,
    _query: String,
    _in_keys: bool,
    _in_values: bool,
    _case_sensitive: bool,
) -> Result<Value> {
    Err(Error::not_implemented())
}

#[tauri::command]
pub fn json_format_node(_session_id: String, _node_id: u64, _max_bytes: u64) -> Result<String> {
    Err(Error::not_implemented())
}

#[tauri::command]
pub fn json_save_formatted(
    _session_id: String,
    _node_id: Option<u64>,
    _dest_path: String,
) -> Result<Value> {
    Err(Error::not_implemented())
}
