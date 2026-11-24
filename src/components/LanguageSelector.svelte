<script lang="ts">
  import { LOCALES, type Locale } from "../lib/i18n";
  import { locale, setLocale, t } from "../lib/i18n/i18n.svelte";

  function onchange(e: Event) {
    void setLocale((e.currentTarget as HTMLSelectElement).value as Locale);
  }
</script>

<label class="field">
  <span>{t("common.language.label")}</span>
  <select value={locale.current} {onchange}>
    {#each LOCALES as l (l)}
      <option value={l}>
        {t(l === "en" ? "common.language.en" : "common.language.es")}
      </option>
    {/each}
  </select>
</label>
