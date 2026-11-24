import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { TOAST_TIMEOUT_MS, toastState, toasts } from "./toasts.svelte";

describe("toasts store", () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => {
    toasts.clear();
    vi.useRealTimers();
  });

  it("stacks toasts in order with unique ids", () => {
    const a = toasts.info("A");
    const b = toasts.success("B", "detail");
    expect(a).not.toBe(b);
    expect(toastState.items.map((x) => x.title)).toEqual(["A", "B"]);
    expect(toastState.items[1]).toMatchObject({
      tone: "success",
      message: "detail",
    });
  });

  it("auto-dismisses non-error toasts after 5 s", () => {
    toasts.warning("W");
    vi.advanceTimersByTime(TOAST_TIMEOUT_MS - 1);
    expect(toastState.items).toHaveLength(1);
    vi.advanceTimersByTime(1);
    expect(toastState.items).toHaveLength(0);
  });

  it("keeps error toasts until dismissed", () => {
    const id = toasts.error("E");
    vi.advanceTimersByTime(TOAST_TIMEOUT_MS * 10);
    expect(toastState.items).toHaveLength(1);
    toasts.dismiss(id);
    expect(toastState.items).toHaveLength(0);
  });

  it("dismisses one toast without touching the others", () => {
    const a = toasts.info("A");
    toasts.info("B");
    toasts.dismiss(a);
    expect(toastState.items.map((x) => x.title)).toEqual(["B"]);
  });
});
