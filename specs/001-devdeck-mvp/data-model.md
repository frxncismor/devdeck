# Data Model

All types live in `devdeck-core` with `serde` (camelCase) and are mirrored by hand in `src/lib/types.ts`.

- **Config**: `{ rootPath?, scanDepth: u8=2, baseBranch="main", lastScope: Scope, language: "en"|"es" = "en", forgottenDays: u32=30, portsAutoRefreshSecs? }`. An unknown stored `language` falls back to `"en"`.
- **Scope**: `{kind:"global"} | {kind:"project", id: ProjectId}`; `ProjectId` = stable hash of the canonical path of the main repo.
- **Project**: `{ id, name, mainPath, worktreePaths[] }`; **ProjectRef** `{id,name}`.
- **Totals**: `{ count, ramBytes, diskBytes }`. **Group<T>**: `{ project?: ProjectRef /* null = "No project" (label rendered by the frontend) */, items: T[], totals }`. **Grouped<T>**: `{ groups: Group<T>[], total: Totals }`.
- **Worktree**: `{ path, repoPath, project?, branch?, detached, isMain, dirtyCount, ahead?, behind?, lastCommit?: {hash, subject, dateIso}, exists }`.
- **ListeningPort**: `{ port, protocol:"tcp"|"udp", pid, processName, exePath?, cwd?, worktreePath?, project? }`.
- **ProcInfo**: `{ pid, name, exe?, cwd?, ramBytes, cpuPercent, startTime, parentPid? }`.
- **DevProcess** (Hidden): `ProcInfo + { kind, project?, worktreePath?, orphan, ageSecs }`.
- **ForgottenWorktree**: `{ worktree, diskBytes?, lastCommitAgeDays, merged, clean, liveProcesses, reasons[] }` where `reasons` are stable codes (`stale`, `merged`, `clean`, `orphaned_processes`) translated by the frontend.
- **HeavyFolder**: `{ path, kind: node_modules|target|.venv|dist|npmCache|pnpmStore|cargoRegistry|cargoGit, diskBytes?, lastUsedIso?, project? }`.
- **CleanupSummary**: `{ recoverableRamBytes, recoverableDiskBytes, byProject: Totals per project }`.
- **JsonSession**: `{ sessionId, path, sizeBytes, rootId, containers }`. **JsonNode**: `{ id, key?, kind: object|array|string|number|bool|null, childCount?, preview?, hasChildren }`.
- **SearchHit**: `{ nodeId?, path:"$.a[3]", kind:"key"|"value", snippet }`.
- **Events**: `job://progress {jobId, done, total?}`, `job://done {jobId}`, `json://search-batch {jobId, hits[]}`.
- **Error**: `{ kind: NotFound|NotAllowed|Git|Io|Parse|Cancelled|Unsupported|NotImplemented, code: string, params?: Record<string, string|number>, detail?: string }`. `code` is a stable snake-case identifier (e.g. `worktree_main_not_removable`, `process_access_denied`, `json_invalid_at_offset`, `json_file_changed`, `folder_not_allowed`); the frontend translates `errors.<code>` (fallback `errors.unknown`). `detail` is raw technical text (git stderr, OS error), shown only in a details area; Rust never returns UI text.
- **Locale resources** (frontend only): `src/lib/i18n/locales/{en,es}/<namespace>.json`, nested objects of strings, `{name}` placeholders, plural suffixes `_one`/`_other`; namespaces `common`, `worktrees`, `ports`, `jsonViewer` (file `json-viewer.json`), `cleanup`.
