<script lang="ts">
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

  const PAGE_SIZE = 50;
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
  let page = $state(0);

  const pages = $derived(Math.max(1, Math.ceil(hits.length / PAGE_SIZE)));
  const visible = $derived(
    hits.slice(page * PAGE_SIZE, (page + 1) * PAGE_SIZE),
  );

  async function run(e: Event) {
    e.preventDefault();
    if (!query || searching || (!inKeys && !inValues)) return;
    hits = [];
    page = 0;
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
</script>

<section class="search">
  <h3>{t("jsonViewer.search.title")}</h3>
  <form onsubmit={run}>
    <input
      type="search"
      bind:value={query}
      placeholder={t("jsonViewer.search.placeholder")}
      aria-label={t("jsonViewer.search.placeholder")}
    />
    <label>
      <input type="checkbox" bind:checked={inKeys} />
      {t("jsonViewer.search.inKeys")}
    </label>
    <label>
      <input type="checkbox" bind:checked={inValues} />
      {t("jsonViewer.search.inValues")}
    </label>
    <label>
      <input type="checkbox" bind:checked={caseSensitive} />
      {t("jsonViewer.search.caseSensitive")}
    </label>
    {#if searching}
      <button type="button" onclick={stop}>
        {t("jsonViewer.search.stop")}
      </button>
    {:else}
      <button type="submit" disabled={!query || (!inKeys && !inValues)}>
        {t("jsonViewer.search.run")}
      </button>
    {/if}
  </form>

  {#if error}
    <p class="error" role="alert">{error}</p>
  {/if}
  {#if searching}
    <p>
      {t("jsonViewer.search.searching", { count: formatNumber(hits.length) })}
    </p>
  {:else if finished}
    <p>
      {hits.length === 0
        ? t("jsonViewer.search.none")
        : t("jsonViewer.search.results", { count: hits.length })}
    </p>
    {#if truncated}
      <p>
        {t("jsonViewer.search.truncated", { count: formatNumber(hits.length) })}
      </p>
    {/if}
  {/if}

  {#if visible.length > 0}
    <ul class="hits">
      {#each visible as hit, i (page * PAGE_SIZE + i)}
        <li>
          <button type="button" onclick={() => onselect(hit)}>
            <span class="badge">
              {hit.kind === "key"
                ? t("jsonViewer.search.hitKey")
                : t("jsonViewer.search.hitValue")}
            </span>
            <code class="path">{hit.path}</code>
            <span class="snippet">{hit.snippet}</span>
          </button>
        </li>
      {/each}
    </ul>
    {#if pages > 1}
      <nav class="pager">
        <button type="button" disabled={page === 0} onclick={() => page--}>
          {t("jsonViewer.search.prev")}
        </button>
        <span>{t("jsonViewer.search.page", { page: page + 1, pages })}</span>
        <button
          type="button"
          disabled={page >= pages - 1}
          onclick={() => page++}
        >
          {t("jsonViewer.search.next")}
        </button>
      </nav>
    {/if}
  {/if}
</section>

<style>
  form {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
    align-items: center;
  }
  form input[type="search"] {
    flex: 1 1 12rem;
  }
  .hits {
    list-style: none;
    margin: 0.5rem 0;
    padding: 0;
    max-height: 18rem;
    overflow: auto;
  }
  .hits button {
    display: flex;
    gap: 0.5rem;
    width: 100%;
    text-align: left;
    border: 0;
    background: none;
    padding: 0.2rem 0.25rem;
    cursor: pointer;
    color: inherit;
  }
  .hits button:hover {
    background: #8882;
  }
  .badge {
    flex: none;
    font-size: 0.7rem;
    text-transform: uppercase;
    opacity: 0.7;
  }
  .path {
    flex: none;
  }
  .snippet {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    opacity: 0.75;
  }
  .pager {
    display: flex;
    gap: 0.5rem;
    align-items: center;
  }
  .error {
    color: #c0392b;
  }
</style>
