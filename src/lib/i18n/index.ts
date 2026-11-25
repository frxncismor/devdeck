import enCommon from "./locales/en/common.json";
import enWorktrees from "./locales/en/worktrees.json";
import enPorts from "./locales/en/ports.json";
import enJsonViewer from "./locales/en/json-viewer.json";
import enReclaim from "./locales/en/reclaim.json";
import enGlossary from "./locales/en/glossary.json";
import esGlossary from "./locales/es/glossary.json";
import esCommon from "./locales/es/common.json";
import esWorktrees from "./locales/es/worktrees.json";
import esPorts from "./locales/es/ports.json";
import esJsonViewer from "./locales/es/json-viewer.json";
import esReclaim from "./locales/es/reclaim.json";

/**
 * Closed catalogue: adding a namespace is an explicit edit of this file.
 * The translator (`t`, `locale`, `setLocale`) lives in `./i18n.svelte`.
 */
export const messages = {
  en: {
    common: enCommon,
    worktrees: enWorktrees,
    ports: enPorts,
    jsonViewer: enJsonViewer,
    reclaim: enReclaim,
    glossary: enGlossary,
  },
  es: {
    common: esCommon,
    worktrees: esWorktrees,
    ports: esPorts,
    jsonViewer: esJsonViewer,
    reclaim: esReclaim,
    glossary: esGlossary,
  },
} as const;

export type Locale = keyof typeof messages;
export const LOCALES: readonly Locale[] = ["en", "es"];
export const DEFAULT_LOCALE: Locale = "en";
