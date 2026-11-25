<script lang="ts">
  import { onMount, untrack } from "svelte";
  import { SvelteSet } from "svelte/reactivity";
  import { open } from "@tauri-apps/plugin-dialog";
  import ArrowDown from "@lucide/svelte/icons/arrow-down";
  import ArrowUp from "@lucide/svelte/icons/arrow-up";
  import FolderOpen from "@lucide/svelte/icons/folder-open";
  import RefreshCw from "@lucide/svelte/icons/refresh-cw";
  import Search from "@lucide/svelte/icons/search";
  import SquareTerminal from "@lucide/svelte/icons/square-terminal";
  import Trash2 from "@lucide/svelte/icons/trash-2";
  import Code from "@lucide/svelte/icons/code";
  import GitBranch from "@lucide/svelte/icons/git-branch";
  import Badge from "../../components/Badge.svelte";
  import Banner from "../../components/Banner.svelte";
  import GlossaryButton from "../../components/GlossaryButton.svelte";
  import Button from "../../components/Button.svelte";
  import ConfirmDialog from "../../components/ConfirmDialog.svelte";
  import DataTable, { type Column } from "../../components/DataTable.svelte";
  import EmptyState from "../../components/EmptyState.svelte";
  import Group from "../../components/Group.svelte";
  import IconButton from "../../components/IconButton.svelte";
  import Spinner from "../../components/Spinner.svelte";
  import StatCard from "../../components/StatCard.svelte";
  import TopBarContent from "../../components/TopBarContent.svelte";
  import { api, toAppError } from "../../lib/api";
  import { configState, updateConfig } from "../../lib/config.svelte";
  import { errorMessage } from "../../lib/errors";
  import { formatRelativeAge } from "../../lib/format";
  import { t } from "../../lib/i18n/i18n.svelte";
  import { projectColor } from "../../lib/projectColor";
  import {
    currentScope,
    refreshProjects,
    scopeState,
  } from "../../lib/scope.svelte";
  import { toasts } from "../../lib/toasts.svelte";
  import type { AppError, Grouped } from "../../lib/types";
  import {
    ageSecs,
    filterGrouped,
    isDirtyError,
    syncStatus,
    worktreeStatus,
    type Worktree,
  } from "./logic";

  let data = $state<Grouped<Worktree> | null>(null);
  let loading = $state(false);
  let error = $state<AppError | null>(null);
  let dirtyOnly = $state(false);
  let query = $state("");
  /** Worktree pending removal and whether the second (discard changes) step is shown. */
  let pending = $state<Worktree | null>(null);
  let forcing = $state(false);
  /** Paths of the worktrees being removed right now (spinner on their row, no double run). */
  const removing = new SvelteSet<string>();
  /** Unix ms of the last successful scan and a coarse clock for "Scanned X ago". */
  let scannedAt = $state<number | null>(null);
  let now = $state(Date.now());

  const root = $derived(configState.value.root);
  const scope = $derived(configState.value.scope);
  const isProject = $derived(scope.kind === "project");
  const visible = $derived(
    data ? filterGrouped(data, { dirtyOnly, query }) : null,
  );
  const ids = $derived(scopeState.projects.map((p) => p.id));
  const stats = $derived.by(() => {
    const items = (data?.groups ?? []).flatMap((g) => g.items);
    return {
      total: items.length,
      dirty: items.filter((w) => w.dirtyCount > 0).length,
      ahead: items.filter((w) => (w.ahead ?? 0) > 0).length,
      behind: items.filter((w) => (w.behind ?? 0) > 0).length,
    };
  });
  const scannedLabel = $derived.by(() => {
    if (scannedAt === null) return "";
    const secs = Math.max(0, Math.floor((now - scannedAt) / 1000));
    return secs < 60
      ? t("worktrees.scannedJustNow")
      : t("worktrees.scannedAgo", { age: formatRelativeAge(secs) });
  });

  const columns = $derived<Column<Worktree>[]>([
    { key: "branch", header: t("common.columns.branch"), width: "1.3fr" },
    { key: "status", header: t("worktrees.columns.status"), width: "130px" },
    { key: "sync", header: t("worktrees.columns.sync"), width: "120px" },
    {
      key: "commit",
      header: t("common.columns.lastCommit"),
      width: "1.6fr",
    },
    {
      key: "actions",
      header: t("common.columns.actions"),
      width: "112px",
      align: "end",
    },
  ]);

  /** Guards against out-of-order responses (same pattern as Ports). */
  let seq = 0;

  async function scan(): Promise<void> {
    const mine = ++seq;
    if (!root) {
      data = null;
      loading = false;
      return;
    }
    loading = true;
    error = null;
    try {
      const result = await api.withScope<Grouped<Worktree>>(
        "worktrees_scan",
        currentScope(),
      );
      if (mine !== seq) return;
      data = result;
      scannedAt = Date.now();
      now = scannedAt;
      // The scan refreshed the backend project list; sync the selector.
      await refreshProjects();
    } catch (e) {
      if (mine === seq) error = toAppError(e);
    } finally {
      if (mine === seq) loading = false;
    }
  }

  // Re-scan when the root or the scope changes.
  $effect(() => {
    void root;
    void configState.value.scope;
    untrack(() => void scan());
  });

  onMount(() => {
    const id = setInterval(() => (now = Date.now()), 30_000);
    return () => clearInterval(id);
  });

  function onKeydown(e: KeyboardEvent): void {
    if (e.key === "F5" && !e.ctrlKey && !e.metaKey && !e.altKey) {
      e.preventDefault();
      if (root && !loading) void scan();
    }
  }

  async function chooseRoot(): Promise<void> {
    const picked = await open({
      directory: true,
      multiple: false,
      title: t("worktrees.root.pickerTitle"),
      defaultPath: root,
    });
    if (typeof picked === "string") await updateConfig({ root: picked });
  }

  function fail(e: unknown): void {
    const err = toAppError(e);
    toasts.error(errorMessage(err), err.detail ?? undefined);
  }

  async function open_(
    cmd: "worktree_open_vscode" | "worktree_open_terminal",
    w: Worktree,
  ): Promise<void> {
    try {
      await api.call(cmd, { path: w.path });
      toasts.success(
        t(
          cmd === "worktree_open_vscode"
            ? "worktrees.toast.openedVscode"
            : "worktrees.toast.openedTerminal",
        ),
        w.branch ?? w.path,
      );
    } catch (e) {
      fail(e);
    }
  }

  async function confirmRemove(): Promise<void> {
    const w = pending;
    if (!w || removing.has(w.path)) return;
    const force = forcing;
    // Close the dialog and keep working in the background: the row shows the progress.
    pending = null;
    forcing = false;
    removing.add(w.path);
    try {
      await api.call("worktree_remove", { path: w.path, force });
      toasts.success(t("worktrees.toast.removed"), w.branch ?? w.path);
      await scan();
    } catch (e) {
      const err = toAppError(e);
      if (isDirtyError(err) && !force) {
        // Second confirmation: discarding uncommitted changes.
        pending = w;
        forcing = true;
        return;
      }
      fail(err);
    } finally {
      removing.delete(w.path);
    }
  }

  function cancelRemove(): void {
    pending = null;
    forcing = false;
  }

  const rowKey = (w: Worktree) => w.path;
</script>

<svelte:window onkeydown={onKeydown} />

<TopBarContent>
  {#snippet meta()}
    {#if loading && data}
      {t("worktrees.scanning")}
    {:else if scannedLabel}
      {scannedLabel}
    {/if}
  {/snippet}
  {#snippet actions()}
    {#if root}
      <Button size="sm" icon={FolderOpen} onclick={() => void chooseRoot()}>
        {t("worktrees.root.change")}
      </Button>
      <Button
        size="sm"
        variant="primary"
        icon={RefreshCw}
        kbd="F5"
        {loading}
        onclick={() => void scan()}
      >
        {t("worktrees.rescan")}
      </Button>
      <GlossaryButton module="worktrees" />
    {/if}
  {/snippet}
</TopBarContent>

{#snippet branchCell(w: Worktree)}
  <div class="branch">
    <div class="b1">
      <GitBranch size={13} aria-hidden="true" />
      <span class="bname mono">
        {w.branch ?? (w.detached ? t("worktrees.badge.detached") : "—")}
      </span>
      {#if w.isMain}<Badge tone="accent" title={t("worktrees.badge.mainHint")}>
          {t("worktrees.badge.main")}
        </Badge><span class="sr-only">
          {t("worktrees.badge.mainHint")}
        </span>{/if}
      {#if w.detached && w.branch}<Badge
          title={t("worktrees.badge.detachedHint")}
        >
          {t("worktrees.badge.detached")}
        </Badge><span class="sr-only">
          {t("worktrees.badge.detachedHint")}
        </span>{/if}
    </div>
    <span class="path subtle" title={w.path}>{w.path}</span>
  </div>
{/snippet}

{#snippet statusCell(w: Worktree)}
  {@const st = worktreeStatus(w)}
  {@const label = t(st.labelKey, { count: st.count ?? 0 })}
  {@const hint = t(st.hintKey)}
  <Badge tone={st.tone} title={hint}>{label}</Badge>
  <span class="sr-only">{hint}</span>
{/snippet}

{#snippet syncCell(w: Worktree)}
  {@const sync = syncStatus(w.ahead, w.behind)}
  {#if sync.kind === "none"}
    <span class="muted" title={t("worktrees.sync.noUpstreamHint")}>
      {t("worktrees.sync.noUpstream")}
    </span>
    <span class="sr-only">{t("worktrees.sync.noUpstreamHint")}</span>
  {:else if sync.kind === "synced"}
    {@const hint = t("worktrees.sync.syncedHint")}
    <span class="muted mono" title={hint} aria-hidden="true">✓</span>
    <span class="sr-only">{t("worktrees.sync.synced")}. {hint}</span>
  {:else}
    {@const hint =
      sync.kind === "ahead"
        ? t("worktrees.sync.aheadHint", { count: sync.ahead })
        : sync.kind === "behind"
          ? t("worktrees.sync.behindHint", { count: sync.behind })
          : t("worktrees.sync.divergedHint", {
              ahead: sync.ahead,
              behind: sync.behind,
            })}
    <span class="sync mono tabular" title={hint}>
      {#if sync.ahead}<span class="up">
          <ArrowUp size={12} aria-hidden="true" />{sync.ahead}
        </span>{/if}
      {#if sync.behind}<span class="down">
          <ArrowDown size={12} aria-hidden="true" />{sync.behind}
        </span>{/if}
    </span>
    <span class="sr-only">{hint}</span>
  {/if}
{/snippet}

{#snippet commitCell(w: Worktree)}
  {#if w.lastCommit}
    <div class="commit">
      <span class="subject" title={w.lastCommit.subject}>
        {w.lastCommit.subject}
      </span>
      <span class="cmeta subtle">
        <span class="mono">{w.lastCommit.hash.slice(0, 7)}</span>
        · {formatRelativeAge(ageSecs(w.lastCommit.timestamp))}
      </span>
    </div>
  {:else}
    <span class="muted">{t("common.worktree.noCommits")}</span>
  {/if}
{/snippet}

{#snippet actionsCell(w: Worktree)}
  <div class="acts">
    <IconButton
      label={t("worktrees.actions.openVscode")}
      icon={Code}
      disabled={!w.exists || removing.has(w.path)}
      onclick={() => void open_("worktree_open_vscode", w)}
    />
    <IconButton
      label={t("worktrees.actions.openTerminal")}
      icon={SquareTerminal}
      disabled={!w.exists || removing.has(w.path)}
      onclick={() => void open_("worktree_open_terminal", w)}
    />
    <IconButton
      variant="danger-ghost"
      label={removing.has(w.path)
        ? t("common.busy.removing")
        : w.isMain
          ? t("worktrees.actions.removeMain")
          : t("worktrees.actions.remove")}
      icon={Trash2}
      loading={removing.has(w.path)}
      disabled={w.isMain}
      onclick={() => (pending = w)}
    />
  </div>
{/snippet}

{#snippet table(items: Worktree[])}
  <DataTable
    {columns}
    rows={items}
    {rowKey}
    rowHeight={54}
    ariaLabel={t("common.sidebar.worktrees")}
  >
    {#snippet cell(w: Worktree, col: Column<Worktree>)}
      {#if col.key === "branch"}
        {@render branchCell(w)}
      {:else if col.key === "status"}
        {@render statusCell(w)}
      {:else if col.key === "sync"}
        {@render syncCell(w)}
      {:else if col.key === "commit"}
        {@render commitCell(w)}
      {:else}
        {@render actionsCell(w)}
      {/if}
    {/snippet}
  </DataTable>
{/snippet}

{#if error && !data}
  <Banner tone="danger" title={errorMessage(error)}>
    {#if error.detail}<span class="mono">{error.detail}</span>{/if}
    {#snippet actions()}
      <Button size="sm" onclick={() => void scan()}>
        {t("worktrees.retry")}
      </Button>
    {/snippet}
  </Banner>
{:else if !root}
  <EmptyState
    icon={FolderOpen}
    title={t("worktrees.empty.noRootTitle")}
    description={t("worktrees.empty.noRoot")}
  >
    <Button
      variant="primary"
      icon={FolderOpen}
      onclick={() => void chooseRoot()}
    >
      {t("worktrees.root.choose")}
    </Button>
  </EmptyState>
{:else if loading && !data}
  <div class="scanning" role="status">
    <Spinner size={20} />
    <strong>{t("worktrees.scanning")}</strong>
    <span class="muted mono">{root}</span>
  </div>
{:else if data}
  {#if error}
    <Banner tone="danger" title={errorMessage(error)}>
      {#if error.detail}<span class="mono">{error.detail}</span>{/if}
    </Banner>
  {/if}

  {#if data.total.count === 0}
    <EmptyState
      icon={GitBranch}
      title={t("worktrees.empty.noWorktreesTitle")}
      description={t("worktrees.empty.noWorktrees")}
    >
      <Button icon={FolderOpen} onclick={() => void chooseRoot()}>
        {t("worktrees.root.change")}
      </Button>
    </EmptyState>
  {:else}
    {#if isProject}
      <div class="stats">
        <StatCard
          label={t("worktrees.stats.worktrees")}
          value={String(stats.total)}
        />
        <StatCard
          label={t("worktrees.stats.modified")}
          value={String(stats.dirty)}
          tone={stats.dirty > 0 ? "warning" : "default"}
        />
        <StatCard
          label={t("worktrees.stats.ahead")}
          value={String(stats.ahead)}
        />
        <StatCard
          label={t("worktrees.stats.behind")}
          value={String(stats.behind)}
        />
      </div>
    {:else}
      <div class="total">
        <strong>{t("common.table.total")}</strong>
        <span class="tabular">
          {t("common.table.items", { count: data.total.count })}
        </span>
        {#if stats.dirty > 0}
          <Badge tone="warning">
            {t("worktrees.stats.modifiedCount", { count: stats.dirty })}
          </Badge>
        {/if}
      </div>
    {/if}

    <div class="filters">
      <label class="search">
        <Search size={14} aria-hidden="true" />
        <span class="sr-only">{t("worktrees.filter.search")}</span>
        <input
          type="search"
          placeholder={t("worktrees.filter.searchPlaceholder")}
          bind:value={query}
        />
      </label>
      <label class="check">
        <input type="checkbox" bind:checked={dirtyOnly} />
        {t("worktrees.filter.dirtyOnly")}
      </label>
    </div>

    {#if !visible || visible.total.count === 0}
      <EmptyState icon={Search} title={t("worktrees.empty.noResults")} />
    {:else if isProject}
      {@render table(visible.groups.flatMap((g) => g.items))}
    {:else}
      {#each visible.groups as group (group.project?.id ?? "")}
        <Group
          name={group.project ? group.project.name : t("common.noProject")}
          color={projectColor(group.project?.id, ids)}
          path={scopeState.projects.find((p) => p.id === group.project?.id)
            ?.mainPath}
          count={group.totals.count}
        >
          {@render table(group.items)}
        </Group>
      {/each}
    {/if}
  {/if}
{/if}

<ConfirmDialog
  open={pending !== null}
  title={forcing
    ? t("common.worktree.remove.forceTitle")
    : t("common.worktree.remove.title")}
  message={pending
    ? t(
        forcing
          ? "common.worktree.remove.forceMessage"
          : "worktrees.remove.message",
        {
          path: pending.path,
        },
      )
    : ""}
  confirmLabel={forcing
    ? t("common.worktree.remove.forceConfirm")
    : t("common.worktree.remove.confirm")}
  onconfirm={() => void confirmRemove()}
  oncancel={cancelRemove}
/>

<style>
  .stats {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: var(--space-3);
  }
  .total {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: 8px 14px;
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface);
    font-size: var(--text-base);
  }
  .filters {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-3);
  }
  .search {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    flex: 1 1 240px;
    max-width: 380px;
    height: var(--control-h);
    padding: 0 10px;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
    color: var(--text-subtle);
  }
  .search input {
    flex: 1;
    min-width: 0;
    border: 0;
    outline: 0;
    background: transparent;
    color: var(--text);
    font: inherit;
    font-size: var(--text-base);
  }
  .search:focus-within {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }
  .check {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    font-size: var(--text-base);
    color: var(--text-muted);
  }
  .check input {
    accent-color: var(--accent);
  }
  .scanning {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-2);
    padding: 64px 24px;
    color: var(--text-muted);
    text-align: center;
  }
  .branch,
  .commit {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .b1 {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
  }
  .bname {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: var(--text-base);
    font-weight: var(--weight-medium);
  }
  .path,
  .subject,
  .cmeta {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: var(--text-sm);
  }
  .subject {
    font-size: var(--text-base);
  }
  .sync {
    display: inline-flex;
    gap: var(--space-2);
    font-size: var(--text-sm);
  }
  .sync span {
    display: inline-flex;
    align-items: center;
    gap: 2px;
  }
  .up {
    color: var(--success);
  }
  .down {
    color: var(--warning);
  }
  .acts {
    display: flex;
    gap: 2px;
  }
</style>
