<script lang="ts">
  import Badge from "../../components/Badge.svelte";
  import Button from "../../components/Button.svelte";
  import EmptyState from "../../components/EmptyState.svelte";
  import type { Column } from "../../components/DataTable.svelte";
  import { formatBytes, formatRelativeAge } from "../../lib/format";
  import { t } from "../../lib/i18n/i18n.svelte";
  import type { Grouped } from "../../lib/types";
  import GroupedRows from "./GroupedRows.svelte";
  import { DEV_KIND_KEYS, formatCpu, type DevProcess } from "./logic";

  interface Props {
    data: Grouped<DevProcess> | null;
    loading: boolean;
    grouped: boolean;
    onterminate: (p: DevProcess) => void;
  }
  let { data, loading, grouped, onterminate }: Props = $props();

  const columns: Column<DevProcess>[] = $derived([
    { key: "process", header: t("common.columns.process") },
    {
      key: "ram",
      header: t("reclaim.processes.columns.ram"),
      width: "100px",
      align: "end",
      kind: "numeric",
    },
    {
      key: "cpu",
      header: t("reclaim.processes.columns.cpu"),
      width: "70px",
      align: "end",
      kind: "numeric",
    },
    {
      key: "age",
      header: t("reclaim.processes.columns.age"),
      width: "110px",
      align: "end",
      kind: "numeric",
    },
    { key: "folder", header: t("reclaim.processes.columns.folder") },
    { key: "actions", header: "", width: "auto", align: "end" },
  ]);
</script>

<p class="desc subtle">{t("reclaim.processes.description")}</p>

{#if loading && !data}
  <p class="subtle">{t("common.state.loading")}</p>
{:else if data && data.total.count === 0}
  <EmptyState title={t("reclaim.processes.empty")} />
{:else if data}
  <GroupedRows
    {data}
    {grouped}
    {columns}
    showRam
    ariaLabel={t("reclaim.tabs.processes")}
    rowKey={(p) => `${p.pid}:${p.startTime}`}
  >
    {#snippet cell(p, col)}
      {#if col.key === "process"}
        <span class="stack">
          <span class="line">
            <span class="strong" title={p.exe ?? p.name}>{p.name}</span>
            {#if p.orphan}
              <Badge tone="warning" title={t("reclaim.processes.orphanTitle")}>
                {t("reclaim.processes.orphan")}
              </Badge>
            {/if}
          </span>
          <span class="subtle small">
            {t(DEV_KIND_KEYS[p.kind])} · PID {p.pid}
          </span>
        </span>
      {:else if col.key === "ram"}
        {formatBytes(p.ramBytes)}
      {:else if col.key === "cpu"}
        {formatCpu(p.cpuPercent)}
      {:else if col.key === "age"}
        {formatRelativeAge(p.ageSecs)}
      {:else if col.key === "folder"}
        <span class="path" title={p.cwd ?? undefined}>
          {p.cwd ?? t("reclaim.processes.unknownFolder")}
        </span>
      {:else}
        <Button size="sm" variant="danger-ghost" onclick={() => onterminate(p)}>
          {t("reclaim.actions.terminate")}
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
  .line {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .strong {
    font-weight: var(--weight-medium);
  }
  .small {
    font-size: 11.5px;
  }
</style>
