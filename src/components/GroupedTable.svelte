<script lang="ts" generics="T">
  import type { Snippet } from "svelte";
  import Group from "./Group.svelte";
  import { t } from "../lib/i18n/i18n.svelte";
  import { formatBytes } from "../lib/format";
  import { projectColor } from "../lib/projectColor";
  import { scopeState } from "../lib/scope.svelte";
  import type { Grouped } from "../lib/types";

  interface Props {
    data: Grouped<T>;
    /** One `<tr>` per item. */
    row: Snippet<[T]>;
    /** Header `<tr>` (rendered in a `<thead>` of every group). */
    head?: Snippet;
    showRam?: boolean;
    showDisk?: boolean;
    /** Grand-total strip above the groups (count + RAM/Disk per `showRam`/`showDisk`). */
    showTotal?: boolean;
  }
  let {
    data,
    row,
    head,
    showRam = false,
    showDisk = false,
    showTotal = false,
  }: Props = $props();

  const ids = $derived(scopeState.projects.map((p) => p.id));
  const pathOf = (id: string | undefined) =>
    scopeState.projects.find((p) => p.id === id)?.mainPath;
</script>

<div class="gt">
  {#if showTotal}
    <div class="total">
      <strong>{t("common.table.total")}</strong>
      <span class="tabular">
        {t("common.table.items", { count: data.total.count })}
      </span>
      {#if showRam}<span class="tabular mono">
          <span class="k">{t("common.units.ram")}</span>
          {formatBytes(data.total.ramBytes)}
        </span>{/if}
      {#if showDisk}<span class="tabular mono">
          <span class="k">{t("common.units.disk")}</span>
          {formatBytes(data.total.diskBytes)}
        </span>{/if}
    </div>
  {/if}
  {#each data.groups as group (group.project?.id ?? "")}
    <Group
      name={group.project ? group.project.name : t("common.noProject")}
      color={projectColor(group.project?.id, ids)}
      path={pathOf(group.project?.id)}
      count={group.totals.count}
      ramBytes={showRam ? group.totals.ramBytes : undefined}
      diskBytes={showDisk ? group.totals.diskBytes : undefined}
    >
      <table>
        {#if head}<thead>{@render head()}</thead>{/if}
        <tbody>
          {#each group.items as item, i (i)}
            {@render row(item)}
          {/each}
        </tbody>
      </table>
    </Group>
  {/each}
</div>

<style>
  .gt {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .total {
    display: flex;
    align-items: center;
    gap: 20px;
    padding: 10px 16px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
  }
  .k {
    font-family: var(--font-sans);
    font-size: var(--text-xs);
    color: var(--text-subtle);
  }
  /* Shared look for the module `<tr>`s until each module moves to DataTable. */
  .gt :global(table) {
    width: 100%;
    border-collapse: collapse;
  }
  .gt :global(th) {
    height: 32px;
    padding: 0 14px;
    text-align: left;
    font-size: 11px;
    font-weight: var(--weight-semibold);
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-subtle);
    border-bottom: 1px solid var(--border);
    white-space: nowrap;
  }
  .gt :global(td) {
    padding: 8px 14px;
    border-bottom: 1px solid var(--border);
    vertical-align: middle;
  }
  .gt :global(tbody tr:last-child td) {
    border-bottom: 0;
  }
  .gt :global(tbody tr:hover) {
    background: var(--surface-hover);
  }
</style>
