import { afterEach, describe, expect, it } from "vitest";
import {
  THEME_KEY,
  initTheme,
  readStoredTheme,
  theme,
  toggleTheme,
} from "./theme.svelte";

afterEach(() => {
  localStorage.clear();
  document.documentElement.className = "";
});

describe("theme", () => {
  it("defaults to dark and ignores junk", () => {
    expect(readStoredTheme()).toBe("dark");
    localStorage.setItem(THEME_KEY, "purple");
    expect(readStoredTheme()).toBe("dark");
  });

  it("initTheme themes <html> from storage (no flash)", () => {
    localStorage.setItem(THEME_KEY, "light");
    expect(initTheme()).toBe("light");
    const cl = document.documentElement.classList;
    expect(
      cl.contains("dd") && cl.contains("light") && !cl.contains("dark"),
    ).toBe(true);
  });

  it("toggle swaps the theme and persists it", () => {
    initTheme();
    toggleTheme();
    expect(theme.current).toBe("light");
    expect(localStorage.getItem(THEME_KEY)).toBe("light");
    toggleTheme();
    expect(theme.current).toBe("dark");
    expect(localStorage.getItem(THEME_KEY)).toBe("dark");
  });
});
