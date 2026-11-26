<script lang="ts">
  import Toast from "./Toast.svelte";
  import { toastState } from "../lib/toasts.svelte";
</script>

<!-- Mount once in the app shell. Always rendered so the live region exists before the first toast. -->
<div class="host" aria-live="polite">
  {#each toastState.items as toast (toast.id)}
    <Toast {toast} />
  {/each}
</div>

<style>
  .host {
    position: fixed;
    right: 20px;
    /* Above the sticky selection bar of the module screens, so errors never cover it. */
    bottom: 84px;
    z-index: 1000;
    display: flex;
    flex-direction: column;
    gap: 10px;
    pointer-events: none;
  }
  .host > :global(*) {
    pointer-events: auto;
  }
</style>
