<script lang="ts">
  import { writeText } from "@tauri-apps/plugin-clipboard-manager";
  import { save } from "@tauri-apps/plugin-dialog";
  import Copy from "@lucide/svelte/icons/copy";
  import Braces from "@lucide/svelte/icons/braces";
  import Button from "../../components/Button.svelte";
  import IconButton from "../../components/IconButton.svelte";
  import { toAppError } from "../../lib/api";
  import { errorMessage } from "../../lib/errors";
  import { formatBytes, formatNumber, formatPercent } from "../../lib/format";
  import { t } from "../../lib/i18n/i18n.svelte";
  import { toasts } from "../../lib/toasts.svelte";
  import { jsonApi, runJob, type JsonNode, type SessionInfo } from "./jsonApi";

  interface Props {
    session: SessionInfo;
    /** Selected node id, or `null` when nothing is selected. */
    nodeId: number | null;
    /** Loaded node data when the selection comes from the tree (not from a search hit). */
    node?: JsonNode | null;
    depth?: number | null;
    /** JSONPath of the selection, resolved by the parent (also feeds the breadcrumb). */
    jsonPath?: string | null;
    /** True while a "save formatted" job runs (lets the file bar disable its own button). */
    saving?: boolean;
  }
  let {
    session,
    nodeId,
    node = null,
    depth = null,
    jsonPath = null,
    saving = $bindable(false),
  }: Props = $props();

  /** Largest value copied or formatted in memory (matches the backend cap). */
  const LIMIT = 5 * 1024 * 1024;
  const PREVIEW_CHARS = 20_000;

  let preview = $state<string | null>(null);
  let previewCut = $state(false);
  let saveJob = $state<string | null>(null);
  let savePercent = $state("");

  // A different selection clears the previous preview.
  $effect(() => {
    void nodeId;
    preview = null;
  });

  async function guard(action: () => Promise<void>) {
    try {
      await action();
    } catch (e) {
      toasts.error(errorMessage(toAppError(e)));
    }
  }

  const copied = () => toasts.success(t("jsonViewer.actions.copied"));

  const copyPath = () =>
    guard(async () => {
      await writeText(
        jsonPath ?? (await jsonApi.path(session.sessionId, nodeId!)),
      );
      copied();
    });

  const copyValue = () =>
    guard(async () => {
      const text = await jsonApi.value(session.sessionId, nodeId!, LIMIT);
      // The backend marks a cut value with a trailing ellipsis; raw JSON never ends with one.
      if (text.endsWith("…")) {
        toasts.error(
          t("jsonViewer.actions.valueCut", { max: formatBytes(LIMIT) }),
        );
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

  /** Pretty-prints the selected subtree into the preview block. */
  export const showPreview = () =>
    guard(async () => {
      if (nodeId === null) return;
      const text = await jsonApi.formatNode(session.sessionId, nodeId, LIMIT);
      previewCut = text.length > PREVIEW_CHARS;
      preview = previewCut ? text.slice(0, PREVIEW_CHARS) : text;
    });

  /** Saves the selected subtree (or the whole file) formatted to a file the user picks. */
  export function saveFormatted(whole: boolean) {
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
        if (done.error) toasts.error(errorMessage(done.error));
        else if (done.cancelled)
          toasts.info(t("jsonViewer.actions.saveCancelled"));
        else
          toasts.success(
            t("jsonViewer.actions.saved", {
              size: formatBytes(done.savedBytes ?? 0),
            }),
          );
      } finally {
        saving = false;
        saveJob = null;
      }
    });
  }

  function cancelSave() {
    if (saveJob) void jsonApi.cancel(saveJob);
  }

  const container = $derived(
    node !== null && (node.kind === "object" || node.kind === "array"),
  );
</script>

<aside class="panel" aria-label={t("jsonViewer.actions.title")}>
  <h3 class="head">{t("jsonViewer.actions.title")}</h3>

  {#if nodeId === null}
    <p class="empty subtle">{t("jsonViewer.actions.none")}</p>
  {:else}
    <section class="block">
      <div class="label">{t("jsonViewer.actions.jsonPath")}</div>
      <div class="path-box mono" title={jsonPath ?? undefined}>
        {jsonPath ?? t("jsonViewer.tree.loading")}
      </div>
      <div class="row">
        <Button size="sm" icon={Copy} onclick={copyPath}>
          {t("jsonViewer.actions.copyPath")}
        </Button>
        <Button size="sm" variant="ghost" onclick={copyValue}>
          {t("jsonViewer.actions.copyValue")}
        </Button>
        <Button size="sm" variant="ghost" onclick={copyFormatted}>
          {t("jsonViewer.actions.copyFormatted")}
        </Button>
        <Button
          size="sm"
          variant="ghost"
          disabled={saving}
          onclick={() => saveFormatted(false)}
        >
          {t("jsonViewer.actions.saveNode")}
        </Button>
      </div>
    </section>

    <dl class="block facts">
      <div>
        <dt>{t("jsonViewer.actions.type")}</dt>
        <dd class="mono">
          {node
            ? t(`jsonViewer.tree.kind.${node.kind}`)
            : t("jsonViewer.actions.unknown")}
        </dd>
      </div>
      <div>
        <dt>{t("jsonViewer.actions.children")}</dt>
        <dd class="mono">
          {container
            ? formatNumber(node?.childCount ?? 0)
            : t("jsonViewer.actions.unknown")}
        </dd>
      </div>
      <div>
        <dt>{t("jsonViewer.actions.depth")}</dt>
        <dd class="mono">
          {depth === null ? t("jsonViewer.actions.unknown") : depth}
        </dd>
      </div>
    </dl>

    <section class="block grow">
      <div class="between">
        <span class="label">{t("jsonViewer.actions.preview")}</span>
        <IconButton
          label={t("jsonViewer.actions.prettyPrint")}
          icon={Braces}
          size={24}
          onclick={showPreview}
        />
      </div>
      {#if preview !== null}
        <pre class="preview mono">{preview}</pre>
        {#if previewCut}
          <p class="subtle small">
            {t("jsonViewer.actions.previewCut", { count: PREVIEW_CHARS })}
          </p>
        {/if}
      {:else}
        <Button size="sm" variant="ghost" onclick={showPreview}>
          {t("jsonViewer.actions.previewLoad")}
        </Button>
      {/if}
    </section>
  {/if}

  {#if saving}
    <div class="block saving" role="status">
      <span>{t("jsonViewer.actions.saving", { percent: savePercent })}</span>
      <Button size="sm" onclick={cancelSave}>
        {t("jsonViewer.actions.cancelSave")}
      </Button>
    </div>
  {/if}
</aside>

<style>
  .panel {
    width: 320px;
    flex: none;
    display: flex;
    flex-direction: column;
    min-height: 0;
    overflow: auto;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
  }
  .head {
    margin: 0;
    padding: 12px 14px;
    border-bottom: 1px solid var(--border);
    font-size: 11px;
    font-weight: var(--weight-semibold);
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-subtle);
  }
  .empty {
    margin: 0;
    padding: 14px;
    font-size: 12.5px;
  }
  .block {
    padding: 14px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    border-bottom: 1px solid var(--border);
  }
  .block.grow {
    flex: 1;
    min-height: 0;
    border-bottom: 0;
    gap: 8px;
  }
  .label {
    font-size: 12px;
    color: var(--text-muted);
  }
  .path-box {
    padding: 9px 10px;
    font-size: 12.5px;
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    overflow-wrap: anywhere;
  }
  .row {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  .facts {
    margin: 0;
    gap: 8px;
  }
  .facts > div {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    font-size: 12px;
  }
  .facts dt {
    color: var(--text-subtle);
  }
  .facts dd {
    margin: 0;
    color: var(--text);
  }
  .between {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .preview {
    margin: 0;
    padding: 10px 12px;
    max-height: 320px;
    overflow: auto;
    font-size: 12px;
    line-height: 1.6;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
  }
  .small {
    margin: 0;
    font-size: 11.5px;
  }
  .saving {
    flex-direction: row;
    align-items: center;
    justify-content: space-between;
    font-size: 12px;
    border-top: 1px solid var(--border);
    border-bottom: 0;
  }
</style>
