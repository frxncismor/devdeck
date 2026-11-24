import { t } from "./i18n/i18n.svelte";
import type { MessageKey } from "./i18n/keys";
import { messages } from "./i18n";
import type { AppError } from "./types";

/** Translated headline for a backend error (`errors.<code>`, fallback `errors.unknown`). */
export function errorMessage(e: AppError): string {
  const known = e.code in messages.en.common.errors;
  const key = (
    known ? `common.errors.${e.code}` : "common.errors.unknown"
  ) as MessageKey;
  return t(key, e.params);
}
