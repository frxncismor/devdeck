<script lang="ts">
  import { formatBytes } from "../../lib/format";
  import { t } from "../../lib/i18n/i18n.svelte";
  import {
    hasRecoverable,
    sortedByProject,
    type ReclaimSummary,
  } from "./logic";

  interface Props {
    summary: ReclaimSummary | null;
    /** Folder/worktree sizes are still being measured. */
    measuring: boolean;
    showProjects: boolean;
  }
  let { summary, measuring, showProjects }: Props = $props();
</script>

<section class="summary" aria-labelledby="reclaim-summary-title">
  <h3 id="reclaim-summary-title">{t("reclaim.summary.title")}</h3>
  {#if summary}
    <div class="cards">
      <div class="card">
        <span class="label">{t("reclaim.summary.ram")}</span>
        <strong>{formatBytes(summary.recoverableRamBytes)}</strong>
      </div>
      <div class="card">
        <span class="label">{t("reclaim.summary.disk")}</span>
        <strong>{formatBytes(summary.recoverableDiskBytes)}</strong>
      </div>
    </div>
    {#if measuring}<p class="hint">{t("reclaim.summary.sizing")}</p>{/if}
    {#if !hasRecoverable(summary)}
      <p>{t("reclaim.summary.nothing")}</p>
    {:else if showProjects && summary.byProject.length > 0}
      <h4>{t("reclaim.summary.byProject")}</h4>
      <table>
        <thead>
          <tr>
            <th>{t("reclaim.summary.columns.project")}</th>
            <th>{t("reclaim.summary.columns.ram")}</th>
            <th>{t("reclaim.summary.columns.disk")}</th>
            <th>{t("reclaim.summary.columns.details")}</th>
          </tr>
        </thead>
        <tbody>
          {#each sortedByProject(summary) as row (row.project?.id ?? "")}
            <tr>
              <td>{row.project ? row.project.name : t("common.noProject")}</td>
              <td>{formatBytes(row.ramBytes)}</td>
              <td>{formatBytes(row.diskBytes)}</td>
              <td>
                {t("reclaim.summary.details", {
                  processes: row.processes,
                  worktrees: row.worktrees,
                  folders: row.folders,
                })}
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
  {:else}
    <p>{t("common.state.loading")}</p>
  {/if}
</section>

<style>
  .cards {
    display: flex;
    gap: 1rem;
    flex-wrap: wrap;
  }
  .card {
    display: flex;
    flex-direction: column;
    padding: 0.5rem 1rem;
    border-radius: 0.5rem;
    background: #8881;
    min-width: 8rem;
  }
  .card strong {
    font-size: 1.5rem;
  }
  .label,
  .hint {
    opacity: 0.7;
    font-size: 0.85rem;
  }
</style>
