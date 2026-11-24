# Feature Specification: DevDeck MVP

**Feature Branch**: `001-devdeck-mvp`  |  **Created**: 2026-11-23  |  **Status**: Draft
**Input**: `runs/11231900/peticion.md` (petición de Paco + ampliaciones: módulo "Oculto", selector Global/proyecto, seguridad con gitleaks).

DevDeck es una app de escritorio (Tauri 2 + Rust, Svelte + TS) con barra lateral de 4 módulos: **Worktrees**,
**Puertos**, **Visor JSON** y **Oculto/Limpieza**, más un **selector de ámbito (Global / proyecto)** transversal.
Plataforma garantizada: Windows. Alcance sencillo, no robusto.

## Conceptos

- **Proyecto**: un repo principal + todos sus worktrees. Identificado por la ruta canónica del repo principal.
- **Ámbito** (scope): `Global` o `Proyecto(id)`. Filtra las vistas de Worktrees, Puertos y Oculto.
- **Sin proyecto**: grupo de Global para lo que no pertenece a ningún proyecto (cachés globales, procesos sin repo).

## User Scenarios & Testing

### US1 — Worktrees de mis repos (P1)
Como dev quiero indicar una carpeta raíz y ver todos los worktrees de cada repo con su estado.
**Prueba independiente**: con una raíz con 2 repos (uno con 2 worktrees) la lista muestra 3 filas con los datos correctos.
1. **Dado** una raíz configurada, **cuando** abro Worktrees, **entonces** veo por repo cada worktree con rama, estado
   (limpio / sucio + nº de archivos), ahead/behind vs upstream y último commit (hash corto, mensaje, fecha).
2. **Dado** un worktree, **cuando** pulso "VS Code" o "Terminal", **entonces** se abre en esa ruta.
3. **Dado** un worktree no principal, **cuando** pulso "Quitar" y confirmo, **entonces** se elimina; el principal no ofrece la acción.
4. **Dado** una rama sin upstream, **entonces** ahead/behind se muestra como "—".

### US2 — Selector Global / proyecto (P1)
Quiero ver todo agrupado por proyecto con totales, o centrarme en un proyecto.
1. **Dado** ámbito Global, **entonces** cada módulo agrupa por proyecto (más "Sin proyecto") con totales de RAM y disco por grupo y total general.
2. **Dado** que elijo un proyecto, **entonces** todos los módulos muestran solo lo de ese repo y sus worktrees.
3. **Dado** que cambio de módulo, **entonces** el ámbito se mantiene; al reiniciar la app se restaura el último.

### US3 — Puertos en escucha (P1)
1. **Dado** procesos escuchando, **entonces** veo puerto, protocolo, PID, nombre y ruta del ejecutable.
2. **Cuando** escribo en la búsqueda, **entonces** filtra por puerto, PID, nombre, ruta o proyecto.
3. **Cuando** pulso "Matar" y confirmo, **entonces** el proceso termina y desaparece al refrescar.
4. **Dado** un proceso cuyo cwd cuelga de un worktree, **entonces** la fila muestra ese worktree/proyecto; si el cwd no se puede leer muestra "desconocido".

### US4 — Seguridad: nadie commitea secretos (P1)
Quiero que ningún commit con secretos entre en el repo, ni local ni en CI.
1. **Dado** un clon nuevo, **cuando** ejecuto `pnpm setup:hooks` y `pnpm setup:gitleaks`, **entonces** `git config core.hooksPath` devuelve `.githooks` y existe `.tools/gitleaks/8.30.1/`.
2. **Dado** un cambio staged con un secreto de prueba, **cuando** hago `git commit`, **entonces** el hook lo bloquea (salida redactada).
3. **Dado** que falta gitleaks o su versión no es 8.30.1, **entonces** el commit se bloquea con instrucciones (fallo cerrado).
4. **Dado** un PR/push, **entonces** el CI repite gitleaks sobre el rango de commits y falla si hay hallazgos.

### US5 — Visor JSON grande (P2)
1. **Cuando** abro un archivo de varios cientos de MB, **entonces** se indexa en segundo plano con progreso y cancelación, y la UI sigue respondiendo.
2. **Cuando** expando un nodo, **entonces** se piden sus hijos paginados; el árbol está virtualizado.
3. **Cuando** busco un texto, **entonces** llegan resultados por lotes (clave/valor) y puedo saltar a cada uno.
4. **Cuando** selecciono un nodo, **entonces** puedo copiar su ruta (`$.a.b[3]`), copiar/exportar su subárbol formateado, o guardar el archivo formateado.
5. **Dado** un JSON inválido, **entonces** se muestra el error con offset sin colgar la app.

### US6 — Oculto / Limpieza (P2)
1. Resumen arriba: RAM y disco recuperables (respetando el ámbito).
2. **Worktrees olvidados**: tamaño en disco, antigüedad del último commit, rama integrada en la base / sin cambios, procesos vivos dentro; acción quitar con confirmación.
3. **Procesos de dev en segundo plano/huérfanos**: RAM, CPU, antigüedad, proyecto/worktree; los de cwd en un worktree ya borrado se marcan "huérfano"; acción terminar con confirmación.
4. **Carpetas regenerables** (node_modules, target, .venv, dist, cachés npm/pnpm/cargo): tamaño y último uso; borrar con confirmación.
5. Solo lectura + acciones confirmadas; nada se ejecuta solo.

### US7 — Entregables profesionales (P3)
CI verde en 3 SO, release automática por tag `v*` con .msi y .exe, README con capturas y GIF, icono propio, CHANGELOG.

## Requirements

### Shell y ámbito
- **FR-001**: Barra lateral con 4 módulos (Worktrees, Puertos, Visor JSON, Oculto) y selector de ámbito siempre visible.
- **FR-002**: Configuración persistida (carpeta raíz, último ámbito, base de ramas) en el almacén de la app.
- **FR-003**: Modelo `Project` (id, nombre, repo principal, worktrees) construido del escaneo de la raíz.
- **FR-004**: Función de atribución `ruta → proyecto` por prefijo de ruta más largo, normalizada, case-insensitive en Windows; sin coincidencia ⇒ "Sin proyecto".
- **FR-005**: Los comandos de Worktrees, Puertos y Oculto aceptan un ámbito y devuelven los datos ya filtrados.
- **FR-006**: En Global la respuesta viene agrupada por proyecto con totales de RAM y disco por grupo y total general; "Sin proyecto" es un grupo más.
- **FR-007**: En ámbito proyecto solo se devuelven elementos atribuidos a ese proyecto.
- **FR-008**: El ámbito seleccionado es un store global del front (persistido); cambiarlo recarga el módulo visible.
- **FR-009**: Errores serializables y mostrados en la UI sin cerrar la app.

### Worktrees
- **FR-010**: Escanear la carpeta raíz (profundidad configurable, por defecto 2) detectando repos git.
- **FR-011**: Por repo, listar worktrees con `git worktree list --porcelain`: ruta, rama (o detached), principal sí/no.
- **FR-012**: Por worktree: nº de archivos modificados y estado limpio/sucio.
- **FR-013**: Por worktree: ahead/behind vs upstream.
- **FR-014**: Por worktree: último commit (hash corto, asunto, fecha).
- **FR-015**: Acción abrir en VS Code (`code`).
- **FR-016**: Acción abrir terminal (Windows Terminal `wt -d`, fallback `pwsh`/`cmd`).
- **FR-017**: Acción quitar worktree con confirmación, rechazada para el principal; si está sucio exige confirmación reforzada (`--force` solo entonces).
- **FR-018**: Listar worktrees es un contrato compartido (`worktrees::list`) reutilizado por Puertos y Oculto.

### Puertos
- **FR-020**: Listar puertos TCP en escucha (UDP opcional, desactivado por defecto) con PID.
- **FR-021**: Por puerto: nombre del proceso y ruta del ejecutable.
- **FR-022**: Búsqueda por puerto, PID, nombre, ruta, proyecto.
- **FR-023**: Matar proceso con confirmación; error claro si falta permiso.
- **FR-024**: Obtener el cwd del proceso (`process_info`) y atribuirlo a worktree/proyecto; "desconocido" si no es legible.
- **FR-025**: Refresco manual y automático opcional cada N s.

### Visor JSON
- **FR-030**: Abrir archivo JSON de cientos de MB sin cargarlo en el webview ni entero en memoria como árbol.
- **FR-031**: Indexación en streaming en una pasada, con progreso (eventos) y cancelación.
- **FR-032**: `json_children(node, offset, limit)` paginado; valores leídos del archivo bajo demanda.
- **FR-033**: Árbol virtualizado con expansión perezosa.
- **FR-034**: Búsqueda en claves y/o valores en segundo plano, resultados por lotes, límite configurable.
- **FR-035**: Copiar ruta del nodo en formato `$.a.b[3]` (claves no identificador: `$["a b"]`).
- **FR-036**: Formatear: pretty-print del subárbol (con tope de tamaño) copiar/guardar, y guardar el archivo completo formateado en streaming.
- **FR-037**: JSON inválido ⇒ error con posición; sin pánico.

### Oculto / Limpieza
- **FR-040**: Resumen con RAM y disco recuperables (en el ámbito).
- **FR-041**: Worktrees olvidados: tamaño, antigüedad del último commit, rama integrada en la base o sin cambios, procesos vivos dentro; criterio "olvidado" configurable (por defecto > 30 días o integrada).
- **FR-042**: Quitar worktree olvidado con confirmación (reutiliza FR-017).
- **FR-043**: Procesos de dev (lista de nombres: node, esbuild, vite, cargo, rustc, rust-analyzer, chrome/chromium headless, playwright, docker, vmmem/wsl…) con RAM, CPU, antigüedad y proyecto/worktree.
- **FR-044**: Marcar "huérfano" el proceso con cwd en un worktree que ya no existe.
- **FR-045**: Terminar proceso con confirmación (reutiliza FR-023).
- **FR-046**: Carpetas regenerables (lista blanca: node_modules, target, .venv, dist, caché npm/pnpm/cargo) con tamaño y último uso.
- **FR-047**: Cálculo de tamaños en segundo plano con progreso y cancelación, sin seguir symlinks/junctions.
- **FR-048**: Borrar carpeta con confirmación; el backend rechaza cualquier ruta que no sea un nombre de la lista blanca o caché conocida.
- **FR-049**: Cachés globales y procesos sin repo aparecen en "Sin proyecto".

### Seguridad
- **FR-060**: `.githooks/pre-commit` ejecuta gitleaks **8.30.1** sobre lo staged con `--redact`; bloquea (fallo cerrado) si gitleaks falta, no es la versión exacta, o `.tools/...` es un symlink.
- **FR-061**: `.gitleaks.toml` con el ruleset por defecto (`useDefault = true`), sin excepciones salvo justificadas en el SDD.
- **FR-062**: Scripts `setup:hooks` (`git config core.hooksPath .githooks`) y `setup:gitleaks` (descarga el binario fijado a `.tools/gitleaks/8.30.1/`, verifica SHA-256, comprueba versión; Windows y Linux x64; en macOS se usa gitleaks 8.30.1 del PATH). `.tools/` en `.gitignore`.
- **FR-063**: CI: job que ejecuta gitleaks 8.30.1 sobre el rango de commits del PR (y del push), con historial completo (`fetch-depth: 0`), `--redact`.
- **FR-064**: Test del instalador (selección de asset, checksum, rutas inválidas) sin red.

### Entregables
- **FR-050**: Tests unitarios Rust de la lógica de cada módulo.
- **FR-051**: CI GitHub Actions (fmt, clippy -D warnings, test, lint/svelte-check/tests front) en windows/macos/ubuntu.
- **FR-052**: Release con tauri-action en tag `v*`, borrador con .msi y NSIS .exe.
- **FR-053**: README con capturas y GIF, icono propio, CHANGELOG Keep a Changelog.

## Success Criteria
- **SC-001**: Lista de 20 worktrees en < 3 s en Windows (git CLI con paralelismo acotado).
- **SC-002**: JSON de 500 MB indexado sin superar ~1,5 GB de RAM y UI sin bloqueos > 100 ms; abrir/expandir nodo < 200 ms tras indexar.
- **SC-003**: Búsqueda en 500 MB entrega el primer lote < 2 s.
- **SC-004**: Cambio de ámbito refresca el módulo < 1 s (los tamaños llegan por progreso).
- **SC-005**: CI verde en las 3 plataformas; un tag `v*` produce un borrador con instaladores.
- **SC-006**: Ninguna acción destructiva se ejecuta sin confirmación (test de contrato en backend).
- **SC-007**: Tras integrar, en devdeck `git config core.hooksPath` devuelve `.githooks`; un secreto de prueba staged bloquea el commit y el job de CI falla sobre un commit con ese secreto.

## Edge Cases
Repo bare; worktree prunable/ruta inexistente; rama detached; sin upstream; raíz inexistente; submódulos; proceso
elevado (cwd/ruta ilegibles); PID reutilizado entre listado y kill (revalidar nombre+PID); JSON de una sola línea
gigante, Unicode, claves duplicadas, números enormes; archivo modificado tras indexar; symlinks/junctions en
node_modules; permisos denegados al medir tamaño; mismo proceso con varios puertos; rutas con mayúsculas distintas
en Windows; gitleaks descargado corrupto (checksum falla).

## Fuera de alcance
Editar JSON; clonar/crear worktrees; commits/push desde la app; limpieza automática o programada; actualizador
automático; telemetría; garantía de Puertos en macOS/Linux; firma de código; escaneo periódico del historial completo.

## Supuestos
- Base de ramas "integrada" = `main` (configurable; fallback `master`/HEAD remoto).
- Borrado de carpetas: **definitivo** (regenerables), con confirmación mostrando tamaño; sin papelera (evita crate `trash` y es más rápido con miles de archivos).
- "Último uso" = mtime máximo de la carpeta y su primer nivel (barato); no atime (desactivado en Windows).
- Una sola raíz de repos; varias raíces fuera de alcance.
- Filtrado/agrupado/totales en Rust; el front solo guarda el ámbito.
- Gestor de paquetes pnpm (igual que la referencia `grim-dawn-builds`); scripts de seguridad en TypeScript con `tsx`.
