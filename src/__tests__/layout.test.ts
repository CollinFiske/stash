import { describe, it, expect } from "vitest";
import { autoLayout } from "../layout";

describe("autoLayout", () => {
  it("returns empty array for 0 apps", () => {
    expect(autoLayout(0)).toHaveLength(0);
  });

  it("full screen for 1 app", () => {
    const [pos] = autoLayout(1);
    expect(pos).toEqual({ x: 0, y: 0, w: 1, h: 1 });
  });

  it("two equal columns for 2 apps", () => {
    const [a, b] = autoLayout(2);
    expect(a.x).toBe(0);
    expect(a.w).toBeCloseTo(0.5);
    expect(b.x).toBeCloseTo(0.5);
    expect(b.w).toBeCloseTo(0.5);
    expect(a.h).toBe(1);
    expect(b.h).toBe(1);
  });

  it("left half + right two quarters for 3 apps", () => {
    const [a, b, c] = autoLayout(3);
    expect(a.x).toBe(0);
    expect(a.w).toBeCloseTo(0.5);
    expect(a.h).toBe(1);
    expect(b.x).toBeCloseTo(0.5);
    expect(b.h).toBeCloseTo(0.5);
    expect(c.y).toBeCloseTo(0.5);
  });

  it("2×2 grid for 4 apps", () => {
    const positions = autoLayout(4);
    expect(positions).toHaveLength(4);
    for (const p of positions) {
      expect(p.w).toBeCloseTo(0.5);
      expect(p.h).toBeCloseTo(0.5);
    }
  });

  it("all positions stay within [0,1] bounds for counts 1–12", () => {
    for (let n = 1; n <= 12; n++) {
      const positions = autoLayout(n);
      expect(positions).toHaveLength(n);
      for (const p of positions) {
        expect(p.x).toBeGreaterThanOrEqual(0);
        expect(p.y).toBeGreaterThanOrEqual(0);
        expect(p.x + p.w).toBeLessThanOrEqual(1 + 1e-9);
        expect(p.y + p.h).toBeLessThanOrEqual(1 + 1e-9);
      }
    }
  });

  it("6 apps produces a 3×2 grid (3 cols, 2 rows)", () => {
    const positions = autoLayout(6);
    expect(positions).toHaveLength(6);
    expect(positions[0].w).toBeCloseTo(1 / 3);
    expect(positions[0].h).toBeCloseTo(0.5);
  });

  it("9 apps produces a 3×3 grid", () => {
    const positions = autoLayout(9);
    expect(positions).toHaveLength(9);
    expect(positions[0].w).toBeCloseTo(1 / 3);
    expect(positions[0].h).toBeCloseTo(1 / 3);
  });

  it("returns distinct positions for each app slot", () => {
    const positions = autoLayout(4);
    const keys = new Set(positions.map((p) => `${p.x},${p.y}`));
    expect(keys.size).toBe(4);
  });
});
