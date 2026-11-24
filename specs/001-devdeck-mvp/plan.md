# Implementation Plan: DevDeck MVP

**Spec**: [spec.md](spec.md) | **Constitution**: `.specify/memory/constitution.md`
Decision details: [research.md](research.md) · models: [data-model.md](data-model.md) · commands: [contracts/commands.md](contracts/commands.md) · getting started: [quickstart.md](quickstart.md)

## Summary
Tauri 2 app with the logic in a library crate `devdeck-core` (no Tauri) and a thin `src-tauri` crate that only exposes
commands. Svelte 5 + TS + Vite frontend (no SvelteKit) with a scope store and a minimal typed i18n layer (en/es). Git via
CLI, ports with `listeners`/`sysinfo`, JSON with a streaming index + offset reads.

## Technical context
Stable Rust (MSRV = Tauri 2's), Tauri 2, Svelte 5 + TS + Vite 6, pnpm. Storage: `store` plugin (config).
Tests: `cargo test` (core), `vitest` (front). Target: Windows 10/11 (guaranteed), macOS/Linux compile.

## Structure
```
Cargo.toml                    # workspace: crates/devdeck-core, src-tauri
crates/devdeck-core/src/
  lib.rs  error.rs  scope.rs  project.rs  process_info.rs  fsutil.rs
  worktrees/{mod,git,scan,open}.rs
  ports/{mod,listing}.rs
  json_viewer/{mod,indexer,reader,search,format}.rs
  cleanup/{mod,sizes,processes,folders,forgotten,summary}.rs
src-tauri/src/
  main.rs  lib.rs            # run(); command registration
  error.rs  state.rs
  commands/{mod,config,project,worktrees,ports,json_viewer,cleanup}.rs
src-tauri/{tauri.conf.json,capabilities/default.json,icons/}
src/                          # Svelte
  main.ts  App.svelte  lib/{api.ts,scope.svelte.ts,types.ts,format.ts}
  lib/i18n/{index.ts,i18n.svelte.ts,keys.ts,i18n.test.ts}
  lib/i18n/locales/{en,es}/{common,worktrees,ports,json-viewer,cleanup}.json   # one file per module and language
  modules/{worktrees,ports,json-viewer,cleanup}/  # one folder per module
  components/{Sidebar,ScopeSelector,LanguageSelector,ConfirmDialog,VirtualList,GroupedTable}.svelte
.githooks/pre-commit  .gitleaks.toml  scripts/{setup-gitleaks.mts,setup-gitleaks.test.mts}
.github/workflows/{ci.yml,secret-scan.yml,release.yml}
```
**Conflict-free extension points**: each module has its own file in `commands/`, its own folder in `src/modules/` and
its own pair of locale files. Phase 1A leaves `commands/mod.rs` with a complete `generate_handler![ ... ]` list holding
every command name as a stub (`NotImplemented`), so phase-2 agents only replace the body in their own file and never touch
the registry. Likewise on the front: `App.svelte` already imports the 4 modules (each `index.svelte` is owned by its
module), and `src/lib/i18n/index.ts` already imports all 10 locale JSON files (created as `{}` in 1A); phase-2 agents only
fill their own two files.

## Decisions (rationale and rejected alternative)
| Topic | Decision | Rejected |
|---|---|---|
| Workspace | `devdeck-core` + `src-tauri` | Everything in `src-tauri`: slow tests with Tauri and UI/logic mixed |
| Frontend | Svelte 5 + Vite without SvelteKit | SvelteKit adapter-static: unneeded routing/SSR; routes = module state |
| Packages | pnpm | npm: the reference (grim-dawn-builds) and the security scripts already use pnpm |
| Git | `git` CLI (`worktree list --porcelain`, `status --porcelain=v2 --branch`, `log -1`) | `git2`: limited worktree support, builds libgit2 (slow/heavy); `gix` still incomplete for status/worktree remove |
| Ports | `listeners` crate (PID per port, Win/Mac/Linux) + `sysinfo` | `netstat2`: less maintained; hand-written `GetExtendedTcpTable` with `windows`: more code and `unsafe` |
| Processes | `sysinfo` (name, exe, cwd, memory, CPU, start_time, kill) | WMI: slow and Windows-only |
| cwd on Windows | `Process::cwd()` (reads the PEB); if `None` ⇒ "unknown" | Injection/dbghelp: robust but out of scope |
| JSON viewer | own streaming index + `memmap2` | Loading into `serde_json::Value`: 500 MB ⇒ several GB |
| Virtual tree | own component (flat list of visible rows + window) | Library: a dependency for ~80 lines of code |
| Sizes | `walkdir` on a blocking thread with a cancel `AtomicBool`; not `jwalk` (extra rayon) | jwalk: faster but more complexity |
| Deletion | `std::fs::remove_dir_all` after allow-list validation; no recycle bin | `trash`: slow with thousands of files, per-OS behavior |
| Scope | Filtering/grouping/totals in Rust (`scope.rs`) | Filtering in the front: duplicated logic, untestable (constitution III) |
| Config | `tauri-plugin-store` | Own JSON file: reinvents it |
| i18n | Own minimal layer: Svelte 5 runes + static JSON + typed keys (~80 lines) | `svelte-i18n`: stores-based API (not runes), ICU message parser and async locale loading we do not need for 2 static languages; plus an extra dependency and untyped keys by default |

## Global / project scope (shared contract, phase 1)
- `project.rs`: `Project {id, name, main_path, worktree_paths}`; `build_projects(root)` uses `worktrees::list`.
- `scope.rs`: `enum Scope { Global, Project(ProjectId) }`; `attribute(path, &[Project]) -> Option<ProjectId>`
  (normalizes separators, strips `\\?\`, lowercases on Windows, component-wise prefix, longest wins);
  `Grouped<T> { groups: Vec<Group<T>>, total: Totals }`, `Group { project: Option<ProjectRef>, items, totals }`,
  `Totals { ram_bytes, disk_bytes, count }`; `group_by_project(items, scope, projects)` generic over an `Attributable` trait
  (`fn path(&self)->Option<&Path>`, `ram()`, `disk()`).
- Front: `scope.svelte.ts` stores the scope (persisted in the config) and passes it to every `api.*`; `GroupedTable.svelte` renders groups
  and shows the "No project" label from `common.noProject` when `group.project` is null.
- Each phase-2 block implements `Attributable` for its type and calls `group_by_project`.

## Internationalization (en / es)
**Decision: own minimal layer, not `svelte-i18n`.** Two static languages, no plurals beyond one/other, no lazy loading. The
layer is ~80 lines, uses Svelte 5 runes (so components re-render on language change with no store subscriptions), and gives
compile-time-checked keys, which `svelte-i18n` does not by default.

- **Resources**: `src/lib/i18n/locales/{en,es}/{common,worktrees,ports,json-viewer,cleanup}.json`. One file per module and per
  language, so phase-2 agents never edit the same file. `common.json` (shell, scope/language selectors, buttons, confirm
  dialog, generic errors, "No project") is owned by 1A; module agents put module-specific copy in their own file and only
  reuse `common.*` keys. Changes to `common.json` after 1A go through the integration phase.
- **Closed index**: `src/lib/i18n/index.ts` statically imports the 10 files and builds
  `messages = { en: { common, worktrees, ports, jsonViewer, cleanup }, es: { ... } }`. The namespace list is fixed in 1A; adding a
  module namespace later is an explicit edit of this one file.
- **Typed keys**: `keys.ts` derives `type MessageKey` from the `en` catalogue with a recursive template-literal type over the
  nested JSON (`"worktrees.actions.remove"`, `"common.noProject"`). `t(key: MessageKey, params?: Record<string, string|number>)`
  therefore fails `svelte-check` on a typo or a deleted key. `es` is typed as `typeof en` structure through the parity test.
- **API** (`i18n.svelte.ts`): `locale` rune (`"en" | "es"`), `setLocale(l)` (persists via `set_config`), `t()` with `{name}`
  interpolation and `_one`/`_other` plural suffix keys selected with `Intl.PluralRules`. Missing key in `es` falls back to `en`
  (and the parity test makes that impossible in CI). Unknown stored language ⇒ `en`.
- **Language selection**: default `en`; stored in `Config.language`; **no automatic system-language detection** (decision: keeps
  behavior predictable for demos and removes a startup branch; revisit only if Paco asks). `LanguageSelector.svelte` lives
  next to `ScopeSelector` in the shell. `<html lang>` follows the locale.
- **Formatting** (`src/lib/format.ts`, locale-aware via the `locale` rune): `formatBytes` (binary scale, numeric part via
  `Intl.NumberFormat`, unit labels KB/MB/GB/TB), `formatNumber`, `formatPercent`, `formatDate`/`formatDateTime`
  (`Intl.DateTimeFormat`), `formatRelativeAge` (`Intl.RelativeTimeFormat`, days/hours/minutes).
- **Backend errors**: Rust `Error` serializes as `{ kind, code, params?, detail? }`. `code` is a stable snake-case identifier
  (e.g. `worktree_main_not_removable`, `process_access_denied`, `json_invalid_at_offset`); `params` carries values to
  interpolate (`{offset}`); `detail` is raw technical text (git stderr, OS error) shown only in an expandable "details" area,
  never as the headline. The front maps `errors.<code>` in `common.json` (fallback `errors.unknown`). Rust returns no UI text:
  not for errors, not for "No project", not for tray/dialog titles (the front passes translated titles to the dialog plugin).
- **Parity test** (`src/lib/i18n/i18n.test.ts`, vitest, part of `pnpm test`): flattens every namespace of `en` and `es` and
  asserts identical key sets and identical `{placeholder}` sets per key; also scans `src/**/*.svelte|ts` for `t("...")` literals
  and asserts each exists in `en`. Run in CI on all 3 OSes.
- **No literal-detection lint rule** (too noisy for the benefit); the manual review in Phase 3 (T087) covers stray literals.
- **Language of artifacts**: code, comments, identifiers, commits, specs, README, CHANGELOG, CI and the repo description are
  English (constitution VIII). Spanish exists only in `locales/es/*.json`.

## Worktrees
`git` invoked with `std::process::Command` behind a `GitRunner` trait (mockable). Scan: walk the root to depth N, a repo = a
directory with `.git` (dir, or file ⇒ linked worktree, resolved to the main one with `git rev-parse --git-common-dir`). Per
worktree 3 git calls in a bounded pool (4 threads). Open: `code <path>` and `wt -d <path>` (fallback `pwsh -NoExit
-WorkingDirectory`, then `cmd /K cd /d`) with `opener`/`std::process`; no shell. Remove: `git worktree remove [--force] <path>`
from the main repo; the backend rejects the main worktree.

## Ports
`listeners::get_all()` → `{pid, name, path, socket}`; enriched with `sysinfo` (path, cwd). The `process_info` module (phase 1,
contract) exposes `snapshot() -> Vec<ProcInfo{pid,name,exe,cwd,ram,cpu,start_time,parent}>` and `kill(pid, expected_name)`
(revalidates the name before killing, avoids PID reuse). Limits: elevated/protected processes ⇒ `cwd=None`, `exe=None`; 32-bit
under 64-bit ⇒ `None`. macOS/Linux: best effort.

## JSON Viewer
1. File mapped with `memmap2` (read-only). An own tokenizer over `&[u8]` (no value allocation) walks it once.
2. Index: only **containers** (object/array) in a `Vec<Node>` of ~32 B: `{offset:u64, end:u64(len), kind:u8, child_count:u32, parent:u32, key_range}`.
   Leaf children are not indexed: `json_children` re-scans the container from its offset and returns the `[offset,limit)` range;
   for containers with millions of children every 1024th child is stored as a jump point (sparse index) so pagination is O(1024).
3. Memory: ~32 B per container + 8 B per jump point. 500 MB with 5 M containers ≈ 160 MB; practical limit ~1–2 GB file / ~20 M containers.
4. Progress: event `json://progress {bytes_done, total}` about every 1 %; cancellation with `AtomicBool`; indexing in `spawn_blocking`.
5. Search: background scan of the mmap (memchr for substrings, optional case-insensitivity); computes each hit's path with a stack; sends batches via the `json://search-batch` event (max 500 per batch, configurable total cap 10 000).
6. Format: subtree ≤ 5 MB pretty-printed in memory (reusing the tokenizer, no parse to `Value`); "save formatted" writes in streaming mode with `BufWriter` and depth-based indentation.
7. Path `$.a.b[3]`: rebuilt by walking up through `parent` + key/index.
8. File modified after indexing: `mtime+len` are stored; if they change, error code `json_file_changed` ("file changed, reopen").

## Hidden / Cleanup (reuses contracts)
- **Processes**: `process_info::snapshot()` filtered by the dev name list; age from `start_time`; attribution by cwd via `scope::attribute`; orphan = cwd with the prefix of a worktree path that no longer exists (a `known_worktrees` record is kept in the store, refreshed on every scan, to detect deleted ones).
- **Forgotten worktrees**: `worktrees::list` + size (cancelable walkdir, not following symlinks/junctions: `file_type().is_symlink()` and, on Windows, the `REPARSE_POINT` attribute); age = last commit date; merged = `git merge-base --is-ancestor <branch> <base>` (cheaper and more deterministic than `branch --merged`); no changes = empty status; live processes = snapshot with cwd under the worktree.
- **Folders**: for each worktree/repo, look for allow-listed names (`node_modules`, `target`, `.venv`, `dist`) without descending into them or `.git`. Windows global caches: npm `%LOCALAPPDATA%\npm-cache`, pnpm `%LOCALAPPDATA%\pnpm\store` (or `pnpm store path`), cargo `%USERPROFILE%\.cargo\registry` and `.cargo\git`. Last use = max mtime (folder + first level).
- **Summary**: recoverable RAM = RAM of orphaned processes + dev processes with no port/cwd in a live worktree; recoverable disk = regenerable folders + forgotten worktrees + caches. Computed in Rust.
- **Safe deletion**: `validate_deletable(path)` canonicalizes, requires an allow-listed name or exactly a known global cache, not a symlink, and being under a known worktree/repo or being the cache; otherwise `Error::NotAllowed`.

## Commands, state, events, plugins
See [contracts/commands.md](contracts/commands.md). State: `AppState { config: Mutex<Config>, json: Mutex<Option<JsonSession>>, jobs: Mutex<HashMap<JobId, Arc<AtomicBool>>> }`.
Errors: `enum Error` with `thiserror` + `Serialize` as `{kind, code, params?, detail?}` (see Internationalization). Plugins: `dialog`, `opener`, `store`, `clipboard-manager`.
Capabilities (Tauri 2) in `capabilities/default.json`: `core:default`, `dialog:allow-open`, `dialog:allow-save`, `store:default`,
`clipboard-manager:allow-write-text`, `opener:allow-open-path` limited; no generic `shell`: `code`/`wt` launches happen in Rust.

## Security (gitleaks 8.30.1)
- `.githooks/pre-commit` adapted from `grim-dawn-builds` (POSIX sh, fail closed, exact version, rejects symlink, `--staged --redact`).
- `.gitleaks.toml`: `[extend] useDefault = true`, title `devdeck secret scan`.
- `scripts/setup-gitleaks.mts` (+ `.test.mts`): official pinned asset with SHA-256 (win32/linux x64), installs to `.tools/gitleaks/8.30.1/`, verifies the version. macOS: the reference has no asset ⇒ gitleaks 8.30.1 is required on PATH (`brew`); decision: do not add hashes without verifying them against the release.
- `package.json` (from the scaffold): `"setup:hooks": "git config core.hooksPath .githooks"`, `"setup:gitleaks": "tsx scripts/setup-gitleaks.mts"`, `"test:scripts"`. Scaffold dependency: `package.json`, `tsx` and `vitest`.
- CI: `secret-scan` job (ubuntu): `fetch-depth: 0`, installs gitleaks 8.30.1 with checksum, `gitleaks git --redact --config .gitleaks.toml --log-opts "<base>..<head>"` (PR: base=`pull_request.base.sha`, head=`pull_request.head.sha`; push: `before..sha`; if `before` is all zeros, only commit `sha`).
- `.tools/` and `.gitleaks-report*` in `.gitignore`. `pnpm setup:hooks` is documented in the README and is **not** wired to the `prepare` script (avoids side effects on install).

## CI and release
`ci.yml`: matrix `windows-latest, macos-latest, ubuntu-latest`; steps: checkout, pnpm + node, rust toolchain (rustfmt, clippy), cache (`Swatinem/rust-cache`), Linux deps (`libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf libssl-dev`),
`cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `pnpm install --frozen-lockfile`,
`pnpm lint`, `pnpm check` (svelte-check), `pnpm test` (includes the i18n parity test), `pnpm test:scripts`. Separate `secret-scan` job.
`release.yml`: `on: push: tags: ['v*']`, Windows job with `tauri-apps/tauri-action@v0` (`releaseDraft: true`, bundles `msi` and `nsis`); macOS/Linux optional and disabled.
Icon: `pnpm tauri icon assets/icon.svg` (own SVG in `assets/`). CHANGELOG Keep a Changelog. README (English): manual screenshots in `docs/img/` (both languages shown) and a GIF recorded with ScreenToGif; how to regenerate them is documented.

## Repository description (README tagline and GitHub "About")
> DevDeck — a Tauri 2 + Rust desktop dashboard for developers: track git worktrees, see which worktree owns each listening port, browse multi-hundred-MB JSON files, and reclaim disk and RAM from forgotten worktrees, orphaned dev processes and regenerable build folders.

Used verbatim as the first line of `README.md`, as `description` in `package.json`/`Cargo.toml`/`tauri.conf.json` (shortened where a field limit applies), and as the GitHub repository description (set by the main orchestrator with Paco's permission, T098).

## Risks
| Risk | Mitigation |
|---|---|
| Unreadable `cwd` (elevated) | "unknown"; not attributed; test with a simulated `ProcSource` |
| JSON index slow/memory-hungry | containers-only index, cancellation, documented limit |
| Destructive deletion | allow-list + canonicalize + confirmation + rejection tests |
| Wrong attribution by prefix (`/a/foo` vs `/a/foobar`) | component-wise prefix, test |
| Linux CI runner without deps | explicit apt step |
| Low memory on the dev machine | do not compile many agents in parallel; `CARGO_BUILD_JOBS=2` |
| No gitleaks macOS asset | PATH + exact version; documented |
| Untranslated/missing keys or stray literals | typed keys + parity test (CI) + Phase 3 literal review (T087) |
| Spanish text leaking into code/specs | constitution VIII; reviewer checks; `es` strings only in `locales/es` |

## Tests per module
Core: temporary git repos (`tempfile` + `git init`) for Worktrees/forgotten; `ProcSource`/`PortSource` traits with simulated data for Ports/processes; generated JSON (fixtures in code: nested, huge arrays, Unicode, invalid); attribution and grouping with simple properties; `validate_deletable` with temp folders and symlinks; security scripts with `vitest` (checksum, platform, paths) and manual hook test with a fake secret. Front: `vitest` for stores, `format.ts` (en and es) and i18n parity; `svelte-check`.
