import type { JsonKind } from "./jsonApi";
import { parsePath } from "./recent";

/** Nesting level of the node a JSONPath points to (root = 0). */
export function depthOfPath(path: string): number {
  return parsePath(path).length - 1;
}

/**
 * Type and child count of a node from its raw JSON text (`json_value`). Used when the selection
 * comes from a search hit, which carries no node data. `childCount` is `null` for leaves and for
 * containers whose text was cut (unparseable).
 */
export function describeValue(
  raw: string,
): { kind: JsonKind; childCount: number | null } | null {
  const text = raw.trimStart();
  const c = text[0];
  if (c === undefined) return null;
  if (c === "{" || c === "[") {
    const kind: JsonKind = c === "{" ? "object" : "array";
    try {
      const v = JSON.parse(text) as unknown;
      const count = Array.isArray(v)
        ? v.length
        : Object.keys(v as object).length;
      return { kind, childCount: count };
    } catch {
      return { kind, childCount: null };
    }
  }
  if (c === '"') return { kind: "string", childCount: null };
  if (c === "t" || c === "f") return { kind: "bool", childCount: null };
  if (c === "n") return { kind: "null", childCount: null };
  if (c === "-" || (c >= "0" && c <= "9"))
    return { kind: "number", childCount: null };
  return null;
}
