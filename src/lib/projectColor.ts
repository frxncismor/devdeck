/** Number of project colors in the tokens (`--project-1..4`); further projects cycle. */
export const PROJECT_COLORS = 4;

/**
 * Stable color of a project by its position in the (ordered) project list:
 * `var(--project-1..4)`, cycling. Unknown / absent project => `var(--project-none)`.
 */
export function projectColor(
  id: string | null | undefined,
  orderedIds: readonly string[],
): string {
  if (id == null) return "var(--project-none)";
  const i = orderedIds.indexOf(id);
  if (i < 0) return "var(--project-none)";
  return `var(--project-${(i % PROJECT_COLORS) + 1})`;
}
