<script lang="ts">
  import { untrack } from "svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import ConfirmDialog from "../../components/ConfirmDialog.svelte";
  import GroupedTable from "../../components/GroupedTable.svelte";
  import { api, toAppError } from "../../lib/api";
  import { configState, updateConfig } from "../../lib/config.svelte";
  import { errorMessage } from "../../lib/errors";
  import { formatRelativeAge } from "../../lib/format";
  import { t } from "../../lib/i18n/i18n.svelte";
  import { currentScope, refreshProjects } from "../../lib/scope.svelte";
  import type { AppError, Grouped } from "../../lib/types";
  import {
    ageSecs,
    filterGrouped,
    formatAheadBehind,
    isDirtyError,
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
  let busy = $state(false);

  const root = $derived(configState.value.root);
  const visible = $derived(
    data ? filterGrouped(data, { dirtyOnly, query }) : null,
  );

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

  async function chooseRoot(): Promise<void> {
    const picked = await open({
      directory: true,
      multiple: false,
      title: t("worktrees.root.pickerTitle"),
      defaultPath: root,
    });
    if (typeof picked === "string") await updateConfig({ root: picked });
  }

  async function act(fn: () => Promise<unknown>): Promise<void> {
    error = null;
    try {
      await fn();
    } catch (e) {
      error = toAppError(e);
    }
  }

  async function confirmRemove(): Promise<void> {
    const w = pending;
    if (!w || busy) return;
    busy = true;
    try {
      await api.call("worktree_remove", { path: w.path, force: forcing });
      pending = null;
      forcing = false;
      await scan();
    } catch (e) {
      const err = toAppError(e);
      if (isDirtyError(err) && !forcing) {
        // Second confirmation: discarding uncommitted changes.
        forcing = true;
        return;
      }
      pending = null;
      forcing = false;
      error = err;
    } finally {
      busy = false;
    }
  }

  function cancelRemove(): void {
    pending = null;
    forcing = false;
  }
</script>

<section class="worktrees">
  <h2>{t("worktrees.title")}</h2>

  <div class="toolbar">
    <span class="root">
      <strong>{t("worktrees.root.label")}:</strong>
      {root ?? t("worktrees.root.none")}
    </span>
    <button type="button" onclick={() => void chooseRoot()}>
      {root ? t("worktrees.root.change") : t("worktrees.root.choose")}
    </button>
    <button
      type="button"
      onclick={() => void scan()}
      disabled={!root || loading}
    >
      {t("common.buttons.refresh")}
    </button>
    <label>
      <span class="sr">{t("worktrees.filter.search")}</span>
      <input
        type="search"
        placeholder={t("worktrees.filter.searchPlaceholder")}
        bind:value={query}
      />
    </label>
    <label>
      <input type="checkbox" bind:checked={dirtyOnly} />
      {t("worktrees.filter.dirtyOnly")}
    </label>
  </div>

  {#if error}
    <p class="error" role="alert">
      {errorMessage(error)}
      {#if error.detail}<small>{error.detail}</small>{/if}
    </p>
  {/if}

  {#if !root}
    <p>{t("worktrees.empty.noRoot")}</p>
  {:else if loading && !data}
    <p>{t("common.state.loading")}</p>
  {:else if data && data.total.count === 0}
    <p>{t("worktrees.empty.noWorktrees")}</p>
  {:else if visible}
    {#if visible.total.count === 0}
      <p>{t("worktrees.empty.noResults")}</p>
    {:else}
      <GroupedTable data={visible}>
        {#snippet head()}
          <tr>
            <th>{t("common.columns.branch")}</th>
            <th>{t("worktrees.columns.status")}</th>
            <th>{t("worktrees.columns.sync")}</th>
            <th>{t("common.columns.lastCommit")}</th>
            <th>{t("common.columns.actions")}</th>
          </tr>
        {/snippet}
        {#snippet row(w: Worktree)}
          {@const sync = formatAheadBehind(w.ahead, w.behind)}
          <tr>
            <td title={w.path}>
              {w.branch ?? (w.detached ? t("worktrees.badge.detached") : "—")}
              {#if w.isMain}<span class="badge">
                  {t("worktrees.badge.main")}
                </span>{/if}
              {#if w.detached && w.branch}<span class="badge">
                  {t("worktrees.badge.detached")}
                </span>{/if}
            </td>
            <td>
              {#if !w.exists}
                <span class="badge warn">{t("common.worktree.missing")}</span>
              {:else if w.dirtyCount > 0}
                <span class="badge dirty">
                  {t("common.worktree.dirty", { count: w.dirtyCount })}
                </span>
              {:else}
                <span class="badge ok">{t("worktrees.badge.clean")}</span>
              {/if}
            </td>
            <td>{sync ?? t("worktrees.sync.noUpstream")}</td>
            <td title={w.lastCommit?.subject}>
              {#if w.lastCommit}
                {formatRelativeAge(ageSecs(w.lastCommit.timestamp))}
              {:else}
                {t("common.worktree.noCommits")}
              {/if}
            </td>
            <td class="actions">
              <button
                type="button"
                disabled={!w.exists}
                onclick={() =>
                  void act(() =>
                    api.call("worktree_open_vscode", { path: w.path }),
                  )}
              >
                {t("worktrees.actions.openVscode")}
              </button>
              <button
                type="button"
                disabled={!w.exists}
                onclick={() =>
                  void act(() =>
                    api.call("worktree_open_terminal", { path: w.path }),
                  )}
              >
                {t("worktrees.actions.openTerminal")}
              </button>
              <button
                type="button"
                disabled={w.isMain}
                onclick={() => (pending = w)}
              >
                {t("worktrees.actions.remove")}
              </button>
            </td>
          </tr>
        {/snippet}
      </GroupedTable>
    {/if}
  {/if}
</section>

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
  {busy}
  onconfirm={() => void confirmRemove()}
  oncancel={cancelRemove}
/>

<style>
  .toolbar {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
    align-items: center;
    margin-bottom: 0.75rem;
  }
  .toolbar .root {
    margin-right: auto;
    word-break: break-all;
  }
  .sr {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
  }
  .badge {
    display: inline-block;
    padding: 0 0.4rem;
    border-radius: 0.6rem;
    font-size: 0.75rem;
    background: #8882;
  }
  .badge.dirty {
    background: #e0a40033;
  }
  .badge.ok {
    background: #2e9e5b33;
  }
  .badge.warn {
    background: #d9383833;
  }
  .actions {
    display: flex;
    gap: 0.25rem;
    flex-wrap: wrap;
  }
  .error small {
    display: block;
    opacity: 0.7;
  }
</style>
