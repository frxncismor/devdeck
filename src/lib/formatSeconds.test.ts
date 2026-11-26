import { afterEach, describe, expect, it } from "vitest";
import { formatSeconds } from "./formatSeconds";
import { applyLocale } from "./i18n/i18n.svelte";

afterEach(() => {
  applyLocale("en");
});

describe("formatSeconds", () => {
  it("uses the decimal separator of the active language", () => {
    applyLocale("en");
    expect(formatSeconds(2400)).toBe("2.4 s");
    applyLocale("es");
    expect(formatSeconds(2400)).toBe("2,4 s");
  });

  it("always shows one decimal", () => {
    expect(formatSeconds(2000)).toBe("2.0 s");
    expect(formatSeconds(40)).toBe("0.0 s");
  });
});
