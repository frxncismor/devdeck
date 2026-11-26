<script lang="ts">
  import HardDrive from "@lucide/svelte/icons/hard-drive";
  import MemoryStick from "@lucide/svelte/icons/memory-stick";
  import StatCard, { type StatSegment } from "../../components/StatCard.svelte";
  import { formatBytes } from "../../lib/format";
  import { t } from "../../lib/i18n/i18n.svelte";
  import { PROJECT_COLORS, projectColor } from "../../lib/projectColor";
  import { scopeState } from "../../lib/scope.svelte";
  import {
    diskHint,
    hasRecoverable,
    type ReclaimSummary,
    type Slice,
  } from "./logic";

  interface Props {
    summary: ReclaimSummary | null;
    /** Folder/worktree sizes are still being measured. */
    measuring: boolean;
    /** Global scope: stacked by project; project scope: by worktree. */
    byProject: boolean;
    ramSlices: Slice[];
    diskSlices: Slice[];
  }
  let { summary, measuring, byProject, ramSlices, diskSlices }: Props =
    $props();

  const ids = $derived(scopeState.projects.map((p) => p.id));

  function segments(slices: Slice[]): StatSegment[] {
    return slices.map((s, i) => ({
      label: s.label,
      value: s.value,
      display: formatBytes(s.value),
      color: byProject
        ? projectColor(s.key, ids)
        : `var(--project-${(i % PROJECT_COLORS) + 1})`,
    }));
  }

  const counts = $derived(
    summary
      ? summary.byProject.reduce(
          (a, p) => ({
            processes: a.processes + p.processes,
            worktrees: a.worktrees + p.worktrees,
            folders: a.folders + p.folders,
          }),
          { processes: 0, worktrees: 0, folders: 0 },
        )
      : { processes: 0, worktrees: 0, folders: 0 },
  );
  const suffix = $derived(
    byProject ? "" : ` · ${t("reclaim.summary.byWorktree")}`,
  );
</script>

<section class="summary">
  {#if summary}
    <div class="cards">
      <StatCard
        size="lg"
        icon={MemoryStick}
        label={t("reclaim.summary.ram")}
        value={formatBytes(summary.recoverableRamBytes)}
        hint={t("reclaim.summary.ramHint", { count: counts.processes }) +
          suffix}
        segments={segments(ramSlices)}
      />
      <StatCard
        size="lg"
        icon={HardDrive}
        label={t("reclaim.summary.disk")}
        value={formatBytes(summary.recoverableDiskBytes)}
        hint={diskHint(counts.worktrees, counts.folders) + suffix}
        segments={segments(diskSlices)}
      />
    </div>
    {#if measuring}<p class="note subtle">{t("reclaim.summary.sizing")}</p>{/if}
    {#if !hasRecoverable(summary) && !measuring}
      <p class="note subtle">{t("reclaim.summary.nothing")}</p>
    {/if}
  {:else}
    <p class="note subtle">{t("common.state.loading")}</p>
  {/if}
</section>

<style>
  .cards {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 12px;
  }
  .note {
    margin: 8px 0 0;
    font-size: 12px;
  }
</style>
