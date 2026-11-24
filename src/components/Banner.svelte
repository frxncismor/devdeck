<script lang="ts">
  import type { Snippet } from "svelte";
  import CircleAlert from "@lucide/svelte/icons/circle-alert";
  import Info from "@lucide/svelte/icons/info";
  import TriangleAlert from "@lucide/svelte/icons/triangle-alert";

  interface Props {
    tone?: "info" | "warning" | "danger";
    title?: string;
    children?: Snippet;
    /** Right-aligned actions (e.g. a retry Button). */
    actions?: Snippet;
  }
  let { tone = "info", title, children, actions }: Props = $props();
  const Icon = $derived(
    tone === "danger" ? CircleAlert : tone === "warning" ? TriangleAlert : Info,
  );
</script>

<div class="banner {tone}" role={tone === "danger" ? "alert" : "status"}>
  <Icon size={16} aria-hidden="true" />
  <div class="text">
    {#if title}<strong>{title}</strong>{/if}
    {#if children}<div class="msg">{@render children()}</div>{/if}
  </div>
  {#if actions}<div class="actions">{@render actions()}</div>{/if}
</div>

<style>
  .banner {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    padding: 10px 14px;
    border-radius: var(--radius-lg);
    border: 1px solid var(--border);
    background: var(--surface);
  }
  .banner :global(svg) {
    flex-shrink: 0;
    margin-top: 2px;
  }
  .text {
    flex: 1;
    min-width: 0;
  }
  strong {
    display: block;
    font-weight: var(--weight-semibold);
  }
  .msg {
    color: var(--text-muted);
  }
  .actions {
    display: flex;
    gap: 8px;
    align-items: center;
  }
  .info {
    background: var(--accent-soft);
    border-color: transparent;
  }
  .info :global(svg) {
    color: var(--accent-text);
  }
  .warning {
    background: var(--warning-soft);
    border-color: transparent;
  }
  .warning :global(svg) {
    color: var(--warning);
  }
  .danger {
    background: var(--danger-soft);
    border-color: transparent;
  }
  .danger :global(svg) {
    color: var(--danger);
  }
</style>
