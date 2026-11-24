<script lang="ts">
  import GroupedTable from "../../components/GroupedTable.svelte";
  import { formatBytes } from "../../lib/format";
  import { t } from "../../lib/i18n/i18n.svelte";
  import type { Grouped } from "../../lib/types";
  import { REASON_KEYS, type ForgottenWorktree } from "./logic";

  interface Props {
    data: Grouped<ForgottenWorktree> | null;
    loading: boolean;
    hasRoot: boolean;
    days: number;
    onremove: (w: ForgottenWorktree) => void;
  }
  let { data, loading, hasRoot, days, onremove }: Props = $props();
</script>

<p class="desc">{t("reclaim.worktrees.description", { days })}</p>

{#if !hasRoot}
  <p>{t("reclaim.worktrees.noRoot")}</p>
{:else if loading && !data}
  <p>{t("common.state.loading")}</p>
{:else if data && data.total.count === 0}
  <p>{t("reclaim.worktrees.empty")}</p>
{:else if data}
  <GroupedTable {data} showDisk>
    {#snippet head()}
      <tr>
        <th>{t("reclaim.worktrees.columns.branch")}</th>
        <th>{t("reclaim.worktrees.columns.age")}</th>
        <th>{t("reclaim.worktrees.columns.size")}</th>
        <th>{t("reclaim.worktrees.columns.state")}</th>
        <th>{t("reclaim.worktrees.columns.actions")}</th>
      </tr>
    {/snippet}
    {#snippet row(w: ForgottenWorktree)}
      <tr>
        <td title={w.worktree.path}>
          {w.worktree.branch ?? "—"}
          <small class="path">{w.worktree.path}</small>
        </td>
        <td>
          {#if w.lastCommitAgeDays != null}
            {t("reclaim.worktrees.age", { count: w.lastCommitAgeDays })}
          {:else}
            {t("reclaim.worktrees.noCommits")}
          {/if}
        </td>
        <td>
          {w.diskBytes != null
            ? formatBytes(w.diskBytes)
            : t("reclaim.common.unknown")}
        </td>
        <td>
          {#each w.reasons as r (r)}
            <span class="badge {r}">{t(REASON_KEYS[r])}</span>
          {/each}
          {#if !w.worktree.exists}
            <span class="badge warn">{t("reclaim.worktrees.missing")}</span>
          {/if}
          {#if w.worktree.dirtyCount > 0}
            <span class="badge warn">
              {t("reclaim.worktrees.dirty", { count: w.worktree.dirtyCount })}
            </span>
          {/if}
          {#if w.liveProcesses > 0}
            <span class="badge warn">
              {t("reclaim.worktrees.processes", { count: w.liveProcesses })}
            </span>
          {/if}
        </td>
        <td>
          <button type="button" onclick={() => onremove(w)}>
            {t("reclaim.actions.removeWorktree")}
          </button>
        </td>
      </tr>
    {/snippet}
  </GroupedTable>
{/if}

<style>
  .desc {
    opacity: 0.8;
  }
  .path {
    display: block;
    opacity: 0.6;
    word-break: break-all;
  }
  .badge {
    display: inline-block;
    margin-right: 0.25rem;
    padding: 0 0.4rem;
    border-radius: 0.6rem;
    font-size: 0.75rem;
    background: #8882;
  }
  .badge.clean {
    background: #2e9e5b33;
  }
  .badge.warn,
  .badge.live_processes {
    background: #e0a40033;
  }
</style>
