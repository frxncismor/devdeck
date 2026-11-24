// Hand-written mirror of the `devdeck-core` serde types (camelCase). See specs/001-devdeck-mvp/data-model.md.

export type Language = "en" | "es";
export type ProjectId = string;

export type Scope = { kind: "global" } | { kind: "project"; id: ProjectId };

export interface Config {
  /** Single repositories root folder (optional until the user picks one). */
  root?: string;
  scanDepth: number;
  baseBranch: string;
  scope: Scope;
  language: Language;
  forgottenDays: number;
  portsAutoRefreshSecs?: number;
}

export interface Project {
  id: ProjectId;
  name: string;
  mainPath: string;
  worktreePaths: string[];
}

export interface ProjectRef {
  id: ProjectId;
  name: string;
}

export interface Totals {
  count: number;
  ramBytes: number;
  diskBytes: number;
}

export interface Group<T> {
  /** `null`/absent = "No project" (the label is rendered by the frontend). */
  project?: ProjectRef | null;
  items: T[];
  totals: Totals;
}

export interface Grouped<T> {
  groups: Group<T>[];
  total: Totals;
}

export type ErrorKind =
  | "NotFound"
  | "NotAllowed"
  | "Git"
  | "Io"
  | "Parse"
  | "Cancelled"
  | "Unsupported"
  | "NotImplemented";

export interface AppError {
  kind: ErrorKind;
  code: string;
  params?: Record<string, string | number>;
  detail?: string;
}
