# Research: DevDeck MVP

Decisiones ya justificadas en `plan.md`. Aquí, lo que conviene verificar al implementar (en el diseño no se ha compilado nada).

| Duda | Decisión provisional | Cómo verificar |
|---|---|---|
| `sysinfo::Process::cwd()` en Windows | Funciona leyendo el PEB para procesos del mismo usuario y bitness; `None` si no | Test manual con node/vite propios y uno elevado |
| Crate `listeners` | Da PID/nombre/ruta por socket en Win/Mac/Linux | Comprobar versión y que expone puerto y protocolo; si no, `netstat2` |
| Último uso de carpeta | mtime máx. (carpeta + primer nivel); atime descartado (deshabilitado por defecto en Windows) | Test con carpeta temporal |
| Rama integrada | `git merge-base --is-ancestor <rama> <base>`; `branch --merged` descartado (más lento) | Test con repo temporal |
| Junctions en Windows | Tratar `REPARSE_POINT` como enlace: no entrar, no contar | Test condicional `#[cfg(windows)]` |
| Tamaño de carpetas | `walkdir` en `spawn_blocking`, progreso cada 250 ms, `AtomicBool` de cancelación | Test con árbol temporal |
| Cachés Windows | npm `%LOCALAPPDATA%\npm-cache`; pnpm `pnpm store path` (fallback `%LOCALAPPDATA%\pnpm\store`); cargo `%USERPROFILE%\.cargo\{registry,git}` | Comprobar en la máquina de Paco |
| gitleaks macOS | Sin asset verificado ⇒ PATH con versión exacta | Revisar la release 8.30.1 si se quiere añadir hash |
| Memoria JSON | ~32 B/contenedor + saltos cada 1024 hijos | Medir con fixture de 100–500 MB generado |
