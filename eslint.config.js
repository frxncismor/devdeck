import js from "@eslint/js";
import globals from "globals";
import svelte from "eslint-plugin-svelte";
import ts from "typescript-eslint";

export default ts.config(
  {
    ignores: [
      "dist/",
      "node_modules/",
      "target/",
      "src-tauri/",
      "crates/",
      ".tools/",
    ],
  },
  js.configs.recommended,
  ...ts.configs.recommended,
  ...svelte.configs["flat/recommended"],
  { languageOptions: { globals: { ...globals.browser } } },
  {
    files: ["**/*.svelte", "**/*.svelte.ts"],
    languageOptions: { parserOptions: { parser: ts.parser } },
  },
);
