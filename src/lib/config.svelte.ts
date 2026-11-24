import { api } from "./api";
import { applyLocale, setLocalePersister } from "./i18n/i18n.svelte";
import type { Config, Language } from "./types";

export const DEFAULT_CONFIG: Config = {
  scanDepth: 2,
  baseBranch: "main",
  scope: { kind: "global" },
  language: "en",
  forgottenDays: 30,
};

export const configState = $state<{ value: Config; loaded: boolean }>({
  value: { ...DEFAULT_CONFIG },
  loaded: false,
});

/** Loads the stored config, applies the stored language and wires language persistence. */
export async function loadConfig(): Promise<void> {
  try {
    configState.value = { ...DEFAULT_CONFIG, ...(await api.getConfig()) };
  } catch {
    configState.value = { ...DEFAULT_CONFIG };
  }
  applyLocale(configState.value.language);
  setLocalePersister((l: Language) => updateConfig({ language: l }));
  configState.loaded = true;
}

export async function updateConfig(patch: Partial<Config>): Promise<void> {
  configState.value = { ...configState.value, ...patch };
  await api.setConfig($state.snapshot(configState.value));
}
