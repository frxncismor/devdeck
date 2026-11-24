# Implementation Plan: DevDeck MVP

**Spec**: [spec.md](spec.md) | **Constitución**: `.specify/memory/constitution.md`
Detalle de decisiones: [research.md](research.md) · modelos: [data-model.md](data-model.md) · comandos: [contracts/commands.md](contracts/commands.md) · arranque: [quickstart.md](quickstart.md)

## Resumen
App Tauri 2 con lógica en una crate de librería `devdeck-core` (sin Tauri) y una crate fina `src-tauri` que solo expone
comandos. Front Svelte 5 + TS + Vite (sin SvelteKit) con store de ámbito. Git por CLI, puertos con `listeners`/`sysinfo`,
JSON con índice en streaming + lectura por offset.

## Contexto técnico
Rust estable (MSRV = la de Tauri 2), Tauri 2, Svelte 5 + TS + Vite 6, pnpm. Almacenamiento: plugin `store` (config).
Tests: `cargo test` (core), `vitest` (front). Destino: Windows 10/11 (garantizado), macOS/Linux compilan.

## Estructura
```
Cargo.toml                    # workspace: crates/devdeck-core, src-tauri
crates/devdeck-core/src/
  lib.rs  error.rs  scope.rs  project.rs  process_info.rs  fsutil.rs
  worktrees/{mod,git,scan}.rs
  ports/{mod,listing}.rs
  json_viewer/{mod,indexer,reader,search,format}.rs
  cleanup/{mod,sizes,processes,folders,forgotten,summary}.rs
src-tauri/src/
  main.rs  lib.rs            # run(); registro de comandos
  error.rs  state.rs
  commands/{mod,config,project,worktrees,ports,json_viewer,cleanup}.rs
src-tauri/{tauri.conf.json,capabilities/default.json,icons/}
src/                          # Svelte
  main.ts  App.svelte  lib/{api.ts,scope.svelte.ts,types.ts,format.ts}
  modules/{worktrees,ports,json-viewer,cleanup}/  # una carpeta por módulo
  components/{Sidebar,ScopeSelector,ConfirmDialog,VirtualList,GroupedTable}.svelte
.githooks/pre-commit  .gitleaks.toml  scripts/{setup-gitleaks.mts,setup-gitleaks.test.mts}
.github/workflows/{ci.yml,release.yml}
```
**Puntos de extensión sin conflicto**: cada módulo tiene su archivo en `commands/` y su carpeta en `src/modules/`. La
fase 1 deja `commands/mod.rs` con un macro/lista `generate_handler![ ... ]` ya completa con los nombres de todos los
comandos como stubs (`NotImplemented`), de modo que los agentes de fase 2 solo reemplazan el cuerpo en su archivo y no
tocan el registro. Igual en front: `App.svelte` ya importa los 4 módulos; cada `index.svelte` es de su dueño.

## Decisiones (justificación y alternativa descartada)
| Tema | Decisión | Descartada |
|---|---|---|
| Workspace | `devdeck-core` + `src-tauri` | Todo en `src-tauri`: tests lentos con Tauri y mezcla UI/lógica |
| Front | Svelte 5 + Vite sin SvelteKit | SvelteKit adapter-static: routing/SSR innecesario; rutas = estado de módulo |
| Paquetes | pnpm | npm: la referencia (grim-dawn-builds) y los scripts de seguridad ya usan pnpm |
| Git | CLI `git` (`worktree list --porcelain`, `status --porcelain=v2 --branch`, `log -1`) | `git2`: soporte de worktrees limitado, compila libgit2 (lento/pesado); `gix` aún incompleto para status/worktree remove |
| Puertos | crate `listeners` (PID por puerto, Win/Mac/Linux) + `sysinfo` | `netstat2`: menos mantenido; `GetExtendedTcpTable` a mano con `windows`: más código y `unsafe` |
| Procesos | `sysinfo` (nombre, exe, cwd, memoria, CPU, start_time, kill) | WMI: lento y solo Windows |
| cwd en Windows | `Process::cwd()` (lee el PEB); si `None` ⇒ "desconocido" | Inyección/dbghelp: robusto pero fuera de alcance |
| Visor JSON | índice propio en streaming + `memmap2` | Cargar con serde_json::Value: 500 MB ⇒ varios GB |
| Árbol virtual | componente propio (lista plana de filas visibles + ventana) | Librería: dependencia por 80 líneas de código |
| Tamaños | `jwalk` no (rayon extra); `walkdir` en hilo bloqueante con `AtomicBool` de cancelación | jwalk: más rápido pero más complejidad |
| Borrado | `std::fs::remove_dir_all` tras validar lista blanca; sin papelera | `trash`: lento con miles de ficheros, comportamiento por SO |
| Ámbito | Filtrado/agrupado/totales en Rust (`scope.rs`) | Filtrar en front: duplica lógica, no testeable (constitución III) |
| Config | `tauri-plugin-store` | Archivo propio JSON: reinventa |

## Ámbito Global / proyecto (contrato compartido, fase 1)
- `project.rs`: `Project {id, name, main_path, worktree_paths}`; `build_projects(root)` usa `worktrees::list`.
- `scope.rs`: `enum Scope { Global, Project(ProjectId) }`; `attribute(path, &[Project]) -> Option<ProjectId>`
  (normaliza separadores, quita `\\?\`, minúsculas en Windows, prefijo por componentes, gana el más largo);
  `Grouped<T> { groups: Vec<Group<T>>, total: Totals }`, `Group { project: Option<ProjectRef>, items, totals }`,
  `Totals { ram_bytes, disk_bytes, count }`; `group_by_project(items, scope, projects)` genérico sobre un trait `Attributable`
  (`fn path(&self)->Option<&Path>`, `ram()`, `disk()`).
- Front: `scope.svelte.ts` guarda el ámbito (persistido en config) y lo pasa a cada `api.*`; `GroupedTable.svelte` renderiza grupos.
- Cada bloque de fase 2 implementa `Attributable` para su tipo y llama `group_by_project`.

## Worktrees
`git` invocado con `std::process::Command` en un trait `GitRunner` (mockeable). Escaneo: recorrer la raíz hasta
profundidad N, un repo = directorio con `.git` (dir o archivo ⇒ worktree enlazado, se resuelve al principal con
`git rev-parse --git-common-dir`). Por worktree 3 llamadas git en un pool acotado (4 hilos). Abrir: `code <ruta>` y
`wt -d <ruta>` (fallback `pwsh -NoExit -WorkingDirectory`, luego `cmd /K cd /d`) con `opener`/`std::process`; sin shell.
Quitar: `git worktree remove [--force] <ruta>` desde el repo principal; el backend rechaza el principal.

## Puertos
`listeners::get_all()` → `{pid, name, path, socket}`; se enriquece con `sysinfo` (ruta, cwd). El módulo `process_info`
(fase 1, contrato) expone `snapshot() -> Vec<ProcInfo{pid,name,exe,cwd,ram,cpu,start_time,parent}>` y `kill(pid, expected_name)`
(revalida nombre antes de matar, evita reutilización de PID). Límites: procesos elevados/protegidos ⇒ `cwd=None`,
`exe=None`; 32 bits bajo 64 ⇒ `None`. macOS/Linux: mejor esfuerzo.

## Visor JSON
1. Archivo mapeado con `memmap2` (solo lectura). Un tokenizador propio sobre `&[u8]` (sin asignar valores) recorre una vez.
2. Índice: solo **contenedores** (objeto/array) en un `Vec<Node>` de ~32 B: `{offset:u64, end:u64(len), kind:u8, child_count:u32, parent:u32, key_range}`.
   Los hijos hoja no se indexan: `json_children` re-escanea el contenedor desde su offset y devuelve el rango `[offset,limit)`;
   para contenedores con millones de hijos se guarda cada 1024.º hijo como punto de salto (índice disperso) para que la paginación sea O(1024).
3. Memoria: ~32 B por contenedor + 8 B por punto de salto. 500 MB con 5 M contenedores ≈ 160 MB; límite práctico ~1–2 GB de archivo / ~20 M contenedores.
4. Progreso: evento `json://progress {bytes_done, total}` cada ~1 %; cancelación con `AtomicBool`; indexación en `spawn_blocking`.
5. Búsqueda: escaneo en segundo plano del mmap (memchr para subcadenas, case-insensitive opcional); calcula la ruta de cada acierto con una pila; envía lotes por evento `json://search-batch` (máx. 500 por lote, tope total configurable 10 000).
6. Formatear: subárbol ≤ 5 MB a pretty-print en memoria (reutilizando el tokenizador, sin parsear a `Value`); "guardar formateado" escribe en streaming con `BufWriter` y sangría por profundidad.
7. Ruta `$.a.b[3]`: se reconstruye subiendo por `parent` + clave/índice.
8. Archivo modificado tras indexar: se guarda `mtime+len`; si cambian, error "archivo cambiado, reabrir".

## Oculto / Limpieza (reutiliza contratos)
- **Procesos**: `process_info::snapshot()` filtrado por lista de nombres de dev; antigüedad por `start_time`; atribución por cwd vía `scope::attribute`; huérfano = cwd con prefijo de una ruta de worktree que ya no existe (se conserva un registro `known_worktrees` en el store, refrescado en cada escaneo, para detectar los borrados).
- **Worktrees olvidados**: `worktrees::list` + tamaño (walkdir cancelable, sin seguir symlinks/junctions: `file_type().is_symlink()` y, en Windows, atributo `REPARSE_POINT`); antigüedad = fecha del último commit; integrada = `git merge-base --is-ancestor <rama> <base>` (más barato y determinista que `branch --merged`); sin cambios = status vacío; procesos vivos = snapshot con cwd bajo el worktree.
- **Carpetas**: por cada worktree/repo, buscar nombres de la lista blanca (`node_modules`, `target`, `.venv`, `dist`) sin entrar en ellas ni en `.git`. Cachés globales Windows: npm `%LOCALAPPDATA%\npm-cache`, pnpm `%LOCALAPPDATA%\pnpm\store` (o `pnpm store path`), cargo `%USERPROFILE%\.cargo\registry` y `.cargo\git`. Último uso = mtime máx. (carpeta + primer nivel).
- **Resumen**: RAM recuperable = suma RAM de procesos huérfanos + de dev sin puerto/cwd en worktree vivo; disco recuperable = carpetas regenerables + worktrees olvidados + cachés. Se calcula en Rust.
- **Borrado seguro**: `validate_deletable(path)` canonicaliza, exige nombre en lista blanca o ser exactamente una caché global conocida, no ser symlink, y estar bajo un worktree/repo conocido o ser la caché; si no, `Error::NotAllowed`.

## Comandos, estado, eventos, plugins
Ver [contracts/commands.md](contracts/commands.md). Estado: `AppState { config: Mutex<Config>, json: Mutex<Option<JsonSession>>, jobs: Mutex<HashMap<JobId, Arc<AtomicBool>>> }`.
Errores: `enum Error` con `thiserror` + `Serialize` como `{kind, message}`. Plugins: `dialog`, `opener`, `store`, `clipboard-manager`.
Capabilities (Tauri 2) en `capabilities/default.json`: `core:default`, `dialog:allow-open`, `dialog:allow-save`, `store:default`,
`clipboard-manager:allow-write-text`, `opener:allow-open-path` limitado; sin `shell` genérico: los lanzamientos de `code`/`wt` van en Rust.

## Seguridad (gitleaks 8.30.1)
- `.githooks/pre-commit` adaptado de `grim-dawn-builds` (POSIX sh, fallo cerrado, versión exacta, rechaza symlink, `--staged --redact`).
- `.gitleaks.toml`: `[extend] useDefault = true`, título `devdeck secret scan`.
- `scripts/setup-gitleaks.mts` (+ `.test.mts`): asset fijo oficial con SHA-256 (win32/linux x64), instala en `.tools/gitleaks/8.30.1/`, verifica versión. macOS: sin asset en la referencia ⇒ se exige gitleaks 8.30.1 en PATH (`brew`); decisión: no añadir hashes sin verificarlos contra la release.
- `package.json` (del scaffold): `"setup:hooks": "git config core.hooksPath .githooks"`, `"setup:gitleaks": "tsx scripts/setup-gitleaks.mts"`, `"test:scripts"`. Dependencia del scaffold: `package.json`, `tsx` y `vitest`.
- CI: job `secret-scan` (ubuntu): `fetch-depth: 0`, instala gitleaks 8.30.1 con checksum, `gitleaks git --redact --config .gitleaks.toml --log-opts "<base>..<head>"` (PR: base=`pull_request.base.sha`, head=`pull_request.head.sha`; push: `before..sha`; si `before` es todo ceros, solo el commit `sha`).
- `.tools/` y `.gitleaks-report*` en `.gitignore`. `pnpm setup:hooks` se documenta en README y en `scripts` `prepare` no (evitar efectos al instalar).

## CI y release
`ci.yml`: matriz `windows-latest, macos-latest, ubuntu-latest`; pasos: checkout, pnpm + node, rust toolchain (rustfmt, clippy), caché (`Swatinem/rust-cache`), deps Linux (`libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf libssl-dev`),
`cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `pnpm install --frozen-lockfile`,
`pnpm lint`, `pnpm check` (svelte-check), `pnpm test`, `pnpm test:scripts`. Job aparte `secret-scan`.
`release.yml`: `on: push: tags: ['v*']`, job windows con `tauri-apps/tauri-action@v0` (`releaseDraft: true`, bundles `msi` y `nsis`); macOS/Linux opcional desactivado.
Icono: `pnpm tauri icon assets/icon.svg` (SVG propio en `assets/`). CHANGELOG Keep a Changelog. README: capturas manuales en `docs/img/` y GIF grabado con ScreenToGif; se documenta cómo regenerarlos.

## Riesgos
| Riesgo | Mitigación |
|---|---|
| `cwd` ilegible (elevados) | "desconocido"; no se atribuye; test con `ProcSource` simulado |
| Índice JSON tarda/gasta RAM | índice solo de contenedores, cancelación, límite documentado |
| Borrado destructivo | lista blanca + canonicalize + confirmación + tests de rechazo |
| Atribución errónea por prefijo (`/a/foo` vs `/a/foobar`) | prefijo por componentes, test |
| Runner CI Linux sin deps | paso apt explícito |
| Memoria baja de la máquina de desarrollo | no compilar en paralelo muchos agentes; `CARGO_BUILD_JOBS=2` |
| gitleaks sin asset macOS | PATH + versión exacta; documentado |

## Pruebas por módulo
Core: repos git temporales (`tempfile` + `git init`) para Worktrees/olvidados; `ProcSource`/`PortSource` traits con datos simulados para Puertos/procesos; JSON generados (`tests/fixtures` en código: anidado, arrays enormes, Unicode, inválido); atribución y agrupado con propiedades simples; `validate_deletable` con carpetas temporales y symlinks; scripts de seguridad con `vitest` (checksum, plataforma, rutas) y prueba manual del hook con un secreto falso. Front: `vitest` para stores y formato; `svelte-check`.
