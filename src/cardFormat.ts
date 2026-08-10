/**
 * Presentation helpers shared by the workspace and template cards.
 *
 * Tag colouring and the two date formatters were copy-pasted identically into
 * GroupCard.vue and TemplateCard.vue; the freeze label/tooltip were built
 * inline in both templates.
 */

import type { HardFreeze } from "./types";

export interface TagColor {
  bg: string;
  border: string;
  text: string;
}

const TAG_COLORS: TagColor[] = [
  { bg: "rgba(99,102,241,0.15)", border: "rgba(99,102,241,0.4)", text: "#818cf8" },
  { bg: "rgba(16,185,129,0.12)", border: "rgba(16,185,129,0.4)", text: "#34d399" },
  { bg: "rgba(245,158,11,0.12)", border: "rgba(245,158,11,0.4)", text: "#fbbf24" },
  { bg: "rgba(239,68,68,0.12)", border: "rgba(239,68,68,0.4)", text: "#f87171" },
  { bg: "rgba(6,182,212,0.12)", border: "rgba(6,182,212,0.4)", text: "#22d3ee" },
  { bg: "rgba(168,85,247,0.12)", border: "rgba(168,85,247,0.4)", text: "#c084fc" },
];

/** Stable colour for a tag — same tag always lands on the same swatch. */
export function tagColor(tag: string): TagColor {
  let hash = 0;
  for (const ch of tag) hash = (hash * 31 + ch.charCodeAt(0)) | 0;
  return TAG_COLORS[Math.abs(hash) % TAG_COLORS.length];
}

/** Absolute creation date, in the viewer's locale. */
export function formatDate(iso: string): string {
  try {
    return new Intl.DateTimeFormat(undefined, {
      month: "short", day: "numeric", year: "numeric",
      hour: "2-digit", minute: "2-digit",
    }).format(new Date(iso));
  } catch {
    return iso;
  }
}

/** Coarse "time since" for the last-used line: `just now`, `12m ago`, `3d ago`. */
export function formatLastUsed(iso: string): string {
  try {
    const diff = Date.now() - new Date(iso).getTime();
    const mins = Math.floor(diff / 60000);
    if (mins < 1) return "just now";
    if (mins < 60) return `${mins}m ago`;
    const hrs = Math.floor(mins / 60);
    if (hrs < 24) return `${hrs}h ago`;
    const days = Math.floor(hrs / 24);
    return `${days}d ago`;
  } catch {
    return "";
  }
}

/**
 * Menu label for the Hard Freeze entry.  The schedule rides along in the label
 * because the menu is now the only place that state is visible.
 */
export function freezeLabel(freeze?: HardFreeze): string {
  if (!freeze) return "Freeze";
  return freeze.enabled ? `Freeze · ${freeze.time}` : "Freeze (off)";
}

/** Tooltip for the Hard Freeze entry. */
export function freezeTitle(freeze?: HardFreeze): string {
  if (!freeze) return "Set Hard Freeze";
  return freeze.enabled ? `Hard Freeze at ${freeze.time}` : "Hard Freeze (disabled)";
}
