<script lang="ts">
  import { writeText } from "@tauri-apps/plugin-clipboard-manager";
  import { save } from "@tauri-apps/plugin-dialog";
  import { toAppError } from "../../lib/api";
  import { formatBytes, formatPercent } from "../../lib/format";
  import { t } from "../../lib/i18n/i18n.svelte";
  import {
    jsonApi,
    jsonErrorMessage,
    runJob,
    type SessionInfo,
  } from "./jsonApi";

  interface Props {
    session: SessionInfo;
    /** Selected node id, or `null` when nothing is selected. */
    nodeId: number | null;
  }
  let { session, nodeId }: Props = $props();

  /** Largest value copied or formatted in memory (matches the backend cap). */
  const LIMIT = 5 * 1024 * 1024;
  const PREVIEW_CHARS = 20_000;

  let message = $state<string | null>(null);
  let error = $state<string | null>(null);
  let preview = $state<string | null>(null);
  let previewCut = $state(false);
  let saving = $state(false);
  let saveJob = $state<string | null>(null);
  let savePercent = $state("");

  // A different selection clears the previous result.
  $effect(() => {
    void nodeId;
    message = null;
    error = null;
    preview = null;
  });

  async function guard(action: () => Promise<void>) {
    message = null;
    error = null;
    try {
      await action();
    } catch (e) {
      error = jsonErrorMessage(toAppError(e));
    }
  }

  const copied = () => (message = t("jsonViewer.actions.copied"));

  const copyPath = () =>
    guard(async () => {
      await writeText(await jsonApi.path(session.sessionId, nodeId!));
      copied();
    });

  const copyValue = () =>
    guard(async () => {
      const text = await jsonApi.value(session.sessionId, nodeId!, LIMIT);
      // The backend marks a cut value with a trailing ellipsis; raw JSON never ends with one.
      if (text.endsWith("…")) {
        error = t("jsonViewer.actions.valueCut", { max: formatBytes(LIMIT) });
        return;
      }
      await writeText(text);
      copied();
    });

  const copyFormatted = () =>
    guard(async () => {
      await writeText(
        await jsonApi.formatNode(session.sessionId, nodeId!, LIMIT),
      );
      copied();
    });

  const showPreview = () =>
    guard(async () => {
      const text = await jsonApi.formatNode(session.sessionId, nodeId!, LIMIT);
      previewCut = text.length > PREVIEW_CHARS;
      preview = previewCut ? text.slice(0, PREVIEW_CHARS) : text;
    });

  function saveFormatted(whole: boolean) {
    return guard(async () => {
      const name = session.path.split(/[\\/]/).pop() ?? "data.json";
      const dest = await save({
        title: t("jsonViewer.actions.saveDialogTitle"),
        defaultPath: name.replace(/(\.[^.]+)?$/, ".formatted.json"),
      });
      if (!dest) return;
      saving = true;
      savePercent = formatPercent(0);
      try {
        const job = await runJob(
          () =>
            jsonApi.saveFormatted(
              session.sessionId,
              whole ? null : nodeId,
              dest,
            ),
          {
            onProgress: (p) =>
              (savePercent = formatPercent(p.total ? p.done / p.total : 0)),
          },
        );
        saveJob = job.ref.jobId;
        const done = await job.done;
        if (done.error) error = jsonErrorMessage(done.error);
        else if (done.cancelled)
          message = t("jsonViewer.actions.saveCancelled");
        else
          message = t("jsonViewer.actions.saved", {
            size: formatBytes(done.savedBytes ?? 0),
          });
      } finally {
        saving = false;
        saveJob = null;
      }
    });
  }

  function cancelSave() {
    if (saveJob) void jsonApi.cancel(saveJob);
  }
</script>

<section class="actions-panel">
  <h3>{t("jsonViewer.actions.title")}</h3>
  <div class="buttons">
    <button type="button" disabled={nodeId === null} onclick={copyPath}>
      {t("jsonViewer.actions.copyPath")}
    </button>
    <button type="button" disabled={nodeId === null} onclick={copyValue}>
      {t("jsonViewer.actions.copyValue")}
    </button>
    <button type="button" disabled={nodeId === null} onclick={copyFormatted}>
      {t("jsonViewer.actions.copyFormatted")}
    </button>
    <button type="button" disabled={nodeId === null} onclick={showPreview}>
      {t("jsonViewer.actions.preview")}
    </button>
    <button
      type="button"
      disabled={nodeId === null || saving}
      onclick={() => saveFormatted(false)}
    >
      {t("jsonViewer.actions.saveFormatted")}
    </button>
    <button type="button" disabled={saving} onclick={() => saveFormatted(true)}>
      {t("jsonViewer.actions.saveWhole")}
    </button>
  </div>
  {#if nodeId === null}
    <p>{t("jsonViewer.actions.none")}</p>
  {/if}
  {#if saving}
    <p>
      {t("jsonViewer.actions.saving", { percent: savePercent })}
      <button type="button" onclick={cancelSave}>
        {t("jsonViewer.actions.cancelSave")}
      </button>
    </p>
  {/if}
  {#if message}
    <p role="status">{message}</p>
  {/if}
  {#if error}
    <p class="error" role="alert">{error}</p>
  {/if}
  {#if preview !== null}
    <pre class="preview">{preview}</pre>
    {#if previewCut}
      <p>{t("jsonViewer.actions.previewCut", { count: PREVIEW_CHARS })}</p>
    {/if}
  {/if}
</section>

<style>
  .buttons {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
  }
  .preview {
    max-height: 16rem;
    overflow: auto;
    background: #8881;
    padding: 0.5rem;
    font-size: 0.8rem;
  }
  .error {
    color: #c0392b;
  }
</style>
