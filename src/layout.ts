import type { RelativePosition } from "./types";

/**
 * Generate a sensible auto-layout grid for N apps.
 * Returns one RelativePosition per app (all values 0–1 fractions of the screen).
 */
export function autoLayout(count: number): RelativePosition[] {
  if (count === 0) return [];
  if (count === 1) return [{ x: 0, y: 0, w: 1, h: 1 }];
  if (count === 2) return [
    { x: 0,   y: 0, w: 0.5, h: 1 },
    { x: 0.5, y: 0, w: 0.5, h: 1 },
  ];
  if (count === 3) return [
    { x: 0,   y: 0,   w: 0.5, h: 1   },
    { x: 0.5, y: 0,   w: 0.5, h: 0.5 },
    { x: 0.5, y: 0.5, w: 0.5, h: 0.5 },
  ];
  if (count === 4) return [
    { x: 0,   y: 0,   w: 0.5, h: 0.5 },
    { x: 0.5, y: 0,   w: 0.5, h: 0.5 },
    { x: 0,   y: 0.5, w: 0.5, h: 0.5 },
    { x: 0.5, y: 0.5, w: 0.5, h: 0.5 },
  ];

  // 5+ apps: nearest-square grid
  const cols = Math.ceil(Math.sqrt(count));
  const rows = Math.ceil(count / cols);
  return Array.from({ length: count }, (_, i) => ({
    x: (i % cols) / cols,
    y: Math.floor(i / cols) / rows,
    w: 1 / cols,
    h: 1 / rows,
  }));
}
