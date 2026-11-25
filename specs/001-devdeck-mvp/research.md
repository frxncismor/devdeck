# Research: DevDeck MVP

Decisions already justified in `plan.md`. This lists what should be verified at implementation time (nothing was compiled during design).

| Question | Provisional decision | How to verify |
|---|---|---|
| `sysinfo::Process::cwd()` on Windows | Works by reading the PEB for same-user, same-bitness processes; `None` otherwise | Manual test with own node/vite and an elevated process |
| Crate `listeners` | Gives PID/name/path per socket on Win/Mac/Linux | Check the version and that it exposes port and protocol; otherwise `netstat2` |
| Folder last use | Max mtime (folder + first level); atime discarded (disabled by default on Windows) | Test with a temporary folder |
| Merged branch | `git merge-base --is-ancestor <branch> <base>`; `branch --merged` discarded (slower) | Test with a temporary repo |
| Junctions on Windows | Treat `REPARSE_POINT` as a link: do not enter, do not count | `#[cfg(windows)]` conditional test |
| Folder size | `walkdir` in `spawn_blocking`, progress every 250 ms, cancel `AtomicBool` | Test with a temporary tree |
| Windows caches | npm `%LOCALAPPDATA%\npm-cache`; pnpm `pnpm store path` (fallback `%LOCALAPPDATA%\pnpm\store`); cargo `%USERPROFILE%\.cargo\{registry,git}` | Check on Paco's machine |
| gitleaks macOS | No verified asset ⇒ PATH with exact version | Review the 8.30.1 release if a hash is wanted |
| JSON memory | ~32 B/container + jumps every 1024 children | Measure with a generated 100–500 MB fixture |
| i18n library | Own ~80-line layer with runes + typed JSON keys instead of `svelte-i18n` | Confirm the recursive key type compiles in `svelte-check` with the Svelte 5 toolchain; if it is too slow on big catalogues, generate `keys.ts` from `en` with a script |
| Byte units in `Intl` | `Intl.NumberFormat` has no KiB unit; use binary scaling with `KB/MB/GB` labels, number part via `Intl` | Unit tests of `formatBytes` for `en` and `es` |

## Resolved decisions (open questions answered by Paco)

| # | Question | Decision |
|---|---|---|
| 1 | gitleaks on macOS | Use `gitleaks` 8.30.1 from the `PATH` (e.g. Homebrew); no downloaded asset or hash for macOS. `setup:gitleaks` prints instructions and exits non-zero if it is missing. |
| 2 | Deleting regenerable folders | Permanent delete (no trash), always behind an explicit confirmation, and only for items that pass the backend allow-list. |
| 3 | "Forgotten" worktree criterion | More than 30 days without a commit, or branch already merged into the base branch; the threshold is configurable. |
| 4 | Base branch | `main` by default, configurable. |
| 5 | Root folders | A single root folder to scan for projects. |
| 6 | Default language | English; no automatic detection of the OS language. |
| 7 | i18n library | Own minimal i18n layer (no `svelte-i18n`). |
| 8 | Name of module 4 | "Reclaim" (UI: en "Reclaim", es "Recuperar"); code identifiers use `reclaim` (`reclaim/`, `commands/reclaim.rs`, `modules/reclaim/`, `locales/{en,es}/reclaim.json`, `reclaim_*` commands). |

GitHub Actions are pinned by tag, not by commit SHA (accepted for the MVP); the CI workflow runs with `permissions: contents: read`.

## UI redesign decisions (T100–T103)

| Topic | Decision | Why / rejected |
|---|---|---|
| Scope | One app-wide scope (as in FR-001 / the scope contract), not "persisted per module" as the handoff suggests | Keeps the spec and the single `scope.svelte.ts` store; per-module scope would change UX and the config contract |
| Theme storage | `localStorage` key `devdeck.theme`, applied in `main.ts` before the config loads | Rust config would need a contract change and would flash the wrong theme; inline `<script>` blocked by the CSP |
| Title bar | Own title bar (`decorations: false`) with the minimum window capabilities | Native decorations cannot follow the dark/light tokens |
| Fonts | Inter + JetBrains Mono bundled via `@fontsource*` (latin woff2) | Google Fonts: network at runtime and CSP changes |
| Icons | `@lucide/svelte` (stroke 1.75) | Hand-written SVGs: more code; the handoff is drawn with Lucide |
| JSON recent files | Last opened paths kept in `localStorage` (`devdeck.json.recent`), flat list | Backend has no history; no contract change |
| Ports | Flat table with an owner chip, not grouped by project | A port has no RAM/disk total; grouping added noise |
| Top bar height | 60 px (canvas and spec) | Handoff token `--topbar-h: 56` contradicts the artboards |
| Author credit | Sidebar footer link "Made by @frxncismor" opened by `tauri-plugin-opener` (`opener:allow-open-url` scoped to the profile URL); hidden when the sidebar is collapsed | `shell` or `window.open` would open it inside the WebView or grant a generic scope; a collapsed-state icon would add noise to a column that is already full |
| Design gaps (left out) | Live counts in sidebar nav, "N repos · scanned X ago" in the root card, version label, Settings entry, RAM/disk per project in the scope menu, palette search of worktrees/ports/files, JSON byte range and clickable breadcrumb, "Go to index" pager, tree reveal of a search hit, Reclaim folder type chips; "Scanned X ago" shown as a time | The backend does not provide the data or the API is sequential; adding it would change contracts (out of scope of a restyle) |
