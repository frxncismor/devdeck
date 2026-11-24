<script lang="ts">
  import type { Component, Snippet } from "svelte";

  interface Props {
    tone?: "neutral" | "accent" | "success" | "warning" | "danger";
    /** Icon (preferred). Without one, a colored dot is shown: never color alone. */
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    icon?: Component<any>;
    title?: string;
    children: Snippet;
  }
  let { tone = "neutral", icon: Icon, title, children }: Props = $props();
</script>

<span class="badge {tone}" {title}>
  {#if Icon}
    <Icon size={12} aria-hidden="true" />
  {:else}
    <span class="dot" aria-hidden="true"></span>
  {/if}
  {@render children()}
</span>

<style>
  .badge {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    height: 20px;
    padding: 0 7px;
    border-radius: var(--radius-sm);
    font-size: var(--text-xs);
    font-weight: var(--weight-medium);
    white-space: nowrap;
    background: var(--surface-hover);
    color: var(--text-muted);
  }
  .dot {
    width: 6px;
    height: 6px;
    border-radius: var(--radius-full);
    background: currentColor;
  }
  .accent {
    background: var(--accent-soft);
    color: var(--accent-text);
  }
  .success {
    background: var(--success-soft);
    color: var(--success);
  }
  .warning {
    background: var(--warning-soft);
    color: var(--warning);
  }
  .danger {
    background: var(--danger-soft);
    color: var(--danger);
  }
</style>
