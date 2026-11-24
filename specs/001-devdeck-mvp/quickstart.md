# Quickstart

Requirements: stable Rust, Node 20+, pnpm, git, VS Code (`code` on PATH), WebView2 (Windows).
```
pnpm install
pnpm setup:hooks        # git config core.hooksPath .githooks
pnpm setup:gitleaks     # downloads gitleaks 8.30.1 into .tools/ (SHA-256 verified)
pnpm tauri dev          # app in development
cargo test --workspace  # Rust logic
pnpm lint && pnpm check && pnpm test   # test includes the en/es i18n parity check
pnpm tauri build        # .msi and .exe in target/release/bundle
```
Release: create the `v0.1.0` tag and push it (done by the main orchestrator with Paco's permission).
Security check: `git config core.hooksPath` returns `.githooks`.
Language: the app starts in English; switch to Spanish with the language selector in the sidebar (stored in the config).
Adding UI text: add the key to `src/lib/i18n/locales/en/<module>.json` **and** `.../es/<module>.json`, then use `t("<namespace>.<key>")`; `pnpm test` fails if they diverge.

## Repository description
README tagline and GitHub "About" text (English):

> DevDeck — a Tauri 2 + Rust desktop dashboard for developers: track git worktrees, see which worktree owns each listening port, browse multi-hundred-MB JSON files, and reclaim disk and RAM from forgotten worktrees, orphaned dev processes and regenerable build folders.
