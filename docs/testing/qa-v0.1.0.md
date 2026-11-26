# QA report: DevDeck v0.1.0 end-to-end scenarios

Informe en español. Fecha: 2026-11-26.

## Veredicto

**v0.1.0 no debería salir tal como está.** Hay un P1: el árbol del visor JSON no hace scroll, así que todo lo que queda por debajo de la primera pantalla es inalcanzable. El resto de módulos cumple su función, con fallos P2 (el más serio: quitar un worktree con un proceso dentro dice que falló, pero ya borró su contenido). Dev y release se comportaron igual en todo lo probado en ambos.

## Entorno

| Dato    | Valor                                                       |
| ------- | ----------------------------------------------------------- |
| Versión | 0.1.0                                                       |
| Commit  | `b1bd36e` (rama `orq/11261315-11261315-especialista-2`)     |
| Sistema | Windows 11 Pro 10.0.26300                                   |
| Memoria | 31,9 GB; libre antes de compilar o arrancar: 18,5 a 19,9 GB |
| Tooling | Node 22.12.0, pnpm 12.18.0, rustc 1.100.0, git 2.45.1       |
| Ventana | tamaño por defecto (captura del webview 1280 x 800)         |

## Montaje

Arnés y datos: [qa-fixtures.md](qa-fixtures.md). Se montó, se aplicó la config de prueba y se arrancó con `scripts/qa/qa-app.ps1`, una build cada vez. La app se manejó por CDP (`scripts/qa/cdp.mts`).

- **Dev:** `qa-app.ps1 -Build dev` (cargo build + vite + exe). No se usó `pnpm tauri dev` literal, para que las cachés falsas no afecten a cargo (así lo pide el arnés).
- **Release:** `pnpm tauri build` completo (exit 0, 160 s): `devdeck.exe` 11,9 MB, MSI 4,0 MB, NSIS 2,7 MB. Se probó el exe; los instaladores **no** se instalaron.
- **JSON:** el diálogo nativo queda fuera de CDP. Se abrió sembrando "recientes" (`cdp.mts abrir-json`). El guardado formateado se probó invocando `json_save_formatted` con `window.__TAURI_INTERNALS__.invoke` (solo en dev).
- **Seguridad:** solo se terminaron, quitaron o borraron elementos del arnés (PID contrastados con `pids.json`, rutas bajo `%TEMP%\devdeck-qa`).
- **Cierre:** config real restaurada y verificada (SHA-256 `FE348F3C…CD504`, igual que antes), datos desmontados, puertos 1420, 9222 y 47010-47015 libres, `target/` borrado (9,25 GB).

## Línea base

Ejecutada una vez, sobre el commit indicado.

| Comando                  | Resultado                          |
| ------------------------ | ---------------------------------- |
| `cargo test --workspace` | pasa: 149 ok, 1 ignorado, 0 fallos |
| `pnpm test`              | pasa: 113/113 (14 ficheros)        |
| `pnpm check`             | 0 errores, 0 avisos (939 ficheros) |
| `pnpm lint`              | pasa                               |
| `pnpm test:scripts`      | 15 ok, 1 omitido                   |
| `pnpm build`             | pasa                               |

No se ejecutaron `cargo fmt --check` ni `cargo clippy`.

## Resumen

| #   | Escenario                      | Dev           | Release       | Bugs                 |
| --- | ------------------------------ | ------------- | ------------- | -------------------- |
| 1   | Worktrees: listar y estados    | pasa          | pasa          | B3, B9               |
| 1b  | Worktrees: quitar              | **falla**     | **falla**     | B2, B5               |
| 2   | Puertos                        | pasa (con B4) | pasa (con B4) | B4                   |
| 3   | Visor JSON                     | **falla**     | **falla**     | B1, B7, B12          |
| 4   | Reclaim                        | **falla**     | **falla**     | B2, B6, B8, B10, B11 |
| 5   | Selector Global / por proyecto | pasa          | pasa          | B11                  |
| 6   | Persistencia                   | pasa          | pasa          | B13                  |
| 7   | Idioma, temas, Ctrl K / B / P  | pasa          | pasa          | B11, B12, B14        |
| M   | Diálogos nativos y externos    | manual        | manual        | ver "Pendiente"      |

## Detalle por escenario

### 1. Worktrees

**Pasos:** abrir Worktrees con los fixtures; revisar estados; quitar `sin-upstream` (cancelar y confirmar), `sucio` (dos confirmaciones), `limpio` (con un `node` vivo dentro) y `borrado` (sin carpeta).

| Comprobación                         | Esperado                           | Real (dev y release)                                                                                                              |
| ------------------------------------ | ---------------------------------- | --------------------------------------------------------------------------------------------------------------------------------- |
| Listado                              | 16 entradas, 2 proyectos           | Igual; 2 con cambios                                                                                                              |
| Estados                              | tabla de qa-fixtures.md            | Todos correctos: Main, In sync, 3 changed files, No remote branch, ahead 2, behind 2, diverged 1/1, Detached HEAD, Missing folder |
| Filtros (búsqueda; solo con cambios) | filtran                            | `diverg` deja solo `divergente`; sin worktrees sucios la lista queda vacía (release)                                              |
| Quitar limpio                        | diálogo; desaparece de git y lista | Pasa; la rama se conserva                                                                                                         |
| Cancelar                             | no cambia nada                     | Pasa (dev)                                                                                                                        |
| Quitar sucio                         | segunda confirmación               | Pasa: "Discard uncommitted changes?"; en dev, cancelarla conserva los ficheros                                                    |
| Quitar con proceso dentro            | no borrar nada, o avisar           | **Falla (B2)**                                                                                                                    |
| Quitar "Missing folder"              | olvida solo ese                    | **Falla (B5)**                                                                                                                    |

Los estados "sin commits recientes" y "mergeada" se muestran en Reclaim, no en Worktrees (comportamiento documentado en el glosario).

Capturas: [columnas solapadas](qa-v0.1.0/worktrees-columnas-solapadas.png), [quitar con proceso](qa-v0.1.0/worktree-quitar-con-proceso.png).

### 2. Puertos

**Pasos:** abrir Puertos; comprobar dueño de 47010-47016; buscar por puerto, nombre, worktree y PID; filtrar TCP/UDP (release); terminar el proceso de 47015.

- Los 7 procesos de prueba salen con su dueño: `webshop`, `limpio`, `olvidado`, `huerfano`, `api-b`, `auth` (UDP) y "No project". Pasa.
- Búsqueda (dev): `47015`, `limpio`, `vite` y el PID devuelven una sola fila. Pasa.
- Terminar: diálogo con nombre, PID y puerto; toast "Process terminated"; `Get-Process` y `Get-NetTCPConnection` confirman PID muerto y puerto libre. Pasa en ambas.
- La columna "Working folder" muestra la ruta del ejecutable (B4). [Captura](qa-v0.1.0/puertos-ruta-exe-es-claro.png).

### 3. Visor JSON

| Caso                                    | Dev                                                      | Release                              |
| --------------------------------------- | -------------------------------------------------------- | ------------------------------------ |
| Pequeño (110 B)                         | árbol, tipos, unicode, JSONPath `$["a b"].unicode`: pasa | pasa                                 |
| Copiar ruta / valor / formateado        | portapapeles correcto                                    | no probado                           |
| Anidado (400 niveles)                   | búsqueda llega a profundidad 400: pasa                   | pasa                                 |
| 9 inválidos                             | "Invalid JSON at byte N" en todos                        | mismos bytes                         |
| Grande (110 MB, 1 132 005 contenedores) | indexado 2,4 a 2,6 s                                     | indexado 0,3 a 0,5 s                 |
| Memoria del proceso (privada)           | 8 → 52 MB (working set pico 227 MB, incluye mmap)        | 9 → 52 MB (pico 215 MB)              |
| Búsqueda al final del fichero           | `$.needle`, ~5 s incluido el arranque del helper         | ~1,2 s (descontado ~1 s)             |
| Búsqueda masiva (`lorem`)               | tope 10 000 con aviso; hueco máximo entre frames 11 ms   | hueco máximo 17 ms                   |
| Cancelar con Esc                        | "Indexing cancelled": pasa                               | no probado (indexa demasiado rápido) |
| Nodo > 5 MB                             | copiar y previsualizar avisan del límite: pasa           | no probado                           |
| Guardar formateado                      | pasa (por comando, fichero pequeño)                      | no probado                           |
| **Scroll del árbol**                    | **falla (B1)**                                           | **falla (B1)**                       |
| Ir al resultado de búsqueda             | **falla (B7)**: el árbol no se despliega                 | no verificado (mismo código)         |

La búsqueda sin distinguir mayúsculas funciona con no ASCII (`CANCIÓN` / `canción`) y "Match case" la respeta (dev). El panel lateral muestra ruta, tipo, hijos, profundidad y vista previa.

Capturas: [árbol sin scroll](qa-v0.1.0/json-arbol-sin-scroll.png), [búsqueda sin desplegar](qa-v0.1.0/json-busqueda-no-despliega.png).

### 4. Reclaim

**Pasos:** abrir Reclaim; revisar las tres pestañas; terminar el huérfano (y en release también `vite`); borrar `api-b\.venv`, `api-b\target` y `webshop\dist`; quitar worktrees.

- Olvidados: `legacy`, `mergeado`, `olvidado` (Processes running), `olvidado-sucio`. Pasa.
- Procesos: huérfano marcado "Deleted worktree". Pasa. Dos `node` ajenos salen como "0 B · 20,783 days ago" (B8).
- Carpetas: 9, incluidas las 4 cachés falsas; los señuelos `docs\target` y `docs\dist` no salen. Pasa.
- Terminar huérfano: diálogo con PID y RAM; proceso muerto. Pasa en ambas.
- Borrar carpetas: el diálogo lista las 3 rutas y 17 MB; desaparecen del disco; disco recuperable 42 → 25 MB; toast "Freed 17 MB." Pasa en ambas.
- Quitar `olvidado` con `vite` vivo (dev): el diálogo avisa de 1 proceso, pero el resultado es B2. En release, terminando antes `vite`, se quita limpio: pasa.
- Quitar `legacy` (dev) y `olvidado-sucio` (release, doble confirmación): pasa, salvo que las carpetas de un worktree quitado siguen listadas (B6).
- Resumen de lo recuperado: solo existe para carpetas ("Freed 17 MB."). Quitar un worktree o terminar un proceso no informa del tamaño recuperado.

[Captura](qa-v0.1.0/reclaim-es-claro.png).

### 5. Selector Global / por proyecto

**Pasos:** elegir Global, `webshop` y `api-b` y recorrer Worktrees, Puertos y Reclaim. Cada proyecto filtra los tres módulos y el reparto pasa a "by worktree". Las cifras cuadran con las listas. Pasa.

Se invocaron a la vez `reclaim_summary` y `reclaim_dev_processes` tres veces: coinciden (18 procesos, mismos bytes). Las diferencias que a veces se ven en pantalla entre la tarjeta y la pestaña vienen de cargarse en momentos distintos (B11).

En dev, tras B2, los procesos de `limpio` y `olvidado` pasan a "No project" y no se marcan huérfanos.

### 6. Persistencia

**Pasos:** proyecto `webshop`, español, tema claro, sidebar colapsada; cerrar y reabrir.

| Ajuste            | ¿Persiste? (dev y release) |
| ----------------- | -------------------------- |
| Idioma            | sí                         |
| Tema              | sí                         |
| Proyecto elegido  | sí                         |
| Carpeta raíz      | sí                         |
| JSON recientes    | sí                         |
| Sidebar colapsada | no (B13)                   |
| Módulo activo     | no, abre Worktrees (B13)   |

### 7. Idioma, temas y atajos

- en/es: textos, fechas ("26 nov 2026, 13:53", "hace 120 días") y tamaños ("1,5 GB") cambian al instante. El glosario sale en español. Excepción: "Indexado en 2.4 s" con punto decimal (B12).
- Temas claro y oscuro: legibles en los módulos vistos.
- Ctrl K: lista módulos y ámbitos, filtra, Enter ejecuta, "No matches", Esc cierra. Ctrl B: 232 → 56 px. Ctrl P: abre el selector. Pasa (dev completo; en release Ctrl K, Ctrl B y Ctrl P).
- F5 reescanea en Worktrees y Reclaim.

## Bugs priorizados

Ninguno se ha arreglado.

### P1 (bloquea release)

**B1. El árbol del visor JSON no hace scroll.**

- Pasos: abrir `grande.json`, desplegar `$` y `orders`, girar la rueda sobre el árbol.
- Esperado: se recorren las filas y se llega a "Load more".
- Real: solo se ven unas 19 filas. Tras 40 giros, `scrollTop` sigue en 0 (`scrollHeight` 5330, `clientHeight` 468 a 492, `overflow-y: visible`; el padre recorta con `overflow: hidden`). Las demás filas y la paginación son inalcanzables. Afecta a cualquier JSON que no quepa en una pantalla.
- Sospechoso: `src/components/VirtualList.svelte`: el `div.virtual` no tiene `overflow: auto` en ningún estilo.

### P2 (importante)

**B2. Quitar un worktree con un proceso dentro "falla" pero ya ha borrado su contenido.**

- Pasos: con un proceso cuyo cwd es el worktree (`limpio` en Worktrees, `olvidado` en Reclaim), pulsar Remove y confirmar.
- Esperado: no se toca nada, o se avisa y se pide terminar el proceso antes.
- Real: error "A git command failed … Permission denied", pero git ya borró todos los ficheros (también los ignorados) y desregistró el worktree. Queda una carpeta vacía que la app no lista. La fila sigue visible como si nada; reintentar da "The item was not found". Con un worktree sucio confirmado, los cambios se pierden aunque el mensaje diga que falló. En Worktrees ni siquiera se avisa de que hay procesos.
- Sospechosos: `crates/devdeck-core/src/worktrees/mod.rs` (`remove_with`, sin comprobación previa de procesos); `confirmRemove` en `src/modules/worktrees/index.svelte` y `confirmPending` en `src/modules/reclaim/index.svelte` no reescanean al fallar.

**B3. La insignia de cambios pisa la columna "VS. REMOTE".**

- Pasos: abrir Worktrees con el tamaño de ventana por defecto.
- Esperado: columnas legibles. Real: "No uncommitted changes" tapa el ✓, el "No remote branch" y las flechas; en español es peor ("Sin cambios sin commitear").
- Sospechoso: `src/modules/worktrees/index.svelte`, columna `status` fija a `130px`.

**B4. Puertos: "Working folder" muestra la ruta del ejecutable.**

- Pasos: abrir Puertos y mirar 47010. Esperado: la carpeta `webshop`. Real: la ruta de `node.exe`.
- Sospechoso: `src/modules/ports/index.svelte:282` (`p.exePath ?? p.cwd`).

**B5. Quitar un worktree "Missing folder" olvida todos los ausentes del repo.**

- Pasos: con `borrado` y `huerfano` ausentes, quitar solo `borrado`.
- Esperado: desaparece solo `borrado`. Real: desaparecen los dos (`git worktree prune`). Además el diálogo dice "The folder will be deleted" de una carpeta que no existe.
- Sospechoso: `crates/devdeck-core/src/worktrees/mod.rs` (`remove_with`).

**B6. Reclaim: carpetas fantasma e inflado del total tras quitar un worktree.**

- Pasos: en Reclaim, quitar `olvidado` (con su proceso ya terminado) y `olvidado-sucio`; abrir "Regenerable folders" sin reescanear.
- Esperado: `olvidado\node_modules` desaparece. Real: sigue listado y se suma como "No project" (17 MB en vez de 11; "6 folders" en vez de 5). F5 lo corrige.
- Sospechoso: `confirmPending` en `src/modules/reclaim/index.svelte` no llama a `loadFolders()`.

**B7. Elegir un resultado de búsqueda no lo muestra en el árbol.**

- Pasos: abrir `anidado.json` y buscar `fondo`.
- Esperado: el árbol se despliega hasta el nodo. Real: sigue colapsado en `$`; solo cambia el panel lateral. Las migas se cortan sin enseñar el último tramo. Junto con B1, un resultado profundo no se puede ver en contexto.
- Sospechosos: `onhit` en `src/modules/json-viewer/index.svelte`, `Tree.svelte`.

**B8. Procesos ilegibles como "0 B · 20,783 days ago · Unknown folder".**

- Pasos: Reclaim > Dev processes con algún `node` protegido o elevado en la máquina.
- Esperado: antigüedad y RAM "desconocidas". Real: se calcula desde la época Unix y se ofrece "Terminate".
- Sospechoso: `crates/devdeck-core/src/reclaim/processes.rs` (`age_secs` con `start_time` 0).

### P3 (menor)

- **B9.** Fila "Missing folder" muestra además "No remote branch" y "No commits", aunque la rama tiene commits (`worktrees/mod.rs`, `list_with`).
- **B10.** Los toasts de error no caducan (es intencional) y tapan el botón "Delete permanently…" de la barra de selección hasta cerrarlos. [Captura](qa-v0.1.0/toasts-tapan-borrar.png). `src/components/ToastHost.svelte`.
- **B11.** Textos y cifras de Reclaim: "1 worktrees · 1 folders" / "1 carpetas" (`reclaim.json`, `diskHint` sin plural); "0 days ago" / "hace 0 días" en lugar de "hoy"; la tarjeta y la pestaña de procesos pueden discrepar (p. ej. "from 14" frente a 12).
- **B12.** "Indexed in 2.4 s" no se localiza (`src/modules/json-viewer/index.svelte:312`, `toFixed(1)`).
- **B13.** No persisten la sidebar colapsada ni el módulo activo (`src/lib/shell.svelte.ts`). Puede ser decisión de diseño.
- **B14.** En español la cabecera "PROTOCOLO" se trunca en Puertos.

## Pendiente (manual) y no cubierto

Manual, fuera del alcance de CDP:

- Elegir la carpeta raíz ("Change…").
- Abrir JSON con "Open file…" / Ctrl O y "Save formatted…" desde sus diálogos nativos.
- Arrastrar un JSON desde el Explorador.
- "Open in VS Code" y "Open terminal" (no se pulsaron).
- Instalar y desinstalar el MSI y el NSIS.

No cubierto: pantalla de primer arranque sin raíz, "Cancel" del escaneo de carpetas, fallo parcial al borrar varias carpetas, procesos Docker o WSL, JSON de más de 110 MB, macOS y Linux.
