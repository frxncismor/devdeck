import type { ProjectRef } from "../../lib/types";

/** Mirror of `devdeck_core::ports::ListeningPort` (camelCase). */
export interface ListeningPort {
  port: number;
  protocol: "tcp" | "udp";
  pid: number;
  processName: string;
  exePath?: string | null;
  /** Absent when the process is protected/elevated: shown as unknown, not an error. */
  cwd?: string | null;
  worktreePath?: string | null;
  project?: ProjectRef | null;
}

/** Last path segment, accepting both separators and trailing ones. */
export function baseName(path: string): string {
  const parts = path.split(/[\\/]+/).filter(Boolean);
  return parts.length ? parts[parts.length - 1] : path;
}

/** Auto refresh interval in ms; `null` (off) for missing, zero or invalid values. Minimum 1 s. */
export function autoRefreshMs(secs?: number | null): number | null {
  if (secs == null || !Number.isFinite(secs) || secs <= 0) return null;
  return Math.max(1, Math.floor(secs)) * 1000;
}

/** Stable key for a row: one process can own several ports and protocols. */
export function rowKey(p: ListeningPort): string {
  return `${p.protocol}:${p.port}:${p.pid}`;
}
