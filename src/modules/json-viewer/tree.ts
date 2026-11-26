import type { JsonNode } from "./jsonApi";

/** Children fetched per page (`json_children`). */
export const PAGE_SIZE = 200;
/** Largest page the backend serves (`MAX_PAGE` in the reader). */
const MAX_PAGE = 10_000;
/** Most children of one container that revealing a node will load. */
export const REVEAL_LIMIT = 100_000;

export interface Entry {
  node: JsonNode;
  depth: number;
  expanded: boolean;
  /** Ids of the children loaded so far, in document order. */
  children: number[];
  loading: boolean;
  /** The page request in flight, if any. */
  pending?: Promise<void>;
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

export interface Reveal {
  /** The target node is now part of `rows()`. */
  found: boolean;
  /** The target, or the deepest ancestor reached. */
  id: number;
  /** Stopped because the next node is past the reveal limit. */
  tooFar?: boolean;
}

/** Keys and indexes of a JSONPath as the backend writes it (`$.a[3]["c d"]`). */
export function pathSegments(path: string): Array<string | number> {
  const re = /\[(\d+)\]|\[("(?:[^"\\]|\\.)*")\]|\.([^.[\]]+)/g;
  return Array.from(path.matchAll(re), ([, index, quoted, key]) => {
    if (index !== undefined) return Number(index);
    return quoted !== undefined ? (JSON.parse(quoted) as string) : key;
  });
}

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

  /** Loads the next page of children of `id` (or joins the request already in flight). */
  loadMore(id: number, limit = this.pageSize): Promise<void> {
    const e = this.entries.get(id);
    if (!e) return Promise.resolve();
    if (e.pending) return e.pending;
    if (this.remaining(id) === 0) return Promise.resolve();
    e.loading = true;
    e.pending = this.fetchPage(id, e, limit);
    return e.pending;
  }

  private async fetchPage(id: number, e: Entry, limit: number): Promise<void> {
    try {
      const page = await this.load(id, e.children.length, limit);
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
      e.pending = undefined;
    }
  }

  /**
   * Expands the ancestors of `targetId`, loading the pages needed to reach it. `path` is its
   * JSONPath; the id tells duplicate keys apart in the last step.
   */
  async reveal(
    path: string,
    targetId: number,
    {
      limit = REVEAL_LIMIT,
      stale = () => false,
    }: { limit?: number; stale?: () => boolean } = {},
  ): Promise<Reveal> {
    const segs = pathSegments(path);
    let id = this.rootId;
    for (const [i, seg] of segs.entries()) {
      const e = this.entries.get(id)!;
      if (stale() || !e.node.hasChildren) break;
      const wasExpanded = e.expanded;
      e.expanded = true;
      const child = () => {
        if (i === segs.length - 1)
          return e.children.includes(targetId) ? targetId : undefined;
        if (typeof seg === "number") return e.children[seg];
        return e.children.find((c) => {
          const n = this.entries.get(c)!.node;
          return n.key === seg && n.hasChildren;
        });
      };
      if (typeof seg === "number" && seg >= limit) return this.stopAt(id, true);
      let next = child();
      while (next === undefined && this.remaining(id) > 0) {
        const loaded = e.children.length;
        if (loaded >= limit) return this.stopAt(id, true);
        // An index says how far to go; a key is searched in growing chunks.
        const want =
          typeof seg === "number"
            ? Math.ceil((seg + 1 - loaded) / this.pageSize) * this.pageSize
            : loaded;
        try {
          await this.loadMore(
            id,
            Math.min(MAX_PAGE, Math.max(this.pageSize, want)),
          );
        } catch (err) {
          if (e.children.length === 0) e.expanded = wasExpanded;
          throw err;
        }
        if (stale() || e.children.length === loaded) break;
        next = child();
      }
      if (next === undefined) return { found: false, id, tooFar: false };
      id = next;
    }
    return { found: id === targetId, id };
  }

  /** Reveal gave up at `id`; it stays expanded when it has something to show. */
  private async stopAt(id: number, tooFar: boolean): Promise<Reveal> {
    const e = this.entries.get(id)!;
    if (e.children.length === 0) await this.loadMore(id);
    return { found: false, id, tooFar };
  }

  /** Position of node `id` in `rows()`, or -1 when it is not visible. */
  rowIndex(id: number): number {
    return this.rows().findIndex((r) => r.type === "node" && r.id === id);
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
