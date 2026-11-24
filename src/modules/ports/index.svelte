<script lang="ts">
  import { untrack } from "svelte";
  import ConfirmDialog from "../../components/ConfirmDialog.svelte";
  import GroupedTable from "../../components/GroupedTable.svelte";
  import { api, toAppError } from "../../lib/api";
  import { configState } from "../../lib/config.svelte";
  import { errorMessage } from "../../lib/errors";
  import { t } from "../../lib/i18n/i18n.svelte";
  import { currentScope } from "../../lib/scope.svelte";
  import type { AppError, Grouped } from "../../lib/types";
  import { autoRefreshMs, baseName, type ListeningPort } from "./logic";

  let data = $state<Grouped<ListeningPort> | null>(null);
  let loading = $state(false);
  let error = $state<AppError | null>(null);
  let query = $state("");
  let includeUdp = $state(false);
  let pending = $state<ListeningPort | null>(null);

  const refreshMs = $derived(
    autoRefreshMs(configState.value.portsAutoRefreshSecs),
  );

  /** Guards against overlapping calls and out-of-order responses. */
  let seq = 0;
  /** Not reactive: only used to skip the debounce when the query did not change. */
  let lastQuery = "";

  async function load(): Promise<void> {
    const mine = ++seq;
    loading = true;
    try {
      const result = await api.withScope<Grouped<ListeningPort>>(
        "ports_list",
        currentScope(),
        { includeUdp, query: query.trim() || null },
      );
      if (mine !== seq) return;
      data = result;
      error = null;
    } catch (e) {
      if (mine === seq) error = toAppError(e);
    } finally {
      if (mine === seq) loading = false;
    }
  }

  // Reload when scope or UDP changes; debounce typing in the search box.
  $effect(() => {
    void configState.value.scope;
    void includeUdp;
    const q = query;
    const handle = setTimeout(
      () => untrack(() => void load()),
      q === lastQuery ? 0 : 250,
    );
    lastQuery = q;
    return () => clearTimeout(handle);
  });

  // Optional auto refresh (read from the config; never changed here).
  $effect(() => {
    const ms = refreshMs;
    if (ms === null) return;
    const id = setInterval(() => {
      if (!pending) untrack(() => void load());
    }, ms);
    return () => clearInterval(id);
  });

  async function confirmKill(): Promise<void> {
    const p = pending;
    if (!p) return;
    pending = null;
    try {
      await api.call("process_kill", {
        pid: p.pid,
        expectedName: p.processName,
        expectedStartTime: p.startTime,
      });
      error = null;
    } catch (e) {
      error = toAppError(e);
    }
    await load();
  }
</script>

<section class="ports">
  <h2>{t("ports.title")}</h2>

  <div class="toolbar">
    <label>
      <span class="sr">{t("ports.filter.search")}</span>
      <input
        type="search"
        placeholder={t("ports.filter.searchPlaceholder")}
        bind:value={query}
      />
    </label>
    <label>
      <input type="checkbox" bind:checked={includeUdp} />
      {t("ports.filter.includeUdp")}
    </label>
    <button type="button" onclick={() => void load()} disabled={loading}>
      {t("common.buttons.refresh")}
    </button>
    <small class="auto">
      {refreshMs === null
        ? t("ports.autoRefresh.off")
        : t("ports.autoRefresh.label", { secs: refreshMs / 1000 })}
    </small>
  </div>

  {#if error}
    <p class="error" role="alert">
      {errorMessage(error)}
      {#if error.detail}<small>{error.detail}</small>{/if}
    </p>
  {/if}

  {#if !data}
    <p>{t("common.state.loading")}</p>
  {:else if data.total.count === 0}
    <p>{query.trim() ? t("ports.empty.noResults") : t("ports.empty.none")}</p>
  {:else}
    <GroupedTable {data}>
      {#snippet head()}
        <tr>
          <th>{t("ports.columns.port")}</th>
          <th>{t("ports.columns.protocol")}</th>
          <th>{t("ports.columns.pid")}</th>
          <th>{t("common.columns.process")}</th>
          <th>{t("ports.columns.worktree")}</th>
          <th>{t("ports.columns.path")}</th>
          <th>{t("common.columns.actions")}</th>
        </tr>
      {/snippet}
      {#snippet row(p: ListeningPort)}
        <tr>
          <td><strong>{p.port}</strong></td>
          <td>{p.protocol.toUpperCase()}</td>
          <td>{p.pid}</td>
          <td title={p.exePath ?? undefined}>{p.processName}</td>
          <td title={p.worktreePath ?? undefined}>
            {p.worktreePath
              ? baseName(p.worktreePath)
              : t("ports.worktreeNone")}
          </td>
          <td class="path" title={p.cwd ?? t("ports.unknownPathHint")}>
            {#if p.cwd}{p.cwd}{:else}<em>{t("ports.unknownPath")}</em>{/if}
          </td>
          <td>
            <button type="button" onclick={() => (pending = p)}>
              {t("ports.actions.kill")}
            </button>
          </td>
        </tr>
      {/snippet}
    </GroupedTable>
  {/if}

  <p class="note"><small>{t("ports.platformNote")}</small></p>
</section>

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
  onconfirm={() => void confirmKill()}
  oncancel={() => (pending = null)}
/>

<style>
  .toolbar {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
    align-items: center;
    margin-bottom: 0.75rem;
  }
  .toolbar .auto {
    margin-left: auto;
    opacity: 0.7;
  }
  .sr {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
  }
  .path {
    word-break: break-all;
  }
  .error small {
    display: block;
    opacity: 0.7;
  }
  .note {
    opacity: 0.7;
  }
</style>
