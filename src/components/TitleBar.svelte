<script lang="ts">
  import { onMount } from "svelte";
  import Copy from "@lucide/svelte/icons/copy";
  import Minus from "@lucide/svelte/icons/minus";
  import Search from "@lucide/svelte/icons/search";
  import Square from "@lucide/svelte/icons/square";
  import X from "@lucide/svelte/icons/x";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { t } from "../lib/i18n/i18n.svelte";
  import { shell } from "../lib/shell.svelte";

  let maximized = $state(false);

  // The Tauri window API is absent in a plain browser (vite preview): every call is guarded.
  function win() {
    try {
      return getCurrentWindow();
    } catch {
      return undefined;
    }
  }

  async function syncMaximized() {
    try {
      maximized = (await win()?.isMaximized()) ?? false;
    } catch {
      maximized = false;
    }
  }

  onMount(() => {
    void syncMaximized();
    let unlisten: (() => void) | undefined;
    try {
      void win()
        ?.onResized(() => void syncMaximized())
        .then((u) => (unlisten = u))
        .catch(() => {});
    } catch {
      // no window API
    }
    return () => unlisten?.();
  });

  function run(action: "minimize" | "toggleMaximize" | "close") {
    void win()
      ?.[action]()
      ?.catch((e: unknown) => console.error(e));
  }
</script>

<header class="titlebar" data-tauri-drag-region>
  <div class="title" data-tauri-drag-region>{t("common.app.name")}</div>
  <button
    type="button"
    class="search"
    onclick={() => (shell.paletteOpen = true)}
    aria-label={t("common.titlebar.search")}
    aria-haspopup="dialog"
  >
    <Search size={13} aria-hidden="true" />
    <span class="ph">{t("common.titlebar.search")}</span>
    <kbd>Ctrl K</kbd>
  </button>
  <div class="controls" data-tauri-drag-region>
    <button
      type="button"
      aria-label={t("common.titlebar.minimize")}
      title={t("common.titlebar.minimize")}
      onclick={() => run("minimize")}
    >
      <Minus size={14} aria-hidden="true" />
    </button>
    <button
      type="button"
      aria-label={maximized
        ? t("common.titlebar.restore")
        : t("common.titlebar.maximize")}
      title={maximized
        ? t("common.titlebar.restore")
        : t("common.titlebar.maximize")}
      onclick={() => run("toggleMaximize")}
    >
      {#if maximized}<Copy size={12} aria-hidden="true" />{:else}<Square
          size={12}
          aria-hidden="true"
        />{/if}
    </button>
    <button
      type="button"
      class="close"
      aria-label={t("common.titlebar.close")}
      title={t("common.titlebar.close")}
      onclick={() => run("close")}
    >
      <X size={14} aria-hidden="true" />
    </button>
  </div>
</header>

<style>
  .titlebar {
    height: var(--titlebar-h);
    flex-shrink: 0;
    display: flex;
    align-items: center;
    background: var(--titlebar);
    border-bottom: 1px solid var(--border);
    user-select: none;
  }
  .title {
    flex: 1;
    padding-left: 14px;
    font-size: var(--text-sm);
    color: var(--text-subtle);
  }
  .search {
    width: 400px;
    max-width: 40%;
    height: 24px;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 6px 0 9px;
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    color: var(--text-subtle);
    font-size: var(--text-sm);
  }
  .search:hover {
    border-color: var(--border-strong);
    color: var(--text-muted);
  }
  .ph {
    flex: 1;
    text-align: left;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .controls {
    flex: 1;
    display: flex;
    justify-content: flex-end;
    align-self: stretch;
  }
  .controls button {
    width: 46px;
    height: var(--titlebar-h);
    display: flex;
    align-items: center;
    justify-content: center;
    background: none;
    border: 0;
    color: var(--text-muted);
  }
  .controls button:hover {
    background: var(--surface-hover);
    color: var(--text);
  }
  .controls .close:hover {
    background: var(--danger-solid);
    color: var(--on-danger);
  }
</style>
