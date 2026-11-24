import { api } from "./api";
import { configState, updateConfig } from "./config.svelte";
import type { Project, Scope } from "./types";

export const scopeState = $state<{ projects: Project[] }>({ projects: [] });

/** Current scope, persisted in the config. Modules read it and pass it to `api.*`. */
export function currentScope(): Scope {
  return configState.value.scope;
}

export async function setScope(scope: Scope): Promise<void> {
  await updateConfig({ scope });
}

export async function refreshProjects(): Promise<void> {
  try {
    scopeState.projects = await api.listProjects();
  } catch {
    scopeState.projects = [];
  }
  const s = configState.value.scope;
  if (s.kind === "project" && !scopeState.projects.some((p) => p.id === s.id)) {
    await setScope({ kind: "global" });
  }
}
