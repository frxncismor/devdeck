<script lang="ts">
  import ChevronDown from "@lucide/svelte/icons/chevron-down";
  import ChevronUp from "@lucide/svelte/icons/chevron-up";
  import SearchIcon from "@lucide/svelte/icons/search";
  import X from "@lucide/svelte/icons/x";
  import IconButton from "../../components/IconButton.svelte";
  import { toAppError } from "../../lib/api";
  import { errorMessage } from "../../lib/errors";
  import { formatNumber } from "../../lib/format";
  import { t } from "../../lib/i18n/i18n.svelte";
  import { jsonApi, runJob, type SearchHit, type SessionInfo } from "./jsonApi";

  interface Props {
    session: SessionInfo;
    onselect: (hit: SearchHit) => void;
  }
  let { session, onselect }: Props = $props();

  const DEFAULT_LIMIT = 10_000;

  let query = $state("");
  let inKeys = $state(true);
  let inValues = $state(true);
  let caseSensitive = $state(false);
  let hits = $state.raw<SearchHit[]>([]);
  let searching = $state(false);
  let jobId = $state<string | null>(null);
  let truncated = $state(false);
  let finished = $state(false);
  let error = $state<string | null>(null);
  let active = $state(-1);

  const current = $derived(active >= 0 ? hits[active] : undefined);
  const canSearch = $derived(query !== "" && (inKeys || inValues));

  async function run(e: Event) {
    e.preventDefault();
    if (!canSearch || searching) return;
    hits = [];
    active = -1;
    error = null;
    truncated = false;
    finished = false;
    searching = true;
    try {
      const job = await runJob(
        () =>
          jsonApi.search(session.sessionId, {
            query,
            inKeys,
            inValues,
            caseSensitive,
            limit: DEFAULT_LIMIT,
          }),
        { onBatch: (b) => (hits = hits.concat(b.hits)) },
      );
      jobId = job.ref.jobId;
      const done = await job.done;
      if (done.error) error = errorMessage(done.error);
      truncated = done.truncated ?? false;
      finished = !done.error;
      if (finished && hits.length > 0) go(0);
    } catch (err) {
      error = errorMessage(toAppError(err));
    } finally {
      searching = false;
      jobId = null;
    }
  }

  function stop() {
    if (jobId) void jsonApi.cancel(jobId);
  }

  function go(index: number) {
    if (hits.length === 0) return;
    active = (index + hits.length) % hits.length;
    onselect(hits[active]);
  }

  function clear() {
    query = "";
    hits = [];
    active = -1;
    finished = false;
    truncated = false;
    error = null;
  }
</script>

<div class="search">
  <form onsubmit={run}>
    <label class="field">
      <SearchIcon size={14} />
      <span class="sr-only">{t("jsonViewer.search.label")}</span>
      <input
        type="search"
        bind:value={query}
        placeholder={t("jsonViewer.search.placeholder")}
      />
      <span class="counter tabular" aria-live="polite">
        {#if searching}
          {t("jsonViewer.search.searching", {
            count: formatNumber(hits.length),
          })}
        {:else if finished}
          {hits.length === 0
            ? t("jsonViewer.search.none")
            : t("jsonViewer.search.counter", {
                current: formatNumber(active + 1),
                total: formatNumber(hits.length),
              })}
        {/if}
      </span>
    </label>
    {#if searching}
      <button type="button" class="stop" onclick={stop}>
        {t("jsonViewer.search.stop")}
      </button>
    {:else}
      <button type="submit" class="run" disabled={!canSearch}>
        {t("jsonViewer.search.run")}
      </button>
    {/if}
    <IconButton
      label={t("jsonViewer.search.prev")}
      icon={ChevronUp}
      variant="secondary"
      size={30}
      disabled={hits.length === 0}
      onclick={() => go(active - 1)}
    />
    <IconButton
      label={t("jsonViewer.search.next")}
      icon={ChevronDown}
      variant="secondary"
      size={30}
      disabled={hits.length === 0}
      onclick={() => go(active + 1)}
    />
    {#if query !== "" || hits.length > 0}
      <IconButton
        label={t("common.buttons.cancel")}
        icon={X}
        size={30}
        onclick={clear}
      />
    {/if}
    <div
      class="toggles"
      role="group"
      aria-label={t("jsonViewer.search.options")}
    >
      <button
        type="button"
        class="toggle"
        aria-pressed={caseSensitive}
        title={t("jsonViewer.search.caseSensitive")}
        aria-label={t("jsonViewer.search.caseSensitive")}
        onclick={() => (caseSensitive = !caseSensitive)}
      >
        Aa
      </button>
      <button
        type="button"
        class="toggle"
        aria-pressed={inKeys}
        onclick={() => (inKeys = !inKeys)}
      >
        {t("jsonViewer.search.inKeys")}
      </button>
      <button
        type="button"
        class="toggle"
        aria-pressed={inValues}
        onclick={() => (inValues = !inValues)}
      >
        {t("jsonViewer.search.inValues")}
      </button>
    </div>
  </form>

  {#if error}
    <p class="error" role="alert">{error}</p>
  {/if}
  {#if truncated}
    <p class="note subtle">
      {t("jsonViewer.search.truncated", { count: formatNumber(hits.length) })}
    </p>
  {/if}
  {#if current}
    <p class="hit">
      <span class="kind">
        {current.kind === "key"
          ? t("jsonViewer.search.hitKey")
          : t("jsonViewer.search.hitValue")}
      </span>
      <span class="path" title={current.path}>{current.path}</span>
      <span class="snippet subtle">{current.snippet}</span>
    </p>
  {/if}
</div>

<style>
  .search {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  form {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .field {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 8px;
    height: var(--control-h);
    min-width: 0;
    padding: 0 10px;
    background: var(--surface-2);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-md);
    color: var(--text-subtle);
  }
  .field:focus-within {
    border-color: var(--accent);
  }
  .field input {
    flex: 1;
    min-width: 0;
    background: none;
    border: 0;
    outline: none;
    color: var(--text);
    font-size: var(--text-base);
  }
  .counter {
    font-size: 12px;
    white-space: nowrap;
    color: var(--text-subtle);
  }
  .run,
  .stop,
  .toggle {
    height: var(--control-h-sm);
    padding: 0 9px;
    border-radius: var(--radius-md);
    font-size: var(--text-sm);
    font-weight: var(--weight-medium);
    white-space: nowrap;
    border: 1px solid var(--border-strong);
    background: var(--surface-2);
    color: var(--text);
  }
  .run {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--on-accent);
  }
  .run:hover:not(:disabled) {
    background: var(--accent-hover);
  }
  .run:disabled {
    opacity: 0.5;
  }
  .stop:hover {
    background: var(--surface-hover);
  }
  .toggles {
    display: flex;
    gap: 2px;
  }
  .toggle {
    border-color: transparent;
    background: transparent;
    color: var(--text-muted);
  }
  .toggle:hover {
    background: var(--surface-hover);
    color: var(--text);
  }
  .toggle[aria-pressed="true"] {
    background: var(--surface-2);
    border-color: var(--border-strong);
    color: var(--text);
  }
  .hit {
    display: flex;
    gap: 8px;
    align-items: baseline;
    margin: 0;
    font-size: 12px;
    min-width: 0;
  }
  .kind {
    flex: none;
    font-size: 10.5px;
    text-transform: uppercase;
    color: var(--accent-text);
  }
  .path {
    flex: none;
    max-width: 50%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-family: var(--font-mono);
  }
  .snippet {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
  }
  .note {
    margin: 0;
    font-size: 12px;
  }
  .error {
    margin: 0;
    color: var(--danger);
    font-size: 12px;
  }
</style>
