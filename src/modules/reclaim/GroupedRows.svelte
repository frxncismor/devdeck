<script lang="ts" generics="T">
  import type { Snippet } from "svelte";
  import DataTable, { type Column } from "../../components/DataTable.svelte";
  import Group from "../../components/Group.svelte";
  import { t } from "../../lib/i18n/i18n.svelte";
  import { projectColor } from "../../lib/projectColor";
  import { scopeState } from "../../lib/scope.svelte";
  import type { Grouped } from "../../lib/types";

  interface Props {
    data: Grouped<T>;
    /** Global scope: one collapsible group per project; project scope: a single table. */
    grouped: boolean;
    columns: Column<T>[];
    cell: Snippet<[T, Column<T>, number]>;
    rowKey: (row: T) => string;
    ariaLabel: string;
    showRam?: boolean;
    showDisk?: boolean;
    rowClass?: (row: T) => string | undefined;
  }
  let {
    data,
    grouped,
    columns,
    cell,
    rowKey,
    ariaLabel,
    showRam = false,
    showDisk = false,
    rowClass,
  }: Props = $props();

  const ids = $derived(scopeState.projects.map((p) => p.id));
  const all = $derived(data.groups.flatMap((g) => g.items));
</script>

{#if grouped}
  {#each data.groups as g (g.project?.id ?? "")}
    <Group
      name={g.project?.name ?? t("common.noProject")}
      color={projectColor(g.project?.id, ids)}
      count={g.totals.count}
      ramBytes={showRam ? g.totals.ramBytes : undefined}
      diskBytes={showDisk ? g.totals.diskBytes : undefined}
    >
      <DataTable
        {columns}
        rows={g.items}
        {rowKey}
        {cell}
        {rowClass}
        {ariaLabel}
      />
    </Group>
  {/each}
{:else}
  <DataTable {columns} rows={all} {rowKey} {cell} {rowClass} {ariaLabel} />
{/if}
