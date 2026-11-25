<script lang="ts">
  import type { Component, Snippet } from "svelte";
  import Inbox from "@lucide/svelte/icons/inbox";

  interface Props {
    title: string;
    description?: string;
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    icon?: Component<any>;
    /** Buttons / extra content below the text. */
    children?: Snippet;
  }
  let { title, description, icon: Icon = Inbox, children }: Props = $props();
</script>

<div class="empty">
  <span class="ico"><Icon size={22} aria-hidden="true" /></span>
  <h2>{title}</h2>
  {#if description}<p>{description}</p>{/if}
  {#if children}<div class="actions">{@render children()}</div>{/if}
</div>

<style>
  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    gap: 8px;
    padding: 48px 24px;
    background: var(--surface);
    border: 1px dashed var(--border-strong);
    border-radius: var(--radius-lg);
  }
  .ico {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 44px;
    height: 44px;
    border-radius: var(--radius-xl);
    background: var(--surface-2);
    color: var(--text-muted);
  }
  h2 {
    font-size: var(--text-md);
    font-weight: var(--weight-semibold);
  }
  p {
    margin: 0;
    max-width: 440px;
    color: var(--text-muted);
  }
  .actions {
    margin-top: 8px;
    display: flex;
    gap: 8px;
  }
</style>
