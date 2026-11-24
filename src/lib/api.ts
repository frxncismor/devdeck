import { invoke } from "@tauri-apps/api/core";
import type { AppError, Config, Project, Scope } from "./types";

/** Normalizes anything thrown by `invoke` into an `AppError`. */
export function toAppError(e: unknown): AppError {
  if (e && typeof e === "object" && "code" in e && "kind" in e)
    return e as AppError;
  return {
    kind: "Io",
    code: "unknown",
    detail: e instanceof Error ? e.message : String(e),
  };
}

async function call<T>(
  cmd: string,
  args?: Record<string, unknown>,
): Promise<T> {
  try {
    return await invoke<T>(cmd, args);
  } catch (e) {
    throw toAppError(e);
  }
}

/** Typed command wrappers. Module agents add their own helpers next to these. */
export const api = {
  getConfig: () => call<Config>("get_config"),
  setConfig: (config: Config) => call<void>("set_config", { config }),
  listProjects: () => call<Project[]>("list_projects"),
  /** Helper for module commands that take the active scope. */
  withScope: <T>(
    cmd: string,
    scope: Scope,
    extra: Record<string, unknown> = {},
  ) => call<T>(cmd, { scope, ...extra }),
  call,
};
