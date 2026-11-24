import { formatPercent } from "../../lib/format";
import type { MessageKey } from "../../lib/i18n/keys";
import type { AppError, Grouped, ProjectRef, Totals } from "../../lib/types";

/** Mirrors of `devdeck_core::reclaim` (camelCase). */

export type DevKind =
  | "node"
  | "esbuild"
  | "vite"
  | "cargo"
  | "rustc"
  | "rustAnalyzer"
  | "chromium"
  | "playwright"
  | "docker"
  | "wsl";

export interface DevProcess {
  pid: number;
  name: string;
  exe?: string | null;
  cwd?: string | null;
  ramBytes: number;
  cpuPercent: number;
  startTime: number;
  parentPid?: number | null;
  kind: DevKind;
  project?: ProjectRef | null;
  worktreePath?: string | null;
  orphan: boolean;
  ageSecs: number;
}

export type ForgottenReason = "stale" | "merged" | "clean" | "live_processes";

export interface ForgottenWorktree {
  worktree: {
    path: string;
    repoPath: string;
    branch?: string | null;
    detached: boolean;
    isMain: boolean;
    dirtyCount: number;
    exists: boolean;
  };
  diskBytes?: number | null;
  lastCommitAgeDays?: number | null;
  merged: boolean;
  clean: boolean;
  liveProcesses: number;
  reasons: ForgottenReason[];
}

export type FolderKind =
  | "node_modules"
  | "target"
  | ".venv"
  | "dist"
  | "npmCache"
  | "pnpmStore"
  | "cargoRegistry"
  | "cargoGit";

export interface HeavyFolder {
  path: string;
  kind: FolderKind;
  diskBytes?: number | null;
  lastUsedIso?: string | null;
  project?: ProjectRef | null;
}

export interface ProjectSummary {
  project?: ProjectRef | null;
  ramBytes: number;
  diskBytes: number;
  processes: number;
  worktrees: number;
  folders: number;
}

export interface ReclaimSummary {
  recoverableRamBytes: number;
  recoverableDiskBytes: number;
  byProject: ProjectSummary[];
}

/** Payload of `job://progress`. */
export interface JobProgress {
  jobId: string;
  done: number;
  total?: number | null;
}

/** Payload of `job://done` for the folder search. */
export interface FoldersDone {
  jobId: string;
  cancelled: boolean;
  error?: AppError;
  folders?: Grouped<HeavyFolder>;
}

export type Pending =
  | { type: "worktree"; item: ForgottenWorktree; force: boolean }
  | { type: "process"; item: DevProcess }
  | { type: "folder"; item: HeavyFolder };

export const DEV_KIND_KEYS: Record<DevKind, MessageKey> = {
  node: "reclaim.kind.node",
  esbuild: "reclaim.kind.esbuild",
  vite: "reclaim.kind.vite",
  cargo: "reclaim.kind.cargo",
  rustc: "reclaim.kind.rustc",
  rustAnalyzer: "reclaim.kind.rustAnalyzer",
  chromium: "reclaim.kind.chromium",
  playwright: "reclaim.kind.playwright",
  docker: "reclaim.kind.docker",
  wsl: "reclaim.kind.wsl",
};

export const FOLDER_KIND_KEYS: Record<FolderKind, MessageKey> = {
  node_modules: "reclaim.folderKind.node_modules",
  target: "reclaim.folderKind.target",
  ".venv": "reclaim.folderKind.venv",
  dist: "reclaim.folderKind.dist",
  npmCache: "reclaim.folderKind.npmCache",
  pnpmStore: "reclaim.folderKind.pnpmStore",
  cargoRegistry: "reclaim.folderKind.cargoRegistry",
  cargoGit: "reclaim.folderKind.cargoGit",
};

export const REASON_KEYS: Record<ForgottenReason, MessageKey> = {
  stale: "reclaim.reason.stale",
  merged: "reclaim.reason.merged",
  clean: "reclaim.reason.clean",
  live_processes: "reclaim.reason.live_processes",
};

/** CPU usage (a percentage such as 12.5) as "12.5%"; negative or NaN values show as 0%. */
export function formatCpu(percent: number): string {
  const p = Number.isFinite(percent) ? Math.max(0, percent) : 0;
  return formatPercent(p / 100);
}

/** `done / total` clamped to 0..1, or `null` when the total is unknown or zero. */
export function progressRatio(
  done: number,
  total?: number | null,
): number | null {
  if (!total || total <= 0) return null;
  return Math.min(1, Math.max(0, done / total));
}

export function hasRecoverable(s: ReclaimSummary | null): boolean {
  return !!s && (s.recoverableRamBytes > 0 || s.recoverableDiskBytes > 0);
}

/** Per-project rows biggest total first (RAM + disk), "No project" always last. */
export function sortedByProject(s: ReclaimSummary): ProjectSummary[] {
  const weight = (p: ProjectSummary) => p.ramBytes + p.diskBytes;
  return [...s.byProject].sort((a, b) => {
    if (!a.project !== !b.project) return a.project ? -1 : 1;
    return weight(b) - weight(a);
  });
}

/** Number of items flagged with `pred` across all groups. */
export function countWhere<T>(
  data: Grouped<T> | null,
  pred: (item: T) => boolean,
): number {
  return data
    ? data.groups.reduce((n, g) => n + g.items.filter(pred).length, 0)
    : 0;
}

/** Totals of a grouped result, or zeros when nothing has loaded yet. */
export function totalsOf<T>(data: Grouped<T> | null): Totals {
  return data?.total ?? { count: 0, ramBytes: 0, diskBytes: 0 };
}

/** What deserves an extra warning before removing a forgotten worktree. */
export function removalWarnings(w: ForgottenWorktree): {
  dirty: number;
  live: number;
} {
  return { dirty: w.worktree.dirtyCount, live: w.liveProcesses };
}

/** Translation key for a rejected folder deletion (`folder_not_allowed` + `reason`), if known. */
export function folderRejectionKey(e: AppError): MessageKey | null {
  if (e.code !== "folder_not_allowed") return null;
  const reason = String(e.params?.reason ?? "");
  const known: Record<string, MessageKey> = {
    link: "reclaim.errors.reason.link",
    not_a_directory: "reclaim.errors.reason.not_a_directory",
    drive_root: "reclaim.errors.reason.drive_root",
    home: "reclaim.errors.reason.home",
    root: "reclaim.errors.reason.root",
    outside_root: "reclaim.errors.reason.outside_root",
    not_allowlisted: "reclaim.errors.reason.not_allowlisted",
  };
  return known[reason] ?? null;
}
