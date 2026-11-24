# Contratos de comandos Tauri

Convención: `Result<T, Error>`; `scope: Scope` donde se indica (filtrado en Rust, FR-005). Cada grupo vive en su archivo `src-tauri/src/commands/<modulo>.rs`.

## config / project (fase 1)
| Comando | Entrada | Salida |
|---|---|---|
| `get_config` | — | `Config` |
| `set_config` | `Config` | `()` |
| `list_projects` | — | `Project[]` |

## worktrees
| Comando | Entrada | Salida |
|---|---|---|
| `worktrees_scan` | `scope` | `Grouped<Worktree>` |
| `worktree_open_vscode` | `path` | `()` |
| `worktree_open_terminal` | `path` | `()` |
| `worktree_remove` | `path, force: bool` | `()` (rechaza el principal) |

## ports
| Comando | Entrada | Salida |
|---|---|---|
| `ports_list` | `scope, includeUdp, query?` | `Grouped<ListeningPort>` |
| `process_kill` | `pid, expectedName` | `()` (compartido con Oculto) |

## json_viewer
| Comando | Entrada | Salida / eventos |
|---|---|---|
| `json_open` | `path` | `{jobId, sessionId}`; eventos `job://progress`, `job://done` |
| `json_cancel` | `jobId` | `()` |
| `json_close` | `sessionId` | `()` |
| `json_root` | `sessionId` | `JsonNode` |
| `json_children` | `sessionId, nodeId, offset, limit` | `JsonNode[]` |
| `json_value` | `sessionId, nodeId, maxBytes` | `string` |
| `json_path` | `sessionId, nodeId` | `string` (`$.a.b[3]`) |
| `json_search` | `sessionId, query, inKeys, inValues, caseSensitive` | `{jobId}`; evento `json://search-batch` |
| `json_format_node` | `sessionId, nodeId, maxBytes` | `string` |
| `json_save_formatted` | `sessionId, nodeId?, destPath` | `{jobId}`; progreso por evento |

## cleanup
| Comando | Entrada | Salida |
|---|---|---|
| `cleanup_summary` | `scope` | `CleanupSummary` |
| `cleanup_forgotten_worktrees` | `scope` | `Grouped<ForgottenWorktree>` |
| `cleanup_dev_processes` | `scope` | `Grouped<DevProcess>` |
| `cleanup_folders` | `scope` | `{jobId}`; lista rápida + tamaños por `job://progress`; resultado final `Grouped<HeavyFolder>` |
| `cleanup_delete_folder` | `path` | `{freedBytes}` (valida lista blanca) |
| `cleanup_cancel` | `jobId` | `()` |

Quitar worktree = `worktree_remove`; terminar proceso = `process_kill`.

## Capabilities (Tauri 2)
`core:default`, `dialog:allow-open`, `dialog:allow-save`, `store:default`, `clipboard-manager:allow-write-text`,
`opener:allow-open-path`. Sin `shell:*` genérico.
