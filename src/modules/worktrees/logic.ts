import { toAppError } from "../../lib/api";
import type { MessageKey } from "../../lib/i18n/keys";
import type { AppError, Grouped, Totals } from "../../lib/types";

export interface LastCommit {
  hash: string;
  subject: string;
  dateIso: string;
  timestamp: number;
}

/** Mirror of `devdeck_core::worktrees::Worktree` (camelCase). */
export interface Worktree {
  path: string;
  repoPath: string;
  project?: { id: string; name: string } | null;
  branch?: string | null;
  detached: boolean;
  isMain: boolean;
  dirtyCount: number;
  ahead?: number | null;
  behind?: number | null;
  lastCommit?: LastCommit | null;
  exists: boolean;
}

export interface Filters {
  /** Only worktrees with uncommitted changes. */
  dirtyOnly: boolean;
  /** Case-insensitive substring over branch, path and last commit subject. */
  query: string;
}

/** "↑2 ↓3"; zero sides are omitted, "✓" when in sync, `null` when there is no upstream. */
export function formatAheadBehind(
  ahead?: number | null,
  behind?: number | null,
): string | null {
  if (ahead == null && behind == null) return null;
  const parts: string[] = [];
  if (ahead) parts.push(`↑${ahead}`);
  if (behind) parts.push(`↓${behind}`);
  return parts.length ? parts.join(" ") : "✓";
}

/** Seconds elapsed since a Unix-seconds timestamp (never negative). */
export function ageSecs(timestamp: number, nowMs: number = Date.now()): number {
  return Math.max(0, Math.floor(nowMs / 1000 - timestamp));
}

export function matches(w: Worktree, f: Filters): boolean {
  if (f.dirtyOnly && w.dirtyCount === 0) return false;
  const q = f.query.trim().toLowerCase();
  if (!q) return true;
  return [w.branch ?? "", w.path, w.lastCommit?.subject ?? ""].some((s) =>
    s.toLowerCase().includes(q),
  );
}

/** Applies the filters, drops groups left empty and recomputes the totals (worktrees have no RAM/disk). */
export function filterGrouped(
  data: Grouped<Worktree>,
  f: Filters,
): Grouped<Worktree> {
  const groups = data.groups
    .map((g) => {
      const items = g.items.filter((w) => matches(w, f));
      return { ...g, items, totals: { ...g.totals, count: items.length } };
    })
    .filter((g) => g.items.length > 0);
  const total: Totals = {
    count: groups.reduce((n, g) => n + g.totals.count, 0),
    ramBytes: 0,
    diskBytes: 0,
  };
  return { groups, total };
}

export function isDirtyError(e: AppError): boolean {
  return e.code === "worktree_dirty";
}

export type StatusKind = "missing" | "dirty" | "clean";

export interface WorktreeStatus {
  kind: StatusKind;
  tone: "success" | "warning";
  /** Short label key; `dirty` takes a numeric `count` (plural). */
  labelKey: MessageKey;
  /** Tooltip / accessible description key. */
  hintKey: MessageKey;
  count?: number;
}

/** Plain-language state of a worktree: what to show and what its tooltip explains. */
export function worktreeStatus(w: Worktree): WorktreeStatus {
  if (!w.exists) {
    return {
      kind: "missing",
      tone: "warning",
      labelKey: "common.worktree.missing",
      hintKey: "worktrees.status.missingHint",
    };
  }
  if (w.dirtyCount > 0) {
    return {
      kind: "dirty",
      tone: "warning",
      labelKey: "worktrees.status.dirty",
      hintKey: "worktrees.status.dirtyHint",
      count: w.dirtyCount,
    };
  }
  return {
    kind: "clean",
    tone: "success",
    labelKey: "worktrees.status.clean",
    hintKey: "worktrees.status.cleanHint",
  };
}

export type SyncKind = "none" | "synced" | "ahead" | "behind" | "diverged";

export interface SyncStatus {
  kind: SyncKind;
  ahead: number;
  behind: number;
}

/** Relation to the upstream branch; `none` when git reports no upstream. */
export function syncStatus(
  ahead?: number | null,
  behind?: number | null,
): SyncStatus {
  if (ahead == null && behind == null)
    return { kind: "none", ahead: 0, behind: 0 };
  const a = ahead ?? 0;
  const b = behind ?? 0;
  const kind: SyncKind =
    a > 0 && b > 0 ? "diverged" : a > 0 ? "ahead" : b > 0 ? "behind" : "synced";
  return { kind, ahead: a, behind: b };
}

export type RemovalOutcome = "removed" | "dirty" | "failed";

/**
 * Removes a worktree and reloads the lists, also after a failure: git may have deleted
 * part of it already. `dirty` (first attempt) changed nothing and asks for confirmation.
 */
export async function removeAndReload(o: {
  force: boolean;
  remove: () => Promise<unknown>;
  onError: (e: AppError) => void;
  reload: () => Promise<unknown>;
}): Promise<RemovalOutcome> {
  let outcome: RemovalOutcome = "removed";
  try {
    await o.remove();
  } catch (e) {
    const err = toAppError(e);
    if (isDirtyError(err) && !o.force) return "dirty";
    o.onError(err);
    outcome = "failed";
  }
  await o.reload();
  return outcome;
}
