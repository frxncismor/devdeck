<script lang="ts">
  import { tick } from "svelte";
  import Layers from "@lucide/svelte/icons/layers";
  import Search from "@lucide/svelte/icons/search";
  import { toAppError } from "../lib/api";
  import { errorMessage } from "../lib/errors";
  import { t } from "../lib/i18n/i18n.svelte";
  import { MODULES, type ModuleId } from "../lib/modules";
  import { filterCommands, type PaletteCommand } from "../lib/palette";
  import { projectColor } from "../lib/projectColor";
  import { scopeState, setScope } from "../lib/scope.svelte";
  import { shell } from "../lib/shell.svelte";
  import { toasts } from "../lib/toasts.svelte";

  interface Props {
    onselect: (id: ModuleId) => void;
  }
  let { onselect }: Props = $props();

  type Cmd = PaletteCommand & { run: () => void; color?: string };

  let dialog: HTMLDialogElement | undefined = $state();
  let input: HTMLInputElement | undefined = $state();
  let query = $state("");
  let index = $state(0);
  const uid = $props.id();

  const ids = $derived(scopeState.projects.map((p) => p.id));

  const commands = $derived<Cmd[]>([
    ...MODULES.map((m) => ({
      id: `module:${m.id}`,
      label: t("common.palette.goTo", { name: t(m.labelKey) }),
      keywords: t(m.labelKey),
      group: "module" as const,
      run: () => onselect(m.id),
    })),
    {
      id: "scope:global",
      label: t("common.palette.scopeTo", { name: t("common.scope.global") }),
      keywords: t("common.scope.label"),
      group: "scope" as const,
      run: () => applyScope({ kind: "global" }),
    },
    ...scopeState.projects.map((p) => ({
      id: `scope:${p.id}`,
      label: t("common.palette.scopeTo", { name: p.name }),
      keywords: `${t("common.scope.label")} ${p.mainPath}`,
      group: "scope" as const,
      color: projectColor(p.id, ids),
      run: () => applyScope({ kind: "project", id: p.id }),
    })),
  ]);

  const results = $derived(filterCommands(commands, query));

  function applyScope(next: Parameters<typeof setScope>[0]) {
    setScope(next).catch((err) => toasts.error(errorMessage(toAppError(err))));
  }

  $effect(() => {
    if (!dialog) return;
    if (shell.paletteOpen && !dialog.open) {
      query = "";
      index = 0;
      dialog.showModal();
      void tick().then(() => input?.focus());
    }
    if (!shell.paletteOpen && dialog.open) dialog.close();
  });

  $effect(() => {
    void query;
    index = 0;
  });

  function run(c: Cmd | undefined) {
    if (!c) return;
    shell.paletteOpen = false;
    c.run();
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      index = results.length ? (index + 1) % results.length : 0;
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      index = results.length
        ? (index - 1 + results.length) % results.length
        : 0;
    } else if (e.key === "Enter") {
      e.preventDefault();
      run(results[index]);
    }
  }

  function onclickDialog(e: MouseEvent) {
    if (e.target === dialog) shell.paletteOpen = false;
  }
</script>

<dialog
  bind:this={dialog}
  aria-label={t("common.palette.title")}
  onclose={() => (shell.paletteOpen = false)}
  onclick={onclickDialog}
>
  <div class="box">
    <div class="field">
      <Search size={15} aria-hidden="true" />
      <input
        bind:this={input}
        bind:value={query}
        {onkeydown}
        type="text"
        role="combobox"
        aria-expanded="true"
        aria-controls="{uid}-list"
        aria-activedescendant={results[index]
          ? `${uid}-${results[index].id}`
          : undefined}
        placeholder={t("common.palette.placeholder")}
        aria-label={t("common.palette.placeholder")}
      />
      <kbd>Esc</kbd>
    </div>
    <ul id="{uid}-list" role="listbox" aria-label={t("common.palette.title")}>
      {#each results as c, i (c.id)}
        <li
          id="{uid}-{c.id}"
          role="option"
          aria-selected={i === index}
          class:sel={i === index}
        >
          <button
            type="button"
            tabindex="-1"
            onclick={() => run(c)}
            onmousemove={() => (index = i)}
          >
            {#if c.color}
              <span class="sq" style:background={c.color} aria-hidden="true"
              ></span>
            {:else if c.group === "scope"}
              <Layers size={14} aria-hidden="true" />
            {:else}
              <span class="sq none" aria-hidden="true"></span>
            {/if}
            {c.label}
          </button>
        </li>
      {/each}
      {#if results.length === 0}
        <li class="none">{t("common.palette.empty")}</li>
      {/if}
    </ul>
  </div>
</dialog>

<style>
  dialog {
    width: min(560px, calc(100vw - 48px));
    margin: 14vh auto 0;
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
  .field {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 48px;
    padding: 0 14px;
    border-bottom: 1px solid var(--border);
    color: var(--text-subtle);
  }
  input {
    flex: 1;
    height: 100%;
    background: none;
    border: 0;
    color: var(--text);
    font-size: var(--text-md);
  }
  input:focus-visible {
    outline: none;
  }
  ul {
    margin: 0;
    padding: 6px;
    list-style: none;
    max-height: 340px;
    overflow: auto;
  }
  li button {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    height: 34px;
    padding: 0 10px;
    border: 0;
    border-radius: var(--radius-md);
    background: none;
    color: var(--text);
    text-align: left;
  }
  li.sel button {
    background: var(--accent-soft);
  }
  .sq {
    width: 10px;
    height: 10px;
    border-radius: 2px;
  }
  .sq.none {
    background: var(--border-strong);
  }
  .none {
    padding: 12px 10px;
    color: var(--text-muted);
  }
</style>
