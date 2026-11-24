<script lang="ts">
  import type { Component } from "svelte";

  export interface StatSegment {
    label: string;
    /** Proportional weight (any unit, e.g. bytes). */
    value: number;
    /** CSS color, e.g. `var(--project-1)`. */
    color: string;
    /** Text shown in the legend, e.g. "4.6 GB". */
    display?: string;
  }

  interface Props {
    size?: "sm" | "lg";
    label: string;
    value: string;
    /** Secondary line under the value (sm) or next to it (lg). */
    hint?: string;
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    icon?: Component<any>;
    /** Value color. */
    tone?: "default" | "warning" | "success" | "danger";
    /** lg only: stacked bar (h8, 2px gaps) + legend. */
    segments?: StatSegment[];
  }
  let {
    size = "sm",
    label,
    value,
    hint,
    icon: Icon,
    tone = "default",
    segments = [],
  }: Props = $props();

  const visible = $derived(segments.filter((s) => s.value > 0));
</script>

<div class="card {size}">
  <div class="label">
    {#if Icon}<Icon size={size === "lg" ? 15 : 14} aria-hidden="true" />{/if}
    {label}
  </div>
  {#if size === "lg"}
    <div class="big">
      <span class="value tabular {tone}">{value}</span>
      {#if hint}<span class="hint">{hint}</span>{/if}
    </div>
    {#if visible.length > 0}
      <div class="bar" role="img" aria-label={label}>
        {#each visible as s (s.label)}
          <span
            class="seg"
            style:flex="{s.value} 1 0"
            style:background={s.color}
            title="{s.label}{s.display ? `: ${s.display}` : ''}"
          ></span>
        {/each}
      </div>
      <ul class="legend">
        {#each visible as s (s.label)}
          <li>
            <span class="sq" style:background={s.color} aria-hidden="true"
            ></span>
            {s.label}
            {#if s.display}<span class="mono tabular figure">
                {s.display}
              </span>{/if}
          </li>
        {/each}
      </ul>
    {/if}
  {:else}
    <div class="value tabular {tone}">{value}</div>
    {#if hint}<div class="hint">{hint}</div>{/if}
  {/if}
</div>

<style>
  .card {
    flex: 1 1 180px;
    min-width: 0;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    padding: 14px 16px;
  }
  .lg {
    flex-basis: 360px;
    padding: 16px 18px;
  }
  .label {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: var(--text-sm);
    color: var(--text-muted);
  }
  .lg .label {
    gap: 8px;
    font-size: 12.5px;
  }
  .value {
    font-size: 22px;
    font-weight: var(--weight-semibold);
    letter-spacing: -0.01em;
    margin-top: 6px;
    color: var(--text);
  }
  .value.warning {
    color: var(--warning);
  }
  .value.success {
    color: var(--success);
  }
  .value.danger {
    color: var(--danger);
  }
  .hint {
    font-size: var(--text-sm);
    color: var(--text-subtle);
    margin-top: 2px;
  }
  .big {
    display: flex;
    align-items: baseline;
    flex-wrap: wrap;
    gap: 4px 10px;
    margin-top: 6px;
  }
  .big .value {
    font-size: var(--text-3xl);
    letter-spacing: -0.02em;
    line-height: 1.1;
    margin-top: 0;
  }
  .big .hint {
    margin-top: 0;
    font-size: 12.5px;
  }
  .bar {
    display: flex;
    gap: 2px;
    height: 8px;
    margin: 14px 0 12px;
  }
  .seg {
    height: 100%;
    border-radius: 2px;
    min-width: 2px;
  }
  .legend {
    display: flex;
    flex-wrap: wrap;
    gap: 6px 16px;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .legend li {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: var(--text-sm);
    color: var(--text-muted);
    white-space: nowrap;
  }
  .sq {
    width: 8px;
    height: 8px;
    border-radius: 2px;
  }
  .figure {
    color: var(--text);
  }
</style>
