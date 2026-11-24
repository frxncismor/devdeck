import { DEFAULT_LOCALE, LOCALES, messages, type Locale } from "./index";
import type { MessageKey } from "./keys";

export type { Locale } from "./index";
export type { MessageKey } from "./keys";

type Params = Record<string, string | number>;

function normalize(value: unknown): Locale {
  return LOCALES.includes(value as Locale) ? (value as Locale) : DEFAULT_LOCALE;
}

const state = $state<{ current: Locale }>({ current: DEFAULT_LOCALE });

/** Reactive current locale (rune-backed: reading `locale.current` in a component tracks it). */
export const locale = {
  get current(): Locale {
    return state.current;
  },
};

type Persist = (l: Locale) => void | Promise<void>;
let persist: Persist | undefined;

/** Registers the function that persists the language (wired to `set_config` in `api.ts`). */
export function setLocalePersister(fn: Persist | undefined): void {
  persist = fn;
}

/** Applies a language without persisting (used when loading the stored config). Unknown => `en`. */
export function applyLocale(value: unknown): Locale {
  const l = normalize(value);
  state.current = l;
  if (typeof document !== "undefined") document.documentElement.lang = l;
  return l;
}

/** Applies a language immediately and persists it. */
export async function setLocale(value: unknown): Promise<void> {
  const l = applyLocale(value);
  await persist?.(l);
}

function lookup(catalogue: unknown, key: string): string | undefined {
  const [ns, ...rest] = key.split(".");
  let node: unknown = (catalogue as Record<string, unknown>)[ns];
  for (const part of rest) {
    if (node === null || typeof node !== "object") return undefined;
    node = (node as Record<string, unknown>)[part];
  }
  return typeof node === "string" ? node : undefined;
}

function resolve(
  l: Locale,
  key: string,
  count: number | undefined,
): string | undefined {
  const cat = messages[l];
  if (count !== undefined) {
    const suffix =
      new Intl.PluralRules(l).select(count) === "one" ? "one" : "other";
    const plural = lookup(cat, `${key}_${suffix}`);
    if (plural !== undefined) return plural;
  }
  return lookup(cat, key);
}

/** Typed translator: `{name}` interpolation, `_one/_other` plurals (via `count`), fallback to `en`. */
export function t(key: MessageKey, params?: Params): string {
  const count = typeof params?.count === "number" ? params.count : undefined;
  const raw =
    resolve(state.current, key, count) ??
    resolve(DEFAULT_LOCALE, key, count) ??
    key;
  if (!params) return raw;
  return raw.replace(/\{(\w+)\}/g, (m, name: string) =>
    name in params ? String(params[name]) : m,
  );
}
