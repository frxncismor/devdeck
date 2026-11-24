# Quickstart

Requisitos: Rust estable, Node 20+, pnpm, git, VS Code (`code` en PATH), WebView2 (Windows).
```
pnpm install
pnpm setup:hooks        # git config core.hooksPath .githooks
pnpm setup:gitleaks     # descarga gitleaks 8.30.1 a .tools/ (SHA-256 verificado)
pnpm tauri dev          # app en desarrollo
cargo test --workspace  # lógica Rust
pnpm lint && pnpm check && pnpm test
pnpm tauri build        # .msi y .exe en target/release/bundle
```
Release: crear el tag `v0.1.0` y subirlo (lo hace el orquestador principal con permiso de Paco).
Comprobación de seguridad: `git config core.hooksPath` devuelve `.githooks`.
