# Tasks: DevDeck MVP

Formato: `T### [P] [USx] descripción — ruta`. `[P]` = paralelizable con otras `[P]` de la misma fase (archivos distintos).
Cada agente trabaja en su propio worktree y solo toca los archivos de su bloque.

## Reparto sugerido

| Fase | Bloque | Agente | Archivos que toca | Depende de |
|---|---|---|---|---|
| 1A | Scaffold (T001–T015) | `scaffold` (secuencial, único) | `Cargo.toml`, `package.json`, `src-tauri/**`, `crates/devdeck-core/src/{lib,error,scope,project,process_info,fsutil}.rs` y stubs, `src/**` shell, `.github/workflows/ci.yml` | — |
| 1B | Seguridad gitleaks (T016–T022) | `seguridad` | `.githooks/pre-commit`, `.gitleaks.toml`, `scripts/setup-gitleaks*.mts`, `.gitignore`, job `secret-scan` en `.github/workflows/secret-scan.yml` | T016 depende del `package.json` de 1A solo para añadir 2 scripts (T021, tras 1A); el resto es independiente y puede ir en paralelo con 1A |
| 2A | Worktrees (T030–T040) | `worktrees` | `crates/devdeck-core/src/worktrees/**`, `src-tauri/src/commands/worktrees.rs`, `src/modules/worktrees/**` | 1A |
| 2B | Puertos (T041–T048) | `puertos` | `crates/devdeck-core/src/ports/**`, `commands/ports.rs`, `src/modules/ports/**` | 1A (contrato `process_info`, `worktrees::list` stub y `scope`) |
| 2C | Visor JSON (T049–T060) | `json` | `crates/devdeck-core/src/json_viewer/**`, `commands/json_viewer.rs`, `src/modules/json-viewer/**` | 1A |
| 2D | Oculto (T061–T073) | `oculto` | `crates/devdeck-core/src/cleanup/**`, `commands/cleanup.rs`, `src/modules/cleanup/**` | 1A (contratos); integra con 2A/2B al final de fase |
| 3 | Integración y pulido (T080–T086) | `integracion` | cualquiera (secuencial) | 2A–2D |
| 4 | Entregables (T090–T097) | `entregables` + `docs` | `.github/workflows/release.yml`, `assets/`, `src-tauri/icons/`, `CHANGELOG.md`, `README.md`, `docs/img/` | 3 |

Conflictos evitados: `commands/mod.rs`, `lib.rs`, `App.svelte` y `Cargo.toml` los cierra la fase 1A; en fase 2 no se editan. Si un bloque necesita una dependencia nueva en `crates/devdeck-core/Cargo.toml`, la añade **solo** el agente de ese bloque en una línea (el integrador resuelve el conflicto trivial de dependencias en `[dependencies]`; para evitarlo, 1A pre-declara todas: `walkdir, memmap2, memchr, listeners, sysinfo, serde, serde_json, thiserror, tempfile (dev), regex?`).

## Fase 1A — Setup / scaffold (secuencial, un agente)
- [ ] T001 Workspace Cargo con `crates/devdeck-core` y `src-tauri`; dependencias pre-declaradas — `Cargo.toml`, `crates/devdeck-core/Cargo.toml`, `src-tauri/Cargo.toml`
- [ ] T002 Proyecto Svelte 5 + TS + Vite con pnpm y scripts `dev/build/lint/check/test` (eslint, prettier, svelte-check, vitest) — `package.json`, `vite.config.ts`, `tsconfig.json`, `svelte.config.js`, `eslint.config.js`
- [ ] T003 Integrar Tauri 2 (`tauri.conf.json`, plugins dialog/opener/store/clipboard, capabilities mínimas) — `src-tauri/tauri.conf.json`, `src-tauri/capabilities/default.json`, `src-tauri/src/{main,lib}.rs`
- [ ] T004 Tipos de error `thiserror`+`Serialize` en core y re-export — `crates/devdeck-core/src/error.rs`, `src-tauri/src/error.rs`
- [ ] T005 [US2] Contrato `project.rs` (`Project`, `ProjectId`, `build_projects`) — `crates/devdeck-core/src/project.rs`
- [ ] T006 [US2] Contrato `scope.rs` (`Scope`, `attribute`, `Totals`, `Group`, `Grouped`, `group_by_project`, trait `Attributable`) con tests (prefijo por componentes, case-insensitive en Windows, `\\?\`, "Sin proyecto") — `crates/devdeck-core/src/scope.rs`
- [ ] T007 Contrato `process_info.rs` (`ProcInfo`, trait `ProcSource`, `SysinfoSource`, `snapshot`, `kill` con revalidación de nombre) y tests con fuente simulada — `crates/devdeck-core/src/process_info.rs`
- [ ] T008 Contrato `worktrees::list(repo) -> Vec<Worktree>` (firma + tipos + stub `NotImplemented`) — `crates/devdeck-core/src/worktrees/mod.rs`
- [ ] T009 `fsutil.rs`: `is_link_or_reparse(path)`, `dir_size(path, cancel, progress)` sin seguir enlaces, con tests — `crates/devdeck-core/src/fsutil.rs`
- [ ] T010 Estado `AppState`, config con plugin store, comandos `get_config/set_config/list_projects` — `src-tauri/src/state.rs`, `src-tauri/src/commands/{mod,config,project}.rs`
- [ ] T011 Un archivo de comandos por módulo con todos los comandos de `contracts/commands.md` como stubs `NotImplemented` y `generate_handler!` completo — `src-tauri/src/commands/{worktrees,ports,json_viewer,cleanup}.rs`
- [ ] T012 [US2] Front: `api.ts`, `types.ts`, store de ámbito persistido `scope.svelte.ts`, `ScopeSelector.svelte`, `GroupedTable.svelte`, `ConfirmDialog.svelte`, `VirtualList.svelte` — `src/lib/*`, `src/components/*`
- [ ] T013 Front: layout con `Sidebar.svelte` y 4 módulos vacíos (un `index.svelte` por carpeta, ya importados en `App.svelte`) — `src/App.svelte`, `src/modules/*/index.svelte`
- [ ] T014 CI base: matriz 3 SO con fmt, clippy -D warnings, test, pnpm lint/check/test — `.github/workflows/ci.yml`
- [ ] T015 `.gitignore`, `rust-toolchain.toml`, `.editorconfig`, CHANGELOG vacío — `.gitignore`, `rust-toolchain.toml`, `CHANGELOG.md`

## Fase 1B — Seguridad gitleaks (bloque propio, antes de cualquier commit de código de módulos)
- [ ] T016 [P] [US4] `.githooks/pre-commit` adaptado de la referencia (8.30.1, `--staged --redact`, fallo cerrado, rechaza symlink) con bit ejecutable (`git update-index --chmod=+x`) — `.githooks/pre-commit`
- [ ] T017 [P] [US4] `.gitleaks.toml` (`useDefault = true`, título devdeck) — `.gitleaks.toml`
- [ ] T018 [P] [US4] `scripts/setup-gitleaks.mts`: asset win32/linux x64 con SHA-256, instalación en `.tools/gitleaks/8.30.1/`, verificación de versión — `scripts/setup-gitleaks.mts`
- [ ] T019 [P] [US4] Test del instalador sin red (asset, checksum, rutas inválidas, versión) — `scripts/setup-gitleaks.test.mts`
- [ ] T020 [P] [US4] `.tools/` y `.gitleaks-report*` en `.gitignore` (línea añadida tras T015) — `.gitignore`
- [ ] T021 [US4] **Depende del `package.json` de 1A (T002)**: añadir `setup:hooks`, `setup:gitleaks`, `test:scripts` y `tsx` — `package.json`
- [ ] T022 [P] [US4] Workflow `secret-scan` (PR y push, `fetch-depth: 0`, rango de commits, gitleaks 8.30.1 con checksum, `--redact`) — `.github/workflows/secret-scan.yml`
- [ ] T022b [US4] Al integrar: ejecutar `pnpm setup:hooks` en el repo principal y comprobar `git config core.hooksPath` = `.githooks` y que un secreto falso staged bloquea — verificación (sin commitear el secreto)

**Checkpoint**: 1A + 1B integrados; solo entonces arrancan los bloques de fase 2.

## Fase 2A — Worktrees (US1) [P]
- [ ] T030 [P] [US1] Trait `GitRunner` + implementación con `Command` — `crates/devdeck-core/src/worktrees/git.rs`
- [ ] T031 [P] [US1] Parser de `worktree list --porcelain`, `status --porcelain=v2 --branch` (dirty, ahead/behind), `log -1` + tests con salidas de ejemplo — `crates/devdeck-core/src/worktrees/git.rs`
- [ ] T032 [P] [US1] Escaneo de la raíz (profundidad, resolución al repo principal) — `crates/devdeck-core/src/worktrees/scan.rs`
- [ ] T033 [US1] `worktrees::list` real + tests con repos git temporales (con worktrees, sucio, sin upstream, detached) — `crates/devdeck-core/src/worktrees/mod.rs`
- [ ] T034 [US1] `remove` (rechaza principal, `--force` solo si sucio confirmado) + tests — `crates/devdeck-core/src/worktrees/mod.rs`
- [ ] T035 [P] [US1] Abrir VS Code y terminal (`wt -d` → `pwsh` → `cmd`) sin shell, con test del constructor de comandos — `crates/devdeck-core/src/worktrees/open.rs`
- [ ] T036 [US1][US2] Comandos `worktrees_scan` (agrupado por ámbito), `worktree_open_*`, `worktree_remove` — `src-tauri/src/commands/worktrees.rs`
- [ ] T037 [US1] UI: selector de carpeta raíz (dialog) persistida, tabla agrupada, badges de estado, acciones y confirmación — `src/modules/worktrees/index.svelte`, `src/modules/worktrees/*.svelte`
- [ ] T038 [US1] Test front (formato de ahead/behind, filtros) — `src/modules/worktrees/*.test.ts`

## Fase 2B — Puertos (US3) [P]
- [ ] T041 [P] [US3] Trait `PortSource` + implementación `listeners`; modelo `ListeningPort` — `crates/devdeck-core/src/ports/listing.rs`
- [ ] T042 [US3] Enriquecer con `process_info` (ruta, cwd) y atribuir con `scope::attribute` (usa los worktrees de `worktrees::list`/`Project`) — `crates/devdeck-core/src/ports/mod.rs`
- [ ] T043 [P] [US3] Búsqueda/filtro (`port, pid, nombre, ruta, proyecto`) + tests — `crates/devdeck-core/src/ports/mod.rs`
- [ ] T044 [US3] Tests con `PortSource` y `ProcSource` simulados (cwd desconocido, varios puertos por PID) — `crates/devdeck-core/src/ports/mod.rs`
- [ ] T045 [US3] Comandos `ports_list` y `process_kill` — `src-tauri/src/commands/ports.rs`
- [ ] T046 [US3] UI: tabla agrupada, búsqueda, matar con confirmación, refresco auto opcional — `src/modules/ports/index.svelte`
- [ ] T047 [US3] Nota en README/UI de comportamiento en macOS/Linux (mejor esfuerzo) — `src/modules/ports/index.svelte`

## Fase 2C — Visor JSON (US5) [P]
- [ ] T049 [P] [US5] Tokenizador sobre `&[u8]` + errores con offset + tests (Unicode, escapes, números, inválidos) — `crates/devdeck-core/src/json_viewer/indexer.rs`
- [ ] T050 [US5] Indexador de contenedores con saltos cada 1024 hijos, progreso y cancelación — `crates/devdeck-core/src/json_viewer/indexer.rs`
- [ ] T051 [P] [US5] Lector por offset con `memmap2`, `children(node, offset, limit)`, `value(node, max)`, detección de archivo cambiado — `crates/devdeck-core/src/json_viewer/reader.rs`
- [ ] T052 [P] [US5] Ruta `$.a.b[3]` con escape de claves — `crates/devdeck-core/src/json_viewer/mod.rs`
- [ ] T053 [US5] Búsqueda por lotes (claves/valores, case, tope) — `crates/devdeck-core/src/json_viewer/search.rs`
- [ ] T054 [P] [US5] Pretty-print de subárbol con tope y guardado formateado en streaming — `crates/devdeck-core/src/json_viewer/format.rs`
- [ ] T055 [US5] Generador de fixtures JSON en tests (anidado, array enorme, una línea, claves duplicadas) y test de memoria/tiempo con fixture ~50 MB marcado `#[ignore]` para 500 MB — `crates/devdeck-core/src/json_viewer/mod.rs`
- [ ] T056 [US5] Comandos `json_*` con sesión en `AppState`, eventos de progreso y búsqueda — `src-tauri/src/commands/json_viewer.rs`
- [ ] T057 [US5] UI: abrir (dialog), barra de progreso/cancelar — `src/modules/json-viewer/index.svelte`
- [ ] T058 [US5] UI: árbol virtualizado con expansión perezosa (lista plana de filas visibles) — `src/modules/json-viewer/Tree.svelte`
- [ ] T059 [US5] UI: búsqueda con resultados paginados, copiar ruta, formatear/copiar/guardar — `src/modules/json-viewer/{Search,NodeActions}.svelte`
- [ ] T060 [US5] Tests front del aplanado del árbol — `src/modules/json-viewer/tree.test.ts`

## Fase 2D — Oculto / Limpieza (US6) [P]
- [ ] T061 [P] [US6] Procesos de dev: lista de nombres, `kind`, antigüedad, atribución, marca huérfano (`known_worktrees`) + tests con `ProcSource` simulado — `crates/devdeck-core/src/cleanup/processes.rs`
- [ ] T062 [P] [US6] Carpetas regenerables: búsqueda por lista blanca sin entrar en ellas, cachés globales Windows, último uso, tamaños con `fsutil::dir_size` — `crates/devdeck-core/src/cleanup/folders.rs`
- [ ] T063 [P] [US6] `validate_deletable` + `delete_folder` (canonicalize, lista blanca, rechaza enlaces) y tests de rechazo — `crates/devdeck-core/src/cleanup/folders.rs`
- [ ] T064 [US6] Worktrees olvidados: `worktrees::list` + tamaño + antigüedad + `merge-base --is-ancestor` + sin cambios + procesos vivos, tests con repos temporales — `crates/devdeck-core/src/cleanup/forgotten.rs`
- [ ] T065 [US6] Resumen recuperable (RAM/disco) y agrupado por proyecto con `scope::group_by_project` + tests — `crates/devdeck-core/src/cleanup/summary.rs`
- [ ] T066 [US6] Comandos `cleanup_*` con trabajos cancelables y progreso — `src-tauri/src/commands/cleanup.rs`
- [ ] T067 [US6] UI: resumen superior, pestañas Worktrees/Procesos/Carpetas, vista agrupada Global/proyecto con totales — `src/modules/cleanup/index.svelte`, `src/modules/cleanup/*.svelte`
- [ ] T068 [US6] UI: acciones con confirmación (quitar worktree, terminar proceso, borrar carpeta mostrando tamaño) — `src/modules/cleanup/*.svelte`
- [ ] T069 [US6] Tests front del resumen/formato — `src/modules/cleanup/*.test.ts`

## Fase 3 — Integración y pulido
- [ ] T080 [US2] Ver que los 3 módulos con ámbito usan el store y recargan al cambiar; arreglar divergencias — `src/modules/*`
- [ ] T081 [US3] Cruce puertos↔worktrees real con `Project` de la raíz configurada (si no cupo en 2B) — `crates/devdeck-core/src/ports/mod.rs`
- [ ] T082 [US6] Oculto reutiliza `worktrees::list` y `process_info` reales de 2A/2B (quitar stubs) — `crates/devdeck-core/src/cleanup/*`
- [ ] T083 Revisión de errores y estados vacíos/cargando en los 4 módulos — `src/modules/*`
- [ ] T084 Pasada de rendimiento (SC-001..SC-004) con datos reales — informe
- [ ] T085 Revisión de seguridad: capabilities mínimas, rutas validadas, sin secretos — `src-tauri/capabilities/default.json`
- [ ] T086 CI verde en las 3 plataformas — `.github/workflows/ci.yml`

## Fase 4 — Entregables (US7)
- [ ] T090 [P] Icono propio SVG y `pnpm tauri icon` — `assets/icon.svg`, `src-tauri/icons/*`
- [ ] T091 [P] Workflow de release con tauri-action (borrador, .msi + NSIS .exe) — `.github/workflows/release.yml`
- [ ] T092 [P] CHANGELOG Keep a Changelog con `0.1.0` — `CHANGELOG.md`
- [ ] T093 [P] Versión 0.1.0 coherente en `package.json`, `Cargo.toml`, `tauri.conf.json` — varios
- [ ] T094 README (qué es, instalación, uso, desarrollo, seguridad `setup:hooks`/`setup:gitleaks`, cómo generar capturas/GIF) — `README.md`
- [ ] T095 Capturas de los 4 módulos y GIF del flujo — `docs/img/*`
- [ ] T096 Probar el release en un tag de prueba (lo ejecuta el principal con permiso de Paco) — verificación
- [ ] T097 `/speckit.analyze` final y cierre — `specs/001-devdeck-mvp/`

## Dependencias
1A → (2A ‖ 2B ‖ 2C ‖ 2D) → 3 → 4. 1B es paralelo a 1A salvo T021 (necesita T002) y T020 (necesita T015). Dentro de 2: T033 depende de T030–T032; T042 de T041; T050 de T049; T064 de `worktrees::list` (contrato T008); T065 de T061–T064.

## Matriz FR → tareas
| FR | Tareas |
|---|---|
| FR-001 | T013 |
| FR-002 | T010, T037 |
| FR-003 | T005 |
| FR-004 | T006 |
| FR-005 | T006, T010, T036, T045, T066 |
| FR-006 | T006, T036, T045, T065, T067 |
| FR-007 | T006, T036, T045, T066 |
| FR-008 | T012, T080 |
| FR-009 | T004, T083 |
| FR-010 | T032 |
| FR-011 | T031, T033 |
| FR-012, FR-013, FR-014 | T031, T033 |
| FR-015, FR-016 | T035, T036 |
| FR-017 | T034, T036 |
| FR-018 | T008, T033 |
| FR-020, FR-021 | T041 |
| FR-022 | T043 |
| FR-023 | T007, T045 |
| FR-024 | T007, T042, T081 |
| FR-025 | T046 |
| FR-030, FR-031 | T049, T050, T056 |
| FR-032 | T051, T056 |
| FR-033 | T058 |
| FR-034 | T053, T059 |
| FR-035 | T052 |
| FR-036 | T054, T059 |
| FR-037 | T049 |
| FR-040 | T065, T067 |
| FR-041 | T064 |
| FR-042 | T034, T068 |
| FR-043, FR-044 | T061 |
| FR-045 | T007, T068 |
| FR-046 | T062 |
| FR-047 | T009, T062 |
| FR-048 | T063, T068 |
| FR-049 | T006, T062, T065 |
| FR-050 | T006, T007, T009, T031–T034, T044, T049–T055, T061–T065 |
| FR-051 | T014, T086 |
| FR-052 | T091, T096 |
| FR-053 | T090, T092, T094, T095 |
| FR-060 | T016 |
| FR-061 | T017 |
| FR-062 | T018, T020, T021 |
| FR-063 | T022 |
| FR-064 | T019 |
