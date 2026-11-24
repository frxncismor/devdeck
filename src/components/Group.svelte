<script lang="ts">
  import type { Snippet } from "svelte";
  import ChevronDown from "@lucide/svelte/icons/chevron-down";
  import ChevronRight from "@lucide/svelte/icons/chevron-right";
  import { t } from "../lib/i18n/i18n.svelte";
  import { formatBytes } from "../lib/format";

  interface Props {
    name: string;
    /** Project color; use `projectColor(id, ids)`. "No project" => `var(--project-none)`. */
    color?: string;
    path?: string;
    count?: number;
    /** Totals in bytes; each is shown only when provided. */
    ramBytes?: number;
    diskBytes?: number;
    open?: boolean;
    children: Snippet;
  }
  let {
    name,
    color = "var(--project-none)",
    path,
    count,
    ramBytes,
    diskBytes,
    open = $bindable(true),
    children,
  }: Props = $props();

  const uid = $props.id();
</script>

<section class="group">
  <h3 class="head">
    <button
      type="button"
      aria-expanded={open}
      aria-controls="{uid}-body"
      onclick={() => (open = !open)}
    >
      {#if open}<ChevronDown size={15} aria-hidden="true" />{:else}<ChevronRight
          size={15}
          aria-hidden="true"
        />{/if}
      <span class="sq" style:background={color} aria-hidden="true"></span>
      <span class="name">{name}</span>
      {#if path}<span class="gpath mono" title={path}>{path}</span>{/if}
      <span class="spacer"></span>
      {#if count !== undefined}
        <span class="count tabular">{t("common.table.items", { count })}</span>
      {/if}
      {#if ramBytes !== undefined}
        <span class="total tabular">
          <span class="k">{t("common.units.ram")}</span>
          {formatBytes(ramBytes)}
        </span>
      {/if}
      {#if diskBytes !== undefined}
        <span class="total tabular">
          <span class="k">{t("common.units.disk")}</span>
          {formatBytes(diskBytes)}
        </span>
      {/if}
    </button>
  </h3>
  <div id="{uid}-body" class="body" hidden={!open}>
    {@render children()}
  </div>
</section>

<style>
  .group {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    overflow: hidden;
  }
  .head {
    font-size: inherit;
    font-weight: inherit;
  }
  .head button {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    height: 40px;
    padding: 0 14px;
    background: var(--surface-2);
    border: 0;
    color: var(--text);
    text-align: left;
  }
  .head button:hover {
    background: var(--surface-hover);
  }
  .head :global(svg) {
    flex-shrink: 0;
    color: var(--text-subtle);
  }
  .sq {
    width: 10px;
    height: 10px;
    border-radius: 2px;
    flex-shrink: 0;
  }
  .name {
    font-weight: var(--weight-semibold);
    white-space: nowrap;
  }
  .gpath {
    min-width: 0;
    font-size: var(--text-xs);
    color: var(--text-subtle);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    direction: rtl;
    text-align: left;
  }
  .spacer {
    flex: 1;
  }
  .count {
    font-size: var(--text-xs);
    color: var(--text-muted);
    background: var(--surface-hover);
    border-radius: var(--radius-full);
    padding: 1px 8px;
    white-space: nowrap;
  }
  .total {
    font-family: var(--font-mono);
    font-size: var(--text-sm);
    color: var(--text);
    white-space: nowrap;
  }
  .k {
    font-family: var(--font-sans);
    font-size: var(--text-xs);
    color: var(--text-subtle);
    margin-right: 3px;
  }
  .body {
    border-top: 1px solid var(--border);
  }
  .body[hidden] {
    display: none;
  }
</style>
