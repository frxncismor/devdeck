# DevDeck Constitution

## Core Principles

### I. Simple scope, not robust
DevDeck is a personal portfolio tool. The simplest solution that meets the acceptance criteria is preferred:
read-only views plus confirmed actions, no automation, no daemons, no sync. No feature adds "just in case"
abstractions.

### II. Logic in Rust, tested
All logic (git, ports, processes, JSON, cleanup, project attribution) lives in the `devdeck-core` crate, with no
dependency on Tauri. It is covered by unit tests that use no network and no UI (temporary git repos, generated JSON,
a simulated system layer behind traits). A function without a test is not done.

### III. Thin frontend
Svelte 5 + TypeScript only presents data and sends commands. It holds no business rules: scope filtering,
attribution, heavy sorting and parsing all happen in Rust. The webview never loads large files whole.

### IV. Windows first
Windows is the guaranteed platform. macOS and Linux must build and pass CI; the Ports module (and process cwd) is
only guaranteed on Windows and degrades to "unknown" elsewhere.

### V. Green CI required
Nothing is merged without: `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`, and `lint` +
`svelte-check` + frontend tests, on Windows, macOS and Linux.

### VI. Safe destructive actions
Killing processes, removing worktrees and deleting folders require explicit confirmation, operate only on items
validated by the backend (allow-list, never the main worktree) and never follow symlinks/junctions.

### VII. Commits, versions and secrets
Conventional Commits and a CHANGELOG (Keep a Changelog). Tokens, credentials and connection strings are never
written or shown in code, logs, screenshots or the README. Commits are protected by a pinned gitleaks pre-commit hook
and a CI scan.

### VIII. English-only project, bilingual UI
Code, identifiers, comments, commit messages, specs, README, CHANGELOG, CI and the repository description are
written in English. The application UI is bilingual: English by default, Spanish as the second language. Every
user-visible string lives in the `en`/`es` resource files (no literals in components), both languages must keep
identical keys (enforced by a test), dates/numbers/sizes are formatted per language with `Intl`, and the backend
returns stable error codes, never UI text.

## Governance
This constitution takes precedence over local preferences. Amending it requires updating this file and justifying
the change in the PR.

**Version**: 1.1.0 | **Ratified**: 2026-11-23 | **Last Amended**: 2026-11-23
