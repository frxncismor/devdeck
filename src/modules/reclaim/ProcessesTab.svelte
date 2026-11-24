<script lang="ts">
  import GroupedTable from "../../components/GroupedTable.svelte";
  import { formatBytes, formatRelativeAge } from "../../lib/format";
  import { t } from "../../lib/i18n/i18n.svelte";
  import type { Grouped } from "../../lib/types";
  import { DEV_KIND_KEYS, formatCpu, type DevProcess } from "./logic";

  interface Props {
    data: Grouped<DevProcess> | null;
    loading: boolean;
    onterminate: (p: DevProcess) => void;
  }
  let { data, loading, onterminate }: Props = $props();
</script>

<p class="desc">{t("reclaim.processes.description")}</p>

{#if loading && !data}
  <p>{t("common.state.loading")}</p>
{:else if data && data.total.count === 0}
  <p>{t("reclaim.processes.empty")}</p>
{:else if data}
  <GroupedTable {data} showRam>
    {#snippet head()}
      <tr>
        <th>{t("reclaim.processes.columns.process")}</th>
        <th>{t("reclaim.processes.columns.ram")}</th>
        <th>{t("reclaim.processes.columns.cpu")}</th>
        <th>{t("reclaim.processes.columns.age")}</th>
        <th>{t("reclaim.processes.columns.folder")}</th>
        <th>{t("reclaim.processes.columns.actions")}</th>
      </tr>
    {/snippet}
    {#snippet row(p: DevProcess)}
      <tr>
        <td title={p.exe ?? p.name}>
          {p.name}
          <small class="kind">{t(DEV_KIND_KEYS[p.kind])} · PID {p.pid}</small>
          {#if p.orphan}
            <span class="badge warn">{t("reclaim.processes.orphan")}</span>
          {/if}
        </td>
        <td>{formatBytes(p.ramBytes)}</td>
        <td>{formatCpu(p.cpuPercent)}</td>
        <td>{formatRelativeAge(p.ageSecs)}</td>
        <td class="folder" title={p.cwd ?? undefined}>
          {p.cwd ?? t("reclaim.processes.unknownFolder")}
        </td>
        <td>
          <button type="button" onclick={() => onterminate(p)}>
            {t("reclaim.actions.terminate")}
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
  .kind {
    display: block;
    opacity: 0.6;
  }
  .folder {
    word-break: break-all;
    max-width: 24rem;
  }
  .badge {
    display: inline-block;
    padding: 0 0.4rem;
    border-radius: 0.6rem;
    font-size: 0.75rem;
    background: #d9383833;
  }
</style>
