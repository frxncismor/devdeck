<script lang="ts">
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import { open } from "@tauri-apps/plugin-dialog";
  import { onDestroy, onMount } from "svelte";
  import Braces from "@lucide/svelte/icons/braces";
  import ChevronRight from "@lucide/svelte/icons/chevron-right";
  import FileJson from "@lucide/svelte/icons/file-json";
  import FolderOpen from "@lucide/svelte/icons/folder-open";
  import Save from "@lucide/svelte/icons/save";
  import Upload from "@lucide/svelte/icons/upload";
  import X from "@lucide/svelte/icons/x";
  import Banner from "../../components/Banner.svelte";
  import Button from "../../components/Button.svelte";
  import IconButton from "../../components/IconButton.svelte";
  import Progress from "../../components/Progress.svelte";
  import TopBarContent from "../../components/TopBarContent.svelte";
  import { toAppError } from "../../lib/api";
  import { errorMessage } from "../../lib/errors";
  import { formatBytes, formatNumber, formatPercent } from "../../lib/format";
  import { t } from "../../lib/i18n/i18n.svelte";
  import { toasts } from "../../lib/toasts.svelte";
  import NodeActions from "./NodeActions.svelte";
  import Search from "./Search.svelte";
  import Tree from "./Tree.svelte";
  import {
    jsonApi,
    runJob,
    type JsonNode,
    type SearchHit,
    type SessionInfo,
  } from "./jsonApi";
  import { loadRecent, parsePath, pushRecent, removeRecent } from "./recent";

  let session = $state<SessionInfo | null>(null);
  let root = $state<JsonNode | null>(null);
  let selectedId = $state<number | null>(null);
  let selectedNode = $state<JsonNode | null>(null);
  let selectedDepth = $state<number | null>(null);
  let selectedPath = $state<string | null>(null);
  let indexedMs = $state<number | null>(null);
  let saving = $state(false);
  let nodeActions = $state<ReturnType<typeof NodeActions> | null>(null);
  let bodyHeight = $state(400);
  const treeHeight = $derived(Math.max(120, bodyHeight - 34));

  let indexing = $state<{
    name: string;
    dir: string;
    done: number;
    total: number;
    startedAt: number;
  } | null>(null);
  let indexJob = $state<string | null>(null);
  let error = $state<string | null>(null);
  let dragging = $state(false);
  let recent = $state(loadRecent());
  let pathSeq = 0;

  const crumbs = $derived(selectedPath ? parsePath(selectedPath) : []);
  const eta = $derived.by(() => {
    const i = indexing;
    if (!i || i.total === 0 || i.done === 0) return null;
    const elapsed = (Date.now() - i.startedAt) / 1000;
    const left = Math.max(0, (elapsed * (i.total - i.done)) / i.done);
    return Math.max(1, Math.round(left));
  });

  function clearSelection() {
    pathSeq++;
    selectedId = null;
    selectedNode = null;
    selectedDepth = null;
    selectedPath = null;
  }

  async function closeFile() {
    const s = session;
    session = null;
    root = null;
    indexedMs = null;
    clearSelection();
    if (s) await jsonApi.close(s.sessionId).catch(() => undefined);
  }

  function select(id: number, node: JsonNode | null, depth: number | null) {
    selectedId = id;
    selectedNode = node;
    selectedDepth = depth;
    selectedPath = null;
    const seq = ++pathSeq;
    const s = session;
    if (!s) return;
    jsonApi
      .path(s.sessionId, id)
      .then((p) => {
        if (seq === pathSeq) selectedPath = p;
      })
      .catch(() => undefined);
  }

  async function pick() {
    const picked = await open({
      multiple: false,
      directory: false,
      title: t("jsonViewer.open.dialogTitle"),
    });
    if (typeof picked === "string") await openPath(picked);
  }

  async function openPath(picked: string) {
    if (indexing) return;
    error = null;
    await closeFile();
    const parts = picked.split(/[\\/]/);
    const name = parts.pop() ?? picked;
    const dir = parts.join(picked.includes("\\") ? "\\" : "/");
    const startedAt = Date.now();
    indexing = { name, dir, done: 0, total: 0, startedAt };
    try {
      const job = await runJob(() => jsonApi.open(picked), {
        onProgress: (p) =>
          (indexing = {
            name,
            dir,
            done: p.done,
            total: p.total ?? 0,
            startedAt,
          }),
      });
      indexJob = job.ref.jobId;
      const done = await job.done;
      if (done.error) error = errorMessage(done.error);
      else if (done.cancelled) toasts.info(t("jsonViewer.status.cancelled"));
      else if (done.session && destroyed) {
        // Left the module while indexing: close the session that nobody will use.
        void jsonApi.close(done.session.sessionId).catch(() => undefined);
      } else if (done.session) {
        session = done.session;
        indexedMs = Date.now() - startedAt;
        recent = pushRecent(picked);
        root = await jsonApi.root(done.session.sessionId);
      }
    } catch (e) {
      error = errorMessage(toAppError(e));
    } finally {
      indexing = null;
      indexJob = null;
    }
  }

  function cancelIndexing() {
    if (indexJob) void jsonApi.cancel(indexJob);
  }

  function onhit(h: SearchHit) {
    select(h.nodeId, null, null);
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.key === "Escape" && indexing) {
      cancelIndexing();
      e.preventDefault();
    } else if (
      e.ctrlKey &&
      !e.altKey &&
      !e.shiftKey &&
      e.key.toLowerCase() === "o"
    ) {
      e.preventDefault();
      if (!indexing) void pick();
    }
  }

  let destroyed = false;
  let unlistenDrop: (() => void) | null = null;
  onMount(() => {
    // Files dropped on the window (the webview swallows HTML drop events).
    try {
      void getCurrentWebview()
        .onDragDropEvent((e) => {
          const p = e.payload;
          if (p.type === "enter" || p.type === "over") dragging = true;
          else if (p.type === "leave") dragging = false;
          else if (p.type === "drop") {
            dragging = false;
            if (!session && p.paths.length > 0) void openPath(p.paths[0]);
          }
        })
        .then((un) => {
          if (destroyed) un();
          else unlistenDrop = un;
        })
        .catch(() => undefined);
    } catch {
      // No Tauri runtime (browser preview): the Open button still works.
    }
  });
  onDestroy(() => {
    destroyed = true;
    unlistenDrop?.();
    if (indexJob) void jsonApi.cancel(indexJob);
    if (session) void jsonApi.close(session.sessionId).catch(() => undefined);
  });
</script>

<svelte:window {onkeydown} />

<TopBarContent>
  {#snippet actions()}
    <Button
      icon={FolderOpen}
      kbd="Ctrl O"
      disabled={indexing !== null}
      onclick={pick}
    >
      {t("jsonViewer.open.button")}
    </Button>
  {/snippet}
</TopBarContent>

<section class="json-viewer">
  {#if error}
    <Banner tone="danger">{error}</Banner>
  {/if}

  {#if indexing}
    <div class="index-stage">
      <div class="skeleton" aria-hidden="true">
        {#each { length: 16 }, i (i)}
          <div class="skel-row" style:padding-left="{12 + (i % 3) * 18}px">
            <span style:width="{Math.max(20, 67 - i * 3)}%"></span>
          </div>
        {/each}
      </div>
      <div class="card" role="status">
        <div class="card-head">
          <span class="icon"><FileJson size={18} /></span>
          <div class="grow">
            <div class="strong">
              {t("jsonViewer.status.indexing", { name: indexing.name })}
            </div>
            <div class="path subtle" title={indexing.dir}>{indexing.dir}</div>
          </div>
          <span class="percent tabular">
            {formatPercent(indexing.total ? indexing.done / indexing.total : 0)}
          </span>
        </div>
        <div class="bar">
          <Progress
            value={indexing.total ? indexing.done / indexing.total : undefined}
            label={t("jsonViewer.status.indexing", { name: indexing.name })}
          />
        </div>
        <div class="stats tabular">
          <span>
            {t("jsonViewer.status.progress", {
              done: formatBytes(indexing.done),
              total: formatBytes(indexing.total),
            })}
          </span>
          {#if eta !== null}
            <span>{t("jsonViewer.status.eta", { time: `${eta} s` })}</span>
          {/if}
        </div>
        <div class="card-foot">
          <span class="subtle">{t("jsonViewer.status.note")}</span>
          <Button kbd="Esc" onclick={cancelIndexing}>
            {t("common.buttons.cancel")}
          </Button>
        </div>
      </div>
    </div>
  {:else if session && root}
    <div class="filebar">
      <span class="icon"><FileJson size={16} /></span>
      <div class="grow">
        <div class="strong">{session.path.split(/[\\/]/).pop()}</div>
        <div class="path subtle" title={session.path}>{session.path}</div>
      </div>
      <span class="stat">
        {t("jsonViewer.file.size")}
        <b class="mono tabular">{formatBytes(session.sizeBytes)}</b>
      </span>
      <span class="stat">
        {t("jsonViewer.file.containers")}
        <b class="mono tabular">{formatNumber(session.containers)}</b>
      </span>
      {#if indexedMs !== null}
        <span class="stat">
          {t("jsonViewer.file.indexedIn")}
          <b class="mono tabular">{(indexedMs / 1000).toFixed(1)} s</b>
        </span>
      {/if}
      <span class="sep" aria-hidden="true"></span>
      <Button
        size="sm"
        icon={Braces}
        disabled={selectedId === null}
        onclick={() => nodeActions?.showPreview()}
      >
        {t("jsonViewer.actions.prettyPrint")}
      </Button>
      <Button
        size="sm"
        variant="primary"
        icon={Save}
        disabled={saving}
        onclick={() => nodeActions?.saveFormatted(true)}
      >
        {t("jsonViewer.actions.saveFormatted")}
      </Button>
      <IconButton
        label={t("jsonViewer.open.close")}
        icon={X}
        onclick={closeFile}
      />
    </div>

    <Search {session} onselect={onhit} />

    <div class="main">
      <div class="treebox">
        <nav class="crumbs" aria-label={t("jsonViewer.tree.path")}>
          {#each crumbs as c, i (i)}
            {#if i > 0}<ChevronRight size={12} />{/if}
            <span class:last={i === crumbs.length - 1}>{c}</span>
          {:else}
            <span class="last">$</span>
          {/each}
        </nav>
        <div class="body" bind:clientHeight={bodyHeight}>
          <Tree
            {session}
            {root}
            {selectedId}
            height={treeHeight}
            onselect={(n, d) => select(n.id, n, d)}
          />
        </div>
      </div>
      <NodeActions
        bind:this={nodeActions}
        bind:saving
        {session}
        nodeId={selectedId}
        node={selectedNode}
        depth={selectedDepth}
        jsonPath={selectedPath}
      />
    </div>
  {:else}
    <div class="drop" class:dragging>
      <span class="icon big"><Upload size={22} /></span>
      <h2>
        {dragging ? t("jsonViewer.drop.active") : t("jsonViewer.drop.title")}
      </h2>
      <p class="subtle">{t("jsonViewer.drop.hint")}</p>
      <Button variant="primary" icon={FolderOpen} onclick={pick}>
        {t("jsonViewer.open.button")}
      </Button>
    </div>
    {#if recent.length > 0}
      <section class="recent" aria-label={t("jsonViewer.recent.title")}>
        <h3>{t("jsonViewer.recent.title")}</h3>
        <ul>
          {#each recent as p (p)}
            <li>
              <button
                type="button"
                class="recent-open"
                onclick={() => openPath(p)}
              >
                <FileJson size={14} />
                <span class="strong">{p.split(/[\\/]/).pop()}</span>
                <span class="path subtle" title={p}>{p}</span>
              </button>
              <IconButton
                label={t("jsonViewer.recent.remove")}
                icon={X}
                size={24}
                onclick={() => (recent = removeRecent(p))}
              />
            </li>
          {/each}
        </ul>
      </section>
    {/if}
  {/if}
</section>

<style>
  .json-viewer {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .grow {
    flex: 1;
    min-width: 0;
  }
  .strong {
    font-weight: var(--weight-semibold);
  }
  .icon {
    width: 32px;
    height: 32px;
    flex: none;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 8px;
    background: var(--accent-soft);
    color: var(--accent-text);
  }
  .path {
    font-family: var(--font-mono);
    font-size: 11.5px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .filebar {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 14px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
  }
  .stat {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: var(--text-subtle);
    white-space: nowrap;
  }
  .stat b {
    color: var(--text);
    font-weight: var(--weight-semibold);
  }
  .sep {
    width: 1px;
    height: 24px;
    background: var(--border);
  }
  .main {
    flex: 1;
    min-height: 0;
    display: flex;
    gap: 12px;
  }
  .treebox {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
  }
  .crumbs {
    display: flex;
    align-items: center;
    gap: 4px;
    height: 34px;
    flex: none;
    padding: 0 12px;
    overflow: hidden;
    white-space: nowrap;
    border-bottom: 1px solid var(--border);
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--text-muted);
  }
  .crumbs span {
    padding: 2px 4px;
  }
  .crumbs .last {
    color: var(--text);
  }
  .body {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
  .drop {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 10px;
    padding: 24px;
    text-align: center;
    border: 1.5px dashed var(--border-strong);
    border-radius: var(--radius-xl);
    background: var(--surface);
  }
  .drop.dragging {
    border-color: var(--accent);
    background: var(--accent-soft);
  }
  .drop h2 {
    margin: 0;
    font-size: var(--text-lg);
    font-weight: var(--weight-semibold);
  }
  .drop p {
    margin: 0 0 6px;
    max-width: 44ch;
    font-size: 12.5px;
  }
  .icon.big {
    width: 48px;
    height: 48px;
    border-radius: 12px;
  }
  .recent {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    overflow: hidden;
  }
  .recent h3 {
    margin: 0;
    padding: 10px 14px;
    border-bottom: 1px solid var(--border);
    font-size: 11px;
    font-weight: var(--weight-semibold);
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-subtle);
  }
  .recent ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .recent li {
    display: flex;
    align-items: center;
    gap: 4px;
    padding-right: 8px;
  }
  .recent-open {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 10px;
    height: 36px;
    padding: 0 14px;
    border: 0;
    background: none;
    color: var(--text);
    text-align: left;
    font-size: 13px;
  }
  .recent-open:hover {
    background: var(--surface-hover);
  }
  .recent-open .path {
    flex: 1;
    min-width: 0;
  }
  .index-stage {
    flex: 1;
    min-height: 0;
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
    overflow: hidden;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
  }
  .skeleton {
    position: absolute;
    inset: 0;
    padding: 12px;
    opacity: 0.6;
  }
  .skel-row {
    display: flex;
    align-items: center;
    height: 26px;
  }
  .skel-row span {
    display: block;
    height: 9px;
    border-radius: 4px;
    background: var(--surface-hover);
  }
  .card {
    position: relative;
    width: min(520px, 90%);
    padding: 22px 24px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-xl);
    box-shadow: var(--shadow-md);
  }
  .card-head {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .card-head .icon {
    width: 36px;
    height: 36px;
    border-radius: 9px;
  }
  .percent {
    font-size: 22px;
    font-weight: var(--weight-semibold);
  }
  .bar {
    margin: 18px 0 10px;
  }
  .stats {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    font-size: 12px;
    color: var(--text-muted);
  }
  .card-foot {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    margin-top: 20px;
    padding-top: 16px;
    border-top: 1px solid var(--border);
    font-size: 12px;
  }
</style>
