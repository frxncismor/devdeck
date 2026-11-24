<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { onDestroy } from "svelte";
  import { toAppError } from "../../lib/api";
  import { formatBytes, formatPercent } from "../../lib/format";
  import { t } from "../../lib/i18n/i18n.svelte";
  import NodeActions from "./NodeActions.svelte";
  import Search from "./Search.svelte";
  import Tree from "./Tree.svelte";
  import {
    jsonApi,
    jsonErrorMessage,
    runJob,
    type JsonNode,
    type SessionInfo,
  } from "./jsonApi";

  let innerHeight = $state(720);
  const treeHeight = $derived(Math.max(240, innerHeight - 360));

  let session = $state<SessionInfo | null>(null);
  let root = $state<JsonNode | null>(null);
  let selectedId = $state<number | null>(null);

  let indexing = $state<{ name: string; done: number; total: number } | null>(
    null,
  );
  let indexJob = $state<string | null>(null);
  let status = $state<string | null>(null);
  let error = $state<string | null>(null);

  async function closeFile() {
    const s = session;
    session = null;
    root = null;
    selectedId = null;
    if (s) await jsonApi.close(s.sessionId).catch(() => undefined);
  }

  async function openFile() {
    error = null;
    status = null;
    const picked = await open({
      multiple: false,
      directory: false,
      title: t("jsonViewer.open.dialogTitle"),
    });
    if (typeof picked !== "string") return;
    await closeFile();
    const name = picked.split(/[\\/]/).pop() ?? picked;
    indexing = { name, done: 0, total: 0 };
    try {
      const job = await runJob(() => jsonApi.open(picked), {
        onProgress: (p) =>
          (indexing = { name, done: p.done, total: p.total ?? 0 }),
      });
      indexJob = job.ref.jobId;
      const done = await job.done;
      if (done.error) error = jsonErrorMessage(done.error);
      else if (done.cancelled) status = t("jsonViewer.status.cancelled");
      else if (done.session) {
        session = done.session;
        root = await jsonApi.root(done.session.sessionId);
      }
    } catch (e) {
      error = jsonErrorMessage(toAppError(e));
    } finally {
      indexing = null;
      indexJob = null;
    }
  }

  function cancelIndexing() {
    if (indexJob) void jsonApi.cancel(indexJob);
  }

  onDestroy(() => {
    if (session) void jsonApi.close(session.sessionId).catch(() => undefined);
  });
</script>

<svelte:window bind:innerHeight />

<section class="json-viewer">
  <h2>{t("common.sidebar.jsonViewer")}</h2>

  <div class="toolbar">
    <button type="button" onclick={openFile} disabled={indexing !== null}>
      {t("jsonViewer.open.button")}
    </button>
    {#if session}
      <button type="button" onclick={closeFile}>
        {t("jsonViewer.open.close")}
      </button>
      <span class="info">
        <code>{session.path}</code>
        · {formatBytes(session.sizeBytes)}
        · {t("jsonViewer.file.containers", { count: session.containers })}
      </span>
    {/if}
  </div>

  {#if indexing}
    <div class="progress">
      <p>{t("jsonViewer.status.indexing", { name: indexing.name })}</p>
      <progress value={indexing.done} max={indexing.total || 1}></progress>
      <span>
        {t("jsonViewer.status.progress", {
          done: formatBytes(indexing.done),
          total: formatBytes(indexing.total),
          percent: formatPercent(
            indexing.total ? indexing.done / indexing.total : 0,
          ),
        })}
      </span>
      <button type="button" onclick={cancelIndexing}>
        {t("jsonViewer.status.cancel")}
      </button>
    </div>
  {/if}

  {#if error}
    <p class="error" role="alert">{error}</p>
  {/if}
  {#if status}
    <p role="status">{status}</p>
  {/if}

  {#if session && root}
    <div class="layout">
      <div class="tree">
        <Tree
          {session}
          {root}
          {selectedId}
          height={treeHeight}
          onselect={(n) => (selectedId = n.id)}
        />
      </div>
      <div class="side">
        <NodeActions {session} nodeId={selectedId} />
        <Search {session} onselect={(h) => (selectedId = h.nodeId)} />
      </div>
    </div>
  {:else if !indexing}
    <p>{t("jsonViewer.intro")}</p>
  {/if}
</section>

<style>
  .toolbar {
    display: flex;
    flex-wrap: wrap;
    gap: 0.75rem;
    align-items: center;
  }
  .progress {
    display: flex;
    flex-wrap: wrap;
    gap: 0.75rem;
    align-items: center;
  }
  .progress progress {
    flex: 1 1 16rem;
  }
  .layout {
    display: grid;
    grid-template-columns: minmax(0, 3fr) minmax(0, 2fr);
    gap: 1rem;
    margin-top: 0.75rem;
  }
  .error {
    color: #c0392b;
  }
</style>
