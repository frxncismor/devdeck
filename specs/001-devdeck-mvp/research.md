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
