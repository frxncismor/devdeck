# Tasks: DevDeck MVP

Format: `T### [P] [USx] description — path`. `[P]` = parallelizable with other `[P]` tasks of the same phase (different files).
Each agent works in its own worktree and only touches the files of its block. A suffix (`T012a`) marks a task inserted
after the original numbering to avoid renumbering; IDs are stable references.

## Suggested split

| Phase | Block | Agent | Files touched | Depends on |
|---|---|---|---|---|
| 1A | Scaffold + i18n infrastructure (T001–T015, T012a–T012d) | `scaffold` (sequential, single) | `Cargo.toml`, `package.json`, `src-tauri/**`, `crates/devdeck-core/src/{lib,error,scope,project,process_info,fsutil}.rs` and stubs, `src/**` shell, `src/lib/i18n/{index,keys}.ts`, `src/lib/i18n/i18n.svelte.ts`, `src/lib/i18n/i18n.test.ts`, `src/lib/i18n/locales/{en,es}/common.json` (filled) and the other 8 locale files created as `{}`, `src/components/LanguageSelector.svelte`, `src/lib/format.ts`, `.github/workflows/ci.yml` | — |
| 1B | gitleaks security (T016–T022b) | `security` | `.githooks/pre-commit`, `.gitleaks.toml`, `scripts/setup-gitleaks*.mts`, `.gitignore`, `secret-scan` job in `.github/workflows/secret-scan.yml` | T021 needs the 1A `package.json` only to add 3 scripts (after 1A); the rest is independent and can run in parallel with 1A |
| 2A | Worktrees (T030–T040) | `worktrees` | `crates/devdeck-core/src/worktrees/**`, `src-tauri/src/commands/worktrees.rs`, `src/modules/worktrees/**`, `src/lib/i18n/locales/{en,es}/worktrees.json` | 1A |
| 2B | Ports (T041–T048) | `ports` | `crates/devdeck-core/src/ports/**`, `commands/ports.rs`, `src/modules/ports/**`, `src/lib/i18n/locales/{en,es}/ports.json` | 1A (`process_info` contract, `worktrees::list` stub and `scope`) |
| 2C | JSON Viewer (T049–T060a) | `json` | `crates/devdeck-core/src/json_viewer/**`, `commands/json_viewer.rs`, `src/modules/json-viewer/**`, `src/lib/i18n/locales/{en,es}/json-viewer.json` | 1A |
| 2D | Reclaim (T061–T070) | `reclaim` | `crates/devdeck-core/src/reclaim/**`, `commands/reclaim.rs`, `src/modules/reclaim/**`, `src/lib/i18n/locales/{en,es}/reclaim.json` | 1A (contracts); integrates with 2A/2B at the end of the phase |
| 3 | Integration and polish (T080–T087) | `integration` | anything (sequential), including `locales/*/common.json` | 2A–2D |
| 4 | Deliverables (T090–T098) | `deliverables` + `docs` | `.github/workflows/release.yml`, `assets/`, `src-tauri/icons/`, `CHANGELOG.md`, `README.md`, `docs/img/` | 3 |

Conflicts avoided: `commands/mod.rs`, `lib.rs`, `App.svelte`, `Cargo.toml` and `src/lib/i18n/index.ts` are closed by phase 1A
and not edited in phase 2. **i18n rule**: one resource file per module and per language; a phase-2 agent edits only its own
`locales/{en,es}/<module>.json` (adding `en` and `es` keys together, FR-056) and only *reads* `common.json`. If a module needs
a shared string, it adds a module-specific key instead of touching `common.json`; the integration phase may later dedupe.
If a block needs a new dependency in `crates/devdeck-core/Cargo.toml`, only that block's agent adds it in a single line (the
integrator resolves the trivial `[dependencies]` conflict; to avoid it, 1A pre-declares all: `walkdir, memmap2, memchr, listeners,
sysinfo, serde, serde_json, thiserror, tempfile (dev), regex?`).

## Phase 1A — Setup / scaffold + i18n infrastructure (sequential, one agent)
- [x] T001 Cargo workspace with `crates/devdeck-core` and `src-tauri`; pre-declared dependencies — `Cargo.toml`, `crates/devdeck-core/Cargo.toml`, `src-tauri/Cargo.toml`
- [x] T002 Svelte 5 + TS + Vite project with pnpm and scripts `dev/build/lint/check/test` (eslint, prettier, svelte-check, vitest) — `package.json`, `vite.config.ts`, `tsconfig.json`, `svelte.config.js`, `eslint.config.js`
- [x] T003 Integrate Tauri 2 (`tauri.conf.json`, dialog/store/clipboard plugins, minimal capabilities) — `src-tauri/tauri.conf.json`, `src-tauri/capabilities/default.json`, `src-tauri/src/{main,lib}.rs`
- [x] T004 [US5] Error types `thiserror`+`Serialize` as `{kind, code, params?, detail?}` with stable snake-case codes, re-exported (no UI text in Rust) — `crates/devdeck-core/src/error.rs`, `src-tauri/src/error.rs`
- [x] T005 [US2] `project.rs` contract (`Project`, `ProjectId`, `build_projects`) — `crates/devdeck-core/src/project.rs`
- [x] T006 [US2] `scope.rs` contract (`Scope`, `attribute`, `Totals`, `Group`, `Grouped`, `group_by_project`, `Attributable` trait) with tests (component-wise prefix, case-insensitive on Windows, `\\?\`, "No project") — `crates/devdeck-core/src/scope.rs`
- [x] T007 `process_info.rs` contract (`ProcInfo`, `ProcSource` trait, `SysinfoSource`, `snapshot`, `kill` with name revalidation) and tests with a simulated source — `crates/devdeck-core/src/process_info.rs`
- [x] T008 `worktrees::list(repo) -> Vec<Worktree>` contract (signature + types + `NotImplemented` stub) — `crates/devdeck-core/src/worktrees/mod.rs`
- [x] T009 `fsutil.rs`: `is_link_or_reparse(path)`, `dir_size(path, cancel, progress)` not following links, with tests — `crates/devdeck-core/src/fsutil.rs`
- [x] T010 [US5] `AppState`, config with the store plugin (including `language`), commands `get_config/set_config/list_projects` — `src-tauri/src/state.rs`, `src-tauri/src/commands/{mod,config,project}.rs`
- [x] T011 One commands file per module with every command from `contracts/commands.md` as `NotImplemented` stubs and a complete `generate_handler!` — `src-tauri/src/commands/{worktrees,ports,json_viewer,reclaim}.rs`
- [x] T012 [US2] Front: `api.ts`, `types.ts`, persisted scope store `scope.svelte.ts`, `ScopeSelector.svelte`, `GroupedTable.svelte` (renders `common.noProject` for a null project), `ConfirmDialog.svelte`, `VirtualList.svelte`; all their text through `t()` — `src/lib/*`, `src/components/*`
- [x] T012a [US5] i18n core: `src/lib/i18n/{index.ts,keys.ts,i18n.svelte.ts}` — closed index importing the 10 locale files, `MessageKey` type derived from `en`, `locale` rune, `setLocale` (persists via `set_config`, sets `<html lang>`), `t(key, params?)` with `{name}` interpolation, `_one/_other` plurals and fallback to `en` — `src/lib/i18n/*`
- [x] T012b [US5] Locale resources: `locales/{en,es}/common.json` filled (sidebar, scope selector, language selector, confirm dialog, generic buttons, `noProject`, `errors.<code>` for every code from T004, `errors.unknown`) and the other 8 files (`worktrees|ports|json-viewer|reclaim` × `en|es`) created as `{}` — `src/lib/i18n/locales/{en,es}/*.json`
- [x] T012c [US5] `LanguageSelector.svelte` in the shell (English / Español, applied immediately, persisted) and `src/lib/format.ts` locale-aware helpers (`formatBytes`, `formatNumber`, `formatPercent`, `formatDate`, `formatDateTime`, `formatRelativeAge` via `Intl`) with unit tests for `en` and `es` — `src/components/LanguageSelector.svelte`, `src/lib/format.ts`, `src/lib/format.test.ts`
- [x] T012d [US5] Parity test: same keys and same `{placeholders}` in `en` and `es` per namespace, and every `t("...")` literal used in `src/**` exists in `en` — `src/lib/i18n/i18n.test.ts`
- [x] T013 Front: layout with `Sidebar.svelte` (+ scope and language selectors) and 4 empty modules (one `index.svelte` per folder, already imported in `App.svelte`) — `src/App.svelte`, `src/modules/*/index.svelte`
- [x] T014 Base CI: 3-OS matrix with fmt, clippy -D warnings, test, pnpm lint/check/test (test includes i18n parity) — `.github/workflows/ci.yml`
- [x] T015 `.gitignore`, `rust-toolchain.toml`, `.editorconfig`, empty CHANGELOG — `.gitignore`, `rust-toolchain.toml`, `CHANGELOG.md`

## Phase 1B — gitleaks security (own block, before any module code commit)
- [x] T016 [P] [US4] `.githooks/pre-commit` adapted from the reference (8.30.1, `--staged --redact`, fail closed, rejects symlink) with the executable bit (`git update-index --chmod=+x`) — `.githooks/pre-commit`
- [x] T017 [P] [US4] `.gitleaks.toml` (`useDefault = true`, devdeck title) — `.gitleaks.toml`
- [x] T018 [P] [US4] `scripts/setup-gitleaks.mts`: win32/linux x64 asset with SHA-256, install into `.tools/gitleaks/8.30.1/`, version verification — `scripts/setup-gitleaks.mts`
- [x] T019 [P] [US4] Installer test without network (asset, checksum, invalid paths, version) — `scripts/setup-gitleaks.test.mts`
- [x] T020 [P] [US4] `.tools/` and `.gitleaks-report*` in `.gitignore` (line added after T015) — `.gitignore`
- [x] T021 [US4] **Depends on the 1A `package.json` (T002)**: add `setup:hooks`, `setup:gitleaks`, `test:scripts` and `tsx` — `package.json`
- [x] T022 [P] [US4] `secret-scan` workflow (PR and push, `fetch-depth: 0`, commit range, gitleaks 8.30.1 with checksum, `--redact`) — `.github/workflows/secret-scan.yml`
- [x] T022b [US4] On integration: run `pnpm setup:hooks` in the main repo and check `git config core.hooksPath` = `.githooks` and that a staged fake secret blocks the commit — verification (the secret is never committed)

**Checkpoint**: 1A + 1B integrated; only then do the phase-2 blocks start.

## Phase 2A — Worktrees (US1) [P]
- [x] T030 [P] [US1] `GitRunner` trait + `Command` implementation — `crates/devdeck-core/src/worktrees/git.rs`
- [x] T031 [P] [US1] Parsers for `worktree list --porcelain`, `status --porcelain=v2 --branch` (dirty, ahead/behind), `log -1` + tests with sample outputs — `crates/devdeck-core/src/worktrees/git.rs`
- [x] T032 [P] [US1] Root scan (depth, resolution to the main repo) — `crates/devdeck-core/src/worktrees/scan.rs`
- [x] T033 [US1] Real `worktrees::list` + tests with temporary git repos (with worktrees, dirty, no upstream, detached) — `crates/devdeck-core/src/worktrees/mod.rs`
- [x] T034 [US1] `remove` (rejects main, `--force` only if dirty and confirmed) + tests — `crates/devdeck-core/src/worktrees/mod.rs`
- [x] T035 [P] [US1] Open VS Code and terminal (`wt -d` → `pwsh` → `cmd`) without a shell, with a command-builder test — `crates/devdeck-core/src/worktrees/open.rs`
- [x] T036 [US1][US2] Commands `worktrees_scan` (grouped by scope), `worktree_open_*`, `worktree_remove` — `src-tauri/src/commands/worktrees.rs`
- [x] T037 [US1][US5] UI: root-folder picker (dialog) persisted, grouped table, status badges, actions and confirmation; all text via `t()`, ages/dates via `format.ts` — `src/modules/worktrees/index.svelte`, `src/modules/worktrees/*.svelte`
- [x] T038 [US1] Front test (ahead/behind formatting, filters) — `src/modules/worktrees/*.test.ts`
- [x] T039 [US5] Worktrees copy: keys in both languages (`en` and `es` together) — `src/lib/i18n/locales/{en,es}/worktrees.json`

## Phase 2B — Ports (US3) [P]
- [x] T041 [P] [US3] `PortSource` trait + `listeners` implementation; `ListeningPort` model — `crates/devdeck-core/src/ports/listing.rs`
- [x] T042 [US3] Enrich with `process_info` (path, cwd) and attribute with `scope::attribute` (uses the worktrees from `worktrees::list`/`Project`) — `crates/devdeck-core/src/ports/mod.rs`
- [x] T043 [P] [US3] Search/filter (`port, pid, name, path, project`) + tests — `crates/devdeck-core/src/ports/mod.rs`
- [x] T044 [US3] Tests with simulated `PortSource` and `ProcSource` (unknown cwd, several ports per PID) — `crates/devdeck-core/src/ports/mod.rs`
- [x] T045 [US3] Commands `ports_list` and `process_kill` — `src-tauri/src/commands/ports.rs`
- [x] T046 [US3][US5] UI: grouped table, search, kill with confirmation, optional auto refresh; all text via `t()` — `src/modules/ports/index.svelte`
- [x] T047 [US3] Note in README/UI about macOS/Linux behavior (best effort) — `src/modules/ports/index.svelte`
- [x] T048 [US5] Ports copy: keys in both languages — `src/lib/i18n/locales/{en,es}/ports.json`

## Phase 2C — JSON Viewer (US6) [P]
- [x] T049 [P] [US6] Tokenizer over `&[u8]` + errors with offset + tests (Unicode, escapes, numbers, invalid) — `crates/devdeck-core/src/json_viewer/indexer.rs`
- [x] T050 [US6] Container indexer with jumps every 1024 children, progress and cancellation — `crates/devdeck-core/src/json_viewer/indexer.rs`
- [x] T051 [P] [US6] Offset reader with `memmap2`, `children(node, offset, limit)`, `value(node, max)`, changed-file detection (`json_file_changed`) — `crates/devdeck-core/src/json_viewer/reader.rs`
- [x] T052 [P] [US6] Path `$.a.b[3]` with key escaping — `crates/devdeck-core/src/json_viewer/mod.rs`
- [x] T053 [US6] Batched search (keys/values, case, cap) — `crates/devdeck-core/src/json_viewer/search.rs`
- [x] T054 [P] [US6] Subtree pretty-print with cap and streaming formatted save — `crates/devdeck-core/src/json_viewer/format.rs`
- [x] T055 [US6] JSON fixture generator in tests (nested, huge array, single line, duplicate keys) and a memory/time test with a ~50 MB fixture, with a 500 MB variant marked `#[ignore]` — `crates/devdeck-core/src/json_viewer/mod.rs`
- [x] T056 [US6] `json_*` commands with a session in `AppState`, progress and search events — `src-tauri/src/commands/json_viewer.rs`
- [x] T057 [US6][US5] UI: open (dialog), progress bar/cancel; all text via `t()` — `src/modules/json-viewer/index.svelte`
- [x] T058 [US6] UI: virtualized tree with lazy expansion (flat list of visible rows) — `src/modules/json-viewer/Tree.svelte`
- [x] T059 [US6][US5] UI: search with paginated results, copy path, format/copy/save; all text via `t()` — `src/modules/json-viewer/{Search,NodeActions}.svelte`
- [x] T060 [US6] Front tests of tree flattening — `src/modules/json-viewer/tree.test.ts`
- [x] T060a [US5] JSON Viewer copy: keys in both languages — `src/lib/i18n/locales/{en,es}/json-viewer.json`

## Phase 2D — Reclaim (US7) [P]
- [x] T061 [P] [US7] Dev processes: name list, `kind`, age, attribution, orphan flag (`known_worktrees`) + tests with a simulated `ProcSource` — `crates/devdeck-core/src/reclaim/processes.rs`
- [x] T062 [P] [US7] Regenerable folders: allow-list search without descending into them, Windows global caches, last use, sizes with `fsutil::dir_size` — `crates/devdeck-core/src/reclaim/folders.rs`
- [x] T063 [P] [US7] `validate_deletable` + `delete_folder` (canonicalize, allow-list, rejects links) and rejection tests — `crates/devdeck-core/src/reclaim/folders.rs`
- [x] T064 [US7] Forgotten worktrees: `worktrees::list` + size + age + `merge-base --is-ancestor` + no changes + live processes, tests with temporary repos — `crates/devdeck-core/src/reclaim/forgotten.rs`
- [x] T065 [US7] Recoverable summary (RAM/disk) and grouping by project with `scope::group_by_project` + tests — `crates/devdeck-core/src/reclaim/summary.rs`
- [x] T066 [US7] `reclaim_*` commands with cancelable jobs and progress — `src-tauri/src/commands/reclaim.rs`
- [x] T067 [US7][US5] UI: top summary, Worktrees/Processes/Folders tabs, Global/project grouped view with totals; all text via `t()` — `src/modules/reclaim/index.svelte`, `src/modules/reclaim/*.svelte`
- [x] T068 [US7][US5] UI: actions with confirmation (remove worktree, terminate process, delete folder showing size); all text via `t()` — `src/modules/reclaim/*.svelte`
- [x] T069 [US7] Front tests of summary/formatting — `src/modules/reclaim/*.test.ts`
- [x] T070 [US5] Reclaim copy: keys in both languages — `src/lib/i18n/locales/{en,es}/reclaim.json`

## Phase 3 — Integration and polish
- [x] T080 [US2] Check that the 3 scope-aware modules use the store and reload on change; fix divergences — `src/modules/*`
- [x] T081 [US3] Real ports↔worktrees cross-reference with the configured root's `Project` (if it did not fit in 2B) — `crates/devdeck-core/src/ports/mod.rs`
- [x] T082 [US7] Reclaim reuses the real `worktrees::list` and `process_info` from 2A/2B (remove stubs) — `crates/devdeck-core/src/reclaim/*`
- [x] T083 Review errors and empty/loading states in the 4 modules (every error code has an `errors.*` translation) — `src/modules/*`
- [x] T084 Performance pass (SC-001..SC-004) with real data — report
- [x] T085 Security review: minimal capabilities, validated paths, no secrets — `src-tauri/capabilities/default.json`
- [x] T086 CI green on the 3 platforms — `.github/workflows/ci.yml` (PR #1, run 45341091142 on fb14de4)
- [x] T087 [US5] i18n review: no literal UI text left in components, Spanish copy reviewed, dedupe shared keys into `common.json`, switching language at runtime updates every view, dates/numbers/sizes follow the language (SC-008, SC-009) — `src/**`, `src/lib/i18n/locales/**`

## Phase 4 — Deliverables (US8)
- [x] T090 [P] Own SVG icon and `pnpm tauri icon` — `assets/icon.svg`, `src-tauri/icons/*`
- [x] T091 [P] Release workflow with tauri-action (draft, .msi + NSIS .exe) — `.github/workflows/release.yml`
- [x] T092 [P] Keep a Changelog CHANGELOG with `0.1.0` — `CHANGELOG.md`
- [x] T093 [P] Consistent version 0.1.0 and the repository description in `package.json`, `Cargo.toml`, `tauri.conf.json` — various
- [x] T094 English README (starts with the repository description from `plan.md`; what it is, install, usage, development, language support, security `setup:hooks`/`setup:gitleaks`, how to generate screenshots/GIF) — `README.md`
- [ ] T095 Screenshots of the 4 modules (English and Spanish) and a GIF of the flow — `docs/img/*`
- [ ] T096 Test the release on a throwaway tag (run by the main orchestrator with Paco's permission) — verification
- [x] T097 Final `/speckit.analyze` and close-out — `specs/001-devdeck-mvp/`
- [ ] T098 Set the GitHub repository description (text in `plan.md`) — run by the main orchestrator with Paco's permission — verification

## Phase 5 — UI redesign (Claude Design handoff; markup and CSS only, no logic or contract changes)
Visual source of truth: `docs/design/` (see `plan.md`, "UI design system"). Decisions and design gaps: `research.md`.
- [x] T100 Version the handoff, tokens as CSS variables, bundled fonts, Lucide icons, dark/light theme, custom title bar + window capabilities, collapsible sidebar, top bar, scope select, command palette, toasts and shared components (R0) — `docs/design/`, `src/styles/`, `src/app.css`, `src/lib/{theme,toasts,shell}.svelte.ts`, `src/components/*`, `src-tauri/{tauri.conf.json,capabilities/default.json}`
- [x] T101 [P] [US1][US3] Restyle Worktrees and Ports (groups, stat cards, data tables, toasts, protocol toggle, 5 s auto refresh while focused) (R1-A) — `src/modules/{worktrees,ports}/**`, `locales/{en,es}/{worktrees,ports}.json`
- [x] T102 [P] [US6][US7] Restyle JSON Viewer (drop zone, recent files, indexing card, file bar, search counter, tree rows, side panel) and Reclaim (stat cards, read-only notice, tabs, multi-select folders, confirmations naming size, toasts) (R1-B) — `src/modules/{json-viewer,reclaim}/**`, `locales/{en,es}/{json-viewer,reclaim}.json`
- [ ] T103 Redesign verification: lint, svelte-check, vitest, cargo test, `pnpm tauri build`, test screenshots in both themes — verification

## Dependencies
1A → (2A ‖ 2B ‖ 2C ‖ 2D) → 3 → 4 → 5 (T100 → (T101 ‖ T102) → T103). 1B is parallel to 1A except T021 (needs T002) and T020 (needs T015). Within 1A: T012a → T012b → T012c, T012d; T012 and T013 need T012a. Within 2: T033 depends on T030–T032; T042 on T041; T050 on T049; T064 on `worktrees::list` (contract T008); T065 on T061–T064; each module's locale task (T039, T048, T060a, T070) runs together with its UI tasks.

## FR → tasks matrix
| FR | Tasks |
|---|---|
| FR-001 | T013, T012c, T100 |
| FR-002 | T010, T037 |
| FR-003 | T005 |
| FR-004 | T006 |
| FR-005 | T006, T010, T036, T045, T066 |
| FR-006 | T006, T036, T045, T065, T067 |
| FR-007 | T006, T036, T045, T066 |
| FR-008 | T012, T080 |
| FR-009 | T004, T083 |
| FR-010 | T032 |
| FR-011 | T031, T033 |
| FR-012, FR-013, FR-014 | T031, T033 |
| FR-015, FR-016 | T035, T036 |
| FR-017 | T034, T036 |
| FR-018 | T008, T033 |
| FR-020, FR-021 | T041 |
| FR-022 | T043 |
| FR-023 | T007, T045 |
| FR-024 | T007, T042, T081 |
| FR-025 | T046 |
| FR-030, FR-031 | T049, T050, T056 |
| FR-032 | T051, T056 |
| FR-033 | T058 |
| FR-034 | T053, T059 |
| FR-035 | T052 |
| FR-036 | T054, T059 |
| FR-037 | T049 |
| FR-040 | T065, T067 |
| FR-041 | T064 |
| FR-042 | T034, T068 |
| FR-043, FR-044 | T061 |
| FR-045 | T007, T068 |
| FR-046 | T062 |
| FR-047 | T009, T062 |
| FR-048 | T063, T068 |
| FR-049 | T006, T062, T065 |
| FR-050 | T012a, T012c, T013 |
| FR-051 | T010, T012a, T012c |
| FR-052 | T012a, T012b, T037, T039, T046, T048, T057, T059, T060a, T067, T068, T070, T087 |
| FR-053 | T012c, T037, T087 |
| FR-054 | T004, T012b, T012, T083 |
| FR-055 | T012d, T014 |
| FR-056 | T039, T048, T060a, T070 |
| FR-060 | T016 |
| FR-061 | T017 |
| FR-062 | T018, T020, T021 |
| FR-063 | T022 |
| FR-064 | T019 |
| FR-070 | T006, T007, T009, T031–T034, T044, T049–T055, T061–T065 |
| FR-071 | T014, T086 |
| FR-072 | T091, T096 |
| FR-073 | T090, T092, T093, T094, T095, T098 |

## Success criterion → tasks
| SC | Tasks | Status |
|---|---|---|
| SC-001 | T031, T080, T084 | Met: 19 worktrees in 0.6–0.7 s |
| SC-002 | T050, T055, T084 | Met (release build, Windows): 524 MB dense fixture, 18 M containers, indexed in 2.3 s, RSS growth 1.25 GB incl. mmap pages, navigation 156 µs |
| SC-003 | T053, T055, T084 | Met: first 100 search hits over 524 MB in 1 ms |
| SC-004 | T005, T080, T084 | Met |
| SC-005 | T014, T086, T091, T096 | Partial: CI green on the 3 OSes (run 45341091142); test tag pending (T096) |
| SC-006 | T007, T034, T045, T063, T068, T085 | Backend validation tested; front confirmation checked manually |
| SC-007 | T016–T022b | Met |
| SC-008 | T012d, T087 | Met |
| SC-009 | T012c | Met |

## User story → tasks
US1 T030–T039 · US2 T005, T006, T012, T036, T080 · US3 T041–T048, T081 · US4 T016–T022b · US5 T004, T010, T012a–T012d, T039, T048, T060a, T070, T087 · US6 T049–T060a · US7 T061–T070, T082 · US8 T090–T098.
