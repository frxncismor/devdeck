<script lang="ts">
  interface Props {
    size?: number;
    /** Accessible label (e.g. t("common.state.loading")). */
    label?: string;
  }
  let { size = 16, label }: Props = $props();
</script>

<span
  class="spinner"
  role={label ? "status" : undefined}
  aria-label={label}
  aria-hidden={label ? undefined : true}
  style:width="{size}px"
  style:height="{size}px"
></span>

<style>
  .spinner {
    display: inline-block;
    flex-shrink: 0;
    border-radius: var(--radius-full);
    border: 2px solid var(--border-strong);
    border-top-color: var(--accent);
    animation: spin 0.8s linear infinite;
  }
  @media (prefers-reduced-motion: reduce) {
    /* No rotation: the busy text next to it carries the state. */
    .spinner {
      animation: none;
      border-top-color: var(--accent);
      border-right-color: var(--accent);
    }
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
