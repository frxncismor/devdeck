# Design handoff

Versioned copy of the DevDeck design handoff.

- `DEVDECK_HANDOFF.md` — implementation summary (layout, scope, components, modules, a11y).
- `tokens.css` — original design tokens. The app uses `src/styles/tokens.css` (same values).
- `canvas/` — design-canvas boards as plain HTML (`*.dc.html`, inline styles inside `<x-dc>`) plus `canvas.json`.
  They are **reference only**: the `support.js` runtime is intentionally missing, so read the markup/CSS
  rather than expecting them to run. They are not part of the build (excluded from Prettier).
