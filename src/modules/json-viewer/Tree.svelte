<script lang="ts">
  import VirtualList from "../../components/VirtualList.svelte";
  import { toAppError } from "../../lib/api";
  import { errorMessage } from "../../lib/errors";
  import { formatNumber } from "../../lib/format";
  import { t } from "../../lib/i18n/i18n.svelte";
  import { jsonApi, type JsonNode, type SessionInfo } from "./jsonApi";
  import { PAGE_SIZE, TreeModel, type Row } from "./tree";

  interface Props {
    session: SessionInfo;
    root: JsonNode;
    selectedId: number | null;
    height?: number;
    onselect: (node: JsonNode) => void;
  }
  let { session, root, selectedId, height = 480, onselect }: Props = $props();

  const ROW_HEIGHT = 24;
  const INDENT = 16;

  // A new model per opened file; mutations bump `version` to refresh the flat list.
  const model = $derived(
    new TreeModel(root, (id, offset, limit) =>
      jsonApi.children(session.sessionId, id, offset, limit),
    ),
  );
  let version = $state(0);
  let error = $state<string | null>(null);
  const rows = $derived.by(() => {
    void version;
    return model.rows();
  });

  async function run(action: () => Promise<void>) {
    error = null;
    const pending = action();
    version++; // shows the "loading" state while the page is fetched
    try {
      await pending;
    } catch (e) {
      error = errorMessage(toAppError(e));
    }
    version++;
  }

  const toggle = (id: number) => run(() => model.toggle(id));
  const more = (id: number) => run(() => model.loadMore(id));

  function label(n: JsonNode, depth: number): string {
    if (depth === 0) return "$";
    if (n.key != null) return n.key;
    return `[${n.index ?? 0}]`;
  }

  function summary(n: JsonNode): string {
    const count = n.childCount ?? 0;
    if (n.kind === "object" || n.kind === "array") {
      if (count === 0) return t("jsonViewer.tree.empty");
      return n.kind === "object"
        ? t("jsonViewer.tree.keys", { count })
        : t("jsonViewer.tree.items", { count });
    }
    return n.preview ?? "";
  }
</script>

{#if error}
  <p class="error" role="alert">{error}</p>
{/if}
<div role="tree" aria-label={t("jsonViewer.tree.label")}>
  <VirtualList items={rows} rowHeight={ROW_HEIGHT} {height} row={rowView} />
</div>

{#snippet rowView(row: Row)}
  {#if row.type === "more"}
    <div class="row" style:padding-left="{row.depth * INDENT + 24}px">
      <button
        type="button"
        disabled={row.loading}
        onclick={() => more(row.parentId)}
      >
        {row.loading
          ? t("jsonViewer.tree.loading")
          : t("jsonViewer.tree.loadMore", {
              count: Math.min(PAGE_SIZE, row.remaining),
              remaining: formatNumber(row.remaining),
            })}
      </button>
    </div>
  {:else}
    {@const n = row.entry.node}
    <div
      class="row"
      class:selected={selectedId === n.id}
      role="treeitem"
      aria-selected={selectedId === n.id}
      aria-expanded={n.hasChildren ? row.entry.expanded : undefined}
      style:padding-left="{row.depth * INDENT}px"
    >
      {#if n.hasChildren}
        <button
          type="button"
          class="twisty"
          aria-label={row.entry.expanded
            ? t("jsonViewer.tree.collapse")
            : t("jsonViewer.tree.expand")}
          onclick={() => toggle(n.id)}
        >
          {row.entry.expanded ? "▾" : "▸"}
        </button>
      {:else}
        <span class="twisty"></span>
      {/if}
      <button
        type="button"
        class="node"
        onclick={() => onselect(n)}
        ondblclick={() => n.hasChildren && toggle(n.id)}
      >
        <span class="key">{label(n, row.depth)}</span>
        <span class="value kind-{n.kind}">{summary(n)}</span>
      </button>
    </div>
  {/if}
{/snippet}

<style>
  .row {
    display: flex;
    align-items: center;
    height: 100%;
    white-space: nowrap;
    font-family: ui-monospace, monospace;
    font-size: 0.85rem;
  }
  .row.selected {
    background: #3b82f633;
  }
  .twisty {
    width: 24px;
    flex: none;
    border: 0;
    background: none;
    padding: 0;
    cursor: pointer;
  }
  .node {
    display: flex;
    gap: 0.6rem;
    min-width: 0;
    border: 0;
    background: none;
    padding: 0 0.25rem;
    cursor: pointer;
    text-align: left;
    color: inherit;
  }
  .key {
    font-weight: 600;
  }
  .value {
    overflow: hidden;
    text-overflow: ellipsis;
    opacity: 0.8;
  }
  .error {
    color: #c0392b;
  }
</style>
