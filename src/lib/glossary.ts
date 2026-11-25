import type { MessageKey } from "./i18n/keys";
import { REASON_HINT_KEYS, REASON_KEYS } from "../modules/reclaim/logic";

export type GlossaryModule = "worktrees" | "reclaim" | "ports";
type Params = Record<string, string | number>;

export interface GlossaryEntry {
  id: string;
  /** The text of the badge/label as shown in the UI (single source: the badge's own key). */
  termKey: MessageKey;
  /** Tooltip text of that badge (same key the badge uses). */
  meaningKey: MessageKey;
  /** "What to do" and a short example live in the `glossary` namespace. */
  todoKey: MessageKey;
  exampleKey: MessageKey;
  /** Sample values for the plural/placeholder keys above. */
  params?: Params;
}

function entry(
  id: string,
  termKey: MessageKey,
  meaningKey: MessageKey,
  params?: Params,
): GlossaryEntry {
  return {
    id,
    termKey,
    meaningKey,
    todoKey: `glossary.${id}.todo` as MessageKey,
    exampleKey: `glossary.${id}.example` as MessageKey,
    params,
  };
}

/** Params for rendering an entry: its sample values plus caller context (e.g. the configured days). */
export function entryParams(e: GlossaryEntry, extra: Params = {}): Params {
  return { ...extra, ...e.params };
}

export const GLOSSARY: Record<GlossaryModule, GlossaryEntry[]> = {
  worktrees: [
    entry("wt-clean", "worktrees.status.clean", "worktrees.status.cleanHint"),
    entry("wt-dirty", "worktrees.status.dirty", "worktrees.status.dirtyHint", {
      count: 3,
    }),
    entry(
      "wt-missing",
      "common.worktree.missing",
      "worktrees.status.missingHint",
    ),
    entry("wt-main", "worktrees.badge.main", "worktrees.badge.mainHint"),
    entry(
      "wt-detached",
      "worktrees.badge.detached",
      "worktrees.badge.detachedHint",
    ),
    entry(
      "wt-sync-none",
      "worktrees.sync.noUpstream",
      "worktrees.sync.noUpstreamHint",
    ),
    entry(
      "wt-sync-synced",
      "worktrees.sync.synced",
      "worktrees.sync.syncedHint",
    ),
    entry("wt-sync-ahead", "worktrees.sync.ahead", "worktrees.sync.aheadHint", {
      count: 2,
    }),
    entry(
      "wt-sync-behind",
      "worktrees.sync.behind",
      "worktrees.sync.behindHint",
      {
        count: 2,
      },
    ),
    entry(
      "wt-sync-diverged",
      "worktrees.sync.diverged",
      "worktrees.sync.divergedHint",
      {
        ahead: 1,
        behind: 2,
      },
    ),
  ],
  reclaim: [
    entry("rc-stale", REASON_KEYS.stale, REASON_HINT_KEYS.stale),
    entry("rc-merged", REASON_KEYS.merged, REASON_HINT_KEYS.merged),
    entry("rc-clean", REASON_KEYS.clean, REASON_HINT_KEYS.clean),
    entry(
      "rc-live",
      REASON_KEYS.live_processes,
      REASON_HINT_KEYS.live_processes,
    ),
    entry(
      "rc-orphan",
      "reclaim.processes.orphan",
      "reclaim.processes.orphanTitle",
    ),
    entry(
      "rc-regenerable",
      "reclaim.tabs.folders",
      "reclaim.folders.description",
    ),
  ],
  ports: [
    entry("pt-unknown", "ports.unknownPath", "ports.unknownPathHint"),
    entry("pt-noproject", "common.noProject", "ports.noProjectHint"),
  ],
};
