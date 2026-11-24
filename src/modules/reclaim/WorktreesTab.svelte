<script lang="ts">
  import Button from "../../components/Button.svelte";
  import Badge from "../../components/Badge.svelte";
  import EmptyState from "../../components/EmptyState.svelte";
  import type { Column } from "../../components/DataTable.svelte";
  import { formatBytes } from "../../lib/format";
  import { t } from "../../lib/i18n/i18n.svelte";
  import type { Grouped } from "../../lib/types";
  import GroupedRows from "./GroupedRows.svelte";
  import { REASON_KEYS, type ForgottenWorktree } from "./logic";

  interface Props {
    data: Grouped<ForgottenWorktree> | null;
    loading: boolean;
    hasRoot: boolean;
    days: number;
    grouped: boolean;
    onremove: (w: ForgottenWorktree) => void;
  }
  let { data, loading, hasRoot, days, grouped, onremove }: Props = $props();

  const columns: Column<ForgottenWorktree>[] = $derived([
    { key: "branch", header: t("common.columns.branch") },
    { key: "age", header: t("common.columns.lastCommit"), width: "140px" },
    {
      key: "size",
      header: t("common.columns.size"),
      width: "100px",
      align: "end",
      kind: "numeric",
    },
    {
      key: "state",
      header: t("reclaim.worktrees.columns.state"),
      width: "minmax(0, 1.3fr)",
    },
    { key: "actions", header: "", width: "auto", align: "end" },
  ]);
</script>

<p class="desc subtle">{t("reclaim.worktrees.description", { days })}</p>

{#if !hasRoot}
  <EmptyState title={t("reclaim.worktrees.noRoot")} />
{:else if loading && !data}
  <p class="subtle">{t("common.state.loading")}</p>
{:else if data && data.total.count === 0}
  <EmptyState title={t("reclaim.worktrees.empty")} />
{:else if data}
  <GroupedRows
    {data}
    {grouped}
    {columns}
    showDisk
    ariaLabel={t("reclaim.tabs.worktrees")}
    rowKey={(w) => w.worktree.path}
  >
    {#snippet cell(w, col)}
      {#if col.key === "branch"}
        <span class="stack">
          <span class="strong">{w.worktree.branch ?? "—"}</span>
          <span class="path subtle" title={w.worktree.path}>
            {w.worktree.path}
          </span>
        </span>
      {:else if col.key === "age"}
        {#if w.lastCommitAgeDays != null}
          {t("reclaim.worktrees.age", { count: w.lastCommitAgeDays })}
        {:else}
          {t("common.worktree.noCommits")}
        {/if}
      {:else if col.key === "size"}
        {w.diskBytes != null
          ? formatBytes(w.diskBytes)
          : t("reclaim.common.unknown")}
      {:else if col.key === "state"}
        <span class="badges">
          {#each w.reasons as r (r)}
            <Badge
              tone={r === "live_processes"
                ? "warning"
                : r === "stale"
                  ? "neutral"
                  : "success"}
            >
              {t(REASON_KEYS[r])}
            </Badge>
          {/each}
          {#if !w.worktree.exists}
            <Badge tone="warning">{t("common.worktree.missing")}</Badge>
          {/if}
          {#if w.worktree.dirtyCount > 0}
            <Badge tone="warning">
              {t("common.worktree.dirty", { count: w.worktree.dirtyCount })}
            </Badge>
          {/if}
          {#if w.liveProcesses > 0 && !w.reasons.includes("live_processes")}
            <Badge tone="warning">
              {t("reclaim.worktrees.processes", { count: w.liveProcesses })}
            </Badge>
          {/if}
        </span>
      {:else}
        <Button size="sm" variant="danger-ghost" onclick={() => onremove(w)}>
          {t("reclaim.actions.removeWorktree")}
        </Button>
      {/if}
    {/snippet}
  </GroupedRows>
{/if}

<style>
  .desc {
    margin: 0 0 4px;
    font-size: 12.5px;
  }
  .stack {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .strong {
    font-weight: var(--weight-medium);
  }
  .badges {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }
</style>
