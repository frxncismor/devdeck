//! One file per module. The command registry (`generate_handler!`) lives in `lib.rs` and is closed
//! by phase 1A: phase-2 agents only replace the bodies in their own file.
pub mod config;
pub mod json_viewer;
pub mod ports;
pub mod project;
pub mod reclaim;
pub mod worktrees;
