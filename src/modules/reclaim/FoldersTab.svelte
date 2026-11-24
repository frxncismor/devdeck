<script lang="ts">
  import GroupedTable from "../../components/GroupedTable.svelte";
  import { formatBytes, formatDate } from "../../lib/format";
  import { t } from "../../lib/i18n/i18n.svelte";
  import type { Grouped } from "../../lib/types";
  import {
    FOLDER_KIND_KEYS,
    progressRatio,
    type HeavyFolder,
    type JobProgress,
  } from "./logic";

  interface Props {
    data: Grouped<HeavyFolder> | null;
    scanning: boolean;
    progress: JobProgress | null;
    cancelled: boolean;
    hasRoot: boolean;
    oncancel: () => void;
    ondelete: (f: HeavyFolder) => void;
  }
  let {
    data,
    scanning,
    progress,
    cancelled,
    hasRoot,
    oncancel,
    ondelete,
  }: Props = $props();

  const ratio = $derived(
    progress ? progressRatio(progress.done, progress.total) : null,
  );
</script>

<p class="desc">{t("reclaim.folders.description")}</p>
{#if !hasRoot}<p class="hint">{t("reclaim.folders.noRoot")}</p>{/if}

{#if scanning}
  <div class="scan" role="status">
    <span>{t("reclaim.folders.scanning")}</span>
    {#if progress}
      <span>
        {#if progress.total}
          {t("reclaim.folders.measured", {
            done: progress.done,
            total: progress.total,
          })}
        {:else}
          {t("reclaim.folders.found", { count: progress.done })}
        {/if}
      </span>
    {/if}
    {#if ratio !== null}
      <progress value={ratio} max="1"></progress>
    {:else}
      <progress></progress>
    {/if}
    <button type="button" onclick={oncancel}>
      {t("common.buttons.cancel")}
    </button>
  </div>
{/if}
{#if cancelled}<p class="hint">{t("reclaim.folders.cancelled")}</p>{/if}

{#if data && data.total.count === 0 && !scanning}
  <p>{t("reclaim.folders.empty")}</p>
{:else if data}
  <GroupedTable {data} showDisk>
    {#snippet head()}
      <tr>
        <th>{t("reclaim.folders.columns.path")}</th>
        <th>{t("reclaim.folders.columns.kind")}</th>
        <th>{t("common.columns.size")}</th>
        <th>{t("reclaim.folders.columns.lastUsed")}</th>
        <th>{t("common.columns.actions")}</th>
      </tr>
    {/snippet}
    {#snippet row(f: HeavyFolder)}
      <tr>
        <td class="path" title={f.path}>{f.path}</td>
        <td>{t(FOLDER_KIND_KEYS[f.kind])}</td>
        <td>
          {f.diskBytes != null
            ? formatBytes(f.diskBytes)
            : t("reclaim.common.unknown")}
        </td>
        <td>
          {f.lastUsedIso ? formatDate(f.lastUsedIso) : t("reclaim.common.none")}
        </td>
        <td>
          <button type="button" onclick={() => ondelete(f)}>
            {t("reclaim.actions.deleteFolder")}
          </button>
        </td>
      </tr>
    {/snippet}
  </GroupedTable>
{/if}

<style>
  .desc,
  .hint {
    opacity: 0.8;
  }
  .scan {
    display: flex;
    gap: 0.75rem;
    align-items: center;
    flex-wrap: wrap;
    margin-bottom: 0.5rem;
  }
  .path {
    word-break: break-all;
    max-width: 32rem;
  }
</style>
