<script lang="ts">
  import RefreshCw from "@lucide/svelte/icons/refresh-cw";
  import { onDestroy, untrack } from "svelte";
  import { SvelteMap, SvelteSet } from "svelte/reactivity";
  import Banner from "../../components/Banner.svelte";
  import GlossaryButton from "../../components/GlossaryButton.svelte";
  import Button from "../../components/Button.svelte";
  import ConfirmDialog from "../../components/ConfirmDialog.svelte";
  import Tabs from "../../components/Tabs.svelte";
  import TopBarContent from "../../components/TopBarContent.svelte";
  import { toAppError } from "../../lib/api";
  import { configState } from "../../lib/config.svelte";
  import { errorMessage } from "../../lib/errors";
  import { formatBytes, formatDateTime } from "../../lib/format";
  import { t } from "../../lib/i18n/i18n.svelte";
  import {
    currentScope,
    refreshProjects,
    scopeState,
  } from "../../lib/scope.svelte";
  import { toasts } from "../../lib/toasts.svelte";
  import type { AppError, Grouped } from "../../lib/types";
  import { removeAndReload } from "../worktrees/logic";
  import FoldersTab from "./FoldersTab.svelte";
  import {
    advanceDelete,
    folderRejectionKey,
    processKey,
    startDeleteProgress,
    type DeleteProgress,
    projectSlices,
    reloadAfterRemoval,
    removalWarnings,
    terminateMessage,
    totalsOf,
    worktreeSlices,
    type DevProcess,
    type ForgottenWorktree,
    type HeavyFolder,
    type JobProgress,
    type Pending,
    type ReclaimSummary,
    type Slice,
  } from "./logic";
  import ProcessesTab from "./ProcessesTab.svelte";
  import { reclaimApi, startFolderScan } from "./reclaimApi";
  import Summary from "./Summary.svelte";
  import WorktreesTab from "./WorktreesTab.svelte";

  type Tab = "worktrees" | "processes" | "folders";

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
  let scannedAt = $state<Date | null>(null);
  let selected = $state<ReadonlySet<string>>(new Set());
  let pending = $state<Pending | null>(null);
  /** Keys (worktree path, process key, folder path) of the items being removed right now. */
  const busyKeys = new SvelteSet<string>();
  /** "Deleting 2 of 3 · 1.4 GB freed" state while folders are being deleted. */
  let deleting = $state<DeleteProgress | null>(null);

  const root = $derived(configState.value.root);
  const scope = $derived(configState.value.scope);
  const measuring = $derived(loadingWorktrees || scanning);
  /** Any of the three scans running: the Rescan button shows a spinner and is disabled. */
  const refreshing = $derived(measuring || loadingProcesses);
  const forgottenDays = $derived(configState.value.forgottenDays);
  const grouped = $derived(scope.kind === "global");

  const tabDefs = $derived([
    {
      id: "worktrees",
      label: t("reclaim.tabs.worktrees"),
      count: totalsOf(worktrees).count,
    },
    {
      id: "processes",
      label: t("reclaim.tabs.processes"),
      count: totalsOf(processes).count,
    },
    {
      id: "folders",
      label: t("reclaim.tabs.folders"),
      count: totalsOf(folders).count,
    },
  ]);

  const allFolders = $derived(
    folders ? folders.groups.flatMap((g) => g.items) : [],
  );
  const selectedFolders = $derived(
    allFolders.filter((f) => selected.has(f.path)),
  );
  const selectedBytes = $derived(
    selectedFolders.reduce((n, f) => n + (f.diskBytes ?? 0), 0),
  );

  // Drop selections whose folder is no longer listed (after a rescan or a deletion).
  $effect(() => {
    const present = new SvelteSet(allFolders.map((f) => f.path));
    const current = untrack(() => selected);
    const kept = [...current].filter((p) => present.has(p));
    if (kept.length !== current.size) selected = new Set(kept);
  });

  function toggleFolder(f: HeavyFolder): void {
    selected = selected.has(f.path)
      ? new Set([...selected].filter((p) => p !== f.path))
      : new Set([...selected, f.path]);
  }

  // Project scope: the roots of the worktrees of that project (main checkout included).
  const scopeRoots = $derived.by(() => {
    if (scope.kind !== "project") return [];
    const id = scope.id;
    const p = scopeState.projects.find((x) => x.id === id);
    return p ? [p.mainPath, ...p.worktreePaths] : [];
  });

  const ramSlices = $derived.by<Slice[]>(() => {
    if (!summary) return [];
    if (scope.kind === "global")
      return projectSlices(summary, "ramBytes", t("common.noProject"));
    return worktreeSlices(
      processes ? processes.groups.flatMap((g) => g.items) : [],
      scopeRoots,
      (p) => p.worktreePath ?? p.cwd,
      (p) => p.ramBytes,
    );
  });

  const diskSlices = $derived.by<Slice[]>(() => {
    if (!summary) return [];
    if (scope.kind === "global")
      return projectSlices(summary, "diskBytes", t("common.noProject"));
    const wts = worktrees ? worktrees.groups.flatMap((g) => g.items) : [];
    const parts = [
      ...worktreeSlices(
        wts,
        scopeRoots,
        (w) => w.worktree.path,
        (w) => w.diskBytes ?? 0,
      ),
      ...worktreeSlices(
        allFolders,
        scopeRoots,
        (f) => f.path,
        (f) => f.diskBytes ?? 0,
      ),
    ];
    const merged = new SvelteMap<string, Slice>();
    for (const s of parts) {
      const k = s.key ?? "";
      const prev = merged.get(k);
      merged.set(k, { ...s, value: (prev?.value ?? 0) + s.value });
    }
    return [...merged.values()].sort((a, b) => b.value - a.value);
  });

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
    scannedAt = new Date();
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

  /** Deletes the folders one by one; a failure stops the loop and reports what was freed so far. */
  async function deleteFolders(items: HeavyFolder[]): Promise<void> {
    let progress = startDeleteProgress(items.length);
    deleting = progress;
    for (const f of items) busyKeys.add(f.path);
    try {
      for (const f of items) {
        const result = await reclaimApi.deleteFolder(f.path);
        progress = advanceDelete(progress, result.freedBytes);
        deleting = progress;
      }
      selected = new Set();
      toasts.success(
        t("reclaim.deleteFolder.done", {
          size: formatBytes(progress.freedBytes),
        }),
      );
    } catch (e) {
      if (progress.done > 0) {
        toasts.warning(
          t("reclaim.deleteFolder.partial", {
            done: progress.done,
            total: items.length,
            size: formatBytes(progress.freedBytes),
          }),
        );
      }
      error = toAppError(e);
    } finally {
      deleting = null;
      for (const f of items) busyKeys.delete(f.path);
    }
    await loadFolders();
  }

  function pendingKey(p: Pending): string | null {
    if (p.type === "worktree") return p.item.worktree.path;
    if (p.type === "process") return processKey(p.item);
    return null;
  }

  async function confirmPending(): Promise<void> {
    const p = pending;
    if (!p || deleting) return;
    const key = pendingKey(p);
    if (key && busyKeys.has(key)) return;
    // Close the dialog and keep working in the background: rows and the progress strip show the state.
    pending = null;
    error = null;
    if (key) busyKeys.add(key);
    try {
      if (p.type === "worktree") {
        const { path } = p.item.worktree;
        const outcome = await removeAndReload({
          force: p.force,
          remove: async () => {
            await reclaimApi.removeWorktree(path, p.force);
            toasts.success(t("reclaim.toast.worktreeRemoved"), path);
          },
          onError: (err) => (error = err),
          reload: () =>
            reloadAfterRemoval({
              folders: loadFolders,
              worktrees: loadWorktrees,
              projects: refreshProjects,
            }),
        });
        // Second confirmation: removing discards the uncommitted changes.
        if (outcome === "dirty") pending = { ...p, force: true };
      } else if (p.type === "process") {
        await reclaimApi.killProcess(p.item.pid, p.item.name, p.item.startTime);
        toasts.success(
          t("reclaim.toast.processTerminated", { name: p.item.name }),
        );
        await loadProcesses();
      } else {
        await deleteFolders(p.items);
      }
    } catch (e) {
      error = toAppError(e);
      // A process that already vanished: refresh what is shown.
      if (p.type === "process") void loadProcesses();
    } finally {
      if (key) busyKeys.delete(key);
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
        message: terminateMessage(p.item),
        confirm: t("reclaim.terminate.confirm"),
      };
    }
    return {
      title: t("reclaim.deleteFolder.title", { count: p.items.length }),
      message: t("reclaim.deleteFolder.message"),
      confirm: t("reclaim.deleteFolder.confirm", {
        size: formatBytes(p.items.reduce((n, f) => n + (f.diskBytes ?? 0), 0)),
      }),
    };
  });
</script>

<TopBarContent>
  {#snippet meta()}
    {#if scannedAt}
      <span class="tabular">
        {t("reclaim.scannedAt", { time: formatDateTime(scannedAt) })}
      </span>
    {/if}
  {/snippet}
  {#snippet actions()}
    <Button icon={RefreshCw} kbd="F5" loading={refreshing} onclick={refreshAll}>
      {t("reclaim.common.refresh")}
    </Button>
    <GlossaryButton module="reclaim" params={{ days: forgottenDays }} />
  {/snippet}
</TopBarContent>

<svelte:window
  onkeydown={(e) => {
    if (e.key === "F5") {
      e.preventDefault();
      if (!refreshing) refreshAll();
    }
  }}
/>

{#if error}
  <Banner tone="danger">
    {describe(error)}
    {#if error.detail}<small class="detail">{error.detail}</small>{/if}
  </Banner>
{/if}
<Banner tone="info">{t("reclaim.readOnly")}</Banner>

<Summary {summary} {measuring} byProject={grouped} {ramSlices} {diskSlices} />

<Tabs
  tabs={tabDefs}
  active={tab}
  onchange={(id) => (tab = id as Tab)}
  ariaLabel={t("reclaim.tabs.label")}
  idPrefix="reclaim"
/>

<div
  id={`reclaim-panel-${tab}`}
  role="tabpanel"
  aria-labelledby={`reclaim-tab-${tab}`}
  class="panel"
>
  {#if tab === "worktrees"}
    <WorktreesTab
      data={worktrees}
      loading={loadingWorktrees}
      hasRoot={!!root}
      days={configState.value.forgottenDays}
      {grouped}
      busy={busyKeys}
      onremove={(item) => (pending = { type: "worktree", item, force: false })}
    />
  {:else if tab === "processes"}
    <ProcessesTab
      data={processes}
      loading={loadingProcesses}
      {grouped}
      busy={busyKeys}
      onterminate={(item) => (pending = { type: "process", item })}
    />
  {:else}
    <FoldersTab
      data={folders}
      {scanning}
      {progress}
      {cancelled}
      hasRoot={!!root}
      {grouped}
      {selected}
      {selectedBytes}
      {deleting}
      busy={busyKeys}
      ontoggle={toggleFolder}
      onclear={() => (selected = new Set())}
      oncancel={() => void cancelFolders()}
      ondelete={() => (pending = { type: "folders", items: selectedFolders })}
    />
  {/if}
</div>

<ConfirmDialog
  open={pending !== null}
  title={dialog.title}
  message={dialog.message}
  confirmLabel={dialog.confirm}
  onconfirm={() => void confirmPending()}
  oncancel={() => (pending = null)}
>
  {#snippet body()}
    {#if pending?.type === "folders"}
      <ul class="paths">
        {#each pending.items as f (f.path)}
          <li class="path" title={f.path}>{f.path}</li>
        {/each}
      </ul>
      <p class="total">
        {t("reclaim.deleteFolder.total")}:
        <b class="mono">
          {formatBytes(
            pending.items.reduce((n, f) => n + (f.diskBytes ?? 0), 0),
          )}
        </b>
      </p>
    {/if}
  {/snippet}
</ConfirmDialog>

<style>
  .detail {
    display: block;
    opacity: 0.8;
  }
  .panel {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .paths {
    margin: 0 0 8px;
    padding: 0;
    list-style: none;
    max-height: 160px;
    overflow: auto;
    font-size: 12px;
  }
  .total {
    margin: 0;
    font-size: 12.5px;
  }
</style>
