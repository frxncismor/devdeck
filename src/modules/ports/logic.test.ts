import { describe, expect, it } from "vitest";
import {
  autoRefreshMs,
  baseName,
  effectiveRefreshMs,
  filterProtocol,
  rowKey,
  workingFolder,
  type ListeningPort,
} from "./logic";

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

describe("filterProtocol", () => {
  const mk = (protocol: "tcp" | "udp", port: number): ListeningPort => ({
    port,
    protocol,
    pid: 1,
    processName: "x",
  });
  const rows = [mk("tcp", 1), mk("udp", 2), mk("tcp", 3)];
  it("keeps everything for all and filters by protocol otherwise", () => {
    expect(filterProtocol(rows, "all")).toHaveLength(3);
    expect(filterProtocol(rows, "tcp").map((r) => r.port)).toEqual([1, 3]);
    expect(filterProtocol(rows, "udp").map((r) => r.port)).toEqual([2]);
  });
});

describe("effectiveRefreshMs", () => {
  it("defaults to 5 s when unset and honours the config otherwise", () => {
    expect(effectiveRefreshMs(undefined)).toBe(5000);
    expect(effectiveRefreshMs(null)).toBe(5000);
    expect(effectiveRefreshMs(0)).toBeNull();
    expect(effectiveRefreshMs(10)).toBe(10000);
  });
});

describe("workingFolder", () => {
  it("shows the cwd, not the executable, and keeps the exe in the tooltip", () => {
    expect(workingFolder({ cwd: "/w/webshop", exePath: "/bin/node" })).toEqual({
      text: "/w/webshop",
      title: "/w/webshop\n/bin/node",
    });
  });
  it("uses the cwd alone when the executable is unknown", () => {
    expect(workingFolder({ cwd: "/w/webshop" })).toEqual({
      text: "/w/webshop",
      title: "/w/webshop",
    });
  });
  it("falls back to the executable, then to nothing", () => {
    expect(workingFolder({ exePath: "/bin/node" })).toEqual({
      text: "/bin/node",
      title: "/bin/node",
    });
    expect(workingFolder({ cwd: null, exePath: null })).toBeNull();
  });
});
