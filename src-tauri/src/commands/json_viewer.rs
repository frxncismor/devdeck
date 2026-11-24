//! JSON viewer commands (phase 2C). One open session at a time lives in `AppState.json`; indexing,
//! search and "save formatted" run as cancelable background jobs that emit `job://progress`,
//! `job://done` and `json://search-batch`.
use crate::error::{Error, ErrorKind, Result};
use crate::state::AppState;
use devdeck_core::json_viewer::search::{self, SearchHit, SearchOptions};
use devdeck_core::json_viewer::{format, JsonDoc, JsonNode, SessionInfo};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, State};

/// The session stored (type-erased) in `AppState.json`.
struct JsonSession {
    id: String,
    doc: Arc<JsonDoc>,
}

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

fn next_id(prefix: &str) -> String {
    format!("{prefix}-{}", NEXT_ID.fetch_add(1, Ordering::Relaxed))
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OpenJob {
    job_id: String,
    session_id: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct JobRef {
    job_id: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct Progress<'a> {
    job_id: &'a str,
    done: u64,
    total: Option<u64>,
}

/// `job://done`: exactly one per job. `error` carries `{kind, code, params?, detail?}`.
#[derive(Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
struct Done<'a> {
    job_id: &'a str,
    cancelled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<Error>,
    #[serde(skip_serializing_if = "Option::is_none")]
    session: Option<SessionInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    total: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    truncated: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    saved_bytes: Option<u64>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct Batch<'a> {
    job_id: &'a str,
    hits: Vec<SearchHit>,
}

fn session_not_found() -> Error {
    Error::not_found("json_session_not_found")
}

/// Looks up the open session and checks the file did not change since indexing.
fn session_doc(state: &AppState, session_id: &str) -> Result<Arc<JsonDoc>> {
    let doc = {
        let guard = state.json.lock().unwrap();
        guard
            .as_ref()
            .and_then(|b| b.downcast_ref::<JsonSession>())
            .filter(|s| s.id == session_id)
            .map(|s| Arc::clone(&s.doc))
            .ok_or_else(session_not_found)?
    };
    doc.check_unchanged()?;
    Ok(doc)
}

fn register_job(state: &AppState) -> (String, Arc<AtomicBool>) {
    let id = next_id("job");
    let flag = Arc::new(AtomicBool::new(false));
    state
        .jobs
        .lock()
        .unwrap()
        .insert(id.clone(), Arc::clone(&flag));
    (id, flag)
}

fn finish_job(app: &AppHandle, job_id: &str, done: Done) {
    app.state::<AppState>().jobs.lock().unwrap().remove(job_id);
    let _ = app.emit("job://done", done);
}

fn failed<'a>(job_id: &'a str, e: Error) -> Done<'a> {
    if e.kind == ErrorKind::Cancelled {
        Done {
            job_id,
            cancelled: true,
            ..Done::default()
        }
    } else {
        Done {
            job_id,
            error: Some(e),
            ..Done::default()
        }
    }
}

#[tauri::command]
pub fn json_open(app: AppHandle, state: State<'_, AppState>, path: String) -> Result<OpenJob> {
    let path = PathBuf::from(path);
    let total = std::fs::metadata(&path)?.len();
    let (job_id, cancel) = register_job(&state);
    let session_id = next_id("json");
    let out = OpenJob {
        job_id: job_id.clone(),
        session_id: session_id.clone(),
    };
    tauri::async_runtime::spawn_blocking(move || {
        let emit_app = app.clone();
        let jid = job_id.clone();
        let mut progress = move |done: u64| {
            let _ = emit_app.emit(
                "job://progress",
                Progress {
                    job_id: &jid,
                    done,
                    total: Some(total),
                },
            );
        };
        let done = match JsonDoc::open(&path, &cancel, &mut progress) {
            Ok(doc) => {
                let info = SessionInfo {
                    session_id: session_id.clone(),
                    path: path.to_string_lossy().into_owned(),
                    size_bytes: doc.size(),
                    root_id: doc.root_id(),
                    containers: doc.containers(),
                };
                let session = JsonSession {
                    id: session_id,
                    doc: Arc::new(doc),
                };
                // Replaces (and drops, unmapping) the previous session.
                *app.state::<AppState>().json.lock().unwrap() = Some(Box::new(session));
                Done {
                    job_id: &job_id,
                    session: Some(info),
                    ..Done::default()
                }
            }
            Err(e) => failed(&job_id, e),
        };
        finish_job(&app, &job_id, done);
    });
    Ok(out)
}

#[tauri::command]
pub fn json_cancel(state: State<'_, AppState>, job_id: String) -> Result<()> {
    if let Some(flag) = state.jobs.lock().unwrap().get(&job_id) {
        flag.store(true, Ordering::Relaxed);
    }
    Ok(())
}

#[tauri::command]
pub fn json_close(state: State<'_, AppState>, session_id: String) -> Result<()> {
    let mut guard = state.json.lock().unwrap();
    let matches = guard
        .as_ref()
        .and_then(|b| b.downcast_ref::<JsonSession>())
        .is_some_and(|s| s.id == session_id);
    if matches {
        *guard = None;
    }
    Ok(())
}

#[tauri::command]
pub fn json_root(state: State<'_, AppState>, session_id: String) -> Result<JsonNode> {
    session_doc(&state, &session_id)?.root()
}

#[tauri::command]
pub fn json_children(
    state: State<'_, AppState>,
    session_id: String,
    node_id: u64,
    offset: u64,
    limit: u64,
) -> Result<Vec<JsonNode>> {
    session_doc(&state, &session_id)?.children(node_id, offset, limit)
}

#[tauri::command]
pub fn json_value(
    state: State<'_, AppState>,
    session_id: String,
    node_id: u64,
    max_bytes: u64,
) -> Result<String> {
    session_doc(&state, &session_id)?.value(node_id, max_bytes)
}

#[tauri::command]
pub fn json_path(state: State<'_, AppState>, session_id: String, node_id: u64) -> Result<String> {
    session_doc(&state, &session_id)?.path_of(node_id)
}

/// Starts a background search; hits arrive in `json://search-batch` events (≤ 500 each) and the
/// final `job://done` carries `{total, truncated}`. `limit` caps the number of hits (default 10 000).
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn json_search(
    app: AppHandle,
    state: State<'_, AppState>,
    session_id: String,
    query: String,
    in_keys: bool,
    in_values: bool,
    case_sensitive: bool,
    limit: Option<u64>,
) -> Result<JobRef> {
    let doc = session_doc(&state, &session_id)?;
    let (job_id, cancel) = register_job(&state);
    let mut opts = SearchOptions::new(&query);
    opts.in_keys = in_keys;
    opts.in_values = in_values;
    opts.case_sensitive = case_sensitive;
    if let Some(l) = limit {
        opts.max_results = usize::try_from(l).unwrap_or(usize::MAX);
    }
    let out = JobRef {
        job_id: job_id.clone(),
    };
    tauri::async_runtime::spawn_blocking(move || {
        let emit_app = app.clone();
        let jid = job_id.clone();
        let mut on_batch = move |hits: Vec<SearchHit>| {
            let _ = emit_app.emit("json://search-batch", Batch { job_id: &jid, hits });
        };
        let done = match search::search(&doc, &opts, &cancel, &mut on_batch) {
            Ok(s) => Done {
                job_id: &job_id,
                total: Some(s.total),
                truncated: Some(s.truncated),
                ..Done::default()
            },
            Err(e) => failed(&job_id, e),
        };
        finish_job(&app, &job_id, done);
    });
    Ok(out)
}

/// Pretty-prints one node (subtree) in memory; refuses values larger than `max_bytes` (≤ 5 MB).
#[tauri::command]
pub fn json_format_node(
    state: State<'_, AppState>,
    session_id: String,
    node_id: u64,
    max_bytes: u64,
) -> Result<String> {
    let doc = session_doc(&state, &session_id)?;
    format::format_node(&doc, node_id, max_bytes)
}

/// Streams the formatted node (or whole file when `node_id` is `None`) to `dest_path`.
#[tauri::command]
pub fn json_save_formatted(
    app: AppHandle,
    state: State<'_, AppState>,
    session_id: String,
    node_id: Option<u64>,
    dest_path: String,
) -> Result<JobRef> {
    let doc = session_doc(&state, &session_id)?;
    let (job_id, cancel) = register_job(&state);
    let out = JobRef {
        job_id: job_id.clone(),
    };
    tauri::async_runtime::spawn_blocking(move || {
        let emit_app = app.clone();
        let jid = job_id.clone();
        let mut progress = move |done: u64, total: u64| {
            let _ = emit_app.emit(
                "job://progress",
                Progress {
                    job_id: &jid,
                    done,
                    total: Some(total),
                },
            );
        };
        let dest: &Path = Path::new(&dest_path);
        let done = match format::save_formatted(&doc, node_id, dest, &cancel, &mut progress) {
            Ok(bytes) => Done {
                job_id: &job_id,
                saved_bytes: Some(bytes),
                ..Done::default()
            },
            Err(e) => failed(&job_id, e),
        };
        finish_job(&app, &job_id, done);
    });
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state_with_session(id: &str, path: &Path) -> AppState {
        let state = AppState::default();
        let doc = JsonDoc::open(path, &AtomicBool::new(false), &mut |_| {}).unwrap();
        *state.json.lock().unwrap() = Some(Box::new(JsonSession {
            id: id.to_owned(),
            doc: Arc::new(doc),
        }));
        state
    }

    fn temp_json(name: &str, text: &str) -> PathBuf {
        let p =
            std::env::temp_dir().join(format!("devdeck-json-cmd-{}-{name}", std::process::id()));
        std::fs::write(&p, text).unwrap();
        p
    }

    #[test]
    fn session_lookup_checks_id_and_changes() {
        let p = temp_json("a.json", r#"{"a":[1,2,3]}"#);
        let state = state_with_session("json-7", &p);
        let doc = session_doc(&state, "json-7").unwrap();
        assert_eq!(doc.root().unwrap().child_count, Some(1));
        assert_eq!(
            session_doc(&state, "json-8").err().unwrap().code,
            "json_session_not_found"
        );

        // Editing the file after indexing is reported, never read blindly.
        let mut f = std::fs::OpenOptions::new().append(true).open(&p).unwrap();
        std::io::Write::write_all(&mut f, b" ").unwrap();
        drop(f);
        assert_eq!(
            session_doc(&state, "json-7").err().unwrap().code,
            "json_file_changed"
        );
        drop(doc);
        *state.json.lock().unwrap() = None;
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn done_payload_shape() {
        let v = serde_json::to_value(failed("job-1", Error::cancelled())).unwrap();
        assert_eq!(v, serde_json::json!({"jobId": "job-1", "cancelled": true}));
        let v = serde_json::to_value(failed(
            "job-2",
            Error::new(ErrorKind::Parse, "json_invalid_at_offset").with_param("offset", 3),
        ))
        .unwrap();
        assert_eq!(v["error"]["code"], "json_invalid_at_offset");
        assert_eq!(v["cancelled"], false);
    }

    #[test]
    fn job_registry_roundtrip() {
        let state = AppState::default();
        let (id, flag) = register_job(&state);
        assert!(state.jobs.lock().unwrap().contains_key(&id));
        assert!(!flag.load(Ordering::Relaxed));
    }
}
