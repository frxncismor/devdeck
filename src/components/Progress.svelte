<script lang="ts">
  interface Props {
    /** Fraction 0..1; omit for an indeterminate bar. */
    value?: number;
    label?: string;
  }
  let { value, label }: Props = $props();
  const pct = $derived(
    value === undefined
      ? undefined
      : Math.round(Math.min(1, Math.max(0, value)) * 100),
  );
</script>

<div
  class="progress"
  class:indeterminate={pct === undefined}
  role="progressbar"
  aria-label={label}
  aria-valuemin="0"
  aria-valuemax="100"
  aria-valuenow={pct}
>
  <span class="fill" style:width={pct === undefined ? undefined : `${pct}%`}
  ></span>
</div>

<style>
  .progress {
    height: 6px;
    border-radius: var(--radius-full);
    background: var(--surface-hover);
    overflow: hidden;
  }
  .fill {
    display: block;
    height: 100%;
    background: var(--accent);
    border-radius: var(--radius-full);
    transition: width var(--duration-base) var(--ease-out);
  }
  .indeterminate .fill {
    width: 35%;
    animation: slide 1.2s var(--ease-out) infinite;
  }
  @keyframes slide {
    from {
      transform: translateX(-100%);
    }
    to {
      transform: translateX(300%);
    }
  }
</style>
