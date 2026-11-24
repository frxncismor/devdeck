# DevDeck — design handoff for implementation

Design canvas: https://claude.ai/code/artifact/dcf7ac42-28e7-49fa-93b7-a8638fafc2ce
Stack: Tauri 2 + Svelte + TypeScript, plain CSS (no UI framework). Windows first.

## Files
- `tokens.css` — all design tokens as CSS variables. Import once (e.g. in `+layout.svelte` or `main.ts`).
  Theme = class on the root element: `<div class="dd dark">` or `<div class="dd light">`.
  Never hard-code colors; always use `var(--token)`.

## Layout
- Window 1280×800 default, min 960×600. Custom title bar (`decorations: false` in Tauri) 36px with
  a centered "Search… Ctrl K" trigger and Windows controls (min / max / close, 46×36 each).
- Body: sidebar (232px, collapses to 56px, Ctrl B) + main column.
- Main column: top bar 60px = **ScopeSelect** › module title (h1, 16/600) … right-aligned meta + actions.
  Content area scrolls, padding 20px 24px, gap 12–16px.

## Scope (central concept)
- Store `scope: 'global' | { project: string }`, persisted per module. Ctrl P opens the selector.
- Global view: results grouped by project in collapsible `Group` sections. Header shows project color
  square, name, path, count badge, RAM and Disk totals. A grand-total strip sits above the groups.
  Items without an owner go in a "No project" group (color `--project-none`).
- Project view: no grouping; a row of `StatCard`s above the list instead.
- Each project gets a stable color `--project-1..4` used in chips, group headers and stacked bars.

## Components (one Svelte file each)
| Component | Notes |
|---|---|
| Sidebar | `collapsed` prop; nav items from a module registry with live counts; root-folder card; EN/ES segmented switch; Settings |
| TitleBar | drag region (`data-tauri-drag-region`), command trigger, window buttons |
| TopBar | slots: title, actions; always renders ScopeSelect first |
| ScopeSelect | 40px button, min-w 240; listbox menu 380w with filter input, Global item, projects with RAM · Disk |
| Group | collapsible section; header h40 on `--surface-2`; `aria-expanded` |
| DataTable | columns config → `grid-template-columns`; header h32 11/600 uppercase; rows h40–54; numbers fixed width, paths `minmax(0,fr)` and truncate from the start (`direction: rtl`) with full path in `title` |
| TreeRow | h26, indent 18px + 1px guide, chevron 18, mono 12.5, type badge right; states: hover, selected (`--accent-soft` + inset accent ring), search match (`--match`) |
| Badge | tone: neutral/accent/success/warning/danger; always icon or dot + text, never color alone |
| Chip | project (+ optional worktree); deleted worktree shows in `--warning` |
| StatCard | `size: sm | lg`; lg has 36px value + stacked bar (h8, 2px gaps) + legend |
| Button | variants primary/secondary/ghost/danger/danger-ghost; sizes sm 26 / md 30 / lg 36; optional icon + kbd hint |
| IconButton | 28px, requires `aria-label` + `title` |
| Dialog | native `<dialog>` + `showModal()`; 440–520w, radius 12; focus Cancel first on destructive; confirm button names action + size ("Delete 1.46 GB") |
| Toast | store-driven, bottom-right 20px, 380w, stack gap 10; `aria-live="polite"`; auto-dismiss 5s except errors |
| Tabs | underline 2px accent, count pill |

Icons: Lucide-style stroke icons (1.75 stroke). `lucide-svelte` matches the design.

## Modules
1. **Worktrees** — empty state (pick root), groups of repos → rows: branch + path, status badge
   (Clean / N modified), ahead ↑ / behind ↓ (or "No upstream"), last commit (message, short hash, relative date),
   actions: Open in VS Code, Open terminal, Remove (disabled with tooltip for the main worktree).
   Rescan button (F5) + "Scanned X ago".
2. **Ports** — filter input (port / process / path), All/TCP/UDP toggle, table: Port, Proto, PID, Process,
   Executable path, Owner chip (or "unknown", lock icon when elevated), Kill (danger-ghost → dialog).
   Info banner about elevated processes whose cwd can't be read. Auto refresh every 5s while visible.
3. **JSON Viewer** — drop zone + Open file (Ctrl O) + recent files grouped by project; indexing card with
   progress, sizes, node count, Cancel (Esc); file bar (name, path, size, nodes, index time, Pretty-print subtree,
   Save formatted…); search with "3 of 41", prev/next, case/keys/values; breadcrumb; virtualized tree;
   arrays page by 100 with pager row + "Go to index"; side panel: JSONPath + Copy path, type, children, depth,
   byte range, pretty preview.
4. **Reclaim** — two large StatCards (Recoverable RAM / disk, stacked by project or, in project scope, by worktree),
   read-only notice, tabs: Forgotten worktrees (size, last commit age, badges merged / no changes / has live
   processes, Remove), Dev processes (RAM, CPU, age, cwd, "Deleted worktree" warning badge, Terminate),
   Regenerable folders (checkbox selection, size, last used, footer "N selected · size" + Delete permanently…).
   Nothing runs automatically; every destructive action goes through a dialog.

## States covered in the canvas ("States" page)
Empty (no root, no ports, no file), scanning, indexing, git not found, permission denied banner,
scope menu open, toasts, remove-worktree / kill-process / delete-folders dialogs, Spanish screen.

## i18n
EN / ES. Spanish strings run ~30% longer; buttons and tabs must not have fixed widths.
Key strings (ES): Puertos, Visor JSON, Recuperar, Ámbito, RAM recuperable, Disco recuperable,
Worktrees olvidados, Procesos de desarrollo, Carpetas regenerables, fusionado, sin cambios,
con procesos activos, Eliminar…, Volver a analizar, Ajustes.

## Accessibility
Real `<button>` / `<a>` / `<input>` + `<label>`; `aria-current="page"` on active nav; `role="tree"`/`treeitem`
with `aria-level`/`aria-expanded`; focus ring 2px `--accent`, offset 2px; all text ≥ 4.5:1 in both themes.
