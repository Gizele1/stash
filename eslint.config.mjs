import js from "@eslint/js";
import tsPlugin from "@typescript-eslint/eslint-plugin";
import tsParser from "@typescript-eslint/parser";
import globals from "globals";

/** @type {import("eslint").Linter.FlatConfig[]} */
export default [
  // Base JS rules
  js.configs.recommended,

  // TypeScript rules for all src/ files (browser environment)
  {
    files: ["src/**/*.{ts,tsx}"],
    languageOptions: {
      parser: tsParser,
      parserOptions: {
        ecmaVersion: "latest",
        sourceType: "module",
      },
      globals: {
        ...globals.browser,
        ...globals.es2021,
      },
    },
    plugins: {
      "@typescript-eslint": tsPlugin,
    },
    rules: {
      ...tsPlugin.configs.recommended.rules,
      // TypeScript handles undefined-variable detection better than ESLint
      "no-undef": "off",
    },
  },

  // Layer boundary: hooks/ cannot import from components/
  {
    files: ["src/hooks/**/*.{ts,tsx}"],
    rules: {
      "no-restricted-imports": [
        "error",
        {
          patterns: [
            {
              group: ["**/components/**", "../components", "../components/**"],
              message:
                "hooks/ cannot import from components/. See docs/architecture/LAYERS.md",
            },
          ],
        },
      ],
    },
  },

  // Layer boundary: types/ cannot import from hooks/ or components/
  {
    files: ["src/types/**/*.{ts,tsx}"],
    rules: {
      "no-restricted-imports": [
        "error",
        {
          patterns: [
            {
              group: ["**/hooks/**", "**/components/**"],
              message:
                "types/ is a leaf layer — cannot import from hooks/ or components/. See docs/architecture/LAYERS.md",
            },
          ],
        },
      ],
    },
  },

  // Ignore build output and backend
  {
    ignores: ["dist/**", "node_modules/**", "src-tauri/**"],
  },
];
