# DevDeck Constitution

## Core Principles

### I. Alcance sencillo, no robusto
DevDeck es una herramienta personal de portafolio. Se prefiere la solución más simple que cumpla el criterio de
aceptación: solo lectura más acciones con confirmación, sin automatismos, sin demonios ni sincronización. Ninguna
funcionalidad añade abstracciones "por si acaso".

### II. Lógica en Rust, testeada
Toda la lógica (git, puertos, procesos, JSON, limpieza, atribución a proyectos) vive en la crate `devdeck-core`,
sin dependencia de Tauri. Se prueba con tests unitarios que no usan red ni UI (repos git temporales, JSON
generados, capa de sistema simulada mediante traits). Una función sin test no se da por terminada.

### III. Frontend fino
Svelte 5 + TypeScript solo presenta datos y envía comandos. No contiene reglas de negocio: filtrado por ámbito,
atribución, ordenación pesada y parseo se hacen en Rust. El webview nunca carga archivos grandes completos.

### IV. Windows primero
Windows es la plataforma garantizada. macOS y Linux deben compilar y pasar el CI; el módulo Puertos (y el cwd de
procesos) solo se garantiza en Windows, en otras plataformas es mejor esfuerzo y se degrada a "desconocido".

### V. CI verde obligatorio
Nada se integra sin: `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`, y `lint` + `svelte-check` +
tests del front, en Windows, macOS y Linux.

### VI. Acciones destructivas seguras
Matar procesos, quitar worktrees y borrar carpetas exigen confirmación explícita, operan solo sobre elementos
validados por el backend (lista blanca, nunca el worktree principal) y nunca siguen symlinks/junctions.

### VII. Commits, versiones y secretos
Conventional Commits y CHANGELOG (Keep a Changelog). Nunca se escriben ni muestran tokens, credenciales ni
cadenas de conexión en código, logs, capturas o README.

## Gobierno
Esta constitución prevalece sobre preferencias locales. Cambiarla exige actualizar este archivo y justificar el
cambio en el PR.

**Version**: 1.0.0 | **Ratified**: 2026-11-23 | **Last Amended**: 2026-11-23
