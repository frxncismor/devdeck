<script lang="ts" generics="T">
  import type { Snippet } from "svelte";
  import { t } from "../lib/i18n/i18n.svelte";
  import { formatBytes } from "../lib/format";
  import type { Grouped } from "../lib/types";

  interface Props {
    data: Grouped<T>;
    row: Snippet<[T]>;
    head?: Snippet;
    showRam?: boolean;
    showDisk?: boolean;
  }
  let { data, row, head, showRam = false, showDisk = false }: Props = $props();
</script>

<table>
  {#if head}<thead>{@render head()}</thead>{/if}
  {#each data.groups as group (group.project?.id ?? "")}
    <tbody>
      <tr class="group">
        <th colspan="99">
          {group.project ? group.project.name : t("common.noProject")}
          <small>
            {t("common.table.items", { count: group.totals.count })}
            {#if showRam}· RAM {formatBytes(group.totals.ramBytes)}{/if}
            {#if showDisk}· {formatBytes(group.totals.diskBytes)}{/if}
          </small>
        </th>
      </tr>
      {#each group.items as item, i (i)}
        {@render row(item)}
      {/each}
    </tbody>
  {/each}
</table>
