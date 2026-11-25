import { locale } from "./i18n/i18n.svelte";

const UNITS = ["B", "KB", "MB", "GB", "TB"] as const;

/** Binary scale (1 KB = 1024 B); the numeric part is localized via `Intl`. */
export function formatBytes(bytes: number): string {
  let value = Math.max(0, bytes);
  let unit = 0;
  while (value >= 1024 && unit < UNITS.length - 1) {
    value /= 1024;
    unit += 1;
  }
  const num = new Intl.NumberFormat(locale.current, {
    maximumFractionDigits: unit === 0 ? 0 : 1,
  }).format(value);
  return `${num} ${UNITS[unit]}`;
}

export function formatNumber(value: number): string {
  return new Intl.NumberFormat(locale.current).format(value);
}

/** `ratio` is a fraction (0.25 => 25%). */
export function formatPercent(ratio: number): string {
  return new Intl.NumberFormat(locale.current, {
    style: "percent",
    maximumFractionDigits: 1,
  }).format(ratio);
}

export function formatDate(iso: string | Date, timeZone?: string): string {
  return new Intl.DateTimeFormat(locale.current, {
    dateStyle: "medium",
    timeZone,
  }).format(new Date(iso));
}

export function formatDateTime(iso: string | Date, timeZone?: string): string {
  return new Intl.DateTimeFormat(locale.current, {
    dateStyle: "medium",
    timeStyle: "short",
    timeZone,
  }).format(new Date(iso));
}

/** Age in seconds rendered as "3 days ago" / "hace 3 días" (days, hours or minutes). */
export function formatRelativeAge(ageSecs: number): string {
  const rtf = new Intl.RelativeTimeFormat(locale.current, {
    numeric: "always",
  });
  if (ageSecs >= 86400) return rtf.format(-Math.floor(ageSecs / 86400), "day");
  if (ageSecs >= 3600) return rtf.format(-Math.floor(ageSecs / 3600), "hour");
  return rtf.format(-Math.max(0, Math.floor(ageSecs / 60)), "minute");
}
