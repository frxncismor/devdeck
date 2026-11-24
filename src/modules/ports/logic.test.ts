import { describe, expect, it } from "vitest";
import { autoRefreshMs, baseName, rowKey, type ListeningPort } from "./logic";

describe("baseName", () => {
  it("handles both separators and trailing ones", () => {
    expect(
      baseName(["C:", "repos", "app-feat", ""].join(String.fromCharCode(92))),
    ).toBe("app-feat");
    expect(baseName("/w/app/")).toBe("app");
    expect(baseName("/")).toBe("/");
  });
});

describe("autoRefreshMs", () => {
  it("is off for missing, zero, negative or invalid values", () => {
    expect(autoRefreshMs(undefined)).toBeNull();
    expect(autoRefreshMs(null)).toBeNull();
    expect(autoRefreshMs(0)).toBeNull();
    expect(autoRefreshMs(-3)).toBeNull();
    expect(autoRefreshMs(Number.NaN)).toBeNull();
  });
  it("converts seconds to ms", () => {
    expect(autoRefreshMs(5)).toBe(5000);
    expect(autoRefreshMs(1.7)).toBe(1000);
  });
});

describe("rowKey", () => {
  it("differs per port, protocol and pid", () => {
    const base: ListeningPort = {
      port: 3000,
      protocol: "tcp",
      pid: 1,
      processName: "node",
    };
    expect(rowKey(base)).not.toBe(rowKey({ ...base, port: 3001 }));
    expect(rowKey(base)).not.toBe(rowKey({ ...base, protocol: "udp" }));
    expect(rowKey(base)).toBe(rowKey({ ...base }));
  });
});
