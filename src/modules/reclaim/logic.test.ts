import { beforeEach, describe, expect, it } from "vitest";
import { applyLocale } from "../../lib/i18n/i18n.svelte";
import { messages } from "../../lib/i18n";
import type { Grouped } from "../../lib/types";
import {
  countWhere,
  DEV_KIND_KEYS,
  FOLDER_KIND_KEYS,
  folderRejectionKey,
  formatCpu,
  hasRecoverable,
  progressRatio,
  projectSlices,
  REASON_KEYS,
  removalWarnings,
  sortedByProject,
  totalsOf,
  worktreeOf,
  worktreeSlices,
  type DevProcess,
  type ForgottenWorktree,
  type ProjectSummary,
  type ReclaimSummary,
} from "./logic";

beforeEach(() => applyLocale("en"));

function ps(over: Partial<ProjectSummary>): ProjectSummary {
  return {
    project: { id: "p", name: "p" },
    ramBytes: 0,
    diskBytes: 0,
    processes: 0,
    worktrees: 0,
    folders: 0,
    ...over,
  };
}

function summary(rows: ProjectSummary[]): ReclaimSummary {
  return {
    recoverableRamBytes: rows.reduce((n, r) => n + r.ramBytes, 0),
    recoverableDiskBytes: rows.reduce((n, r) => n + r.diskBytes, 0),
    byProject: rows,
  };
}

describe("formatCpu", () => {
  it("renders a percentage with at most one decimal", () => {
    expect(formatCpu(12.5)).toBe("12.5%");
    expect(formatCpu(0)).toBe("0%");
    expect(formatCpu(100)).toBe("100%");
  });
  it("localizes the decimal separator", () => {
    applyLocale("es");
    expect(formatCpu(12.5)).toMatch(/^12,5\s?%$/);
  });
  it("clamps negatives and NaN to zero", () => {
    expect(formatCpu(-3)).toBe("0%");
    expect(formatCpu(Number.NaN)).toBe("0%");
  });
});

describe("progressRatio", () => {
  it("is null without a usable total", () => {
    expect(progressRatio(3, undefined)).toBeNull();
    expect(progressRatio(3, null)).toBeNull();
    expect(progressRatio(3, 0)).toBeNull();
  });
  it("is clamped between 0 and 1", () => {
    expect(progressRatio(1, 4)).toBe(0.25);
    expect(progressRatio(9, 4)).toBe(1);
    expect(progressRatio(-1, 4)).toBe(0);
  });
});

describe("summary helpers", () => {
  it("hasRecoverable is false for null and empty summaries", () => {
    expect(hasRecoverable(null)).toBe(false);
    expect(hasRecoverable(summary([]))).toBe(false);
    expect(hasRecoverable(summary([ps({ ramBytes: 1 })]))).toBe(true);
    expect(hasRecoverable(summary([ps({ diskBytes: 1 })]))).toBe(true);
  });

  it("sorts projects by recoverable size and keeps No project last", () => {
    const s = summary([
      ps({ project: null, ramBytes: 10_000 }),
      ps({ project: { id: "a", name: "a" }, ramBytes: 1, diskBytes: 1 }),
      ps({ project: { id: "b", name: "b" }, diskBytes: 500 }),
    ]);
    expect(sortedByProject(s).map((r) => r.project?.name ?? null)).toEqual([
      "b",
      "a",
      null,
    ]);
    // does not mutate the input
    expect(s.byProject[0].project).toBeNull();
  });
});

describe("grouped helpers", () => {
  const data: Grouped<{ orphan: boolean }> = {
    groups: [
      {
        project: null,
        items: [{ orphan: true }, { orphan: false }],
        totals: { count: 2, ramBytes: 5, diskBytes: 0 },
      },
      {
        project: { id: "p", name: "p" },
        items: [{ orphan: true }],
        totals: { count: 1, ramBytes: 1, diskBytes: 0 },
      },
    ],
    total: { count: 3, ramBytes: 6, diskBytes: 0 },
  };
  it("counts items across groups", () => {
    expect(countWhere(data, (i) => i.orphan)).toBe(2);
    expect(countWhere(null, () => true)).toBe(0);
  });
  it("totalsOf falls back to zeros", () => {
    expect(totalsOf(data).ramBytes).toBe(6);
    expect(totalsOf(null)).toEqual({ count: 0, ramBytes: 0, diskBytes: 0 });
  });
});

describe("removalWarnings", () => {
  it("reports uncommitted changes and live processes", () => {
    const w = {
      worktree: { dirtyCount: 3 },
      liveProcesses: 2,
    } as unknown as ForgottenWorktree;
    expect(removalWarnings(w)).toEqual({ dirty: 3, live: 2 });
  });
});

describe("folderRejectionKey", () => {
  const rejected = (reason?: string) => ({
    kind: "NotAllowed" as const,
    code: "folder_not_allowed",
    params: reason ? { reason } : undefined,
  });
  it("maps every backend reason to a key", () => {
    for (const r of [
      "link",
      "not_a_directory",
      "drive_root",
      "home",
      "root",
      "outside_root",
      "not_allowlisted",
    ]) {
      expect(folderRejectionKey(rejected(r))).toBe(
        `reclaim.errors.reason.${r}`,
      );
    }
  });
  it("is null for unknown reasons and other errors", () => {
    expect(folderRejectionKey(rejected("weird"))).toBeNull();
    expect(folderRejectionKey(rejected())).toBeNull();
    expect(folderRejectionKey({ kind: "Io", code: "io_error" })).toBeNull();
  });
});

describe("translation keys", () => {
  function has(key: string): boolean {
    let node: unknown = messages.en;
    for (const part of key.split(".")) {
      node = (node as Record<string, unknown> | undefined)?.[part];
    }
    return typeof node === "string";
  }
  it("every kind, folder kind, reason and rejection key exists in en and es", () => {
    const keys = [
      ...Object.values(DEV_KIND_KEYS),
      ...Object.values(FOLDER_KIND_KEYS),
      ...Object.values(REASON_KEYS),
      "reclaim.errors.reason.link",
      "reclaim.errors.reason.home",
    ];
    for (const k of keys) {
      expect(has(k), k).toBe(true);
      let node: unknown = messages.es;
      for (const part of k.split(".")) {
        node = (node as Record<string, unknown> | undefined)?.[part];
      }
      expect(typeof node, `es ${k}`).toBe("string");
    }
  });
});

describe("DevProcess shape", () => {
  it("is what the backend serializes (flattened ProcInfo + extras)", () => {
    const p: DevProcess = {
      pid: 1,
      name: "node.exe",
      ramBytes: 1,
      cpuPercent: 0,
      startTime: 0,
      kind: "rustAnalyzer",
      orphan: false,
      ageSecs: 0,
    };
    expect(DEV_KIND_KEYS[p.kind]).toBe("reclaim.kind.rustAnalyzer");
  });
});

describe("slices", () => {
  it("projectSlices drops empty projects and sorts biggest first", () => {
    const s = summary([
      ps({ project: { id: "a", name: "A" }, ramBytes: 5 }),
      ps({ project: { id: "b", name: "B" }, ramBytes: 0 }),
      ps({ project: null, ramBytes: 9 }),
    ]);
    expect(projectSlices(s, "ramBytes", "None")).toEqual([
      { key: null, label: "None", value: 9 },
      { key: "a", label: "A", value: 5 },
    ]);
  });

  it("worktreeOf picks the longest containing root (Windows paths too)", () => {
    const roots = ["C:\\dev\\orbit", "C:\\dev\\orbit.wt\\feat"];
    expect(worktreeOf("c:/dev/orbit.wt/feat/node_modules", roots)).toBe(
      "C:\\dev\\orbit.wt\\feat",
    );
    expect(worktreeOf("C:\\dev\\orbit\\dist", roots)).toBe("C:\\dev\\orbit");
    expect(worktreeOf("C:\\dev\\orbit2\\dist", roots)).toBeNull();
    expect(worktreeOf(null, roots)).toBeNull();
  });

  it("worktreeSlices sums per worktree and labels by folder name", () => {
    const items = [
      { p: "/r/main/a", n: 3 },
      { p: "/r/main/b", n: 4 },
      { p: "/r/wt/x/c", n: 10 },
      { p: "/elsewhere", n: 99 },
    ];
    expect(
      worktreeSlices(
        items,
        ["/r/main", "/r/wt/x"],
        (i) => i.p,
        (i) => i.n,
      ),
    ).toEqual([
      { key: "/r/wt/x", label: "x", value: 10 },
      { key: "/r/main", label: "main", value: 7 },
    ]);
  });
});
