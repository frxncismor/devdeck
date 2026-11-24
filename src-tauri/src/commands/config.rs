use crate::error::{Error, ErrorKind, Result};
use crate::state::AppState;
use devdeck_core::config::Config;
use tauri::{AppHandle, State};
use tauri_plugin_store::StoreExt;

const STORE_FILE: &str = "config.json";
const STORE_KEY: &str = "config";

fn store_err(e: impl std::fmt::Display) -> Error {
    Error::new(ErrorKind::Io, "config_invalid").with_detail(e.to_string())
}

/// Loads the stored config into `AppState` (called once at startup).
pub fn load(app: &AppHandle, state: &AppState) -> Result<()> {
    let store = app.store(STORE_FILE).map_err(store_err)?;
    let cfg = store
        .get(STORE_KEY)
        .and_then(|v| serde_json::from_value::<Config>(v).ok())
        .unwrap_or_default();
    *state.config.lock().unwrap() = cfg;
    Ok(())
}

#[tauri::command]
pub fn get_config(state: State<'_, AppState>) -> Result<Config> {
    Ok(state.config.lock().unwrap().clone())
}

#[tauri::command]
pub fn set_config(app: AppHandle, state: State<'_, AppState>, config: Config) -> Result<()> {
    let store = app.store(STORE_FILE).map_err(store_err)?;
    store.set(STORE_KEY, serde_json::to_value(&config).map_err(store_err)?);
    store.save().map_err(store_err)?;
    let mut current = state.config.lock().unwrap();
    // A different root or scan depth invalidates the cached projects; they are rebuilt lazily.
    if current.root != config.root || current.scan_depth != config.scan_depth {
        state.projects.lock().unwrap().clear();
    }
    *current = config;
    Ok(())
}
