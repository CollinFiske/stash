import js from "@eslint/js";
import pluginVue from "eslint-plugin-vue";
import tseslint from "typescript-eslint";
import globals from "globals";

export default tseslint.config(
  // Ignore build artifacts and generated code
  {
    ignores: [
      "dist/",
      "src-tauri/target/",
      "node_modules/",
      "src-tauri/src/main.rs", // generated entry point
    ],
  },

  js.configs.recommended,
  ...tseslint.configs.recommended,
  ...pluginVue.configs["flat/recommended"],

  {
    files: ["**/*.ts", "**/*.vue"],
    languageOptions: {
      globals: {
        ...globals.browser,
        // Injected by Vite's `define` from package.json. TypeScript learns about
        // it from src/vite-env.d.ts, but ESLint's no-undef does not read .d.ts.
        __APP_VERSION__: "readonly",
      },
      parserOptions: {
        // Let vue-eslint-parser handle .vue files; use TS parser for <script>
        parser: tseslint.parser,
        extraFileExtensions: [".vue"],
      },
    },
    rules: {
      // ── TypeScript ────────────────────────────────────────────────────────
      "@typescript-eslint/no-explicit-any": "warn",
      "@typescript-eslint/no-unused-vars": [
        "error",
        { argsIgnorePattern: "^_", varsIgnorePattern: "^_", caughtErrorsIgnorePattern: "^_" },
      ],
      // Allow empty catch blocks (common in Tauri error-ignorance patterns)
      "no-empty": ["error", { allowEmptyCatch: true }],

      // ── Vue ───────────────────────────────────────────────────────────────
      // Single-word component names are fine for pages/modals in a desktop app
      "vue/multi-word-component-names": "off",
      // Props without defaults are fine when using TypeScript required types
      "vue/require-default-prop": "off",
      // We use self-closing on void elements only — don't force it elsewhere
      "vue/html-self-closing": "off",
      // Allow v-html (we use it for SVG icons; content is internal, not user input)
      "vue/no-v-html": "off",
    },
  },
);
