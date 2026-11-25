<script lang="ts" module>
  export interface Column<T = unknown> {
    key: string;
    header: string;
    /** CSS grid track: "96px" (numbers/fixed) or "minmax(0, 1fr)" (paths/flex). Default "minmax(0, 1fr)". */
    width?: string;
    align?: "start" | "end" | "center";
    /** Default renderer: `numeric` => mono tabular; `path` => truncated from the start, full value in `title`. */
    kind?: "text" | "numeric" | "path";
    /** Default cell text when no `cell` snippet is given. */
    value?: (row: T) => string;
  }
</script>

<script lang="ts" generics="T">
  import type { Snippet } from "svelte";

  interface Props {
    columns: Column<T>[];
    rows: T[];
    /** Stable key per row (defaults to the index). */
    rowKey?: (row: T, index: number) => string | number;
    /** Custom cell content; `class="path"` and a `title` give the truncated-from-start path look. */
    cell?: Snippet<[T, Column<T>, number]>;
    /** Row height in px (40–54). */
    rowHeight?: number;
    ariaLabel?: string;
    /** Rendered instead of the rows when `rows` is empty. */
    empty?: Snippet;
    /** Optional per-row state class hook. */
    rowClass?: (row: T) => string | undefined;
  }
  let {
    columns,
    rows,
    rowKey,
    cell,
    rowHeight = 40,
    ariaLabel,
    empty,
    rowClass,
  }: Props = $props();

  const template = $derived(
    columns.map((c) => c.width ?? "minmax(0, 1fr)").join(" "),
  );

  function text(row: T, col: Column<T>): string {
    if (col.value) return col.value(row);
    const v = (row as Record<string, unknown>)[col.key];
    return v == null ? "" : String(v);
  }
</script>

<div class="table" role="table" aria-label={ariaLabel}>
  <div class="row head" role="row" style:grid-template-columns={template}>
    {#each columns as col (col.key)}
      <div class="th {col.align ?? 'start'}" role="columnheader">
        {col.header}
      </div>
    {/each}
  </div>
  {#each rows as row, i (rowKey ? rowKey(row, i) : i)}
    <div
      class="row body {rowClass?.(row) ?? ''}"
      role="row"
      style:grid-template-columns={template}
      style:min-height="{rowHeight}px"
    >
      {#each columns as col (col.key)}
        <div class="td {col.align ?? 'start'} {col.kind ?? 'text'}" role="cell">
          {#if cell}
            {@render cell(row, col, i)}
          {:else if col.kind === "path"}
            <span class="path" title={text(row, col)}>{text(row, col)}</span>
          {:else}
            {text(row, col)}
          {/if}
        </div>
      {/each}
    </div>
  {/each}
  {#if rows.length === 0 && empty}
    <div class="empty">{@render empty()}</div>
  {/if}
</div>

<style>
  .table {
    min-width: 0;
  }
  .row {
    display: grid;
    align-items: center;
    column-gap: 12px;
    padding: 0 14px;
  }
  .head {
    height: 32px;
    background: var(--surface);
    border-bottom: 1px solid var(--border);
  }
  .th {
    font-size: 11px;
    font-weight: var(--weight-semibold);
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-subtle);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .body {
    border-bottom: 1px solid var(--border);
  }
  .body:last-child {
    border-bottom: 0;
  }
  .body:hover {
    background: var(--surface-hover);
  }
  .td {
    min-width: 0;
    font-size: var(--text-base);
    color: var(--text);
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .td.numeric {
    font-family: var(--font-mono);
    font-variant-numeric: tabular-nums;
  }
  .th.end,
  .td.end {
    justify-content: flex-end;
    text-align: right;
  }
  .th.center,
  .td.center {
    justify-content: center;
    text-align: center;
  }
  .td :global(.path),
  .td .path {
    min-width: 0;
    flex: 1;
    font-size: var(--text-sm);
    color: var(--text-muted);
  }
  .empty {
    padding: 20px 14px;
    color: var(--text-muted);
  }
</style>
