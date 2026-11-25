# DevDeck

[![CI](https://github.com/frxncismor/devdeck/actions/workflows/ci.yml/badge.svg)](https://github.com/frxncismor/devdeck/actions/workflows/ci.yml)
[![Secret scan](https://github.com/frxncismor/devdeck/actions/workflows/secret-scan.yml/badge.svg)](https://github.com/frxncismor/devdeck/actions/workflows/secret-scan.yml)
[![Release](https://github.com/frxncismor/devdeck/actions/workflows/release.yml/badge.svg)](https://github.com/frxncismor/devdeck/actions/workflows/release.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

A Tauri 2 + Rust desktop dashboard for developers: track git worktrees, see which worktree owns each listening port, browse multi-hundred-MB JSON files, and reclaim disk and RAM from forgotten worktrees and orphaned dev processes.

![DevDeck demo](docs/img/demo.gif)

## What it does

DevDeck is a single window with four modules in a collapsible sidebar, a custom title bar with a command palette (**Ctrl K**), a **dark / light theme** switch, a **Global / project scope selector** and an **English / Spanish language selector**. Everything is read-only until you confirm an action; nothing runs on its own.

### Worktrees

Point DevDeck at a root folder and it finds every git repo and lists all of its worktrees: branch, clean/dirty state with file count, ahead/behind versus upstream, and the last commit. Open a worktree in VS Code or a terminal, or remove a linked worktree (with confirmation; dirty worktrees need a stronger one).

![Worktrees](docs/img/worktrees-en.png)

### Ports

Lists listening TCP ports (UDP optional) with PID, process name and executable path, and attributes each process to the worktree or project its working directory lives in. Search by port, PID, name, path or project; kill a process after confirming; refresh manually or automatically.

![Ports](docs/img/ports-en.png)

### JSON Viewer

Opens JSON files of several hundred MB without loading them into the webview. The file is indexed in one streaming pass in the background (with progress and cancellation); the tree is virtualized and children are fetched on demand. Search keys and values, copy a node's path (`$.a.b[3]`), copy or save a formatted subtree, or save the whole file formatted. Invalid JSON is reported with its offset.

![JSON Viewer](docs/img/json-viewer-en.png)

### Reclaim

Shows how much RAM and disk you can recover, in the current scope:

- **Forgotten worktrees** - size, last-commit age, merged or unchanged branches, and live processes inside; removable with confirmation.
- **Dev processes** - node, vite, cargo, rust-analyzer, headless browsers, Docker and similar, with RAM, CPU and age. Processes whose working directory is in a deleted worktree are flagged as orphaned.
- **Regenerable folders** - `node_modules`, `target`, `.venv`, `dist` and package-manager caches, with size and last use. The backend rejects any path that is not on the allow-list.

![Reclaim](docs/img/reclaim-en.png)

## Language support

The UI ships in English (default) and Spanish. Switching is instant, persisted, and also changes date, number and size formatting. A test fails if the two locale files differ in keys or placeholders.

<p>
  <img src="docs/img/worktrees-es.png" alt="Worktrees (Spanish)" width="49%">
  <img src="docs/img/reclaim-es.png" alt="Reclaim (Spanish)" width="49%">
</p>

## Tech stack

- **Shell:** [Tauri 2](https://tauri.app/) (Rust), WebView2 on Windows
- **Frontend:** Svelte 5, TypeScript, Vite, Vitest
- **Backend:** Rust workspace, `sysinfo`, `listeners`, `memmap2`, `walkdir`
- **Tooling:** pnpm, ESLint, Prettier, svelte-check, clippy, rustfmt, gitleaks

## Architecture

```mermaid
flowchart LR
  UI["Svelte UI<br/>src/"] -- "invoke / events" --> T["src-tauri<br/>Tauri commands + state"]
  T --> C["devdeck-core<br/>crates/devdeck-core"]
  C --> G["git CLI"]
  C --> S["OS: processes, sockets, files"]
```

- `crates/devdeck-core` holds all logic (worktrees, ports, JSON viewer, reclaim, project/scope attribution) with no Tauri dependency, so it is unit-tested directly. System access sits behind small traits and is simulated in tests.
- `src-tauri` is a thin layer: Tauri commands, app state, a typed error type, and the `store`, `dialog` and `clipboard-manager` plugins.
- `src/` is the Svelte frontend, one folder per module under `src/modules/`, with translations in `src/lib/i18n/locales/{en,es}`.
- The backend never sends UI text. Errors carry a stable `kind` and `code`, and the frontend translates them.
- Large JSON files are indexed once by streaming; the frontend only ever receives paginated children and search batches.

## Install

Download the latest installer from [GitHub Releases](https://github.com/frxncismor/devdeck/releases):

- `.msi` (Windows Installer), or
- `-setup.exe` (NSIS)

The installers are **not code-signed**, so Windows SmartScreen may show an "unknown publisher" warning. Choose **More info -> Run anyway** if you trust the download.

## Platform notes

- **Windows** is the supported platform: installers are built and the terminal action (`wt`, falling back to `pwsh`/`cmd`) targets Windows.
- **macOS / Linux**: CI builds, lints and tests on both, and the Ports module uses a cross-platform socket library, but it is best effort only and not guaranteed. No installers are published for them.

## Development

Requirements: Node.js 22 and pnpm, the Rust stable toolchain (pinned via `rust-toolchain.toml`, with rustfmt and clippy), `git`, and [WebView2](https://developer.microsoft.com/microsoft-edge/webview2/) on Windows. On Linux, install the Tauri system dependencies (see the [Tauri prerequisites](https://tauri.app/start/prerequisites/)).

```sh
pnpm install
pnpm setup:hooks      # use .githooks (gitleaks pre-commit)
pnpm setup:gitleaks   # download the pinned gitleaks 8.30.1 (SHA-256 verified)
pnpm tauri dev        # run the app
```

Checks (the same ones CI runs):

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
pnpm lint
pnpm check
pnpm test
pnpm test:scripts
```

Build the installers with `pnpm tauri build` (bundles: MSI and NSIS).

## CI and release

- **CI** (`.github/workflows/ci.yml`) runs on pull requests and pushes to `main`, on Windows, macOS and Ubuntu: frontend build, `cargo fmt`, `clippy -D warnings`, `cargo test`, `pnpm lint`, `pnpm check`, `pnpm test` and `pnpm test:scripts`.
- **Secret scan** (`.github/workflows/secret-scan.yml`) runs gitleaks over each PR or push commit range.
- **Release** (`.github/workflows/release.yml`): pushing a tag matching `v*` builds on Windows with `tauri-action` and creates a **draft** GitHub release with the `.msi` and NSIS `.exe`.

## Security

- A `pre-commit` hook runs a pinned gitleaks (8.30.1) on staged changes and fails closed if it is missing, has the wrong version, or the tool path is a symlink. The setup script verifies the download's SHA-256.
- CI repeats the scan over the full commit range.
- Tauri capabilities are minimal: only the dialog and clipboard-write permissions the frontend uses; no shell or opener access. A strict CSP is set.
- Destructive actions always require confirmation. The backend validates paths: worktree actions only accept known worktrees, the main worktree cannot be removed, and folder deletion only accepts allow-listed names.

## Screenshots

The images in `docs/img/` are captured from the release build (dark theme) against throwaway demo data, so no real project, path or process shows up. To regenerate them (Windows):

1. Create the demo data: three small git repos with linked worktrees and regenerable folders under `C:\devdeck-demo` (`gen-demo.ps1`), and a large anonymous JSON file (`node gen-json.js <out.json>`, about 130 MB).
2. Start one or two demo servers with their working directory inside a demo worktree, for example `node -e "require('http').createServer().listen(5173)"`, and point the DevDeck root folder at `C:\devdeck-demo` (Global scope, English).
3. Install the release build and run the UI Automation script (`capture-t095.ps1`): it opens DevDeck once, clicks through each module and language, and saves every window with `PrintWindow`.
4. Optimize the PNGs with `magick <in>.png -strip -define png:compression-level=9 <out>.png`.
5. Build `docs/img/demo.gif` with ffmpeg: scale the frames to about 1100 px wide, cross-fade them with `xfade`, and use a two-pass palette (`palettegen` + `paletteuse`) to keep it under 5 MB.

The scripts are kept with the development notes, not in this repository.

## Contributing

Contributions are welcome. Read [CONTRIBUTING.md](CONTRIBUTING.md) and the [Code of Conduct](CODE_OF_CONDUCT.md); report vulnerabilities privately as described in [SECURITY.md](SECURITY.md). To get started, look for [good first issues](https://github.com/frxncismor/devdeck/issues?q=is%3Aopen+label%3A%22good+first+issue%22).

## License

[MIT](LICENSE) - (c) 2026 Francisco J. Moreno
