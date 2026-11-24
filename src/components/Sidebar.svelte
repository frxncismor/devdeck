<script lang="ts" module>
  export type ModuleId = "worktrees" | "ports" | "jsonViewer" | "reclaim";
</script>

<script lang="ts">
  import { t } from "../lib/i18n/i18n.svelte";
  import LanguageSelector from "./LanguageSelector.svelte";
  import ScopeSelector from "./ScopeSelector.svelte";

  interface Props {
    active: ModuleId;
    onselect: (id: ModuleId) => void;
  }
  let { active, onselect }: Props = $props();
</script>

<aside class="sidebar">
  <h1>{t("common.app.name")}</h1>
  <nav aria-label={t("common.sidebar.label")}>
    <button
      class:active={active === "worktrees"}
      onclick={() => onselect("worktrees")}
    >
      {t("common.sidebar.worktrees")}
    </button>
    <button class:active={active === "ports"} onclick={() => onselect("ports")}>
      {t("common.sidebar.ports")}
    </button>
    <button
      class:active={active === "jsonViewer"}
      onclick={() => onselect("jsonViewer")}
    >
      {t("common.sidebar.jsonViewer")}
    </button>
    <button
      class:active={active === "reclaim"}
      onclick={() => onselect("reclaim")}
    >
      {t("common.sidebar.reclaim")}
    </button>
  </nav>
  <div class="selectors">
    <ScopeSelector />
    <LanguageSelector />
  </div>
</aside>
