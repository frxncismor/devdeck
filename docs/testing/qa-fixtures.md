# QA fixtures and app driver (Windows)

Throwaway data and scripts to walk through end-to-end scenarios of the dev and release builds without touching real repositories. The scripts live in `scripts/qa/` (they are developer tools, like the rest of `scripts/`); this page is their manual. They need PowerShell 7, `git`, Node and `pnpm install`.

## Full cycle

Run from the repository root:

```powershell
pwsh -NoProfile -File scripts/qa/qa-fixtures.ps1 -Accion montar
pwsh -NoProfile -File scripts/qa/qa-config.ps1   -Accion aplicar        # DevDeck must be closed
pwsh -NoProfile -File scripts/qa/qa-app.ps1      -Accion arrancar -Build dev   # or -Build release
pnpm exec tsx scripts/qa/cdp.mts texto
# ... scenarios ...
pwsh -NoProfile -File scripts/qa/qa-app.ps1      -Accion parar
pwsh -NoProfile -File scripts/qa/qa-config.ps1   -Accion restaurar      # ALWAYS, even after a failure
pwsh -NoProfile -File scripts/qa/qa-fixtures.ps1 -Accion desmontar
```

Each script also has `-Accion estado`. The release exe is built with `pnpm tauri build --no-bundle` (`target/release/devdeck.exe`); use `-Exe <path>` for an installed copy. The dev build runs `cargo build -p devdeck` (skip with `-SinCompilar`), starts vite on port 1420 and launches `target/debug/devdeck.exe`; it does not use `pnpm tauri dev`, so the environment overrides below do not affect cargo.

## Safety rules

- The **Ports** and **Dev processes** lists are machine-wide: they also show your real processes. Only terminate PIDs listed in `%TEMP%\devdeck-qa\pids.json`.
- Only launch the app with `qa-app.ps1`. It points `LOCALAPPDATA` and `CARGO_HOME` at fake folders, so the npm, pnpm and cargo caches offered for deletion are fake. Launched any other way, Reclaim offers your **real** global caches.
- It also sets `WEBVIEW2_USER_DATA_FOLDER`, so theme and recent files never touch your WebView2 profile.
- `config.json` is backed up to `%TEMP%\devdeck-qa-config-backup\` (outside the QA root) and restored byte for byte, verified by SHA-256. Applying twice never overwrites the backup. Dev and release share the identifier, so the same config serves both.
- `desmontar` deletes only a root that carries the `.devdeck-qa-root` marker and stops only processes whose PID **and** start time match `pids.json`.

## Options

| Script   | Option                             | Meaning                                                     |
| -------- | ---------------------------------- | ----------------------------------------------------------- |
| all      | `-Raiz <dir>`                      | QA root (default `%TEMP%\devdeck-qa`); pass it to all three |
| fixtures | `-JsonGrandeMB 110`                | Size of `json\grande.json`; `0` skips it                    |
| fixtures | `-PuertoBase 47010`                | Uses seven consecutive ports; fails if one is busy          |
| fixtures | `-SinProcesos`                     | No listening processes (and no `huerfano` worktree)         |
| config   | `-Idioma en\|es`, `-DiasOlvido 30` | Language and `forgottenDays`                                |
| config   | `-SinRaiz`                         | No root configured (first-run screen)                       |
| app      | `-PuertoCdp 9222`                  | Set `DEVDECK_CDP_PORT` for `cdp.mts` if you change it       |

## What gets created

Root given to the app: `%TEMP%\devdeck-qa\projects` (scan depth 2). Bare remotes are in `remotes\`. Git runs with its own global config (no signing, hooks or LFS from yours). Mounting takes about 25 s.

### Worktrees (project `webshop`, 13 entries; `api-b`, 3)

| Worktree         | State                              | Expected in the app                                              |
| ---------------- | ---------------------------------- | ---------------------------------------------------------------- |
| `webshop` (main) | pushed                             | Main, In sync, cannot be removed                                 |
| `limpio`         | pushed, clean                      | No uncommitted changes, In sync                                  |
| `sucio`          | 2 modified + 1 untracked           | 3 changed files; removal needs the stronger confirmation         |
| `sin-upstream`   | never pushed                       | No remote branch                                                 |
| `adelantado`     | 2 unpushed commits                 | Ahead 2                                                          |
| `atrasado`       | 2 commits only on the remote       | Behind 2                                                         |
| `divergente`     | 1 and 1                            | Diverged                                                         |
| `olvidado`       | last commit 90 days ago, clean     | Reclaim: No recent commits, 6 MB, Processes running              |
| `olvidado-sucio` | 60 days, 1 uncommitted file        | Reclaim: No recent commits, 1 changed file                       |
| `mergeado`       | merged into `main` with `--no-ff`  | Reclaim: Branch merged                                           |
| `detached`       | detached HEAD                      | Detached HEAD                                                    |
| `borrado`        | folder deleted by hand             | Missing folder                                                   |
| `huerfano`       | junction removed under a live node | Missing folder; its process is flagged Deleted worktree          |
| `api-b/auth`     | recent, never pushed               | Totals and scope selector                                        |
| `api-b/legacy`   | 45 days                            | Reclaim: No recent commits (not forgotten with `-DiasOlvido 60`) |

Removing a **missing** worktree runs `git worktree prune`, which also forgets the other missing one: check the orphan scenario first, or remount.

### Processes and ports (Ports, Reclaim > Dev processes)

| Port      | Process             | Working folder        | Scenario                     |
| --------- | ------------------- | --------------------- | ---------------------------- |
| 47010     | `node.exe`          | `webshop`             | Attributed to the main repo  |
| 47011     | `node.exe`          | `webshop-wt/limpio`   | Attributed to a worktree     |
| 47012     | `vite.exe`          | `webshop-wt/olvidado` | Vite kind; Processes running |
| 47013     | `node.exe`          | `webshop-wt/huerfano` | Orphan (Deleted worktree)    |
| 47014     | `rust-analyzer.exe` | `api-b`               | Second project               |
| 47015     | `node.exe`          | `loose`               | No project                   |
| 47016 UDP | `node.exe`          | `api-b-wt/auth`       | UDP; shown under All and UDP |

`vite.exe` and `rust-analyzer.exe` are renamed copies of `node.exe`. All bind to `127.0.0.1`. While a process lives inside a worktree, Windows cannot delete that folder, so removing `olvidado` before terminating 47012 exercises the error path. Killed processes are not restarted: remount.

### Regenerable folders

Listed: `webshop/node_modules` (8 MB), `webshop/dist` (3 MB, dated 200 days ago), `webshop-wt/olvidado/node_modules` (6 MB), `api-b/target` (12 MB, next to `Cargo.toml`), `api-b/.venv` (2 MB), and the four fake caches under No project. Must **not** be listed: `webshop/docs/target` and `webshop/docs/dist` (no marker file next to them).

### JSON (`%TEMP%\devdeck-qa\json`)

- `pequeno.json`: types, nested key with a space, non-ASCII text.
- `anidado.json`: 400 nesting levels.
- `grande.json`: about 110 MB, written in streaming; the string `aguja-al-final` is at the very end (search to the end).
- `invalido-*.json`: truncado, coma-final, comillas-simples, clave-sin-comillas, basura-final, nan, escape, vacio, tarde (error near the end of a 240 KB file).

## Driving the app: `cdp.mts`

The app is WebView2, so with `--remote-debugging-port` (passed through `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS`) `playwright-core` attaches with `chromium.connectOverCDP`. No browser is downloaded. Checked against both the dev and the release build.

```powershell
pnpm exec tsx scripts/qa/cdp.mts texto [selector]            # visible text (default body; use "main")
pnpm exec tsx scripts/qa/cdp.mts captura out.png             # screenshot of the webview
pnpm exec tsx scripts/qa/cdp.mts clic "role=button[name='Ports']"
pnpm exec tsx scripts/qa/cdp.mts clic "role=tab[name=/Dev processes/]"
pnpm exec tsx scripts/qa/cdp.mts tecla Control+k
pnpm exec tsx scripts/qa/cdp.mts escribir "input:visible" limpio
pnpm exec tsx scripts/qa/cdp.mts esperar "text=47010"
pnpm exec tsx scripts/qa/cdp.mts eval "location.href"
pnpm exec tsx scripts/qa/cdp.mts abrir-json $env:TEMP\devdeck-qa\json\grande.json
pnpm exec tsx scripts/qa/cdp.mts pasos steps.json            # [["clic","..."],["pausa","500"],["texto","main"]]
```

- Selectors are Playwright selectors; the first match is used, so narrow with `:visible` when hidden dialogs also match.
- A failed step exits non-zero (timeout 15 s, `DEVDECK_CDP_TIMEOUT` in ms).
- `abrir-json` reloads the page and replaces the recent files list, because the native file dialog is out of reach of CDP. The tree opens with the root collapsed.
- Not reachable through CDP: native dialogs (folder/file pickers, save), drag and drop from Explorer, and what Open in VS Code / terminal launch. Close any terminal opened inside the QA root before `desmontar`.
