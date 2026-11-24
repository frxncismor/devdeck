<script lang="ts">
  import { t } from "../lib/i18n/i18n.svelte";

  interface Props {
    open: boolean;
    title?: string;
    message: string;
    confirmLabel?: string;
    busy?: boolean;
    onconfirm: () => void;
    oncancel: () => void;
  }
  let {
    open,
    title,
    message,
    confirmLabel,
    busy = false,
    onconfirm,
    oncancel,
  }: Props = $props();

  let dialog: HTMLDialogElement | undefined = $state();

  $effect(() => {
    if (!dialog) return;
    if (open && !dialog.open) dialog.showModal();
    if (!open && dialog.open) dialog.close();
  });
</script>

<dialog bind:this={dialog} {oncancel}>
  <h2>{title ?? t("common.confirm.title")}</h2>
  <p>{message}</p>
  <div class="actions">
    <button type="button" onclick={oncancel}>
      {t("common.confirm.cancel")}
    </button>
    <button type="button" class="danger" disabled={busy} onclick={onconfirm}>
      {confirmLabel ?? t("common.confirm.confirm")}
    </button>
  </div>
</dialog>
