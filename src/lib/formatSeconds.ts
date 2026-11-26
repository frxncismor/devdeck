import { locale } from "./i18n/i18n.svelte";

/** A duration in milliseconds as seconds with one decimal ("2.4 s" / "2,4 s"). */
export function formatSeconds(ms: number): string {
  const num = new Intl.NumberFormat(locale.current, {
    minimumFractionDigits: 1,
    maximumFractionDigits: 1,
  }).format(ms / 1000);
  return `${num} s`;
}
