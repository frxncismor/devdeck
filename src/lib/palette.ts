export interface PaletteCommand {
  id: string;
  label: string;
  /** Extra searchable words. */
  keywords?: string;
  group: "module" | "scope";
}

/** Case-insensitive AND match of every query word against label + keywords; keeps input order. */
export function filterCommands<T extends PaletteCommand>(
  commands: readonly T[],
  query: string,
): T[] {
  const words = query.toLowerCase().split(/\s+/).filter(Boolean);
  if (words.length === 0) return [...commands];
  return commands.filter((c) => {
    const hay = `${c.label} ${c.keywords ?? ""}`.toLowerCase();
    return words.every((w) => hay.includes(w));
  });
}
