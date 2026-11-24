import { describe, expect, it } from "vitest";
import { PROJECT_COLORS, projectColor } from "./projectColor";

describe("projectColor", () => {
  const ids = ["a", "b", "c", "d", "e"];

  it("assigns --project-1..4 by project order", () => {
    expect(projectColor("a", ids)).toBe("var(--project-1)");
    expect(projectColor("b", ids)).toBe("var(--project-2)");
    expect(projectColor("d", ids)).toBe("var(--project-4)");
  });

  it("cycles after the palette is exhausted", () => {
    expect(PROJECT_COLORS).toBe(4);
    expect(projectColor("e", ids)).toBe("var(--project-1)");
  });

  it("is stable for the same order", () => {
    expect(projectColor("c", ids)).toBe(projectColor("c", [...ids]));
  });

  it("uses --project-none for no/unknown project", () => {
    expect(projectColor(undefined, ids)).toBe("var(--project-none)");
    expect(projectColor(null, ids)).toBe("var(--project-none)");
    expect(projectColor("zzz", ids)).toBe("var(--project-none)");
  });
});
