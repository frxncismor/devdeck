<script lang="ts">
  import { tick } from "svelte";
  import Check from "@lucide/svelte/icons/check";
  import ChevronDown from "@lucide/svelte/icons/chevron-down";
  import Layers from "@lucide/svelte/icons/layers";
  import { toAppError } from "../lib/api";
  import { configState } from "../lib/config.svelte";
  import { errorMessage } from "../lib/errors";
  import { t } from "../lib/i18n/i18n.svelte";
  import { projectColor } from "../lib/projectColor";
  import { scopeState, setScope } from "../lib/scope.svelte";
  import { shell } from "../lib/shell.svelte";
  import { toasts } from "../lib/toasts.svelte";
  import type { Scope } from "../lib/types";

  let filter = $state("");
  let input: HTMLInputElement | undefined = $state();
  let root: HTMLDivElement | undefined = $state();
  const uid = $props.id();

  const scope = $derived(configState.value.scope);
  const ids = $derived(scopeState.projects.map((p) => p.id));
  const current = $derived(
    scope.kind === "project"
      ? scopeState.projects.find((p) => p.id === scope.id)
      : undefined,
  );
  const shown = $derived(
    scopeState.projects.filter((p) =>
      p.name.toLowerCase().includes(filter.trim().toLowerCase()),
    ),
  );
  const showGlobal = $derived(
    filter.trim() === "" ||
      t("common.scope.global")
        .toLowerCase()
        .includes(filter.trim().toLowerCase()),
  );

  $effect(() => {
    if (shell.scopeOpen) {
      filter = "";
      void tick().then(() => input?.focus());
    }
  });

  function choose(next: Scope) {
    shell.scopeOpen = false;
    setScope(next).catch((err) => {
      const msg = errorMessage(toAppError(err));
      console.error(msg);
      toasts.error(msg);
    });
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.stopPropagation();
      shell.scopeOpen = false;
      root?.querySelector<HTMLButtonElement>("button.trigger")?.focus();
    }
  }

  function onfocusout(e: FocusEvent) {
    if (!root?.contains(e.relatedTarget as Node | null))
      shell.scopeOpen = false;
  }
</script>

<div
  class="scope"
  bind:this={root}
  {onkeydown}
  {onfocusout}
  role="presentation"
>
  <button
    type="button"
    class="trigger"
    class:open={shell.scopeOpen}
    aria-haspopup="listbox"
    aria-expanded={shell.scopeOpen}
    aria-controls="{uid}-menu"
    onclick={() => (shell.scopeOpen = !shell.scopeOpen)}
  >
    <span class="ico">
      {#if current}
        <span
          class="sq"
          style:background={projectColor(current.id, ids)}
          aria-hidden="true"
        ></span>
      {:else}
        <Layers size={15} aria-hidden="true" />
      {/if}
    </span>
    <span class="txt">
      <span class="lbl">{t("common.scope.label")}</span>
      <span class="val">
        {current ? current.name : t("common.scope.global")}
        {#if !current}
          <span class="sub">
            · {t("common.scope.projects", {
              count: scopeState.projects.length,
            })}
          </span>
        {/if}
      </span>
    </span>
    <ChevronDown size={15} aria-hidden="true" />
  </button>

  {#if shell.scopeOpen}
    <div class="menu" id="{uid}-menu">
      <input
        bind:this={input}
        bind:value={filter}
        type="search"
        placeholder={t("common.scope.filter")}
        aria-label={t("common.scope.filter")}
      />
      <div class="list" role="listbox" aria-label={t("common.scope.label")}>
        {#if showGlobal}
          <button
            type="button"
            role="option"
            aria-selected={scope.kind === "global"}
            class="opt"
            onclick={() => choose({ kind: "global" })}
          >
            <Layers size={14} aria-hidden="true" />
            <span class="n">{t("common.scope.global")}</span>
            <span class="p">
              {t("common.scope.projects", {
                count: scopeState.projects.length,
              })}
            </span>
            {#if scope.kind === "global"}<Check
                size={14}
                aria-hidden="true"
              />{/if}
          </button>
        {/if}
        {#each shown as p (p.id)}
          <button
            type="button"
            role="option"
            aria-selected={scope.kind === "project" && scope.id === p.id}
            class="opt"
            onclick={() => choose({ kind: "project", id: p.id })}
          >
            <span
              class="sq"
              style:background={projectColor(p.id, ids)}
              aria-hidden="true"
            ></span>
            <span class="n">{p.name}</span>
            <span class="p mono" title={p.mainPath}>{p.mainPath}</span>
            {#if scope.kind === "project" && scope.id === p.id}<Check
                size={14}
                aria-hidden="true"
              />{/if}
          </button>
        {/each}
        {#if !showGlobal && shown.length === 0}
          <div class="none">{t("common.scope.noMatch")}</div>
        {/if}
      </div>
    </div>
  {/if}
</div>

<style>
  .scope {
    position: relative;
  }
  .trigger {
    height: 40px;
    min-width: 240px;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 0 10px 0 7px;
    background: var(--surface);
    border: 1px solid var(--border-strong);
    box-shadow: var(--shadow-sm);
    border-radius: var(--radius-lg);
    color: var(--text);
    text-align: left;
  }
  .trigger.open {
    border-color: var(--accent);
    box-shadow: 0 0 0 3px var(--accent-soft);
  }
  .ico {
    width: 26px;
    height: 26px;
    border-radius: 6px;
    background: var(--accent-soft);
    color: var(--accent-text);
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }
  .sq {
    width: 10px;
    height: 10px;
    border-radius: 2px;
    flex-shrink: 0;
  }
  .txt {
    display: flex;
    flex-direction: column;
    line-height: 1.2;
    min-width: 0;
    flex: 1;
  }
  .lbl {
    font-size: 10px;
    letter-spacing: 0.07em;
    text-transform: uppercase;
    color: var(--text-subtle);
    font-weight: var(--weight-semibold);
  }
  .val {
    font-weight: var(--weight-semibold);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .sub {
    font-weight: var(--weight-regular);
    color: var(--text-muted);
  }
  .trigger :global(svg:last-child) {
    color: var(--text-subtle);
    flex-shrink: 0;
  }
  .menu {
    position: absolute;
    top: calc(100% + 6px);
    left: 0;
    z-index: 50;
    width: 380px;
    max-width: calc(100vw - 48px);
    padding: 8px;
    background: var(--surface-raised);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-lg);
  }
  input {
    width: 100%;
    height: var(--control-h);
    padding: 0 10px;
    background: var(--surface);
    color: var(--text);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-md);
  }
  .list {
    display: flex;
    flex-direction: column;
    gap: 2px;
    margin-top: 8px;
    max-height: 320px;
    overflow: auto;
  }
  .opt {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 34px;
    padding: 0 10px;
    border: 0;
    border-radius: var(--radius-md);
    background: none;
    color: var(--text);
    text-align: left;
  }
  .opt:hover {
    background: var(--surface-hover);
  }
  .opt[aria-selected="true"] {
    background: var(--accent-soft);
  }
  .n {
    font-weight: var(--weight-medium);
    white-space: nowrap;
  }
  .p {
    flex: 1;
    min-width: 0;
    font-size: var(--text-xs);
    color: var(--text-subtle);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    direction: rtl;
    text-align: left;
  }
  .none {
    padding: 10px;
    color: var(--text-muted);
  }
</style>
