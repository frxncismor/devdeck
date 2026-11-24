<script lang="ts">
  import { toAppError } from "../lib/api";
  import { errorMessage } from "../lib/errors";
  import { t } from "../lib/i18n/i18n.svelte";
  import { configState } from "../lib/config.svelte";
  import { scopeState, setScope } from "../lib/scope.svelte";

  const value = $derived(
    configState.value.scope.kind === "project"
      ? configState.value.scope.id
      : "",
  );

  function onchange(e: Event) {
    const v = (e.currentTarget as HTMLSelectElement).value;
    setScope(v === "" ? { kind: "global" } : { kind: "project", id: v }).catch(
      (err) => console.error(errorMessage(toAppError(err))),
    );
  }
</script>

<label class="field">
  <span>{t("common.scope.label")}</span>
  <select {value} {onchange}>
    <option value="">{t("common.scope.global")}</option>
    {#each scopeState.projects as p (p.id)}
      <option value={p.id}>{p.name}</option>
    {/each}
  </select>
</label>
