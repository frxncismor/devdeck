import type { Component } from "svelte";
import Braces from "@lucide/svelte/icons/braces";
import Gauge from "@lucide/svelte/icons/gauge";
import GitBranch from "@lucide/svelte/icons/git-branch";
import Plug from "@lucide/svelte/icons/plug";
import type { MessageKey } from "./i18n/keys";

export type ModuleId = "worktrees" | "ports" | "jsonViewer" | "reclaim";

export interface ModuleDef {
  id: ModuleId;
  /** i18n key of the nav label and the TopBar title. */
  labelKey: MessageKey;
  icon: Component;
}

/** Single source for the sidebar, the command palette and the TopBar title. */
export const MODULES: readonly ModuleDef[] = [
  {
    id: "worktrees",
    labelKey: "common.sidebar.worktrees",
    icon: GitBranch as unknown as Component,
  },
  {
    id: "ports",
    labelKey: "common.sidebar.ports",
    icon: Plug as unknown as Component,
  },
  {
    id: "jsonViewer",
    labelKey: "common.sidebar.jsonViewer",
    icon: Braces as unknown as Component,
  },
  {
    id: "reclaim",
    labelKey: "common.sidebar.reclaim",
    icon: Gauge as unknown as Component,
  },
];
