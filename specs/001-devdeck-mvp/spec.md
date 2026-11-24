# Feature Specification: DevDeck MVP

**Feature Branch**: `001-devdeck-mvp`  |  **Created**: 2026-11-23  |  **Status**: Draft
**Input**: `runs/11231900/peticion.md` (Paco's request + additions: "Reclaim" module, Global/project scope selector, gitleaks security, English-only project with bilingual UI).

DevDeck is a desktop app (Tauri 2 + Rust, Svelte + TS) with a sidebar of 4 modules: **Worktrees**, **Ports**,
**JSON Viewer** and **Reclaim**, plus a cross-cutting **scope selector (Global / project)** and a
**language selector (English / Spanish)**. Guaranteed platform: Windows. Simple scope, not robust.

## Concepts

- **Project**: a main repo + all its worktrees. Identified by the canonical path of the main repo.
- **Scope**: `Global` or `Project(id)`. Filters the Worktrees, Ports and Reclaim views.
- **No project**: the Global group for anything that belongs to no project (global caches, processes without a repo).
- **Language**: `en` (default) or `es`. Controls every user-visible string and the date/number/size formatting.

## User Scenarios & Testing

### US1 — Worktrees of my repos (P1)
As a dev I want to point at a root folder and see every worktree of each repo with its state.
**Independent test**: with a root containing 2 repos (one with 2 worktrees) the list shows 3 rows with correct data.
1. **Given** a configured root, **when** I open Worktrees, **then** I see, per repo, each worktree with branch, state
   (clean / dirty + file count), ahead/behind vs upstream and last commit (short hash, subject, date).
2. **Given** a worktree, **when** I click "VS Code" or "Terminal", **then** it opens at that path.
3. **Given** a non-main worktree, **when** I click "Remove" and confirm, **then** it is removed; the main worktree does not offer the action.
4. **Given** a branch without upstream, **then** ahead/behind is shown as "—".

### US2 — Global / project scope selector (P1)
I want to see everything grouped by project with totals, or focus on a single project.
1. **Given** Global scope, **then** each module groups by project (plus "No project") with RAM and disk totals per group and a grand total.
2. **Given** I pick a project, **then** every module shows only that repo and its worktrees.
3. **Given** I switch module, **then** the scope is kept; on app restart the last one is restored.

### US3 — Listening ports (P1)
1. **Given** processes listening, **then** I see port, protocol, PID, name and executable path.
2. **When** I type in the search box, **then** it filters by port, PID, name, path or project.
3. **When** I click "Kill" and confirm, **then** the process ends and disappears on refresh.
4. **Given** a process whose cwd lives under a worktree, **then** the row shows that worktree/project; if the cwd cannot be read it shows "unknown".

### US4 — Security: nobody commits secrets (P1)
I want no commit containing secrets to enter the repo, locally or in CI.
1. **Given** a fresh clone, **when** I run `pnpm setup:hooks` and `pnpm setup:gitleaks`, **then** `git config core.hooksPath` returns `.githooks` and `.tools/gitleaks/8.30.1/` exists.
2. **Given** a staged change with a test secret, **when** I run `git commit`, **then** the hook blocks it (redacted output).
3. **Given** gitleaks is missing or its version is not 8.30.1, **then** the commit is blocked with instructions (fail closed).
4. **Given** a PR/push, **then** CI repeats gitleaks over the commit range and fails on findings.

### US5 — Bilingual interface (P1)
As a user (or a recruiter reviewing the project) I want the UI in English or Spanish and to switch at any time.
**Independent test**: switching the language changes every visible text, date, number and size format without restart.
1. **Given** a first launch with no saved preference, **then** the UI is in English.
2. **When** I pick "Español" in the language selector, **then** all texts switch to Spanish immediately and the choice is saved in the config; after restart it is restored.
3. **Given** Spanish, **then** dates, numbers and sizes use Spanish conventions (e.g. `1,5 GB`, `23 nov 2026`); in English, `1.5 GB`, `Nov 23, 2026`.
4. **Given** a backend error, **then** the UI shows a translated message chosen from the error's stable code; Rust never supplies UI text.
5. **Given** the repo, **then** a test fails if `en` and `es` resource files differ in keys or placeholders.

### US6 — Large JSON viewer (P2)
1. **When** I open a file of several hundred MB, **then** it is indexed in the background with progress and cancellation, and the UI stays responsive.
2. **When** I expand a node, **then** its children are requested paginated; the tree is virtualized.
3. **When** I search for text, **then** results arrive in batches (key/value) and I can jump to each one.
4. **When** I select a node, **then** I can copy its path (`$.a.b[3]`), copy/export its formatted subtree, or save the formatted file.
5. **Given** invalid JSON, **then** the error is shown with its offset without hanging the app.

### US7 — Reclaim (P2)
1. Summary on top: recoverable RAM and disk (respecting the scope).
2. **Forgotten worktrees**: disk size, last-commit age, branch merged into base / no changes, live processes inside; remove action with confirmation.
3. **Background/orphaned dev processes**: RAM, CPU, age, project/worktree; those whose cwd is in an already-deleted worktree are flagged "orphaned"; terminate action with confirmation.
4. **Regenerable folders** (node_modules, target, .venv, dist, npm/pnpm/cargo caches): size and last use; delete with confirmation.
5. Read-only plus confirmed actions; nothing runs by itself.

### US8 — Professional deliverables (P3)
Green CI on 3 OSes, automatic release on `v*` tag with .msi and .exe, README with screenshots and GIF, custom icon, CHANGELOG, English repository description.

## Requirements

### Shell and scope
- **FR-001**: Sidebar with 4 modules (Worktrees, Ports, JSON Viewer, Reclaim; Spanish label of the last one: "Recuperar"), an always-visible scope selector and an always-visible language selector.
- **FR-002**: Persisted configuration (root folder, last scope, branch base, language) in the app store.
- **FR-003**: `Project` model (id, name, main repo, worktrees) built from the root scan.
- **FR-004**: Attribution function `path → project` by longest path prefix, normalized, case-insensitive on Windows; no match ⇒ "No project".
- **FR-005**: Worktrees, Ports and Reclaim commands accept a scope and return already-filtered data.
- **FR-006**: In Global the response is grouped by project with RAM and disk totals per group and a grand total; "No project" is just another group.
- **FR-007**: In project scope only items attributed to that project are returned.
- **FR-008**: The selected scope is a global frontend store (persisted); changing it reloads the visible module.
- **FR-009**: Errors are serializable as `{kind, code, params?, detail?}` and shown in the UI without closing the app (translation per FR-054).

### Worktrees
- **FR-010**: Scan the root folder (configurable depth, default 2) detecting git repos.
- **FR-011**: Per repo, list worktrees with `git worktree list --porcelain`: path, branch (or detached), main yes/no.
- **FR-012**: Per worktree: number of modified files and clean/dirty state.
- **FR-013**: Per worktree: ahead/behind vs upstream.
- **FR-014**: Per worktree: last commit (short hash, subject, date).
- **FR-015**: Open in VS Code action (`code`).
- **FR-016**: Open terminal action (Windows Terminal `wt -d`, fallback `pwsh`/`cmd`).
- **FR-017**: Remove worktree action with confirmation, rejected for the main worktree; if dirty it requires reinforced confirmation (`--force` only then).
- **FR-018**: Listing worktrees is a shared contract (`worktrees::list`) reused by Ports and Reclaim.

### Ports
- **FR-020**: List listening TCP ports (UDP optional, off by default) with PID.
- **FR-021**: Per port: process name and executable path.
- **FR-022**: Search by port, PID, name, path, project.
- **FR-023**: Kill process with confirmation; clear error if permission is missing.
- **FR-024**: Obtain the process cwd (`process_info`) and attribute it to a worktree/project; "unknown" if unreadable.
- **FR-025**: Manual refresh and optional automatic refresh every N s.

### JSON Viewer
- **FR-030**: Open a JSON file of hundreds of MB without loading it into the webview or entirely into memory as a tree.
- **FR-031**: Single-pass streaming indexing with progress (events) and cancellation.
- **FR-032**: Paginated `json_children(node, offset, limit)`; values read from the file on demand.
- **FR-033**: Virtualized tree with lazy expansion.
- **FR-034**: Search in keys and/or values in the background, results in batches, configurable limit.
- **FR-035**: Copy node path in `$.a.b[3]` format (non-identifier keys: `$["a b"]`).
- **FR-036**: Format: pretty-print the subtree (with size cap) to copy/save, and save the whole formatted file in streaming mode.
- **FR-037**: Invalid JSON ⇒ error with position; no panic.

### Reclaim
- **FR-040**: Summary with recoverable RAM and disk (in scope).
- **FR-041**: Forgotten worktrees: size, last-commit age, branch merged into base or no changes, live processes inside; "forgotten" criterion configurable (default > 30 days or merged).
- **FR-042**: Remove a forgotten worktree with confirmation (reuses FR-017).
- **FR-043**: Dev processes (name list: node, esbuild, vite, cargo, rustc, rust-analyzer, headless chrome/chromium, playwright, docker, vmmem/wsl…) with RAM, CPU, age and project/worktree.
- **FR-044**: Flag "orphaned" the process whose cwd is in a worktree that no longer exists.
- **FR-045**: Terminate process with confirmation (reuses FR-023).
- **FR-046**: Regenerable folders (allow-list: node_modules, target, .venv, dist, npm/pnpm/cargo cache) with size and last use.
- **FR-047**: Size computation in the background with progress and cancellation, without following symlinks/junctions.
- **FR-048**: Delete folder with confirmation; the backend rejects any path that is not an allow-listed name or known cache.
- **FR-049**: Global caches and processes without a repo appear under "No project".

### Internationalization (i18n)
- **FR-050**: The UI ships in English (default) and Spanish. A language selector is always visible (FR-001) and applies immediately without restart.
- **FR-051**: The chosen language is persisted in the config (`language: "en" | "es"`). Default `en`; the system language is **not** auto-detected (predictable, simpler; recorded as a decision in the plan).
- **FR-052**: Every user-visible string lives in resource files `src/lib/i18n/locales/{en,es}/<module>.json` (one file per module and language); components contain no literal UI text and call the typed translator `t(key, params?)`.
- **FR-053**: Dates, numbers, sizes, percentages and relative times are formatted with `Intl` according to the active language (shared helpers in `src/lib/format.ts`).
- **FR-054**: The backend never returns UI text: errors carry a stable `kind` + `code` (+ optional `params`), and the frontend maps `errors.<code>` to the translated message. Group labels such as "No project" are rendered by the frontend from a null project.
- **FR-055**: A front test fails when `en` and `es` differ in keys or in `{placeholders}` per namespace, and when a key used via `t()` in code is missing from `en`.
- **FR-056**: Each module block adds its own keys to its own resource files (en and es together) in the same change that adds its UI.

### Security
- **FR-060**: `.githooks/pre-commit` runs gitleaks **8.30.1** on staged content with `--redact`; it blocks (fail closed) if gitleaks is missing, is not the exact version, or `.tools/...` is a symlink.
- **FR-061**: `.gitleaks.toml` with the default ruleset (`useDefault = true`), no exceptions unless justified in the SDD.
- **FR-062**: Scripts `setup:hooks` (`git config core.hooksPath .githooks`) and `setup:gitleaks` (downloads the pinned binary into `.tools/gitleaks/8.30.1/`, verifies SHA-256, checks version; Windows and Linux x64; on macOS gitleaks 8.30.1 from PATH is used). `.tools/` in `.gitignore`.
- **FR-063**: CI: job running gitleaks 8.30.1 over the PR (and push) commit range, with full history (`fetch-depth: 0`), `--redact`.
- **FR-064**: Installer test (asset selection, checksum, invalid paths) without network.

### Deliverables
- **FR-070**: Rust unit tests for each module's logic.
- **FR-071**: GitHub Actions CI (fmt, clippy -D warnings, test, front lint/svelte-check/tests) on windows/macos/ubuntu.
- **FR-072**: Release with tauri-action on `v*` tag, draft with .msi and NSIS .exe.
- **FR-073**: README (English) with screenshots and GIF, custom icon, Keep a Changelog CHANGELOG, and the English repository description (see quickstart).

## Success Criteria
- **SC-001**: List of 20 worktrees in < 3 s on Windows (git CLI with bounded parallelism).
- **SC-002**: 500 MB JSON indexed without exceeding ~1.5 GB RAM and UI with no blocks > 100 ms; opening/expanding a node < 200 ms after indexing.
- **SC-003**: Search over 500 MB delivers the first batch in < 2 s.
- **SC-004**: Scope change refreshes the module in < 1 s (sizes arrive via progress).
- **SC-005**: CI green on the 3 platforms; a `v*` tag produces a draft with installers.
- **SC-006**: No destructive action runs without confirmation (backend contract test).
- **SC-007**: After integration, in devdeck `git config core.hooksPath` returns `.githooks`; a staged test secret blocks the commit and the CI job fails on a commit with that secret.
- **SC-008**: The i18n parity test passes: `en` and `es` have exactly the same keys and placeholders in every namespace; switching language changes all visible text with no restart and no leftover literals (manual check in Phase 3).
- **SC-009**: Date/number/size formatting follows the active language (unit tests of `format.ts` for `en` and `es`).

## Edge Cases
Bare repo; prunable worktree/non-existent path; detached branch; no upstream; non-existent root; submodules; elevated
process (unreadable cwd/path); PID reused between listing and kill (revalidate name+PID); single-line giant JSON,
Unicode, duplicate keys, huge numbers; file modified after indexing; symlinks/junctions in node_modules; permission
denied while measuring size; same process with several ports; paths with different casing on Windows; corrupted
downloaded gitleaks (checksum fails); missing translation key at runtime (falls back to English, never shows the raw key blank); unknown saved language value (falls back to `en`).

## Out of Scope
Editing JSON; cloning/creating worktrees; commits/push from the app; automatic or scheduled cleanup; auto-updater;
telemetry; Ports guarantee on macOS/Linux; code signing; periodic full-history scanning; languages beyond en/es;
automatic system-language detection; right-to-left layouts.

## Assumptions
- Branch base for "merged" = `main` (configurable; fallback `master`/remote HEAD).
- Folder deletion is **permanent** (regenerable), with confirmation showing the size; no recycle bin (avoids the `trash` crate and is faster with thousands of files).
- "Last use" = max mtime of the folder and its first level (cheap); not atime (disabled on Windows).
- A single repo root; multiple roots are out of scope.
- Filtering/grouping/totals in Rust; the frontend only stores the scope and the language.
- Package manager pnpm (same as the `grim-dawn-builds` reference); security scripts in TypeScript with `tsx`.
- Process names, paths, git output, branch names and commit subjects are shown as-is (data, not translated).
