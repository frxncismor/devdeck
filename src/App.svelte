<script lang="ts">
  import { setLucideProps } from "@lucide/svelte/context";
  import CommandPalette from "./components/CommandPalette.svelte";
  import Sidebar from "./components/Sidebar.svelte";
  import TitleBar from "./components/TitleBar.svelte";
  import ToastHost from "./components/ToastHost.svelte";
  import TopBar from "./components/TopBar.svelte";
  import { t } from "./lib/i18n/i18n.svelte";
  import { MODULES } from "./lib/modules";
  import { shell, topbarSlots } from "./lib/shell.svelte";
  import { theme } from "./lib/theme.svelte";
  import Worktrees from "./modules/worktrees/index.svelte";
  import Ports from "./modules/ports/index.svelte";
  import JsonViewer from "./modules/json-viewer/index.svelte";
  import Reclaim from "./modules/reclaim/index.svelte";
  import "./app.css";

  // Design: Lucide stroke 1.75 everywhere.
  setLucideProps({ strokeWidth: 1.75 });

  const title = $derived.by(() => {
    const m = MODULES.find((x) => x.id === shell.active);
    return m ? t(m.labelKey) : "";
  });

  function onkeydown(e: KeyboardEvent) {
    if (!e.ctrlKey || e.altKey || e.shiftKey) return;
    const k = e.key.toLowerCase();
    if (k === "k") shell.paletteOpen = !shell.paletteOpen;
    else if (k === "b") shell.sidebarCollapsed = !shell.sidebarCollapsed;
    else if (k === "p") shell.scopeOpen = !shell.scopeOpen;
    else return;
    e.preventDefault();
  }
</script>

<svelte:window {onkeydown} />

<div class="dd {theme.current} app">
  <TitleBar />
  <div class="body">
    <Sidebar active={shell.active} onselect={(id) => (shell.active = id)} />
    <main>
      <TopBar {title} meta={topbarSlots.meta} actions={topbarSlots.actions} />
      <div class="content">
        {#if shell.active === "worktrees"}<Worktrees />{/if}
        {#if shell.active === "ports"}<Ports />{/if}
        {#if shell.active === "jsonViewer"}<JsonViewer />{/if}
        {#if shell.active === "reclaim"}<Reclaim />{/if}
      </div>
    </main>
  </div>
  <CommandPalette onselect={(id) => (shell.active = id)} />
  <ToastHost />
</div>

<style>
  .app {
    height: 100vh;
    display: flex;
    flex-direction: column;
    background: var(--bg);
    color: var(--text);
    font-family: var(--font-sans);
    font-size: var(--text-base);
    line-height: var(--leading-normal);
    overflow: hidden;
  }
  .body {
    flex: 1;
    min-height: 0;
    display: flex;
  }
  main {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .content {
    flex: 1;
    min-height: 0;
    overflow: auto;
    padding: 20px 24px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
</style>
