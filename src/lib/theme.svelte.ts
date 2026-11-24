export type Theme = "dark" | "light";

/** Front-only preference (NOT part of the Rust config contract). */
export const THEME_KEY = "devdeck.theme";

const state = $state<{ current: Theme }>({ current: "dark" });

/** Reactive current theme. */
export const theme = {
  get current(): Theme {
    return state.current;
  },
};

export function readStoredTheme(storage?: Pick<Storage, "getItem">): Theme {
  try {
    const v = (storage ?? localStorage).getItem(THEME_KEY);
    return v === "light" ? "light" : "dark";
  } catch {
    return "dark";
  }
}

function applyToDocument(t: Theme): void {
  if (typeof document === "undefined") return;
  const el = document.documentElement;
  el.classList.add("dd");
  el.classList.remove("dark", "light");
  el.classList.add(t);
}

/** Reads the stored theme and themes `<html>` synchronously; call before mounting (no flash). */
export function initTheme(): Theme {
  state.current = readStoredTheme();
  applyToDocument(state.current);
  return state.current;
}

export function setTheme(t: Theme): void {
  state.current = t;
  applyToDocument(t);
  try {
    localStorage.setItem(THEME_KEY, t);
  } catch {
    // Storage unavailable: the theme still applies for this session.
  }
}

export function toggleTheme(): void {
  setTheme(state.current === "dark" ? "light" : "dark");
}
