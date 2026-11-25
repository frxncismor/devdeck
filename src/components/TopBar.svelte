<script lang="ts">
  import type { Snippet } from "svelte";
  import ChevronRight from "@lucide/svelte/icons/chevron-right";
  import ScopeSelect from "./ScopeSelect.svelte";

  interface Props {
    /** Module title (h1). */
    title: string;
    /** Right-aligned secondary info ("Scanned 2 min ago"). */
    meta?: Snippet;
    /** Right-aligned actions (Buttons). */
    actions?: Snippet;
  }
  let { title, meta, actions }: Props = $props();
</script>

<header class="topbar">
  <ScopeSelect />
  <ChevronRight size={16} aria-hidden="true" class="sep" />
  <h1>{title}</h1>
  <div class="spacer"></div>
  <div class="right">
    {#if meta}<div class="meta">{@render meta()}</div>{/if}
    {#if actions}<div class="actions">{@render actions()}</div>{/if}
  </div>
</header>

<style>
  .topbar {
    height: 60px;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 0 20px;
    border-bottom: 1px solid var(--border);
    background: var(--bg);
  }
  .topbar :global(.sep) {
    color: var(--text-subtle);
    flex-shrink: 0;
  }
  h1 {
    font-size: var(--text-lg);
    font-weight: var(--weight-semibold);
    letter-spacing: -0.01em;
    white-space: nowrap;
  }
  .spacer {
    flex: 1;
  }
  .right,
  .meta,
  .actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .right {
    gap: 14px;
  }
  .meta {
    font-size: var(--text-sm);
    color: var(--text-subtle);
    white-space: nowrap;
  }
</style>
