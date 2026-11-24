//! Worktrees contract (phase 1A). The real implementation lands in phase 2A (T030–T034).

use crate::error::{Error, Result};
use crate::project::ProjectRef;
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LastCommit {
    pub hash: String,
    pub subject: String,
    pub date_iso: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Worktree {
    pub path: PathBuf,
    pub repo_path: PathBuf,
    pub project: Option<ProjectRef>,
    pub branch: Option<String>,
    pub detached: bool,
    pub is_main: bool,
    pub dirty_count: u32,
    pub ahead: Option<u32>,
    pub behind: Option<u32>,
    pub last_commit: Option<LastCommit>,
    pub exists: bool,
}

/// Lists the worktrees of the repository at `repo`. Stub: returns `NotImplemented` until phase 2A.
pub fn list(_repo: &Path) -> Result<Vec<Worktree>> {
    Err(Error::not_implemented())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ErrorKind;

    #[test]
    fn list_stub_is_not_implemented() {
        let e = list(Path::new(".")).unwrap_err();
        assert_eq!(e.kind, ErrorKind::NotImplemented);
    }
}
