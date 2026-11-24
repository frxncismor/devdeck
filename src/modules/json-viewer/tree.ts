import type { JsonNode } from "./jsonApi";

/** Children fetched per page (`json_children`). */
export const PAGE_SIZE = 200;

export interface Entry {
  node: JsonNode;
  depth: number;
  expanded: boolean;
  /** Ids of the children loaded so far, in document order. */
  children: number[];
  loading: boolean;
}

export type Row =
  | { type: "node"; id: number; depth: number; entry: Entry }
  | {
      type: "more";
      parentId: number;
      depth: number;
      remaining: number;
      loading: boolean;
    };

export type Loader = (
  id: number,
  offset: number,
  limit: number,
) => Promise<JsonNode[]>;

/**
 * Lazy tree state kept as a map of loaded containers. `rows()` flattens the visible part
 * into a list for the virtualized renderer: collapsed subtrees and unloaded pages cost nothing.
 */
export class TreeModel {
  readonly entries = new Map<number, Entry>();
  readonly rootId: number;

  constructor(
    root: JsonNode,
    private readonly load: Loader,
    private readonly pageSize = PAGE_SIZE,
  ) {
    this.rootId = root.id;
    this.entries.set(root.id, {
      node: root,
      depth: 0,
      expanded: false,
      children: [],
      loading: false,
    });
  }

  remaining(id: number): number {
    const e = this.entries.get(id);
    if (!e) return 0;
    return Math.max(0, (e.node.childCount ?? 0) - e.children.length);
  }

  /** Expands (loading the first page on demand) or collapses a container. */
  async toggle(id: number): Promise<void> {
    const e = this.entries.get(id);
    if (!e || !e.node.hasChildren) return;
    if (e.expanded) {
      e.expanded = false;
      return;
    }
    e.expanded = true;
    if (e.children.length === 0) {
      try {
        await this.loadMore(id);
      } catch (err) {
        e.expanded = false;
        throw err;
      }
    }
  }

  /** Loads the next page of children of `id`. */
  async loadMore(id: number): Promise<void> {
    const e = this.entries.get(id);
    if (!e || e.loading || this.remaining(id) === 0) return;
    e.loading = true;
    try {
      const page = await this.load(id, e.children.length, this.pageSize);
      for (const n of page) {
        if (this.entries.has(n.id)) continue;
        this.entries.set(n.id, {
          node: n,
          depth: e.depth + 1,
          expanded: false,
          children: [],
          loading: false,
        });
        e.children.push(n.id);
      }
    } finally {
      e.loading = false;
    }
  }

  /** Flat list of visible rows (pre-order), including "load more" rows. */
  rows(): Row[] {
    const out: Row[] = [];
    type Work =
      | { t: "node"; id: number }
      | { t: "more"; parentId: number; depth: number };
    const stack: Work[] = [{ t: "node", id: this.rootId }];
    while (stack.length > 0) {
      const w = stack.pop()!;
      if (w.t === "more") {
        const parent = this.entries.get(w.parentId)!;
        out.push({
          type: "more",
          parentId: w.parentId,
          depth: w.depth,
          remaining: this.remaining(w.parentId),
          loading: parent.loading,
        });
        continue;
      }
      const e = this.entries.get(w.id);
      if (!e) continue;
      out.push({ type: "node", id: w.id, depth: e.depth, entry: e });
      if (!e.expanded) continue;
      if (this.remaining(w.id) > 0)
        stack.push({ t: "more", parentId: w.id, depth: e.depth + 1 });
      for (let i = e.children.length - 1; i >= 0; i--)
        stack.push({ t: "node", id: e.children[i] });
    }
    return out;
  }
}
