# Tauri command contracts

Convention: `Result<T, Error>`; `scope: Scope` where indicated (filtered in Rust, FR-005). Each group lives in its own file `src-tauri/src/commands/<module>.rs`.
`Error` is `{kind, code, params?, detail?}` (stable codes, no UI text; the frontend translates, FR-054).

## config / project (phase 1)
| Command | Input | Output |
|---|---|---|
| `get_config` | — | `Config` (includes `language`) |
| `set_config` | `Config` | `()` |
| `list_projects` | — | `Project[]` |

## worktrees
| Command | Input | Output |
|---|---|---|
| `worktrees_scan` | `scope` | `Grouped<Worktree>` |
| `worktree_open_vscode` | `path` | `()` |
| `worktree_open_terminal` | `path` | `()` |
| `worktree_remove` | `path, force: bool` | `()` (rejects the main worktree) |

## ports
| Command | Input | Output |
|---|---|---|
| `ports_list` | `scope, includeUdp, query?` | `Grouped<ListeningPort>` |
| `process_kill` | `pid, expectedName` | `()` (shared with Hidden) |

## json_viewer
| Command | Input | Output / events |
|---|---|---|
| `json_open` | `path` | `{jobId, sessionId}`; events `job://progress`, `job://done` |
| `json_cancel` | `jobId` | `()` |
| `json_close` | `sessionId` | `()` |
| `json_root` | `sessionId` | `JsonNode` |
| `json_children` | `sessionId, nodeId, offset, limit` | `JsonNode[]` |
| `json_value` | `sessionId, nodeId, maxBytes` | `string` |
| `json_path` | `sessionId, nodeId` | `string` (`$.a.b[3]`) |
| `json_search` | `sessionId, query, inKeys, inValues, caseSensitive` | `{jobId}`; event `json://search-batch` |
| `json_format_node` | `sessionId, nodeId, maxBytes` | `string` |
| `json_save_formatted` | `sessionId, nodeId?, destPath` | `{jobId}`; progress by event |

## cleanup
| Command | Input | Output |
|---|---|---|
| `cleanup_summary` | `scope` | `CleanupSummary` |
| `cleanup_forgotten_worktrees` | `scope` | `Grouped<ForgottenWorktree>` |
| `cleanup_dev_processes` | `scope` | `Grouped<DevProcess>` |
| `cleanup_folders` | `scope` | `{jobId}`; quick list + sizes via `job://progress`; final result `Grouped<HeavyFolder>` |
| `cleanup_delete_folder` | `path` | `{freedBytes}` (validates the allow-list) |
| `cleanup_cancel` | `jobId` | `()` |

Remove worktree = `worktree_remove`; terminate process = `process_kill`.

Language is not a backend concern: it is part of `Config` (`get_config`/`set_config`), and every command returns data and
error codes only, never translated text.

## Capabilities (Tauri 2)
`core:default`, `dialog:allow-open`, `dialog:allow-save`, `store:default`, `clipboard-manager:allow-write-text`,
`opener:allow-open-path`. No generic `shell:*`.
