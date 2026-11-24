<script lang="ts">
  import { untrack } from "svelte";
  import Lock from "@lucide/svelte/icons/lock";
  import RefreshCw from "@lucide/svelte/icons/refresh-cw";
  import Search from "@lucide/svelte/icons/search";
  import Plug from "@lucide/svelte/icons/plug";
  import X from "@lucide/svelte/icons/x";
  import Banner from "../../components/Banner.svelte";
  import Button from "../../components/Button.svelte";
  import Chip from "../../components/Chip.svelte";
  import ConfirmDialog from "../../components/ConfirmDialog.svelte";
  import DataTable, { type Column } from "../../components/DataTable.svelte";
  import EmptyState from "../../components/EmptyState.svelte";
  import Spinner from "../../components/Spinner.svelte";
  import TopBarContent from "../../components/TopBarContent.svelte";
  import { api, toAppError } from "../../lib/api";
  import { configState } from "../../lib/config.svelte";
  import { errorMessage } from "../../lib/errors";
  import { t } from "../../lib/i18n/i18n.svelte";
  import { projectColor } from "../../lib/projectColor";
  import { currentScope, scopeState } from "../../lib/scope.svelte";
  import { toasts } from "../../lib/toasts.svelte";
  import type { AppError, Grouped } from "../../lib/types";
  import {
    baseName,
    effectiveRefreshMs,
    filterProtocol,
    rowKey,
    type ListeningPort,
    type ProtocolFilter,
  } from "./logic";

  let data = $state<Grouped<ListeningPort> | null>(null);
  let loading = $state(false);
  let error = $state<AppError | null>(null);
  let query = $state("");
  let protocol = $state<ProtocolFilter>("all");
  let pending = $state<ListeningPort | null>(null);
  let killing = $state(false);
  /** Window focus / visibility: auto refresh only runs while the user can see the module. */
  let active = $state(
    typeof document === "undefined"
      ? true
      : document.hasFocus() && document.visibilityState === "visible",
  );

  const refreshMs = $derived(
    effectiveRefreshMs(configState.value.portsAutoRefreshSecs),
  );
  const ids = $derived(scopeState.projects.map((p) => p.id));
  const rows = $derived(
    filterProtocol(data?.groups.flatMap((g) => g.items) ?? [], protocol),
  );

  const protocols: { id: ProtocolFilter; label: () => string }[] = [
    { id: "all", label: () => t("ports.filter.all") },
    { id: "tcp", label: () => "TCP" },
    { id: "udp", label: () => "UDP" },
  ];

  const columns = $derived<Column<ListeningPort>[]>([
    {
      key: "port",
      header: t("ports.columns.port"),
      width: "76px",
      kind: "numeric",
    },
    { key: "protocol", header: t("ports.columns.protocol"), width: "72px" },
    {
      key: "pid",
      header: t("ports.columns.pid"),
      width: "80px",
      kind: "numeric",
    },
    { key: "process", header: t("common.columns.process"), width: "1fr" },
    { key: "path", header: t("ports.columns.path"), width: "1.6fr" },
    { key: "owner", header: t("ports.columns.owner"), width: "190px" },
    {
      key: "actions",
      header: t("common.columns.actions"),
      width: "104px",
      align: "end",
    },
  ]);

  /** Guards against overlapping calls and out-of-order responses. */
  let seq = 0;
  let inflight = false;
  /** Not reactive: only used to skip the debounce when the query did not change. */
  let lastQuery = "";

  async function load(silent = false): Promise<void> {
    // Auto refresh never overlaps a request that is still running.
    if (silent && inflight) return;
    const mine = ++seq;
    inflight = true;
    if (!silent) loading = true;
    try {
      // Always ask for UDP too: the TCP/UDP toggle filters in the front.
      const result = await api.withScope<Grouped<ListeningPort>>(
        "ports_list",
        currentScope(),
        { includeUdp: true, query: query.trim() || null },
      );
      if (mine !== seq) return;
      data = result;
      error = null;
    } catch (e) {
      if (mine === seq) error = toAppError(e);
    } finally {
      if (mine === seq) {
        loading = false;
        inflight = false;
      }
    }
  }

  // Reload when the scope changes; debounce typing in the search box.
  $effect(() => {
    void configState.value.scope;
    const q = query;
    const handle = setTimeout(
      () => untrack(() => void load()),
      q === lastQuery ? 0 : 250,
    );
    lastQuery = q;
    return () => clearTimeout(handle);
  });

  // Auto refresh while the window is focused and visible (every 5 s unless configured).
  $effect(() => {
    const ms = refreshMs;
    if (ms === null || !active) return;
    const id = setInterval(() => {
      if (!pending) untrack(() => void load(true));
    }, ms);
    return () => clearInterval(id);
  });

  function syncActive(): void {
    const next = document.hasFocus() && document.visibilityState === "visible";
    if (next && !active) void load(true);
    active = next;
  }

  async function confirmKill(): Promise<void> {
    const p = pending;
    if (!p || killing) return;
    killing = true;
    try {
      await api.call("process_kill", {
        pid: p.pid,
        expectedName: p.processName,
        expectedStartTime: p.startTime,
      });
      pending = null;
      toasts.success(
        t("ports.toast.killed", { name: p.processName }),
        t("ports.toast.killedDetail", { pid: p.pid, port: p.port }),
      );
    } catch (e) {
      pending = null;
      const err = toAppError(e);
      toasts.error(errorMessage(err), err.detail ?? undefined);
    } finally {
      killing = false;
    }
    await load();
  }
</script>

<svelte:window onfocus={syncActive} onblur={syncActive} />
<svelte:document onvisibilitychange={syncActive} />

<TopBarContent>
  {#snippet meta()}
    {#if data}
      {t("ports.count", { count: rows.length })}
      ·
      {refreshMs === null
        ? t("ports.autoRefresh.off")
        : t("ports.autoRefresh.label", { secs: refreshMs / 1000 })}
    {/if}
  {/snippet}
  {#snippet actions()}
    <Button
      size="sm"
      icon={RefreshCw}
      disabled={loading}
      onclick={() => void load()}
    >
      {t("common.buttons.refresh")}
    </Button>
  {/snippet}
</TopBarContent>

<Banner tone="info" title={t("ports.elevated.title")}>
  {t("ports.elevated.message")}
  {t("ports.platformNote")}
</Banner>

<div class="filters">
  <label class="search">
    <Search size={14} aria-hidden="true" />
    <span class="sr-only">{t("ports.filter.search")}</span>
    <input
      type="search"
      placeholder={t("ports.filter.searchPlaceholder")}
      bind:value={query}
    />
    {#if query}
      <button
        type="button"
        class="clear"
        aria-label={t("ports.filter.clear")}
        title={t("ports.filter.clear")}
        onclick={() => (query = "")}
      >
        <X size={13} aria-hidden="true" />
      </button>
    {/if}
  </label>
  <div class="seg" role="group" aria-label={t("ports.columns.protocol")}>
    {#each protocols as p (p.id)}
      <button
        type="button"
        class:on={protocol === p.id}
        aria-pressed={protocol === p.id}
        onclick={() => (protocol = p.id)}
      >
        {p.label()}
      </button>
    {/each}
  </div>
</div>

{#if error && !data}
  <Banner tone="danger" title={errorMessage(error)}>
    {#if error.detail}<span class="mono">{error.detail}</span>{/if}
    {#snippet actions()}
      <Button size="sm" onclick={() => void load()}>
        {t("ports.retry")}
      </Button>
    {/snippet}
  </Banner>
{:else if !data}
  <div class="loading" role="status">
    <Spinner size={18} />
    <span>{t("common.state.loading")}</span>
  </div>
{:else}
  {#if error}
    <Banner tone="danger" title={errorMessage(error)}>
      {#if error.detail}<span class="mono">{error.detail}</span>{/if}
    </Banner>
  {/if}
  {#if rows.length === 0}
    <EmptyState
      icon={Plug}
      title={query.trim() || protocol !== "all"
        ? t("ports.empty.noResults")
        : t("ports.empty.none")}
    />
  {:else}
    <DataTable
      {columns}
      {rows}
      {rowKey}
      rowHeight={44}
      ariaLabel={t("common.sidebar.ports")}
    >
      {#snippet cell(p: ListeningPort, col: Column<ListeningPort>)}
        {#if col.key === "port"}
          <strong class="mono tabular">{p.port}</strong>
        {:else if col.key === "protocol"}
          <span class="proto mono">{p.protocol.toUpperCase()}</span>
        {:else if col.key === "pid"}
          <span class="mono tabular muted">{p.pid}</span>
        {:else if col.key === "process"}
          <span class="pname" title={p.processName}>{p.processName}</span>
        {:else if col.key === "path"}
          {@const full = p.exePath ?? p.cwd}
          {#if full}
            <span class="path mono" title={full}>{full}</span>
          {:else}
            <span class="muted">—</span>
          {/if}
        {:else if col.key === "owner"}
          {#if p.project}
            <Chip
              project={p.project.name}
              color={projectColor(p.project.id, ids)}
              worktree={p.worktreePath ? baseName(p.worktreePath) : undefined}
              title={p.worktreePath ?? p.cwd ?? undefined}
            />
          {:else if !p.cwd}
            <span class="unknown muted" title={t("ports.unknownPathHint")}>
              <Lock size={12} aria-hidden="true" />
              {t("ports.unknownPath")}
            </span>
          {:else}
            <span class="muted" title={p.cwd}>{t("common.noProject")}</span>
          {/if}
        {:else}
          <Button
            size="sm"
            variant="danger-ghost"
            onclick={() => (pending = p)}
          >
            {t("ports.actions.kill")}
          </Button>
        {/if}
      {/snippet}
    </DataTable>
  {/if}
{/if}

<ConfirmDialog
  open={pending !== null}
  title={t("ports.kill.title")}
  message={pending
    ? t("ports.kill.message", {
        name: pending.processName,
        pid: pending.pid,
        port: pending.port,
      })
    : ""}
  confirmLabel={t("ports.kill.confirm")}
  busy={killing}
  onconfirm={() => void confirmKill()}
  oncancel={() => (pending = null)}
/>

<style>
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
    flex: 1 1 260px;
    max-width: 420px;
    height: var(--control-h);
    padding: 0 10px;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
    color: var(--text-subtle);
  }
  .search:focus-within {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
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
  .clear {
    display: inline-flex;
    padding: 2px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text-subtle);
    cursor: pointer;
  }
  .clear:hover {
    color: var(--text);
  }
  .seg {
    display: inline-flex;
    padding: 2px;
    gap: 2px;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
  }
  .seg button {
    height: 24px;
    padding: 0 12px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text-muted);
    font: inherit;
    font-size: var(--text-sm);
    font-weight: var(--weight-medium);
    cursor: pointer;
  }
  .seg button:hover {
    color: var(--text);
  }
  .seg button.on {
    background: var(--accent-soft);
    color: var(--accent-text);
  }
  .loading {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: var(--space-2);
    padding: 48px 24px;
    color: var(--text-muted);
  }
  .proto {
    font-size: var(--text-sm);
    color: var(--text-muted);
  }
  .pname,
  .path {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
  }
  .path {
    font-size: var(--text-sm);
  }
  .unknown {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: var(--text-sm);
  }
</style>
