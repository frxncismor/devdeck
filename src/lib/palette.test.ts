import { describe, expect, it } from "vitest";
import { filterCommands, type PaletteCommand } from "./palette";

const cmds: PaletteCommand[] = [
  { id: "m1", label: "Go to Ports", keywords: "puertos", group: "module" },
  { id: "m2", label: "Go to JSON Viewer", group: "module" },
  {
    id: "s1",
    label: "Scope: devdeck",
    keywords: "c:/dev/devdeck",
    group: "scope",
  },
];

describe("filterCommands", () => {
  it("returns everything for an empty query", () => {
    expect(filterCommands(cmds, "  ").map((c) => c.id)).toEqual([
      "m1",
      "m2",
      "s1",
    ]);
  });

  it("matches case-insensitively on the label", () => {
    expect(filterCommands(cmds, "json").map((c) => c.id)).toEqual(["m2"]);
  });

  it("matches keywords", () => {
    expect(filterCommands(cmds, "puertos").map((c) => c.id)).toEqual(["m1"]);
  });

  it("requires every word (AND, any order)", () => {
    expect(filterCommands(cmds, "scope dev").map((c) => c.id)).toEqual(["s1"]);
    expect(filterCommands(cmds, "viewer ports")).toEqual([]);
  });
});
