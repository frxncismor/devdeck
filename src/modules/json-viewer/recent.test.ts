import { beforeEach, describe, expect, it } from "vitest";
import { loadRecent, parsePath, pushRecent, removeRecent } from "./recent";

describe("recent files", () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it("keeps the newest first, without duplicates, capped at 5", () => {
    for (const p of ["a", "b", "c", "d", "e", "f", "c"]) pushRecent(p);
    expect(loadRecent()).toEqual(["c", "f", "e", "d", "b"]);
  });

  it("removes a path and survives corrupt storage", () => {
    pushRecent("a");
    pushRecent("b");
    expect(removeRecent("a")).toEqual(["b"]);
    localStorage.setItem("devdeck.json.recent", "{oops");
    expect(loadRecent()).toEqual([]);
  });
});

describe("parsePath", () => {
  it("splits keys, indices and quoted keys", () => {
    expect(parsePath('$.orders[1204]["a b"].customer')).toEqual([
      "$",
      "orders",
      "[1204]",
      "a b",
      "customer",
    ]);
    expect(parsePath("$")).toEqual(["$"]);
  });
});
