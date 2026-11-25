<script lang="ts">
  import Banner from "../../components/Banner.svelte";
  import Button from "../../components/Button.svelte";
  import EmptyState from "../../components/EmptyState.svelte";
  import Progress from "../../components/Progress.svelte";
  import Spinner from "../../components/Spinner.svelte";
  import type { Column } from "../../components/DataTable.svelte";
  import { formatBytes, formatDate } from "../../lib/format";
  import { t } from "../../lib/i18n/i18n.svelte";
  import type { Grouped } from "../../lib/types";
  import GroupedRows from "./GroupedRows.svelte";
  import {
    currentFolderIndex,
    FOLDER_KIND_KEYS,
    type DeleteProgress,
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
    grouped: boolean;
    /** Paths of the checked folders. */
    selected: ReadonlySet<string>;
    /** Total size of the checked folders (bytes). */
    selectedBytes: number;
    /** Progress of a running deletion, or `null`. */
    deleting: DeleteProgress | null;
    /** Paths being deleted right now. */
    busy: ReadonlySet<string>;
    ontoggle: (f: HeavyFolder) => void;
    onclear: () => void;
    oncancel: () => void;
    ondelete: () => void;
  }
  let {
    data,
    scanning,
    progress,
    cancelled,
    hasRoot,
    grouped,
    selected,
    selectedBytes,
    deleting,
    busy,
    ontoggle,
    onclear,
    oncancel,
    ondelete,
  }: Props = $props();

  const ratio = $derived(
    progress ? progressRatio(progress.done, progress.total) : null,
  );

  const columns: Column<HeavyFolder>[] = $derived([
    { key: "select", header: "", width: "36px" },
    {
      key: "kind",
      header: t("reclaim.folders.columns.kind"),
      width: "150px",
    },
    { key: "path", header: t("reclaim.folders.columns.path"), kind: "path" },
    {
      key: "size",
      header: t("common.columns.size"),
      width: "100px",
      align: "end",
      kind: "numeric",
    },
    {
      key: "lastUsed",
      header: t("reclaim.folders.columns.lastUsed"),
      width: "130px",
    },
  ]);
</script>

<p class="desc subtle">{t("reclaim.folders.description")}</p>
{#if !hasRoot}<p class="subtle">{t("reclaim.folders.noRoot")}</p>{/if}

{#if scanning}
  <div class="scan" role="status">
    <span>{t("reclaim.folders.scanning")}</span>
    {#if progress}
      <span class="subtle tabular">
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
    <div class="bar">
      <Progress
        value={ratio ?? undefined}
        label={t("reclaim.folders.scanning")}
      />
    </div>
    <Button size="sm" onclick={oncancel}>{t("common.buttons.cancel")}</Button>
  </div>
{/if}
{#if deleting}
  <div class="scan" role="status">
    <Spinner size={16} />
    <span class="tabular">
      {t("reclaim.deleteFolder.progress", {
        current: currentFolderIndex(deleting),
        total: deleting.total,
        size: formatBytes(deleting.freedBytes),
      })}
    </span>
  </div>
{/if}
{#if cancelled}
  <Banner tone="info">{t("reclaim.folders.cancelled")}</Banner>
{/if}

{#if data && data.total.count === 0 && !scanning}
  <EmptyState title={t("reclaim.folders.empty")} />
{:else if data}
  <GroupedRows
    {data}
    {grouped}
    {columns}
    showDisk
    ariaLabel={t("reclaim.tabs.folders")}
    rowKey={(f) => f.path}
    rowClass={(f) => (selected.has(f.path) ? "selected" : undefined)}
  >
    {#snippet cell(f, col)}
      {#if col.key === "select"}
        {#if busy.has(f.path)}
          <Spinner size={14} label={t("reclaim.deleteFolder.rowBusy")} />
        {:else}
          <input
            type="checkbox"
            checked={selected.has(f.path)}
            disabled={!!deleting}
            aria-label={t("reclaim.folders.select", { name: f.path })}
            onchange={() => ontoggle(f)}
          />
        {/if}
      {:else if col.key === "kind"}
        <span class="mono">{t(FOLDER_KIND_KEYS[f.kind])}</span>
      {:else if col.key === "path"}
        <span class="path" title={f.path}>{f.path}</span>
      {:else if col.key === "size"}
        {f.diskBytes != null
          ? formatBytes(f.diskBytes)
          : t("reclaim.common.unknown")}
      {:else}
        <span class="subtle">
          {f.lastUsedIso ? formatDate(f.lastUsedIso) : t("reclaim.common.none")}
        </span>
      {/if}
    {/snippet}
  </GroupedRows>

  {#if selected.size > 0}
    <div class="footer" role="region" aria-live="polite">
      <span class="count tabular">
        <b>{t("reclaim.folders.selected", { count: selected.size })}</b>
        <span class="subtle">·</span>
        <span class="mono">{formatBytes(selectedBytes)}</span>
      </span>
      <span class="spacer"></span>
      <Button size="sm" variant="ghost" disabled={!!deleting} onclick={onclear}>
        {t("reclaim.folders.clear")}
      </Button>
      <Button
        size="sm"
        variant="danger"
        loading={!!deleting}
        onclick={ondelete}
      >
        {deleting
          ? t("common.busy.deleting")
          : t("reclaim.folders.deleteSelected")}
      </Button>
    </div>
  {/if}
{/if}

<style>
  .desc {
    margin: 0 0 4px;
    font-size: 12.5px;
  }
  .scan {
    display: flex;
    gap: 12px;
    align-items: center;
    flex-wrap: wrap;
    font-size: 12.5px;
  }
  .bar {
    flex: 1 1 160px;
    min-width: 120px;
  }
  .footer {
    position: sticky;
    bottom: 0;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 14px;
    background: var(--surface-raised);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-md);
    font-size: 12.5px;
  }
  .spacer {
    flex: 1;
  }
  input[type="checkbox"] {
    margin: 0;
  }
</style>
