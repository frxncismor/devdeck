import type { Snippet } from "svelte";
import { MODULES, type ModuleId } from "./modules";

/** Front-only preferences (NOT part of the Rust config contract), like the theme. */
export const SHELL_ACTIVE_KEY = "devdeck.shell.active";
export const SHELL_COLLAPSED_KEY = "devdeck.shell.sidebarCollapsed";

function read(key: string): string | null {
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
}

function write(key: string, value: string): void {
  try {
    localStorage.setItem(key, value);
  } catch {
    // Storage unavailable: the choice still applies for this session.
  }
}

const state = $state<{
  active: ModuleId;
  sidebarCollapsed: boolean;
  paletteOpen: boolean;
  scopeOpen: boolean;
}>({
  active:
    MODULES.find((m) => m.id === read(SHELL_ACTIVE_KEY))?.id ?? "worktrees",
  sidebarCollapsed: read(SHELL_COLLAPSED_KEY) === "true",
  paletteOpen: false,
  scopeOpen: false,
});

/** App-shell UI state shared by the title bar, sidebar, top bar and palette. */
export const shell = {
  get active(): ModuleId {
    return state.active;
  },
  set active(id: ModuleId) {
    state.active = id;
    write(SHELL_ACTIVE_KEY, id);
  },
  get sidebarCollapsed(): boolean {
    return state.sidebarCollapsed;
  },
  set sidebarCollapsed(value: boolean) {
    state.sidebarCollapsed = value;
    write(SHELL_COLLAPSED_KEY, String(value));
  },
  get paletteOpen(): boolean {
    return state.paletteOpen;
  },
  set paletteOpen(value: boolean) {
    state.paletteOpen = value;
  },
  get scopeOpen(): boolean {
    return state.scopeOpen;
  },
  set scopeOpen(value: boolean) {
    state.scopeOpen = value;
  },
};

/** Slots a module screen feeds into the app TopBar (see `TopBarContent.svelte`). */
export const topbarSlots = $state<{
  meta?: Snippet;
  actions?: Snippet;
}>({});
