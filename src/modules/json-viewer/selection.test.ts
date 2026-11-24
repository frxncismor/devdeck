import { describe, expect, it } from "vitest";
import { depthOfPath, describeValue } from "./selection";

describe("depthOfPath", () => {
  it("counts segments below the root", () => {
    expect(depthOfPath("$")).toBe(0);
    expect(depthOfPath("$.meta.search.needle")).toBe(3);
    expect(depthOfPath("$.orders[1204].customer")).toBe(3);
    expect(depthOfPath('$["a.b"][3]')).toBe(2);
    expect(depthOfPath('$["a b"].x')).toBe(2);
  });
});

describe("describeValue", () => {
  it("detects the kind from the raw JSON text", () => {
    expect(describeValue('"hi"')).toEqual({ kind: "string", childCount: null });
    expect(describeValue(" 12.5")).toEqual({
      kind: "number",
      childCount: null,
    });
    expect(describeValue("-3")).toEqual({ kind: "number", childCount: null });
    expect(describeValue("true")).toEqual({ kind: "bool", childCount: null });
    expect(describeValue("false")).toEqual({ kind: "bool", childCount: null });
    expect(describeValue("null")).toEqual({ kind: "null", childCount: null });
  });
  it("counts the children of a complete container", () => {
    expect(describeValue('{"a":1,"b":[1,2]}')).toEqual({
      kind: "object",
      childCount: 2,
    });
    expect(describeValue("[1,2,3]")).toEqual({ kind: "array", childCount: 3 });
    expect(describeValue("{}")).toEqual({ kind: "object", childCount: 0 });
  });
  it("keeps the kind but not the count when the value was cut", () => {
    expect(describeValue('{"a":1,"b":…')).toEqual({
      kind: "object",
      childCount: null,
    });
  });
  it("returns null for empty or unknown text", () => {
    expect(describeValue("")).toBeNull();
    expect(describeValue("?")).toBeNull();
  });
});
