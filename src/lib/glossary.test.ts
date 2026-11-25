import { beforeEach, describe, expect, it } from "vitest";
import { applyLocale, t } from "./i18n/i18n.svelte";
import { LOCALES } from "./i18n";
import {
  REASON_HINT_KEYS,
  REASON_KEYS,
  type ForgottenReason,
} from "../modules/reclaim/logic";
import {
  worktreeStatus,
  type SyncKind,
  type Worktree,
} from "../modules/worktrees/logic";
import { GLOSSARY, entryParams, type GlossaryEntry } from "./glossary";

const wt = (over: Partial<Worktree>): Worktree => ({
  path: "/r/a",
  repoPath: "/r/a",
  detached: false,
  isMain: false,
  dirtyCount: 0,
  exists: true,
  ...over,
});

const has = (list: GlossaryEntry[], termKey: string) =>
  list.some((e) => e.termKey === termKey);

describe("glossary covers every badge state", () => {
  it("worktree statuses (clean, dirty, missing) use the badge label keys", () => {
    for (const w of [wt({}), wt({ dirtyCount: 2 }), wt({ exists: false })]) {
      const s = worktreeStatus(w);
      const e = GLOSSARY.worktrees.find((x) => x.termKey === s.labelKey);
      expect(e, s.kind).toBeDefined();
      expect(e?.meaningKey).toBe(s.hintKey);
    }
  });

  it("every sync kind has an entry", () => {
    const kinds: SyncKind[] = ["none", "synced", "ahead", "behind", "diverged"];
    for (const k of kinds) {
      expect(
        GLOSSARY.worktrees.some((e) => e.id === `wt-sync-${k}`),
        k,
      ).toBe(true);
    }
  });

  it("main and detached badges have entries", () => {
    expect(has(GLOSSARY.worktrees, "worktrees.badge.main")).toBe(true);
    expect(has(GLOSSARY.worktrees, "worktrees.badge.detached")).toBe(true);
  });

  it("every Reclaim reason badge has an entry with its tooltip as meaning", () => {
    for (const r of Object.keys(REASON_KEYS) as ForgottenReason[]) {
      const e = GLOSSARY.reclaim.find((x) => x.termKey === REASON_KEYS[r]);
      expect(e, r).toBeDefined();
      expect(e?.meaningKey).toBe(REASON_HINT_KEYS[r]);
    }
  });

  it("Reclaim orphan and Ports unknown-owner badges have entries", () => {
    expect(has(GLOSSARY.reclaim, "reclaim.processes.orphan")).toBe(true);
    expect(has(GLOSSARY.ports, "ports.unknownPath")).toBe(true);
    expect(has(GLOSSARY.ports, "common.noProject")).toBe(true);
  });
});

describe("glossary texts exist in every language", () => {
  beforeEach(() => applyLocale("en"));
  for (const locale of LOCALES) {
    it(`resolves term, meaning, todo and example in ${locale}`, () => {
      applyLocale(locale);
      for (const e of Object.values(GLOSSARY).flat()) {
        const p = entryParams(e, { days: 30 });
        for (const key of [e.termKey, e.meaningKey, e.todoKey, e.exampleKey]) {
          const text = t(key, p);
          expect(text, `${e.id}:${key}`).not.toBe(key);
          expect(text.length).toBeGreaterThan(0);
        }
      }
    });
  }

  it("has unique ids", () => {
    const ids = Object.values(GLOSSARY)
      .flat()
      .map((e) => e.id);
    expect(new Set(ids).size).toBe(ids.length);
  });
});
