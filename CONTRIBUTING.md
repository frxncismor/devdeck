# Contributing to DevDeck

Thanks for your interest in DevDeck! Bug reports, fixes, translations and new ideas are all welcome. By participating you agree to follow the [Code of Conduct](CODE_OF_CONDUCT.md).

## Requirements

- **Rust**: the stable toolchain, pinned by `rust-toolchain.toml` (with `rustfmt` and `clippy`). `rustup` installs it automatically.
- **Node.js 22** (the version CI uses) and **pnpm** (the exact version is set in the `packageManager` field of `package.json`).
- **git**.
- **Windows**: Microsoft C++ Build Tools (MSVC) and [WebView2](https://developer.microsoft.com/microsoft-edge/webview2/). Windows is the supported platform.
- **Linux**: the Tauri system dependencies. CI installs `libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf libssl-dev`; see also the [Tauri prerequisites](https://tauri.app/start/prerequisites/).
- **macOS**: see the Tauri prerequisites. macOS and Linux are best effort.

## Setup

```sh
pnpm install
pnpm setup:hooks      # use .githooks (gitleaks pre-commit)
pnpm setup:gitleaks   # download the pinned gitleaks 8.30.1 (SHA-256 verified)
```

The pre-commit hook scans staged changes with gitleaks and **fails closed** if gitleaks is missing. Check it is active with `git config core.hooksPath` (it prints `.githooks`).

Run the app in development:

```sh
pnpm tauri dev
```

## Verify locally

Run the same checks as CI before opening a PR:

```sh
pnpm build                                             # the Tauri crate needs ../dist to exist
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
pnpm lint                                              # eslint + prettier --check
pnpm check                                             # svelte-check
pnpm test                                              # vitest, includes the i18n parity test
pnpm test:scripts
```

Fix formatting with `cargo fmt --all` and `pnpm exec prettier --write <files>`. Code style is also set in `.editorconfig` and `.prettierrc.json`.

## Commits

We use [Conventional Commits](https://www.conventionalcommits.org/): `type(scope): summary`, in English, imperative and specific. Common types are `feat`, `fix`, `docs`, `refactor`, `test`, `chore` and `ci`. Examples from the history:

```text
feat(ui): restyle Worktrees and Ports with the design handoff
fix(reclaim): pass the expected start time when terminating a dev process
fix(ci): compare worktree paths canonically
docs: add JSON Viewer screenshot to README
```

## Branches and pull requests

1. Fork the repository and create a branch from `main` (for example `fix/ports-pid-column`).
2. Keep **one topic per PR**; small PRs are reviewed faster.
3. Add or update tests for behavior changes. Logic lives in `crates/devdeck-core` with no Tauri dependency, so it is unit-tested directly.
4. Add a line under `## [Unreleased]` in [CHANGELOG.md](CHANGELOG.md) for user-visible changes.
5. Open the PR against `main` and fill in the template. **CI must be green** (Windows, macOS and Ubuntu) and the secret scan must pass.
6. Add screenshots for UI changes.

Never commit secrets, tokens or personal paths.

## Internationalization

The UI ships in English and Spanish. Every user-facing string must exist in **both** locales:

- Add the key to `src/lib/i18n/locales/en/<module>.json` **and** `src/lib/i18n/locales/es/<module>.json`, then use `t("<namespace>.<key>")`.
- `pnpm test` includes a parity test that fails if the two locales differ in keys or placeholders.
- The backend never sends UI text: errors carry a stable `kind` and `code`, and the frontend translates them.

## Larger features: Spec-Driven Development

For anything bigger than a small fix, open an issue first to agree on the idea. The project plans larger work with [Spec Kit](https://github.com/github/spec-kit): spec, then plan, then tasks, kept under `specs/<NNN-feature>/` (see `specs/001-devdeck-mvp/` for an example, and `.specify/` for the templates and commands). A short description in the issue or PR is enough for most contributions; maintainers will help you scope it.

## Reporting bugs and proposing features

Use the [issue templates](https://github.com/frxncismor/devdeck/issues/new/choose): **Bug report** or **Feature request**. Please include your DevDeck version, OS and steps to reproduce, and remove secrets and personal paths from logs. Look for [`good first issue`](https://github.com/frxncismor/devdeck/issues?q=is%3Aopen+label%3A%22good+first+issue%22) labels if you want a place to start.

## Security

Do **not** open public issues for vulnerabilities. See [SECURITY.md](SECURITY.md).

## License

DevDeck is released under the [MIT License](LICENSE). By contributing, you agree that your contributions are licensed under the same terms.
