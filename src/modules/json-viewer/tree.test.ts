import { describe, expect, it } from "vitest";
import type { JsonNode } from "./jsonApi";
import { TreeModel, type Row } from "./tree";

function container(
  id: number,
  childCount: number,
  kind: "object" | "array" = "object",
  key?: string,
): JsonNode {
  return { id, key, kind, childCount, hasChildren: childCount > 0 };
}

function leaf(id: number, key: string): JsonNode {
  return { id, key, kind: "number", preview: "1", hasChildren: false };
}

/** root{ a[ x, y ], b{ c }, d(leaf), e[], big[450 leaves] }. */
function makeLoader() {
  const calls: Array<[number, number, number]> = [];
  const kids: Record<number, JsonNode[]> = {
    0: [
      container(1, 2, "array", "a"),
      container(2, 1, "object", "b"),
      leaf(100, "d"),
      container(3, 0, "array", "e"),
      container(4, 450, "array", "big"),
    ],
    1: [leaf(101, "x"), leaf(102, "y")],
    2: [leaf(103, "c")],
    4: Array.from({ length: 450 }, (_, i) => leaf(1000 + i, String(i))),
  };
  const load = async (id: number, offset: number, limit: number) => {
    calls.push([id, offset, limit]);
    return (kids[id] ?? []).slice(offset, offset + limit);
  };
  return { load, calls };
}

const ids = (rows: Row[]) =>
  rows.map((r) => (r.type === "node" ? r.id : `more:${r.parentId}`));

describe("TreeModel flattening", () => {
  it("shows only the root until something is expanded", () => {
    const { load, calls } = makeLoader();
    const m = new TreeModel(container(0, 5), load);
    expect(ids(m.rows())).toEqual([0]);
    expect(calls).toEqual([]);
  });

  it("expands lazily in document order with depths", async () => {
    const { load, calls } = makeLoader();
    const m = new TreeModel(container(0, 5), load);
    await m.toggle(0);
    expect(ids(m.rows())).toEqual([0, 1, 2, 100, 3, 4]);
    await m.toggle(1);
    await m.toggle(2);
    const rows = m.rows();
    expect(ids(rows)).toEqual([0, 1, 101, 102, 2, 103, 100, 3, 4]);
    const depths = rows.map((r) => (r.type === "node" ? r.depth : -1));
    expect(depths).toEqual([0, 1, 2, 2, 1, 2, 1, 1, 1]);
    // Collapsed containers are never fetched.
    expect(calls.map((c) => c[0])).toEqual([0, 1, 2]);
  });

  it("collapse hides descendants and re-expand does not refetch", async () => {
    const { load, calls } = makeLoader();
    const m = new TreeModel(container(0, 5), load);
    await m.toggle(0);
    await m.toggle(1);
    await m.toggle(0);
    expect(ids(m.rows())).toEqual([0]);
    await m.toggle(0);
    expect(ids(m.rows())).toEqual([0, 1, 101, 102, 2, 100, 3, 4]);
    expect(calls.length).toBe(2);
  });

  it("ignores leaves and empty containers", async () => {
    const { load, calls } = makeLoader();
    const m = new TreeModel(container(0, 5), load);
    await m.toggle(0);
    await m.toggle(100);
    await m.toggle(3);
    expect(calls.length).toBe(1);
    expect(m.entries.get(3)!.expanded).toBe(false);
  });

  it("pages big containers with a trailing load-more row", async () => {
    const { load, calls } = makeLoader();
    const m = new TreeModel(container(0, 5), load, 200);
    await m.toggle(0);
    await m.toggle(4);
    let rows = m.rows();
    expect(rows).toHaveLength(6 + 200 + 1);
    expect(rows.at(-1)).toMatchObject({
      type: "more",
      parentId: 4,
      depth: 2,
      remaining: 250,
    });
    await m.loadMore(4);
    await m.loadMore(4);
    rows = m.rows();
    expect(rows).toHaveLength(6 + 450);
    expect(rows.some((r) => r.type === "more")).toBe(false);
    expect(calls.filter((c) => c[0] === 4)).toEqual([
      [4, 0, 200],
      [4, 200, 200],
      [4, 400, 200],
    ]);
    await m.loadMore(4); // nothing left: no extra request
    expect(calls.filter((c) => c[0] === 4)).toHaveLength(3);
  });

  it("places the load-more row after the loaded children", async () => {
    const { load } = makeLoader();
    const m = new TreeModel(container(0, 5), load, 1);
    await m.toggle(0); // 1 of 5 children loaded
    expect(ids(m.rows())).toEqual([0, 1, "more:0"]);
  });

  it("collapses again when the first page fails", async () => {
    const m = new TreeModel(container(0, 2), async () => {
      throw new Error("boom");
    });
    await expect(m.toggle(0)).rejects.toThrow("boom");
    expect(m.entries.get(0)!.expanded).toBe(false);
    expect(ids(m.rows())).toEqual([0]);
  });

  it("flattens a very deep chain without recursion", async () => {
    const depth = 20000;
    const load = async (id: number) => [
      container(id + 1, id + 1 < depth ? 1 : 0),
    ];
    const m = new TreeModel(container(0, 1), load);
    for (let i = 0; i < depth - 1; i++) await m.toggle(i);
    expect(m.rows()).toHaveLength(depth);
  });
});
