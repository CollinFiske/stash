/// <reference types="vite/client" />

// Brings in Vite's ambient module declarations — notably `*?raw`, which the
// extension tests use to load `extension/background.js` as a string without
// pulling Node's `fs`/`path` types into the app's type environment.

/** Injected by Vite from package.json — see `define` in vite.config.ts. */
declare const __APP_VERSION__: string;
