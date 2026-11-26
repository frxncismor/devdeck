import { describe, expect, it } from "vitest";
import type { Grouped } from "../../lib/types";
import {
  removeAndReload,
  ageSecs,
  filterGrouped,
  formatAheadBehind,
  isDirtyError,
  syncStatus,
  worktreeStatus,
  type Worktree,
} from "./logic";

function wt(over: Partial<Worktree>): Worktree {
  return {
    path: "/r/app",
    repoPath: "/r/app",
    detached: false,
    isMain: false,
    dirtyCount: 0,
    exists: true,
    ...over,
  };
}

describe("formatAheadBehind", () => {
  it("is null without upstream", () => {
    expect(formatAheadBehind(undefined, undefined)).toBeNull();
    expect(formatAheadBehind(null, null)).toBeNull();
  });
  it("omits zero sides and shows a check when in sync", () => {
    expect(formatAheadBehind(2, 3)).toBe("↑2 ↓3");
    expect(formatAheadBehind(2, 0)).toBe("↑2");
    expect(formatAheadBehind(0, 5)).toBe("↓5");
    expect(formatAheadBehind(0, 0)).toBe("✓");
  });
});

describe("ageSecs", () => {
  it("computes elapsed seconds and clamps future timestamps", () => {
    expect(ageSecs(1000, 1_060_000)).toBe(60);
    expect(ageSecs(2000, 1_000_000)).toBe(0);
  });
});

describe("filterGrouped", () => {
  const data: Grouped<Worktree> = {
    groups: [
      {
        project: { id: "a", name: "alpha" },
        items: [
          wt({ branch: "main", isMain: true }),
          wt({ path: "/w/a-feat", branch: "feat/login", dirtyCount: 2 }),
        ],
        totals: { count: 2, ramBytes: 0, diskBytes: 0 },
      },
      {
        project: { id: "b", name: "beta" },
        items: [wt({ path: "/r/beta", branch: "main" })],
        totals: { count: 1, ramBytes: 0, diskBytes: 0 },
      },
    ],
    total: { count: 3, ramBytes: 0, diskBytes: 0 },
  };

  it("keeps everything without filters", () => {
    const r = filterGrouped(data, { dirtyOnly: false, query: "" });
    expect(r.total.count).toBe(3);
    expect(r.groups).toHaveLength(2);
  });
  it("dirty-only drops clean worktrees and empty groups, recomputing totals", () => {
    const r = filterGrouped(data, { dirtyOnly: true, query: "" });
    expect(r.groups).toHaveLength(1);
    expect(r.groups[0].totals.count).toBe(1);
    expect(r.total.count).toBe(1);
  });
  it("query matches branch case-insensitively", () => {
    const r = filterGrouped(data, { dirtyOnly: false, query: " LOGIN " });
    expect(r.total.count).toBe(1);
    expect(r.groups[0].items[0].branch).toBe("feat/login");
  });
  it("query matches the path", () => {
    const r = filterGrouped(data, { dirtyOnly: false, query: "/r/beta" });
    expect(r.groups.map((g) => g.project?.name)).toEqual(["beta"]);
  });
});

describe("isDirtyError", () => {
  it("detects the dirty code", () => {
    expect(isDirtyError({ kind: "NotAllowed", code: "worktree_dirty" })).toBe(
      true,
    );
    expect(isDirtyError({ kind: "Git", code: "git_failed" })).toBe(false);
  });
});

describe("worktreeStatus", () => {
  it("explains a clean worktree in plain words", () => {
    expect(worktreeStatus(wt({}))).toEqual({
      kind: "clean",
      tone: "success",
      labelKey: "worktrees.status.clean",
      hintKey: "worktrees.status.cleanHint",
    });
  });

  it("counts changed files when dirty", () => {
    const s = worktreeStatus(wt({ dirtyCount: 3 }));
    expect(s.kind).toBe("dirty");
    expect(s.tone).toBe("warning");
    expect(s.count).toBe(3);
    expect(s.labelKey).toBe("worktrees.status.dirty");
    expect(s.hintKey).toBe("worktrees.status.dirtyHint");
  });

  it("flags a missing folder before anything else", () => {
    const s = worktreeStatus(wt({ exists: false, dirtyCount: 2 }));
    expect(s.kind).toBe("missing");
    expect(s.hintKey).toBe("worktrees.status.missingHint");
  });
});

describe("syncStatus", () => {
  it("has no upstream when both sides are unknown", () => {
    expect(syncStatus(null, undefined).kind).toBe("none");
  });
  it("is synced when both counts are zero", () => {
    expect(syncStatus(0, 0).kind).toBe("synced");
  });
  it("reports ahead only", () => {
    expect(syncStatus(2, 0)).toMatchObject({ kind: "ahead", ahead: 2 });
  });
  it("reports behind only", () => {
    expect(syncStatus(0, 4)).toMatchObject({ kind: "behind", behind: 4 });
  });
  it("reports diverged when both sides differ", () => {
    expect(syncStatus(1, 5)).toMatchObject({
      kind: "diverged",
      ahead: 1,
      behind: 5,
    });
  });
});

describe("removeAndReload", () => {
  const run = async (fail: unknown, force = false) => {
    const calls: string[] = [];
    const outcome = await removeAndReload({
      force,
      remove: async () => {
        calls.push("remove");
        if (fail) throw fail;
      },
      onError: () => calls.push("error"),
      reload: async () => {
        calls.push("reload");
      },
    });
    return { outcome, calls };
  };
  const dirty = { kind: "Git", code: "worktree_dirty" };
  const inUse = { kind: "Git", code: "worktree_in_use" };

  it("reloads after a successful removal", async () => {
    expect(await run(null)).toEqual({
      outcome: "removed",
      calls: ["remove", "reload"],
    });
  });
  it("reports the error and also reloads after a failure", async () => {
    expect(await run(inUse)).toEqual({
      outcome: "failed",
      calls: ["remove", "error", "reload"],
    });
  });
  it("asks for confirmation on a dirty worktree without reloading", async () => {
    expect(await run(dirty)).toEqual({ outcome: "dirty", calls: ["remove"] });
  });
  it("treats a dirty error on a forced removal as a failure", async () => {
    expect(await run(dirty, true)).toEqual({
      outcome: "failed",
      calls: ["remove", "error", "reload"],
    });
  });
});
