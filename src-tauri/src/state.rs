use devdeck_core::{config::Config, project::Project};
use std::any::Any;
use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

/// Shared application state. Phase-2 agents use the fields below without editing this file.
#[derive(Default)]
pub struct AppState {
    pub config: Mutex<Config>,
    /// Known projects; refreshed by the worktrees scan (phase 2A).
    pub projects: Mutex<Vec<Project>>,
    /// Open JSON viewer session (phase 2C stores its own session type and downcasts).
    pub json: Mutex<Option<Box<dyn Any + Send>>>,
    /// Cancelable background jobs: job id -> cancel flag.
    pub jobs: Mutex<HashMap<String, Arc<AtomicBool>>>,
}
