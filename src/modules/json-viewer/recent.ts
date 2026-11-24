/** Recently opened JSON files, kept in localStorage (front-only, flat list of paths). */
const KEY = "devdeck.json.recent";
const MAX = 5;

function read(): string[] {
  try {
    const raw = JSON.parse(localStorage.getItem(KEY) ?? "[]") as unknown;
    return Array.isArray(raw)
      ? raw.filter((p): p is string => typeof p === "string").slice(0, MAX)
      : [];
  } catch {
    return [];
  }
}

function write(list: string[]): string[] {
  try {
    localStorage.setItem(KEY, JSON.stringify(list));
  } catch {
    // Storage unavailable: the list just lives for this session.
  }
  return list;
}

export const loadRecent = read;

/** Puts `path` first (deduplicated) and keeps the newest `MAX`. */
export function pushRecent(path: string): string[] {
  return write([path, ...read().filter((p) => p !== path)].slice(0, MAX));
}

export function removeRecent(path: string): string[] {
  return write(read().filter((p) => p !== path));
}

/** Splits a JSONPath such as `$.orders[1204]["a b"].customer` into breadcrumb segments. */
export function parsePath(path: string): string[] {
  const out: string[] = ["$"];
  const re = /\[(\d+)\]|\["((?:[^"\\]|\\.)*)"\]|\.([^.[\]]+)/g;
  let m: RegExpExecArray | null;
  while ((m = re.exec(path)) !== null) {
    if (m[1] !== undefined) out.push(`[${m[1]}]`);
    else if (m[2] !== undefined) out.push(m[2]);
    else out.push(m[3]);
  }
  return out;
}
