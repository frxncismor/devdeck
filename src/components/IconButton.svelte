<script lang="ts">
  import type { Component } from "svelte";
  import type { HTMLButtonAttributes } from "svelte/elements";
  import Spinner from "./Spinner.svelte";

  interface Props extends Omit<HTMLButtonAttributes, "children" | "title"> {
    /** Required: used for `aria-label` and `title`. */
    label: string;
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    icon: Component<any>;
    variant?: "ghost" | "secondary" | "danger-ghost";
    /** Button size in px (default 28). */
    size?: number;
    pressed?: boolean;
    /** Long action running: spinner instead of the icon, disabled, `aria-busy`. Pass the busy text as `label`. */
    loading?: boolean;
  }
  let {
    label,
    icon: Icon,
    variant = "ghost",
    size = 28,
    pressed,
    loading = false,
    disabled,
    type = "button",
    ...rest
  }: Props = $props();
</script>

<button
  {type}
  class="ib {variant}"
  style:width="{size}px"
  style:height="{size}px"
  aria-label={label}
  title={label}
  aria-pressed={pressed}
  aria-busy={loading || undefined}
  disabled={disabled || loading}
  class:loading
  {...rest}
>
  {#if loading}<Spinner size={14} />{:else}<Icon
      size={15}
      aria-hidden="true"
    />{/if}
</button>

<style>
  .ib {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 0;
    flex-shrink: 0;
    border-radius: var(--radius-md);
    border: 1px solid transparent;
    background: transparent;
    color: var(--text-muted);
  }
  .ib:disabled {
    opacity: 0.45;
  }
  .ib.loading:disabled {
    opacity: 0.8;
    cursor: progress;
  }
  .ib:hover:not(:disabled),
  .ib[aria-pressed="true"] {
    background: var(--surface-hover);
    color: var(--text);
  }
  .secondary {
    background: var(--surface-2);
    border-color: var(--border-strong);
  }
  .danger-ghost {
    color: var(--danger);
  }
  .danger-ghost:hover:not(:disabled) {
    background: var(--danger-soft);
    color: var(--danger);
  }
</style>
