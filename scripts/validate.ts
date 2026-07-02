/**
 * Stash validate runner — `bun run validate`
 *
 * Runs all quality checks in sequence, collects failures, and prints a final
 * pass/fail summary.  Exits with code 1 if anything failed.
 *
 * Steps:
 *   1  ESLint          — code style + type-aware lint rules
 *   2  vue-tsc         — full TypeScript + Vue template type check
 *   3  Vitest          — unit tests for pure logic modules
 *   4  arch-check      — architecture drift (commands, imports, type exports)
 *   5  cargo clippy    — Rust lint (warnings treated as errors)
 *   6  cargo test      — Rust unit tests
 *   7  vite build      — frontend bundle (catches import errors, tree-shaking issues)
 */

import { execSync } from "child_process";
import { join } from "path";

const ROOT = join(import.meta.dirname, "..");
const RUST = join(ROOT, "src-tauri");

interface Step {
  label: string;
  cmd: string;
  cwd?: string;
}

const STEPS: Step[] = [
  { label: "Lint          ESLint",      cmd: "bunx eslint src --ext .ts,.vue",   cwd: ROOT },
  { label: "Type check    vue-tsc",     cmd: "bunx vue-tsc --noEmit",            cwd: ROOT },
  { label: "Unit tests    Vitest",      cmd: "bunx vitest run",                  cwd: ROOT },
  { label: "Arch check    custom",      cmd: "bun run scripts/arch-check.ts",    cwd: ROOT },
  { label: "Rust lint     clippy",      cmd: "cargo clippy -- -D warnings",      cwd: RUST  },
  { label: "Rust tests    cargo test",  cmd: "cargo test",                       cwd: RUST  },
  { label: "Build         vite",        cmd: "bunx vite build",                  cwd: ROOT  },
];

const TOTAL = STEPS.length;
const failures: string[] = [];
const timings: number[] = [];

const line = "─".repeat(52);

console.log(`\n${line}`);
console.log("  STASH — validate");
console.log(`${line}\n`);

for (let i = 0; i < STEPS.length; i++) {
  const step = STEPS[i];
  const label = `[${i + 1}/${TOTAL}] ${step.label}`;
  process.stdout.write(`▸ ${label}\n`);

  const t0 = Date.now();
  try {
    execSync(step.cmd, { cwd: step.cwd, stdio: "inherit" });
    const elapsed = ((Date.now() - t0) / 1000).toFixed(1);
    timings.push(Date.now() - t0);
    console.log(`  ✓ passed  (${elapsed}s)\n`);
  } catch {
    const elapsed = ((Date.now() - t0) / 1000).toFixed(1);
    timings.push(-(Date.now() - t0));
    console.error(`  ✗ FAILED  (${elapsed}s)\n`);
    failures.push(step.label.trim());
  }
}

console.log(line);

if (failures.length === 0) {
  const total = (timings.reduce((a, b) => a + Math.abs(b), 0) / 1000).toFixed(1);
  console.log(`\n✅  All ${TOTAL} checks passed  (${total}s total)\n`);
  process.exit(0);
} else {
  console.error(`\n❌  ${failures.length} of ${TOTAL} check(s) failed:\n`);
  for (const f of failures) console.error(`     • ${f}`);
  console.error(
    "\nFix the issues above and re-run:  bun run validate\n",
  );
  process.exit(1);
}
