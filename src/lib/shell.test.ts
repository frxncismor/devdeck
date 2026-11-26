import { afterEach, describe, expect, it, vi } from "vitest";
import { SHELL_ACTIVE_KEY, SHELL_COLLAPSED_KEY } from "./shell.svelte";

/** A fresh module instance, as after restarting the app. */
async function restart() {
  vi.resetModules();
  return (await import("./shell.svelte")).shell;
}

afterEach(() => localStorage.clear());

describe("shell persistence", () => {
  it("defaults to Worktrees with the sidebar expanded", async () => {
    const shell = await restart();
    expect(shell.active).toBe("worktrees");
    expect(shell.sidebarCollapsed).toBe(false);
  });

  it("restores the active module and the collapsed sidebar", async () => {
    const first = await restart();
    first.active = "reclaim";
    first.sidebarCollapsed = true;
    const second = await restart();
    expect(second.active).toBe("reclaim");
    expect(second.sidebarCollapsed).toBe(true);
    second.sidebarCollapsed = false;
    expect((await restart()).sidebarCollapsed).toBe(false);
  });

  it("ignores junk in storage", async () => {
    localStorage.setItem(SHELL_ACTIVE_KEY, "nope");
    localStorage.setItem(SHELL_COLLAPSED_KEY, "yes");
    const shell = await restart();
    expect(shell.active).toBe("worktrees");
    expect(shell.sidebarCollapsed).toBe(false);
  });

  it("does not persist the palette or the scope picker", async () => {
    const first = await restart();
    first.paletteOpen = true;
    first.scopeOpen = true;
    const second = await restart();
    expect(second.paletteOpen).toBe(false);
    expect(second.scopeOpen).toBe(false);
  });
});
