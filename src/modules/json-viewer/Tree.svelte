<script lang="ts">
  import ChevronDown from "@lucide/svelte/icons/chevron-down";
  import ChevronRight from "@lucide/svelte/icons/chevron-right";
  import { tick } from "svelte";
  import Button from "../../components/Button.svelte";
  import VirtualList from "../../components/VirtualList.svelte";
  import { toAppError } from "../../lib/api";
  import { errorMessage } from "../../lib/errors";
  import { formatNumber } from "../../lib/format";
  import { t } from "../../lib/i18n/i18n.svelte";
  import { jsonApi, type JsonNode, type SessionInfo } from "./jsonApi";
  import { PAGE_SIZE, REVEAL_LIMIT, TreeModel, type Row } from "./tree";

  interface Props {
    session: SessionInfo;
    root: JsonNode;
    selectedId: number | null;
    height?: number;
    /** `depth` is the nesting level of the node (root = 0). */
    onselect: (node: JsonNode, depth: number) => void;
  }
  let {
    session,
    root,
    selectedId,
    height: fallbackHeight = 480,
    onselect,
  }: Props = $props();

  // The list fills the space between the error line and the footer: measured, not guessed, so
  // the footer never overlaps the last row.
  let viewport = $state(0);
  const height = $derived(viewport > 0 ? viewport : fallbackHeight);

  const ROW_HEIGHT = 26;

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
  const rendered = $derived(
    Math.min(rows.length, Math.ceil(height / ROW_HEIGHT) + 8),
  );

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

  let list = $state<ReturnType<typeof VirtualList<Row>> | null>(null);
  let tooFar = $state(false);
  let revealSeq = 0;

  /** Expands the tree down to a node (a search hit) and scrolls to its row. */
  export async function reveal(path: string, nodeId: number) {
    const seq = ++revealSeq;
    const stale = () => seq !== revealSeq;
    tooFar = false;
    await run(async () => {
      const r = await model.reveal(path, nodeId, { stale });
      if (stale()) return;
      tooFar = r.tooFar ?? false;
      version++;
      await tick(); // the list must have its new height before it can scroll there
      const index = model.rowIndex(r.id);
      if (index >= 0) list?.scrollToIndex(index);
    });
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
        ? `{ ${t("jsonViewer.tree.keys", { count, n: formatNumber(count) })} }`
        : `[ ${t("jsonViewer.tree.items", { count, n: formatNumber(count) })} ]`;
    }
    return n.preview ?? "";
  }

  const isContainer = (n: JsonNode) =>
    n.kind === "object" || n.kind === "array";
  const kindLabel = (n: JsonNode) => t(`jsonViewer.tree.kind.${n.kind}`);
</script>

<div class="tree-wrap">
  {#if error}
    <p class="error" role="alert">{error}</p>
  {/if}
  {#if tooFar}
    <p class="note subtle" role="status">
      {t("jsonViewer.tree.revealTooFar", { max: formatNumber(REVEAL_LIMIT) })}
    </p>
  {/if}
  <div
    role="tree"
    aria-label={t("jsonViewer.tree.label")}
    class="tree"
    bind:clientHeight={viewport}
  >
    <div class="fill">
      <VirtualList
        bind:this={list}
        items={rows}
        rowHeight={ROW_HEIGHT}
        {height}
        row={rowView}
      />
    </div>
  </div>
  <p class="foot subtle">
    {t("jsonViewer.tree.virtualized", {
      rendered: formatNumber(rendered),
      total: formatNumber(rows.length),
    })}
  </p>
</div>

{#snippet rowView(row: Row)}
  {#if row.type === "more"}
    <div class="row pager">
      <span
        class="indent"
        style:width="{row.depth * 18 + 22}px"
        aria-hidden="true"
      >
        {#each { length: row.depth }, i (i)}
          <span class="guide"></span>
        {/each}
      </span>
      <Button
        size="sm"
        variant="ghost"
        disabled={row.loading}
        onclick={() => more(row.parentId)}
      >
        {row.loading
          ? t("jsonViewer.tree.loading")
          : t("jsonViewer.tree.loadMore", {
              count: Math.min(PAGE_SIZE, row.remaining),
              remaining: formatNumber(row.remaining),
            })}
      </Button>
    </div>
  {:else}
    {@const n = row.entry.node}
    {@const name = label(n, row.depth)}
    <div
      class="row"
      class:selected={selectedId === n.id}
      role="treeitem"
      tabindex="-1"
      aria-level={row.depth + 1}
      aria-selected={selectedId === n.id}
      aria-expanded={n.hasChildren ? row.entry.expanded : undefined}
      onclick={() => onselect(n, row.depth)}
      onkeydown={(e) => {
        if (e.key === "Enter") onselect(n, row.depth);
      }}
      ondblclick={() => n.hasChildren && toggle(n.id)}
    >
      <span class="indent" style:width="{row.depth * 18}px" aria-hidden="true">
        {#each { length: row.depth }, i (i)}
          <span class="guide"></span>
        {/each}
      </span>
      {#if n.hasChildren}
        <button
          type="button"
          class="twisty"
          aria-label={t(
            row.entry.expanded
              ? "jsonViewer.tree.collapse"
              : "jsonViewer.tree.expand",
            { name },
          )}
          onclick={(e) => {
            e.stopPropagation();
            void toggle(n.id);
          }}
        >
          {#if row.entry.expanded}<ChevronDown size={14} />{:else}<ChevronRight
              size={14}
            />{/if}
        </button>
      {:else}
        <span class="twisty-gap"></span>
      {/if}
      <span class="key" class:index={n.key == null}>{name}</span>
      <span class="colon">:</span>
      <span class="value kind-{n.kind}" class:container={isContainer(n)}>
        {summary(n)}
      </span>
      <span class="badge">{kindLabel(n)}</span>
    </div>
  {/if}
{/snippet}

<style>
  .tree-wrap {
    display: flex;
    flex-direction: column;
    min-height: 0;
    flex: 1;
  }
  .tree {
    position: relative;
    flex: 1;
    min-height: 0;
  }
  /* Absolute so the list's fixed height never feeds back into the measured viewport. */
  .fill {
    position: absolute;
    inset: 0;
    overflow: hidden;
  }
  .row {
    display: flex;
    align-items: center;
    height: 100%;
    padding: 0 12px 0 8px;
    gap: 4px;
    white-space: nowrap;
    font-family: var(--font-mono);
    font-size: 12.5px;
    cursor: default;
  }
  .row:hover {
    background: var(--surface-hover);
  }
  .row.selected {
    background: var(--accent-soft);
    box-shadow: inset 2px 0 0 var(--accent);
  }
  .row.pager {
    cursor: auto;
  }
  .row.pager:hover {
    background: none;
  }
  .indent {
    display: flex;
    flex: none;
    align-self: stretch;
  }
  .guide {
    width: 18px;
    flex: none;
    align-self: stretch;
    margin-left: 0;
    border-left: 1px solid var(--border);
    margin-right: -1px;
    box-sizing: border-box;
  }
  .twisty,
  .twisty-gap {
    width: 18px;
    height: 18px;
    flex: none;
  }
  .twisty {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0;
    border: 0;
    border-radius: var(--radius-sm);
    background: none;
    color: var(--text-subtle);
  }
  .twisty:hover {
    color: var(--text);
  }
  .key {
    color: var(--syntax-key);
  }
  .key.index,
  .colon {
    color: var(--text-subtle);
  }
  .value {
    flex: 1;
    min-width: 0;
    margin-left: 6px;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .value.container {
    color: var(--text-subtle);
  }
  .kind-string {
    color: var(--syntax-string);
  }
  .kind-number {
    color: var(--syntax-number);
  }
  .kind-bool {
    color: var(--syntax-bool);
  }
  .kind-null {
    color: var(--syntax-null);
  }
  .badge {
    display: inline-flex;
    align-items: center;
    flex: none;
    height: 18px;
    padding: 0 6px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    font-size: 10.5px;
    color: var(--text-subtle);
  }
  .foot {
    flex: none;
    margin: 0;
    padding: 0 12px;
    height: 24px;
    display: flex;
    align-items: center;
    border-top: 1px solid var(--border);
    font-size: 11.5px;
  }
  .note {
    margin: 0;
    padding: 6px 12px;
    font-size: 12px;
  }
  .error {
    margin: 0;
    padding: 6px 12px;
    color: var(--danger);
    font-size: 12px;
  }
</style>
