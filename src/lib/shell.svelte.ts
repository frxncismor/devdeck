import type { Snippet } from "svelte";
import type { ModuleId } from "./modules";

/** App-shell UI state shared by the title bar, sidebar, top bar and palette. */
export const shell = $state<{
  active: ModuleId;
  sidebarCollapsed: boolean;
  paletteOpen: boolean;
  scopeOpen: boolean;
}>({
  active: "worktrees",
  sidebarCollapsed: false,
  paletteOpen: false,
  scopeOpen: false,
});

/** Slots a module screen feeds into the app TopBar (see `TopBarContent.svelte`). */
export const topbarSlots = $state<{
  meta?: Snippet;
  actions?: Snippet;
}>({});
