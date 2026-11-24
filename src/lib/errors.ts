import { formatBytes } from "./format";
import { t } from "./i18n/i18n.svelte";
import type { MessageKey } from "./i18n/keys";
import { messages } from "./i18n";
import type { AppError } from "./types";

/**
 * Translated headline for a backend error: every code Rust emits has an entry in
 * `common.errors.<code>` (single catalogue); unknown codes fall back to `errors.unknown`.
 * Byte-size params (`maxBytes`) are formatted for the current language.
 */
export function errorMessage(e: AppError): string {
  const known = e.code in messages.en.common.errors;
  const key = (
    known ? `common.errors.${e.code}` : "common.errors.unknown"
  ) as MessageKey;
  const params = { ...e.params };
  if (typeof params.maxBytes === "number")
    params.maxBytes = formatBytes(params.maxBytes);
  return t(key, params);
}
