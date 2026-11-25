//! DevDeck core logic. No Tauri dependency; no UI text (errors carry stable codes).

pub mod config;
pub mod error;
pub mod fsutil;
pub mod json_viewer;
pub mod ports;
pub mod process_info;
pub mod project;
pub mod reclaim;
pub mod scope;
pub mod worktrees;

pub use error::{Error, ErrorKind, Result};
