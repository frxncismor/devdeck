import { describe, expect, it } from "vitest";
import { messages } from "./index";
import { applyLocale, setLocale, setLocalePersister, t } from "./i18n.svelte";
import type { MessageKey } from "./keys";

type Json = { [k: string]: string | Json };

function flatten(obj: Json, prefix = ""): Record<string, string> {
  const out: Record<string, string> = {};
  for (const [k, v] of Object.entries(obj)) {
    if (typeof v === "string") out[prefix + k] = v;
    else Object.assign(out, flatten(v, `${prefix}${k}.`));
  }
  return out;
}

const placeholders = (s: string) =>
  [...s.matchAll(/\{(\w+)\}/g)].map((m) => m[1]).sort();

describe("i18n parity (en vs es)", () => {
  for (const ns of Object.keys(messages.en) as (keyof typeof messages.en)[]) {
    it(`namespace "${ns}" has identical keys and placeholders`, () => {
      const en = flatten(messages.en[ns] as Json);
      const es = flatten(messages.es[ns] as Json);
      expect(Object.keys(es).sort()).toEqual(Object.keys(en).sort());
      for (const key of Object.keys(en)) {
        expect(placeholders(es[key]), `placeholders of ${ns}.${key}`).toEqual(
          placeholders(en[key]),
        );
      }
    });
  }

  it("has the same namespaces in both languages", () => {
    expect(Object.keys(messages.es).sort()).toEqual(
      Object.keys(messages.en).sort(),
    );
  });
});

describe("t() literals used in src exist in en", () => {
  const sources = import.meta.glob(["/src/**/*.svelte", "/src/**/*.ts"], {
    query: "?raw",
    import: "default",
    eager: true,
  }) as Record<string, string>;

  const known = new Set<string>();
  for (const [ns, cat] of Object.entries(messages.en)) {
    for (const k of Object.keys(flatten(cat as Json))) {
      known.add(`${ns}.${k}`);
      known.add(`${ns}.${k}`.replace(/_(one|other)$/, ""));
    }
  }

  it("finds every literal key", () => {
    const missing: string[] = [];
    for (const [file, text] of Object.entries(sources)) {
      if (file.endsWith(".test.ts")) continue;
      for (const m of text.matchAll(/\bt\(\s*["'`]([\w.-]+)["'`]/g)) {
        if (!known.has(m[1])) missing.push(`${file}: ${m[1]}`);
      }
    }
    expect(missing).toEqual([]);
  });
});

describe("t()", () => {
  it("defaults to English", () => {
    applyLocale("en");
    expect(t("common.noProject")).toBe("No project");
  });

  it("switches language and updates <html lang>", async () => {
    await setLocale("es");
    expect(t("common.noProject")).toBe("Sin proyecto");
    expect(document.documentElement.lang).toBe("es");
    await setLocale("en");
    expect(document.documentElement.lang).toBe("en");
  });

  it("falls back to en for an unknown stored language", () => {
    applyLocale("fr");
    expect(t("common.noProject")).toBe("No project");
  });

  it("interpolates {name} params", () => {
    applyLocale("es");
    expect(t("common.errors.json_invalid_at_offset", { offset: 42 })).toBe(
      "JSON no válido en el byte 42",
    );
  });

  it("selects _one/_other plurals", () => {
    applyLocale("en");
    expect(t("common.table.items", { count: 1 })).toBe("1 item");
    expect(t("common.table.items", { count: 3 })).toBe("3 items");
    applyLocale("es");
    expect(t("common.table.items", { count: 1 })).toBe("1 elemento");
    expect(t("common.table.items", { count: 0 })).toBe("0 elementos");
  });

  it("falls back to the key itself when missing everywhere", () => {
    expect(t("common.nope" as MessageKey)).toBe("common.nope");
  });

  it("persists through the registered persister", async () => {
    const seen: string[] = [];
    setLocalePersister((l) => {
      seen.push(l);
    });
    await setLocale("es");
    expect(seen).toEqual(["es"]);
    setLocalePersister(undefined);
    applyLocale("en");
  });
});

describe("backend error codes", () => {
  const rust = import.meta.glob(
    ["/crates/**/src/**/*.rs", "/src-tauri/src/**/*.rs"],
    { query: "?raw", import: "default", eager: true },
  ) as Record<string, string>;

  it("has a common.errors translation for every code Rust emits", () => {
    const codes = new Set<string>();
    for (const text of Object.values(rust)) {
      // Drop the unit tests: they may use throwaway codes.
      const prod = text.split("#[cfg(test)]")[0];
      for (const m of prod.matchAll(/Error::\w+\([^)]*?"([a-z][a-z_]*)"\s*\)/g))
        codes.add(m[1]);
    }
    expect(codes.size).toBeGreaterThan(10);
    const known = Object.keys(messages.en.common.errors);
    expect([...codes].filter((c) => !known.includes(c))).toEqual([]);
  });
});

describe("i18n strings have no unresolved braces", () => {
  for (const l of ["en", "es"] as const) {
    it(`every "${l}" string only uses braces as {placeholder}`, () => {
      const flat = flatten(messages[l] as unknown as Json);
      const bad = Object.entries(flat).filter(([, s]) =>
        /[{}]/.test(s.replace(/\{\w+\}/g, "")),
      );
      expect(bad.map(([k]) => k)).toEqual([]);
    });
  }
});

describe("plural keys resolve with a numeric count", () => {
  it("jsonViewer.tree.keys/items never fall back to the raw key", () => {
    for (const l of ["en", "es"] as const) {
      applyLocale(l);
      for (const count of [1, 2, 1234]) {
        for (const key of [
          "jsonViewer.tree.keys",
          "jsonViewer.tree.items",
        ] as MessageKey[]) {
          const out = t(key, { count, n: String(count) });
          expect(out).not.toBe(key);
          expect(out).toContain(String(count));
          expect(out).not.toMatch(/[{}]/);
        }
      }
    }
    applyLocale("en");
    expect(t("jsonViewer.tree.keys", { count: 1, n: "1" })).toBe("1 key");
    expect(t("jsonViewer.tree.items", { count: 1234, n: "1,234" })).toBe(
      "1,234 items",
    );
  });
});
