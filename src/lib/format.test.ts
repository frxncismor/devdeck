import { afterEach, describe, expect, it } from "vitest";
import { applyLocale } from "./i18n/i18n.svelte";
import {
  formatBytes,
  formatDate,
  formatDateTime,
  formatNumber,
  formatPercent,
  formatRelativeAge,
} from "./format";

afterEach(() => applyLocale("en"));

describe("format (en)", () => {
  it("formatBytes uses binary scale", () => {
    applyLocale("en");
    expect(formatBytes(0)).toBe("0 B");
    expect(formatBytes(1023)).toBe("1,023 B");
    expect(formatBytes(1536)).toBe("1.5 KB");
    expect(formatBytes(5 * 1024 ** 3)).toBe("5 GB");
  });
  it("formatNumber / formatPercent", () => {
    applyLocale("en");
    expect(formatNumber(1234567.5)).toBe("1,234,567.5");
    expect(formatPercent(0.256)).toBe("25.6%");
  });
  it("formatDate / formatDateTime", () => {
    applyLocale("en");
    const iso = "2026-03-09T14:05:00Z";
    expect(formatDate(iso, "UTC")).toBe("Mar 9, 2026");
    expect(formatDateTime(iso, "UTC")).toContain("2:05");
  });
  it("formatRelativeAge", () => {
    applyLocale("en");
    expect(formatRelativeAge(3 * 86400)).toBe("3 days ago");
    expect(formatRelativeAge(2 * 3600)).toBe("2 hours ago");
    expect(formatRelativeAge(5 * 60)).toBe("5 minutes ago");
  });
});

describe("format (es)", () => {
  it("formatBytes uses decimal comma", () => {
    applyLocale("es");
    expect(formatBytes(1536)).toBe("1,5 KB");
    expect(formatBytes(5 * 1024 ** 3)).toBe("5 GB");
  });
  it("formatNumber / formatPercent", () => {
    applyLocale("es");
    expect(formatNumber(1234567.5)).toBe("1.234.567,5");
    expect(formatPercent(0.256)).toMatch(/^25,6\s?%$/);
  });
  it("formatDate", () => {
    applyLocale("es");
    expect(formatDate("2026-03-09T14:05:00Z", "UTC")).toBe("9 mar 2026");
  });
  it("formatRelativeAge", () => {
    applyLocale("es");
    expect(formatRelativeAge(3 * 86400)).toBe("hace 3 días");
    expect(formatRelativeAge(2 * 3600)).toBe("hace 2 horas");
    expect(formatRelativeAge(5 * 60)).toBe("hace 5 minutos");
  });
});
