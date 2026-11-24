<script lang="ts">
  import Sidebar, { type ModuleId } from "./components/Sidebar.svelte";
  import Worktrees from "./modules/worktrees/index.svelte";
  import Ports from "./modules/ports/index.svelte";
  import JsonViewer from "./modules/json-viewer/index.svelte";
  import Reclaim from "./modules/reclaim/index.svelte";
  import "./app.css";

  let active = $state<ModuleId>("worktrees");
</script>

<div class="shell">
  <Sidebar {active} onselect={(id) => (active = id)} />
  <main>
    {#if active === "worktrees"}<Worktrees />{/if}
    {#if active === "ports"}<Ports />{/if}
    {#if active === "jsonViewer"}<JsonViewer />{/if}
    {#if active === "reclaim"}<Reclaim />{/if}
  </main>
</div>
