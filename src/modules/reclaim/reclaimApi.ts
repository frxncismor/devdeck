import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { api } from "../../lib/api";
import type { Grouped, Scope } from "../../lib/types";
import type {
  DevProcess,
  FoldersDone,
  ForgottenWorktree,
  JobProgress,
  ReclaimSummary,
} from "./logic";

export interface FolderScan {
  jobId: string;
  /** Resolves once with the final `job://done` payload. */
  done: Promise<FoldersDone>;
}

/**
 * Starts the folder search job and wires its events. Listeners are registered before the command
 * is invoked and early events are buffered, so nothing emitted before the job id is known is lost.
 */
export async function startFolderScan(
  scope: Scope,
  onProgress: (p: JobProgress) => void,
): Promise<FolderScan> {
  let jobId: string | null = null;
  const earlyProgress: JobProgress[] = [];
  const earlyDone: FoldersDone[] = [];
  let finish!: (d: FoldersDone) => void;
  const done = new Promise<FoldersDone>((resolve) => (finish = resolve));
  const unlisten: UnlistenFn[] = [];
  const cleanup = () => unlisten.forEach((u) => u());

  const complete = (d: FoldersDone) => {
    cleanup();
    finish(d);
  };

  unlisten.push(
    await listen<JobProgress>("job://progress", (e) => {
      if (jobId === null) earlyProgress.push(e.payload);
      else if (e.payload.jobId === jobId) onProgress(e.payload);
    }),
    await listen<FoldersDone>("job://done", (e) => {
      if (jobId === null) earlyDone.push(e.payload);
      else if (e.payload.jobId === jobId) complete(e.payload);
    }),
  );

  try {
    const job = await api.withScope<{ jobId: string }>(
      "reclaim_folders",
      scope,
    );
    jobId = job.jobId;
  } catch (e) {
    cleanup();
    throw e;
  }
  earlyProgress.filter((p) => p.jobId === jobId).forEach((p) => onProgress(p));
  const already = earlyDone.find((d) => d.jobId === jobId);
  if (already) complete(already);
  return { jobId, done };
}

export const reclaimApi = {
  summary: (scope: Scope) =>
    api.withScope<ReclaimSummary>("reclaim_summary", scope),
  processes: (scope: Scope) =>
    api.withScope<Grouped<DevProcess>>("reclaim_dev_processes", scope),
  forgotten: (scope: Scope) =>
    api.withScope<Grouped<ForgottenWorktree>>(
      "reclaim_forgotten_worktrees",
      scope,
    ),
  cancel: (jobId: string) => api.call<void>("reclaim_cancel", { jobId }),
  deleteFolder: (path: string) =>
    api.call<{ freedBytes: number }>("reclaim_delete_folder", { path }),
  removeWorktree: (path: string, force: boolean) =>
    api.call<void>("worktree_remove", { path, force }),
  killProcess: (
    pid: number,
    expectedName: string,
    expectedStartTime?: number,
  ) => api.call<void>("process_kill", { pid, expectedName, expectedStartTime }),
};
