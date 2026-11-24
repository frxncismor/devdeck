<script lang="ts">
  import { onDestroy, untrack } from "svelte";
  import ConfirmDialog from "../../components/ConfirmDialog.svelte";
  import { toAppError } from "../../lib/api";
  import { configState } from "../../lib/config.svelte";
  import { errorMessage } from "../../lib/errors";
  import { formatBytes } from "../../lib/format";
  import { t } from "../../lib/i18n/i18n.svelte";
  import { currentScope, refreshProjects } from "../../lib/scope.svelte";
  import type { AppError, Grouped } from "../../lib/types";
  import { isDirtyError } from "../worktrees/logic";
  import FoldersTab from "./FoldersTab.svelte";
  import {
    folderRejectionKey,
    removalWarnings,
    type DevProcess,
    type ForgottenWorktree,
    type HeavyFolder,
    type JobProgress,
    type Pending,
    type ReclaimSummary,
  } from "./logic";
  import ProcessesTab from "./ProcessesTab.svelte";
  import { reclaimApi, startFolderScan } from "./reclaimApi";
  import Summary from "./Summary.svelte";
  import WorktreesTab from "./WorktreesTab.svelte";

  type Tab = "worktrees" | "processes" | "folders";
  const TABS: Tab[] = ["worktrees", "processes", "folders"];

  let tab = $state<Tab>("worktrees");
  let summary = $state<ReclaimSummary | null>(null);
  let worktrees = $state<Grouped<ForgottenWorktree> | null>(null);
  let processes = $state<Grouped<DevProcess> | null>(null);
  let folders = $state<Grouped<HeavyFolder> | null>(null);
  let loadingWorktrees = $state(false);
  let loadingProcesses = $state(false);
  let scanning = $state(false);
  let progress = $state<JobProgress | null>(null);
  let cancelled = $state(false);
  let jobId = $state<string | null>(null);
  let error = $state<AppError | null>(null);
  let notice = $state<string | null>(null);
  let pending = $state<Pending | null>(null);
  let busy = $state(false);

  const root = $derived(configState.value.root);
  const scope = $derived(configState.value.scope);
  const measuring = $derived(loadingWorktrees || scanning);

  function fail(e: unknown): void {
    error = toAppError(e);
  }

  function describe(e: AppError): string {
    const key = folderRejectionKey(e);
    const head = errorMessage(e);
    return key ? `${head} ${t(key)}` : head;
  }

  async function loadSummary(): Promise<void> {
    try {
      summary = await reclaimApi.summary(currentScope());
    } catch (e) {
      fail(e);
    }
  }

  /** Out-of-order guards: only the latest call of each loader may assign data or clear its spinner. */
  let processesSeq = 0;
  let worktreesSeq = 0;
  let foldersSeq = 0;

  async function loadProcesses(): Promise<void> {
    const mine = ++processesSeq;
    loadingProcesses = true;
    try {
      const result = await reclaimApi.processes(currentScope());
      if (mine !== processesSeq) return;
      processes = result;
    } catch (e) {
      if (mine === processesSeq) fail(e);
    } finally {
      if (mine === processesSeq) loadingProcesses = false;
    }
    await loadSummary();
  }

  async function loadWorktrees(): Promise<void> {
    const mine = ++worktreesSeq;
    loadingWorktrees = true;
    try {
      const result = await reclaimApi.forgotten(currentScope());
      if (mine !== worktreesSeq) return;
      worktrees = result;
    } catch (e) {
      if (mine === worktreesSeq) fail(e);
    } finally {
      if (mine === worktreesSeq) loadingWorktrees = false;
    }
    await loadSummary();
  }

  async function loadFolders(): Promise<void> {
    // A newer scan (scope change or refresh) supersedes the running one: cancel it and discard its result.
    const mine = ++foldersSeq;
    const scanScope = JSON.stringify(currentScope());
    if (jobId) void reclaimApi.cancel(jobId).catch(() => undefined);
    jobId = null;
    scanning = true;
    cancelled = false;
    progress = null;
    try {
      const scan = await startFolderScan(currentScope(), (p) => {
        if (mine === foldersSeq) progress = p;
      });
      if (mine !== foldersSeq) {
        void reclaimApi.cancel(scan.jobId).catch(() => undefined);
        return;
      }
      jobId = scan.jobId;
      const done = await scan.done;
      if (mine !== foldersSeq || scanScope !== JSON.stringify(currentScope()))
        return;
      if (done.cancelled) cancelled = true;
      else if (done.error) error = done.error;
      else if (done.folders) folders = done.folders;
    } catch (e) {
      if (mine === foldersSeq) fail(e);
    } finally {
      if (mine === foldersSeq) {
        scanning = false;
        jobId = null;
        progress = null;
      }
    }
    if (mine === foldersSeq) await loadSummary();
  }

  async function cancelFolders(): Promise<void> {
    if (jobId) await reclaimApi.cancel(jobId).catch(fail);
  }

  function refreshAll(): void {
    error = null;
    notice = null;
    void loadProcesses();
    void loadWorktrees();
    void loadFolders();
  }

  onDestroy(() => {
    foldersSeq++;
    if (jobId) void reclaimApi.cancel(jobId).catch(() => undefined);
  });

  // Reload everything when the scope or the root changes (and once on mount).
  $effect(() => {
    void scope;
    void root;
    untrack(refreshAll);
  });

  async function confirmPending(): Promise<void> {
    const p = pending;
    if (!p || busy) return;
    busy = true;
    error = null;
    notice = null;
    try {
      if (p.type === "worktree") {
        await reclaimApi.removeWorktree(p.item.worktree.path, p.force);
        pending = null;
        await loadWorktrees();
        await refreshProjects();
      } else if (p.type === "process") {
        await reclaimApi.killProcess(p.item.pid, p.item.name, p.item.startTime);
        pending = null;
        await loadProcesses();
      } else {
        const freed = await reclaimApi.deleteFolder(p.item.path);
        pending = null;
        notice = t("reclaim.deleteFolder.done", {
          size: formatBytes(freed.freedBytes),
        });
        await loadFolders();
      }
    } catch (e) {
      const err = toAppError(e);
      if (p.type === "worktree" && isDirtyError(err) && !p.force) {
        // Second confirmation: removing discards the uncommitted changes.
        pending = { ...p, force: true };
        return;
      }
      pending = null;
      error = err;
      // A process that already vanished or a folder that is gone: refresh what is shown.
      if (p.type === "process") void loadProcesses();
    } finally {
      busy = false;
    }
  }

  const dialog = $derived.by(() => {
    const p = pending;
    if (!p) return { title: "", message: "", confirm: "" };
    if (p.type === "worktree") {
      const path = p.item.worktree.path;
      if (p.force) {
        return {
          title: t("common.worktree.remove.forceTitle"),
          message: t("common.worktree.remove.forceMessage", { path }),
          confirm: t("common.worktree.remove.forceConfirm"),
        };
      }
      const w = removalWarnings(p.item);
      let message = t("reclaim.removeWorktree.message", {
        path,
        size:
          p.item.diskBytes != null
            ? formatBytes(p.item.diskBytes)
            : t("reclaim.common.unknown"),
      });
      if (w.live > 0) {
        message += ` ${t("reclaim.removeWorktree.liveWarning", { count: w.live })}`;
      }
      return {
        title: t("common.worktree.remove.title"),
        message,
        confirm: t("common.worktree.remove.confirm"),
      };
    }
    if (p.type === "process") {
      return {
        title: t("reclaim.terminate.title"),
        message: t("reclaim.terminate.message", {
          name: p.item.name,
          pid: p.item.pid,
          ram: formatBytes(p.item.ramBytes),
        }),
        confirm: t("reclaim.terminate.confirm"),
      };
    }
    return {
      title: t("reclaim.deleteFolder.title"),
      message: t("reclaim.deleteFolder.message", {
        path: p.item.path,
        size:
          p.item.diskBytes != null
            ? formatBytes(p.item.diskBytes)
            : t("reclaim.common.unknown"),
      }),
      confirm: t("reclaim.deleteFolder.confirm"),
    };
  });
</script>

<section class="reclaim">
  <h2>{t("reclaim.title")}</h2>
  <p class="intro">{t("reclaim.intro")}</p>

  <div class="toolbar">
    <button type="button" onclick={refreshAll}>
      {t("reclaim.common.refresh")}
    </button>
  </div>

  {#if error}
    <p class="error" role="alert">
      {describe(error)}
      {#if error.detail}<small>{error.detail}</small>{/if}
    </p>
  {/if}
  {#if notice}<p class="notice" role="status">{notice}</p>{/if}

  <Summary {summary} {measuring} showProjects={scope.kind === "global"} />

  <div class="tabs" role="tablist" aria-label={t("reclaim.tabs.label")}>
    {#each TABS as id (id)}
      <button
        type="button"
        role="tab"
        id={`reclaim-tab-${id}`}
        aria-selected={tab === id}
        aria-controls="reclaim-panel"
        class:active={tab === id}
        onclick={() => (tab = id)}
      >
        {t(`reclaim.tabs.${id}`)}
      </button>
    {/each}
  </div>

  <div
    id="reclaim-panel"
    role="tabpanel"
    aria-labelledby={`reclaim-tab-${tab}`}
  >
    {#if tab === "worktrees"}
      <WorktreesTab
        data={worktrees}
        loading={loadingWorktrees}
        hasRoot={!!root}
        days={configState.value.forgottenDays}
        onremove={(item) =>
          (pending = { type: "worktree", item, force: false })}
      />
    {:else if tab === "processes"}
      <ProcessesTab
        data={processes}
        loading={loadingProcesses}
        onterminate={(item) => (pending = { type: "process", item })}
      />
    {:else}
      <FoldersTab
        data={folders}
        {scanning}
        {progress}
        {cancelled}
        hasRoot={!!root}
        oncancel={() => void cancelFolders()}
        ondelete={(item) => (pending = { type: "folder", item })}
      />
    {/if}
  </div>
</section>

<ConfirmDialog
  open={pending !== null}
  title={dialog.title}
  message={dialog.message}
  confirmLabel={dialog.confirm}
  {busy}
  onconfirm={() => void confirmPending()}
  oncancel={() => (pending = null)}
/>

<style>
  .intro {
    opacity: 0.8;
  }
  .toolbar {
    margin-bottom: 0.75rem;
  }
  .tabs {
    display: flex;
    gap: 0.25rem;
    margin: 1rem 0 0.75rem;
    border-bottom: 1px solid #8884;
  }
  .tabs button {
    border: none;
    border-bottom: 2px solid transparent;
    background: none;
    padding: 0.4rem 0.8rem;
    cursor: pointer;
  }
  .tabs button.active {
    border-bottom-color: currentColor;
    font-weight: 600;
  }
  .error small {
    display: block;
    opacity: 0.7;
  }
</style>
