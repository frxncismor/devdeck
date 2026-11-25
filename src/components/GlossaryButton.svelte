<script lang="ts">
  import CircleHelp from "@lucide/svelte/icons/circle-question-mark";
  import X from "@lucide/svelte/icons/x";
  import { entryParams, GLOSSARY, type GlossaryModule } from "../lib/glossary";
  import { t } from "../lib/i18n/i18n.svelte";
  import type { MessageKey } from "../lib/i18n/keys";
  import IconButton from "./IconButton.svelte";

  interface Props {
    module: GlossaryModule;
    /** Extra values for the meanings (e.g. the configured inactivity days). */
    params?: Record<string, string | number>;
  }
  let { module, params = {} }: Props = $props();

  let dialog: HTMLDialogElement | undefined = $state();
  let opener: HTMLElement | null = null;
  const uid = $props.id();
  const entries = $derived(GLOSSARY[module]);
  const moduleTitle = $derived(
    t(`glossary.moduleTitle.${module}` as MessageKey),
  );

  function show(e: MouseEvent) {
    opener = e.currentTarget as HTMLElement;
    dialog?.showModal();
  }
  /** Native <dialog> closes on Esc; focus goes back to the "?" button explicitly. */
  function onclose() {
    opener?.focus();
  }
</script>

<IconButton
  label={t("glossary.open")}
  icon={CircleHelp}
  aria-haspopup="dialog"
  onclick={show}
/>

<dialog bind:this={dialog} {onclose} aria-labelledby="{uid}-title">
  <header>
    <div>
      <h2 id="{uid}-title">{t("glossary.title")} · {moduleTitle}</h2>
      <p class="intro">{t("glossary.intro")}</p>
    </div>
    <IconButton
      label={t("glossary.close")}
      icon={X}
      onclick={() => dialog?.close()}
    />
  </header>
  <dl>
    {#each entries as e (e.id)}
      {@const p = entryParams(e, params)}
      <div class="entry">
        <dt>{t(e.termKey, p)}</dt>
        <dd class="meaning">{t(e.meaningKey, p)}</dd>
        <dd>
          <span class="k">{t("glossary.todo")}</span>
          {t(e.todoKey)}
        </dd>
        <dd class="ex">
          <span class="k">{t("glossary.example")}</span>
          {t(e.exampleKey)}
        </dd>
      </div>
    {/each}
  </dl>
</dialog>

<style>
  dialog {
    width: min(680px, calc(100vw - 48px));
    max-height: calc(100vh - 64px);
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
  header {
    position: sticky;
    top: 0;
    display: flex;
    justify-content: space-between;
    gap: var(--space-3);
    padding: 16px 20px 12px;
    background: var(--surface-raised);
    border-bottom: 1px solid var(--border);
  }
  h2 {
    font-size: var(--text-lg);
    font-weight: var(--weight-semibold);
  }
  .intro {
    margin: 4px 0 0;
    color: var(--text-muted);
    font-size: var(--text-sm);
  }
  dl {
    margin: 0;
    padding: 4px 20px 16px;
  }
  .entry {
    padding: 12px 0;
    border-bottom: 1px solid var(--border);
  }
  .entry:last-child {
    border-bottom: 0;
  }
  dt {
    font-weight: var(--weight-semibold);
    font-size: var(--text-base);
  }
  dd {
    margin: 4px 0 0;
    font-size: var(--text-sm);
    color: var(--text-muted);
    overflow-wrap: anywhere;
  }
  .meaning {
    color: var(--text);
  }
  .k {
    margin-right: 6px;
    color: var(--text-subtle);
    font-size: var(--text-2xs);
    font-weight: var(--weight-semibold);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .ex {
    font-style: italic;
  }
</style>
