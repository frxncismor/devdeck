import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { api } from "../../lib/api";
import { errorMessage } from "../../lib/errors";
import { formatBytes } from "../../lib/format";
import { t } from "../../lib/i18n/i18n.svelte";
import type { MessageKey } from "../../lib/i18n/keys";
import type { AppError } from "../../lib/types";

export type JsonKind =
  "object" | "array" | "string" | "number" | "bool" | "null";

export interface JsonNode {
  id: number;
  /** Object member name. */
  key?: string | null;
  /** Position inside an array parent. */
  index?: number | null;
  kind: JsonKind;
  childCount?: number | null;
  /** Raw JSON text of a leaf (truncated). */
  preview?: string | null;
  hasChildren: boolean;
}

export interface SessionInfo {
  sessionId: string;
  path: string;
  sizeBytes: number;
  rootId: number;
  containers: number;
}

export interface SearchHit {
  nodeId: number;
  path: string;
  kind: "key" | "value";
  snippet: string;
}

export interface JobProgress {
  jobId: string;
  done: number;
  total?: number | null;
}

export interface JobDone {
  jobId: string;
  cancelled: boolean;
  error?: AppError;
  session?: SessionInfo;
  total?: number;
  truncated?: boolean;
  savedBytes?: number;
}

export interface SearchBatch {
  jobId: string;
  hits: SearchHit[];
}

export interface SearchParams {
  query: string;
  inKeys: boolean;
  inValues: boolean;
  caseSensitive: boolean;
  limit?: number;
}

export const jsonApi = {
  open: (path: string) =>
    api.call<{ jobId: string; sessionId: string }>("json_open", { path }),
  cancel: (jobId: string) => api.call<void>("json_cancel", { jobId }),
  close: (sessionId: string) => api.call<void>("json_close", { sessionId }),
  root: (sessionId: string) => api.call<JsonNode>("json_root", { sessionId }),
  children: (
    sessionId: string,
    nodeId: number,
    offset: number,
    limit: number,
  ) =>
    api.call<JsonNode[]>("json_children", {
      sessionId,
      nodeId,
      offset,
      limit,
    }),
  value: (sessionId: string, nodeId: number, maxBytes: number) =>
    api.call<string>("json_value", { sessionId, nodeId, maxBytes }),
  path: (sessionId: string, nodeId: number) =>
    api.call<string>("json_path", { sessionId, nodeId }),
  search: (sessionId: string, p: SearchParams) =>
    api.call<{ jobId: string }>("json_search", { sessionId, ...p }),
  formatNode: (sessionId: string, nodeId: number, maxBytes: number) =>
    api.call<string>("json_format_node", { sessionId, nodeId, maxBytes }),
  saveFormatted: (sessionId: string, nodeId: number | null, destPath: string) =>
    api.call<{ jobId: string }>("json_save_formatted", {
      sessionId,
      nodeId,
      destPath,
    }),
};

export interface JobHandlers {
  onProgress?: (p: JobProgress) => void;
  onBatch?: (b: SearchBatch) => void;
}

/**
 * Starts a backend job and routes its events. Listeners are attached before the command runs and
 * early events are buffered, so nothing emitted before the `jobId` is known is lost.
 */
export async function runJob<R extends { jobId: string }>(
  start: () => Promise<R>,
  handlers: JobHandlers = {},
): Promise<{ ref: R; done: Promise<JobDone> }> {
  let id: string | null = null;
  const buffered: Array<() => void> = [];
  let unlisten: UnlistenFn[] = [];
  let finish!: (d: JobDone) => void;
  const done = new Promise<JobDone>((resolve) => (finish = resolve));
  const cleanup = () => {
    unlisten.forEach((u) => u());
    unlisten = [];
  };
  const route = (jobId: string, fn: () => void) => {
    if (id === null)
      buffered.push(() => {
        if (jobId === id) fn();
      });
    else if (jobId === id) fn();
  };
  unlisten = await Promise.all([
    listen<JobProgress>("job://progress", (e) =>
      route(e.payload.jobId, () => handlers.onProgress?.(e.payload)),
    ),
    listen<SearchBatch>("json://search-batch", (e) =>
      route(e.payload.jobId, () => handlers.onBatch?.(e.payload)),
    ),
    listen<JobDone>("job://done", (e) =>
      route(e.payload.jobId, () => {
        cleanup();
        finish(e.payload);
      }),
    ),
  ]);
  let ref: R;
  try {
    ref = await start();
  } catch (e) {
    cleanup();
    throw e;
  }
  id = ref.jobId;
  buffered.splice(0).forEach((f) => f());
  return { ref, done };
}

const LOCAL_ERRORS: Record<string, MessageKey> = {
  json_value_too_large: "jsonViewer.errors.valueTooLarge",
  json_save_same_file: "jsonViewer.errors.saveSameFile",
  json_session_not_found: "jsonViewer.errors.sessionNotFound",
  json_node_not_found: "jsonViewer.errors.nodeNotFound",
  json_too_large: "jsonViewer.errors.tooLarge",
};

/** Module-specific error copy first, then the shared `common.errors.*` catalogue. */
export function jsonErrorMessage(e: AppError): string {
  const key = LOCAL_ERRORS[e.code];
  if (!key) return errorMessage(e);
  const params = { ...e.params };
  if (typeof params.maxBytes === "number")
    params.maxBytes = formatBytes(params.maxBytes);
  return t(key, params);
}
