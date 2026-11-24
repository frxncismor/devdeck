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
  role="status"
  aria-label={label}
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
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
