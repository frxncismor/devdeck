<script lang="ts" module>
  export interface TabDef {
    id: string;
    label: string;
    count?: number;
  }
</script>

<script lang="ts">
  interface Props {
    tabs: TabDef[];
    /** Active tab id. */
    active: string;
    onchange: (id: string) => void;
    ariaLabel?: string;
    /** Prefix for ids: tab `${idPrefix}-tab-<id>`, panel should be `${idPrefix}-panel-<id>`. */
    idPrefix?: string;
  }
  let {
    tabs,
    active,
    onchange,
    ariaLabel,
    idPrefix = "tabs",
  }: Props = $props();

  function onkeydown(e: KeyboardEvent, index: number) {
    let next: number;
    if (e.key === "ArrowRight") next = (index + 1) % tabs.length;
    else if (e.key === "ArrowLeft")
      next = (index - 1 + tabs.length) % tabs.length;
    else if (e.key === "Home") next = 0;
    else if (e.key === "End") next = tabs.length - 1;
    else return;
    e.preventDefault();
    onchange(tabs[next].id);
    document.getElementById(`${idPrefix}-tab-${tabs[next].id}`)?.focus();
  }
</script>

<div class="tabs" role="tablist" aria-label={ariaLabel}>
  {#each tabs as tab, i (tab.id)}
    <button
      type="button"
      role="tab"
      id="{idPrefix}-tab-{tab.id}"
      aria-selected={active === tab.id}
      aria-controls="{idPrefix}-panel-{tab.id}"
      tabindex={active === tab.id ? 0 : -1}
      class:active={active === tab.id}
      onclick={() => onchange(tab.id)}
      onkeydown={(e) => onkeydown(e, i)}
    >
      {tab.label}
      {#if tab.count !== undefined}<span class="pill tabular">
          {tab.count}
        </span>{/if}
    </button>
  {/each}
</div>

<style>
  .tabs {
    display: flex;
    gap: 4px;
    border-bottom: 1px solid var(--border);
  }
  button {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    height: 36px;
    padding: 0 12px;
    background: none;
    border: 0;
    border-bottom: 2px solid transparent;
    margin-bottom: -1px;
    color: var(--text-muted);
    font-weight: var(--weight-medium);
    white-space: nowrap;
  }
  button:hover {
    color: var(--text);
  }
  button.active {
    color: var(--text);
    border-bottom-color: var(--accent);
  }
  .pill {
    font-size: var(--text-xs);
    min-width: 20px;
    padding: 0 6px;
    line-height: 18px;
    text-align: center;
    border-radius: var(--radius-full);
    background: var(--surface-hover);
    color: var(--text-muted);
  }
  .active .pill {
    background: var(--accent-soft);
    color: var(--accent-text);
  }
</style>
