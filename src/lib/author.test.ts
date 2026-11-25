import { beforeEach, describe, expect, it, vi } from "vitest";

const openUrl = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/plugin-opener", () => ({ openUrl }));

import { AUTHOR_URL, openAuthor } from "./author";

beforeEach(() => {
  openUrl.mockReset().mockResolvedValue(undefined);
});

describe("openAuthor", () => {
  it("opens the author URL in the system browser and stops WebView navigation", () => {
    const event = { preventDefault: vi.fn() };
    openAuthor(event);
    expect(event.preventDefault).toHaveBeenCalledOnce();
    expect(openUrl).toHaveBeenCalledWith("https://github.com/frxncismor");
    expect(AUTHOR_URL).toBe("https://github.com/frxncismor");
  });

  it("swallows an opener failure", async () => {
    const err = vi.spyOn(console, "error").mockImplementation(() => {});
    openUrl.mockRejectedValue(new Error("denied"));
    openAuthor();
    await Promise.resolve();
    await Promise.resolve();
    expect(err).toHaveBeenCalled();
    err.mockRestore();
  });
});
