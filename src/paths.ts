/**
 * Path, executable and URL helpers shared across the UI.
 *
 * These were previously copy-pasted into App.vue, CommandPalette.vue,
 * GroupCard.vue and TemplateCard.vue in four slightly different forms.
 */

import type { BrowserWindowState } from "./types";

/** Last segment of a path, with any trailing separators ignored. */
export function baseName(path: string): string {
  const parts = path.replace(/\\/g, "/").replace(/\/+$/, "").split("/");
  return parts[parts.length - 1] || path;
}

/** Lower-cased executable name without its extension: `C:\…\Code.exe` → `code`. */
export function exeBase(path: string): string {
  return baseName(path).toLowerCase().replace(/\.exe$/i, "");
}

/** Hostname of a URL, or the original string when it will not parse. */
export function hostOf(url: string): string {
  try {
    return new URL(url).hostname;
  } catch {
    return url;
  }
}

/** `"12 tabs"` / `"1 tab"` for a captured browser window. */
export function tabSummary(win: BrowserWindowState): string {
  const n = win.tabs.length;
  return `${n} tab${n !== 1 ? "s" : ""}`;
}

/** Named tab groups in a window, ignoring untitled ones. */
export function groupTitles(win: BrowserWindowState): string[] {
  return win.groups.map((g) => g.title).filter((t) => t.trim().length > 0);
}
