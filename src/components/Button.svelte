<script lang="ts">
  import type { Component, Snippet } from "svelte";
  import type { HTMLButtonAttributes } from "svelte/elements";
  import Spinner from "./Spinner.svelte";

  interface Props extends Omit<HTMLButtonAttributes, "children"> {
    variant?: "primary" | "secondary" | "ghost" | "danger" | "danger-ghost";
    size?: "sm" | "md" | "lg";
    /** Lucide icon component, rendered before the label. */
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    icon?: Component<any>;
    /** Keyboard hint shown after the label (e.g. "F5"). */
    kbd?: string;
    /** Long action running: shows a spinner instead of the icon, disables the button and sets `aria-busy`. */
    loading?: boolean;
    children?: Snippet;
    /** `bind:ref` to the underlying <button>. */
    ref?: HTMLButtonElement | null;
  }
  let {
    variant = "secondary",
    size = "md",
    icon: Icon,
    kbd,
    loading = false,
    disabled,
    type = "button",
    children,
    ref = $bindable(null),
    ...rest
  }: Props = $props();
  const iconSize = $derived(size === "sm" ? 13 : 15);
</script>

<button
  bind:this={ref}
  {type}
  class="btn {variant} {size}"
  class:loading
  disabled={disabled || loading}
  aria-busy={loading || undefined}
  {...rest}
>
  {#if loading}<Spinner size={iconSize} />{:else if Icon}<Icon
      size={iconSize}
      aria-hidden="true"
    />{/if}
  {#if children}<span class="label">{@render children()}</span>{/if}
  {#if kbd}<span class="kbd">{kbd}</span>{/if}
</button>

<style>
  .btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 7px;
    border-radius: var(--radius-md);
    font-weight: var(--weight-medium);
    white-space: nowrap;
    border: 1px solid transparent;
    transition:
      background var(--duration-fast) var(--ease-out),
      color var(--duration-fast) var(--ease-out);
  }
  .btn :global(svg) {
    flex-shrink: 0;
  }
  .btn:disabled {
    opacity: 0.45;
  }
  /* Busy, not unavailable: keep it readable. */
  .btn.loading:disabled {
    opacity: 0.8;
    cursor: progress;
  }
  .sm {
    height: var(--control-h-sm);
    padding: 0 9px;
    font-size: var(--text-sm);
  }
  .md {
    height: var(--control-h);
    padding: 0 12px;
    font-size: var(--text-base);
  }
  .lg {
    height: 36px;
    padding: 0 16px;
    font-size: var(--text-base);
  }
  .kbd {
    font-family: var(--font-mono);
    font-size: var(--text-2xs);
    opacity: 0.75;
    margin-left: 2px;
  }
  .primary {
    background: var(--accent);
    color: var(--on-accent);
    border-color: var(--accent);
  }
  .primary:hover:not(:disabled) {
    background: var(--accent-hover);
  }
  .secondary {
    background: var(--surface-2);
    color: var(--text);
    border-color: var(--border-strong);
  }
  .secondary:hover:not(:disabled),
  .ghost:hover:not(:disabled) {
    background: var(--surface-hover);
    color: var(--text);
  }
  .ghost {
    background: transparent;
    color: var(--text-muted);
  }
  .danger {
    background: var(--danger-solid);
    color: var(--on-danger);
    border-color: var(--danger-solid);
  }
  .danger:hover:not(:disabled) {
    background: var(--danger-solid-hover);
  }
  .danger-ghost {
    background: transparent;
    color: var(--danger);
    border-color: var(--border-strong);
  }
  .danger-ghost:hover:not(:disabled) {
    background: var(--danger-soft);
  }
</style>
