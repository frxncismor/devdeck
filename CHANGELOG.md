# Changelog

All notable changes to this project are documented in this file.
The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project
adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.1.0] - 2026-11-26

First public release.

### Added

- **Shell:** sidebar with four modules, a Global / project scope selector (grouped totals per project, last scope restored on restart) and a persisted configuration (root folder, scope, base branch, language).
- **Worktrees:** scan a root folder for git repos and list every worktree with branch, clean/dirty state, ahead/behind, and last commit; open in VS Code or a terminal; remove linked worktrees with confirmation (stronger confirmation when dirty; the main worktree is protected).
- **Ports:** list listening TCP ports (optional UDP) with PID, process name and executable path; attribute each process to a worktree or project; search; kill with confirmation; manual and optional automatic refresh.
- **JSON Viewer:** streaming background indexing of multi-hundred-MB files with progress and cancellation; virtualized lazy tree with paginated children; key/value search in batches; copy node path; copy or save formatted subtrees and the whole file; invalid JSON reported with its offset.
- **Reclaim:** recoverable RAM and disk summary; forgotten worktrees; dev processes with orphaned detection; regenerable folders (allow-listed) with background size computation; every action confirmed.
- **Internationalization:** English and Spanish UI with instant switching, locale-aware date, number and size formatting, backend errors translated from stable codes, and a parity test between locales.
- **UI:** design system with dark and light themes, a command palette (Ctrl K), a custom title bar, a collapsible sidebar, and consistent styling across Worktrees, Ports, JSON Viewer and Reclaim.
- **UI:** in-app status glossary: a "?" button in Worktrees, Reclaim and Ports lists every label, what it means and what to do. The README has the same Glossary.
- **UI:** author credit "Made by @frxncismor" in the sidebar footer; the link opens in the system browser through the Tauri opener plugin, scoped to that single URL.
- **CI and release:** GitHub Actions CI (fmt, clippy, tests, lint, svelte-check) on Windows, macOS and Ubuntu; tagging `v*` creates a draft release with MSI and NSIS installers.
- **Security:** gitleaks 8.30.1 pre-commit hook (fail closed) with setup scripts, a gitleaks scan over each PR/push in CI, minimal Tauri capabilities, a strict CSP, and backend path validation.
- **Community:** CONTRIBUTING, Code of Conduct (Contributor Covenant 2.1), SECURITY policy, and issue and pull request templates.
- **Docs:** real screenshots (Worktrees, Ports, Reclaim in English; Worktrees and Reclaim in Spanish) and a demo GIF in the README, captured from the redesigned UI with demo data; design handoff and tokens under `docs/design/`.

### Changed

- **UI:** plain-language status labels with tooltips in Worktrees and Reclaim ("No uncommitted changes", "N changed files", "No remote branch", "No recent commits", "Branch merged"); columns renamed "Changes" and "Vs. remote".
- **UI:** long actions (remove worktree, terminate process, delete folders, rescan) run in the background with per-item progress, a spinner on the affected row or button, per-folder delete progress and a final toast with the result.

### Fixed

<!-- Pre-release fixes found during QA go here, one bullet per fix: "- **Module:** what was wrong and what happens now." -->

- **JSON Viewer:** search results show the correct path, the side panel shows the values of search hits, the tree footer no longer covers the last row, and key/item counts no longer show a raw key.

[Unreleased]: https://github.com/frxncismor/devdeck/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/frxncismor/devdeck/releases/tag/v0.1.0
