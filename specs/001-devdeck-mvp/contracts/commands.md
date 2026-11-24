# Tauri command contracts

Convention: `Result<T, Error>`; `scope: Scope` where indicated (filtered in Rust, FR-005). Each group lives in its own file `src-tauri/src/commands/<module>.rs`.
`Error` is `{kind, code, params?, detail?}` (stable codes, no UI text; the frontend translates, FR-054).

## config / project (phase 1)
| Command | Input | Output |
|---|---|---|
| `get_config` | — | `Config` (includes `language`) |
| `set_config` | `Config` | `()` |
| `list_projects` | — | `Project[]` (warmed at startup from `Config.root`; rebuilt lazily after a root/depth change or a worktree removal) |

## worktrees
| Command | Input | Output |
|---|---|---|
| `worktrees_scan` | `scope` | `Grouped<Worktree>` |
| `worktree_open_vscode` | `path` | `()` (`path` must be a known project path, else `not_allowed`) |
| `worktree_open_terminal` | `path` | `()` (same check) |
| `worktree_remove` | `path, force: bool` | `()` (rejects the main worktree and unknown paths; dirty needs `force`, else `worktree_dirty`) |

## ports
| Command | Input | Output |
|---|---|---|
| `ports_list` | `scope, includeUdp, query?` | `Grouped<ListeningPort>` |
| `process_kill` | `pid, expectedName` | `()` (shared with Reclaim) |

## json_viewer
| Command | Input | Output / events |
|---|---|---|
| `json_open` | `path` | `{jobId, sessionId}`; events `job://progress {jobId, done, total}`, `job://done {jobId, cancelled, error?, session?}` |
| `json_cancel` | `jobId` | `()` |
| `json_close` | `sessionId` | `()` |
| `json_root` | `sessionId` | `JsonNode` |
| `json_children` | `sessionId, nodeId, offset, limit` | `JsonNode[]` |
| `json_value` | `sessionId, nodeId, maxBytes` | `string` |
| `json_path` | `sessionId, nodeId` | `string` (`$.a.b[3]`) |
| `json_search` | `sessionId, query, inKeys, inValues, caseSensitive, limit?` | `{jobId}`; event `json://search-batch` |
| `json_format_node` | `sessionId, nodeId, maxBytes` | `string` |
| `json_save_formatted` | `sessionId, nodeId?, destPath` | `{jobId}`; progress by event; `job://done {…, savedBytes?}`; refuses the source file (`json_save_same_file`) |

## reclaim
| Command | Input | Output |
|---|---|---|
| `reclaim_summary` | `scope` | `ReclaimSummary` |
| `reclaim_forgotten_worktrees` | `scope` | `Grouped<ForgottenWorktree>` |
| `reclaim_dev_processes` | `scope` | `Grouped<DevProcess>` |
| `reclaim_folders` | `scope` | `{jobId}`; folders found (no total) then sizing via `job://progress {jobId, done, total?}`; final result in `job://done {jobId, cancelled, error?, folders?: Grouped<HeavyFolder>}` |
| `reclaim_delete_folder` | `path` | `{freedBytes}` (validates the allow-list; rejection = `folder_not_allowed` with `params.reason` in `link, not_a_directory, drive_root, home, root, outside_root, not_allowlisted`) |
| `reclaim_cancel` | `jobId` | `()` |

Remove worktree = `worktree_remove`; terminate process = `process_kill`.

Language is not a backend concern: it is part of `Config` (`get_config`/`set_config`), and every command returns data and
error codes only, never translated text.

## Capabilities (Tauri 2)
`core:default`, `dialog:allow-open`, `dialog:allow-save`, `clipboard-manager:allow-write-text` (only what the front end calls). The store plugin is used from Rust only (no `store:*`), the opener plugin is not installed. No generic `shell:*`.
