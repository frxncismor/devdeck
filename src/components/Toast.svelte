<script lang="ts">
  import CircleAlert from "@lucide/svelte/icons/circle-alert";
  import CircleCheck from "@lucide/svelte/icons/circle-check";
  import Info from "@lucide/svelte/icons/info";
  import TriangleAlert from "@lucide/svelte/icons/triangle-alert";
  import X from "@lucide/svelte/icons/x";
  import { t } from "../lib/i18n/i18n.svelte";
  import { toasts, type Toast } from "../lib/toasts.svelte";

  let { toast }: { toast: Toast } = $props();
  const Icon = $derived(
    toast.tone === "error"
      ? CircleAlert
      : toast.tone === "warning"
        ? TriangleAlert
        : toast.tone === "success"
          ? CircleCheck
          : Info,
  );
</script>

<div
  class="toast {toast.tone}"
  role={toast.tone === "error" ? "alert" : undefined}
>
  <Icon size={16} aria-hidden="true" />
  <div class="text">
    <strong>{toast.title}</strong>
    {#if toast.message}<div class="msg">{toast.message}</div>{/if}
  </div>
  <button
    type="button"
    class="close"
    aria-label={t("common.toast.dismiss")}
    title={t("common.toast.dismiss")}
    onclick={() => toasts.dismiss(toast.id)}
  >
    <X size={14} aria-hidden="true" />
  </button>
</div>

<style>
  .toast {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    width: 380px;
    max-width: calc(100vw - 40px);
    padding: 12px 12px 12px 14px;
    background: var(--surface-raised);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-md);
  }
  .toast :global(svg) {
    flex-shrink: 0;
    margin-top: 2px;
  }
  .text {
    flex: 1;
    min-width: 0;
  }
  strong {
    font-weight: var(--weight-semibold);
  }
  .msg {
    color: var(--text-muted);
    overflow-wrap: anywhere;
  }
  .close {
    display: inline-flex;
    padding: 4px;
    background: none;
    border: 0;
    border-radius: var(--radius-sm);
    color: var(--text-subtle);
  }
  .close:hover {
    background: var(--surface-hover);
    color: var(--text);
  }
  .info :global(svg:first-child) {
    color: var(--accent-text);
  }
  .success :global(svg:first-child) {
    color: var(--success);
  }
  .warning :global(svg:first-child) {
    color: var(--warning);
  }
  .error :global(svg:first-child) {
    color: var(--danger);
  }
</style>
