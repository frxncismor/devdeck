import { createRawSnippet, flushSync, mount, unmount } from "svelte";
import { afterEach, describe, expect, it } from "vitest";
import VirtualList from "./VirtualList.svelte";

const items = Array.from({ length: 100 }, (_, i) => i);
const row = createRawSnippet<[number, number]>((item) => ({
  render: () => `<span class="r">${item()}</span>`,
}));

let app: ReturnType<typeof mount> | undefined;

function setup() {
  const target = document.body.appendChild(document.createElement("div"));
  app = mount(VirtualList<number>, {
    target,
    props: { items, rowHeight: 20, height: 200, row },
  });
  flushSync();
  const el = target.querySelector<HTMLElement>(".virtual")!;
  const shown = () =>
    [...el.querySelectorAll(".r")].map((n) => Number(n.textContent));
  const list = app as unknown as { scrollToIndex: (i: number) => void };
  return { el, shown, list };
}

afterEach(() => {
  if (app) void unmount(app);
  app = undefined;
  document.body.innerHTML = "";
});

describe("VirtualList", () => {
  it("is its own scroll container", () => {
    const { el } = setup();
    expect(el.style.overflowY).toBe("auto");
    expect(el.style.height).toBe("200px");
  });

  it("moves the rendered window when scrolled", () => {
    const { el, shown } = setup();
    expect(shown()[0]).toBe(0);
    el.scrollTop = 1000;
    el.dispatchEvent(new Event("scroll"));
    flushSync();
    expect(shown()).toContain(50);
    expect(shown()).not.toContain(0);
  });

  it("scrolls an off-screen row into view, centred", () => {
    const { el, shown, list } = setup();
    list.scrollToIndex(60);
    flushSync();
    expect(el.scrollTop).toBe(60 * 20 - 90);
    expect(shown()).toContain(60);
  });

  it("does not move when the row is already visible", () => {
    const { el, list } = setup();
    list.scrollToIndex(3);
    expect(el.scrollTop).toBe(0);
  });
});
