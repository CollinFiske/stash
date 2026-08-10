import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import { readFileSync } from "fs";

const host = process.env.TAURI_DEV_HOST;

// package.json is the single source of truth for the version. The About page
// used to hardcode its own string, which silently drifted out of sync with the
// three files that actually define the release number.
const pkg = JSON.parse(
  readFileSync(new URL("./package.json", import.meta.url), "utf-8")
) as { version: string };

export default defineConfig({
  plugins: [vue()],
  define: {
    __APP_VERSION__: JSON.stringify(pkg.version),
  },
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
  envPrefix: ["VITE_", "TAURI_ENV_"],
  build: {
    target:
      process.env.TAURI_ENV_PLATFORM === "windows"
        ? "chrome105"
        : process.env.TAURI_ENV_PLATFORM === "macos"
          ? "safari13"
          : "firefox115",
    minify: process.env.TAURI_ENV_DEBUG ? false : "esbuild",
    sourcemap: !!process.env.TAURI_ENV_DEBUG,
  },
});
