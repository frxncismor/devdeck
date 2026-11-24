<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import Folder from "@lucide/svelte/icons/folder";
  import Languages from "@lucide/svelte/icons/languages";
  import Moon from "@lucide/svelte/icons/moon";
  import PanelLeft from "@lucide/svelte/icons/panel-left";
  import Sun from "@lucide/svelte/icons/sun";
  import SquareTerminal from "@lucide/svelte/icons/square-terminal";
  import { toAppError } from "../lib/api";
  import { configState, updateConfig } from "../lib/config.svelte";
  import { errorMessage } from "../lib/errors";
  import { LOCALES, type Locale } from "../lib/i18n";
  import { locale, setLocale, t } from "../lib/i18n/i18n.svelte";
  import { MODULES, type ModuleId } from "../lib/modules";
  import { shell } from "../lib/shell.svelte";
  import { theme, toggleTheme } from "../lib/theme.svelte";
  import { toasts } from "../lib/toasts.svelte";
  import IconButton from "./IconButton.svelte";

  interface Props {
    active: ModuleId;
    onselect: (id: ModuleId) => void;
    /** Optional right-aligned live counts per module (e.g. "9", "22.8 GB"). */
    counts?: Partial<Record<ModuleId, string>>;
  }
  let { active, onselect, counts = {} }: Props = $props();

  const collapsed = $derived(shell.sidebarCollapsed);
  const root = $derived(configState.value.root);

  async function chooseRoot(): Promise<void> {
    try {
      const picked = await open({
        directory: true,
        multiple: false,
        title: t("worktrees.root.pickerTitle"),
        defaultPath: root,
      });
      if (typeof picked === "string") await updateConfig({ root: picked });
    } catch (err) {
      toasts.error(errorMessage(toAppError(err)));
    }
  }

  function pickLocale(l: Locale) {
    void setLocale(l).catch((err) =>
      console.error(errorMessage(toAppError(err))),
    );
  }

  const langName = (l: Locale) =>
    t(l === "en" ? "common.language.en" : "common.language.es");
  const themeLabel = $derived(
    theme.current === "dark"
      ? t("common.theme.toLight")
      : t("common.theme.toDark"),
  );
</script>

<aside class="sidebar" class:collapsed aria-label={t("common.sidebar.sidebar")}>
  <div class="brand">
    <span class="logo"><SquareTerminal size={18} aria-hidden="true" /></span>
    {#if !collapsed}
      <span class="name">{t("common.app.name")}</span>
      <span class="spacer"></span>
      <IconButton
        icon={PanelLeft}
        label="{t('common.sidebar.collapse')} (Ctrl B)"
        onclick={() => (shell.sidebarCollapsed = true)}
      />
    {/if}
  </div>

  {#if !collapsed}
    <div class="section">{t("common.sidebar.label")}</div>
  {:else}
    <div class="gap"></div>
  {/if}
  <nav aria-label={t("common.sidebar.label")}>
    {#each MODULES as m (m.id)}
      {@const Icon = m.icon}
      <button
        type="button"
        class="nav"
        class:current={active === m.id}
        aria-current={active === m.id ? "page" : undefined}
        aria-label={collapsed ? t(m.labelKey) : undefined}
        title={collapsed ? t(m.labelKey) : undefined}
        onclick={() => onselect(m.id)}
      >
        <Icon size={collapsed ? 17 : 16} aria-hidden="true" />
        {#if !collapsed}
          <span class="lbl">{t(m.labelKey)}</span>
          {#if counts[m.id]}<span class="count tabular">
              {counts[m.id]}
            </span>{/if}
        {/if}
      </button>
    {/each}
  </nav>

  <div class="grow"></div>

  {#if !collapsed}
    <div class="root">
      <div class="k">{t("worktrees.root.label")}</div>
      <div class="rootpath mono" title={root ?? t("worktrees.root.none")}>
        <Folder size={13} aria-hidden="true" />
        <span class="val">{root ?? t("worktrees.root.none")}</span>
      </div>
      <button type="button" class="link" onclick={() => void chooseRoot()}>
        {root ? t("worktrees.root.change") : t("worktrees.root.choose")}
      </button>
    </div>

    <div class="lang" role="group" aria-label={t("common.language.label")}>
      <span class="ico"><Languages size={15} aria-hidden="true" /></span>
      <div class="seg">
        {#each LOCALES as l (l)}
          <button
            type="button"
            aria-pressed={locale.current === l}
            onclick={() => pickLocale(l)}
          >
            {langName(l)}
          </button>
        {/each}
      </div>
    </div>
    <div class="foot">
      <button type="button" class="nav" onclick={toggleTheme}>
        {#if theme.current === "dark"}<Sun
            size={16}
            aria-hidden="true"
          />{:else}<Moon size={16} aria-hidden="true" />{/if}
        <span class="lbl">{themeLabel}</span>
      </button>
    </div>
  {:else}
    <IconButton
      icon={Folder}
      label="{t('worktrees.root.label')}: {root ?? t('worktrees.root.none')}"
      size={34}
      onclick={() => void chooseRoot()}
    />
    <IconButton
      icon={Languages}
      label="{t('common.language.label')}: {langName(locale.current)}"
      size={34}
      onclick={() =>
        pickLocale(
          LOCALES[(LOCALES.indexOf(locale.current) + 1) % LOCALES.length],
        )}
    />
    <IconButton
      icon={theme.current === "dark" ? Sun : Moon}
      label={themeLabel}
      size={34}
      onclick={toggleTheme}
    />
    <IconButton
      icon={PanelLeft}
      label="{t('common.sidebar.expand')} (Ctrl B)"
      size={34}
      onclick={() => (shell.sidebarCollapsed = false)}
    />
  {/if}
</aside>

<style>
  .sidebar {
    width: var(--sidebar-w);
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    background: var(--sidebar);
    border-right: 1px solid var(--border);
    padding: 12px 10px;
    overflow: hidden;
    transition: width var(--duration-base) var(--ease-out);
  }
  .sidebar.collapsed {
    width: var(--sidebar-w-collapsed);
    align-items: center;
    gap: 4px;
    padding: 12px 0;
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 0 2px 0 4px;
  }
  .collapsed .brand {
    padding: 0;
  }
  .logo {
    width: 28px;
    height: 28px;
    border-radius: 7px;
    background: var(--accent);
    color: var(--on-accent);
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }
  .name {
    font-size: var(--text-md);
    font-weight: var(--weight-bold);
    letter-spacing: -0.01em;
  }
  .spacer,
  .grow {
    flex: 1;
  }
  .section {
    margin: 22px 10px 8px;
    font-size: 11px;
    font-weight: var(--weight-semibold);
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-subtle);
  }
  .gap {
    height: 14px;
  }
  nav {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .collapsed nav {
    gap: 4px;
  }
  .nav {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    height: 34px;
    padding: 0 10px;
    border-radius: var(--radius-md);
    background: transparent;
    color: var(--text-muted);
    border: 1px solid transparent;
    font-size: var(--text-base);
    font-weight: var(--weight-medium);
    text-align: left;
  }
  .collapsed .nav {
    width: 36px;
    justify-content: center;
    padding: 0;
  }
  .nav:hover {
    background: var(--surface-hover);
    color: var(--text);
  }
  .nav.current {
    background: var(--surface-2);
    color: var(--text);
    border-color: var(--border);
  }
  .nav.current :global(svg) {
    color: var(--accent-text);
  }
  .lbl {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .count {
    margin-left: auto;
    font-size: var(--text-xs);
    color: var(--text-subtle);
  }
  .root {
    padding: 10px;
    margin-bottom: 10px;
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface);
  }
  .root .k {
    font-size: 11px;
    color: var(--text-subtle);
    margin-bottom: 4px;
  }
  .root .rootpath {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: var(--text-sm);
    min-width: 0;
  }
  .root .rootpath :global(svg) {
    flex-shrink: 0;
    color: var(--text-subtle);
  }
  .val {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    direction: rtl;
    text-align: left;
  }
  .link {
    margin-top: 6px;
    padding: 0;
    background: none;
    border: 0;
    color: var(--accent-text);
    font-size: 11.5px;
  }
  .link:hover {
    color: var(--text);
    text-decoration: underline;
  }
  .lang {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 2px;
  }
  .ico {
    color: var(--text-subtle);
    display: flex;
  }
  .seg {
    flex: 1;
    display: flex;
    gap: 2px;
    padding: 2px;
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
  }
  .seg button {
    flex: 1;
    min-width: 0;
    height: 24px;
    padding: 0 6px;
    border-radius: 5px;
    font-size: var(--text-sm);
    font-weight: var(--weight-medium);
    background: none;
    color: var(--text-subtle);
    border: 1px solid transparent;
    white-space: nowrap;
  }
  .seg button[aria-pressed="true"] {
    background: var(--surface-raised);
    color: var(--text);
    box-shadow: var(--shadow-sm);
    border-color: var(--border-strong);
  }
  .foot {
    margin-top: 6px;
  }
</style>
