# Data Model

Todos los tipos viven en `devdeck-core` con `serde` (camelCase) y se reflejan a mano en `src/lib/types.ts`.

- **Config**: `{ rootPath?, scanDepth: u8=2, baseBranch="main", lastScope: Scope, forgottenDays: u32=30, portsAutoRefreshSecs? }`
- **Scope**: `{kind:"global"} | {kind:"project", id: ProjectId}`; `ProjectId` = hash estable de la ruta canónica del repo principal.
- **Project**: `{ id, name, mainPath, worktreePaths[] }`; **ProjectRef** `{id,name}`.
- **Totals**: `{ count, ramBytes, diskBytes }`. **Group<T>**: `{ project?: ProjectRef /* null = "Sin proyecto" */, items: T[], totals }`. **Grouped<T>**: `{ groups: Group<T>[], total: Totals }`.
- **Worktree**: `{ path, repoPath, project?, branch?, detached, isMain, dirtyCount, ahead?, behind?, lastCommit?: {hash, subject, dateIso}, exists }`.
- **ListeningPort**: `{ port, protocol:"tcp"|"udp", pid, processName, exePath?, cwd?, worktreePath?, project? }`.
- **ProcInfo**: `{ pid, name, exe?, cwd?, ramBytes, cpuPercent, startTime, parentPid? }`.
- **DevProcess** (Oculto): `ProcInfo + { kind, project?, worktreePath?, orphan, ageSecs }`.
- **ForgottenWorktree**: `{ worktree, diskBytes?, lastCommitAgeDays, merged, clean, liveProcesses, reasons[] }`.
- **HeavyFolder**: `{ path, kind: node_modules|target|.venv|dist|npmCache|pnpmStore|cargoRegistry|cargoGit, diskBytes?, lastUsedIso?, project? }`.
- **CleanupSummary**: `{ recoverableRamBytes, recoverableDiskBytes, byProject: Totals por proyecto }`.
- **JsonSession**: `{ sessionId, path, sizeBytes, rootId, containers }`. **JsonNode**: `{ id, key?, kind: object|array|string|number|bool|null, childCount?, preview?, hasChildren }`.
- **SearchHit**: `{ nodeId?, path:"$.a[3]", kind:"key"|"value", snippet }`.
- **Eventos**: `job://progress {jobId, done, total?}`, `job://done {jobId}`, `json://search-batch {jobId, hits[]}`.
- **Error**: `{ kind: NotFound|NotAllowed|Git|Io|Parse|Cancelled|Unsupported|NotImplemented, message }`.
