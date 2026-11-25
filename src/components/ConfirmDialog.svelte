<script lang="ts">
  import type { Snippet } from "svelte";
  import { t } from "../lib/i18n/i18n.svelte";
  import Button from "./Button.svelte";

  interface Props {
    open: boolean;
    title?: string;
    message: string;
    /** Name the action (and its size): "Delete 1.46 GB". */
    confirmLabel?: string;
    busy?: boolean;
    /** Destructive (default): solid danger confirm button and Cancel focused first. */
    destructive?: boolean;
    /** Extra content between the message and the footer (lists, paths...). */
    body?: Snippet;
    onconfirm: () => void;
    oncancel: () => void;
  }
  let {
    open,
    title,
    message,
    confirmLabel,
    busy = false,
    destructive = true,
    body,
    onconfirm,
    oncancel,
  }: Props = $props();

  let dialog: HTMLDialogElement | undefined = $state();
  let cancelBtn: HTMLButtonElement | null = $state(null);
  let confirmBtn: HTMLButtonElement | null = $state(null);
  const uid = $props.id();

  $effect(() => {
    if (!dialog) return;
    if (open && !dialog.open) {
      dialog.showModal();
      (destructive ? cancelBtn : confirmBtn)?.focus();
    }
    if (!open && dialog.open) dialog.close();
  });

  function oncancelEvent(e: Event) {
    // Let the parent own the state: it flips `open`, the effect closes the dialog.
    e.preventDefault();
    oncancel();
  }
</script>

<dialog
  bind:this={dialog}
  oncancel={oncancelEvent}
  aria-labelledby="{uid}-title"
>
  <div class="content">
    <h2 id="{uid}-title">{title ?? t("common.confirm.title")}</h2>
    <p>{message}</p>
    {#if body}<div class="extra">{@render body()}</div>{/if}
  </div>
  <div class="actions">
    <Button bind:ref={cancelBtn} variant="secondary" onclick={oncancel}>
      {t("common.confirm.cancel")}
    </Button>
    <Button
      bind:ref={confirmBtn}
      variant={destructive ? "danger" : "primary"}
      disabled={busy}
      onclick={onconfirm}
    >
      {confirmLabel ?? t("common.confirm.confirm")}
    </Button>
  </div>
</dialog>

<style>
  dialog {
    width: min(520px, calc(100vw - 48px));
    min-width: min(440px, calc(100vw - 48px));
    padding: 0;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-xl);
    background: var(--surface-raised);
    color: var(--text);
    box-shadow: var(--shadow-lg);
  }
  dialog::backdrop {
    background: var(--overlay);
  }
  .content {
    padding: 20px 22px 18px;
  }
  h2 {
    font-size: var(--text-lg);
    font-weight: var(--weight-semibold);
  }
  p {
    margin: 8px 0 0;
    color: var(--text-muted);
    overflow-wrap: anywhere;
  }
  .extra {
    margin-top: 12px;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 12px 22px;
    background: var(--surface-2);
    border-top: 1px solid var(--border);
  }
</style>
