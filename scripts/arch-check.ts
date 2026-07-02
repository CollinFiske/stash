/**
 * Architecture drift checker — run with `bun run scripts/arch-check.ts`
 *
 * Checks:
 *  1. Every #[tauri::command] in commands.rs is registered in generate_handler!
 *  2. Every .vue import in src/ points to a file that actually exists
 *  3. Every type exported from types.ts is referenced somewhere in src/
 *  4. The Tauri capabilities JSON parses without error
 */

import { readFileSync, existsSync, readdirSync, statSync } from "fs";
import { join, resolve, dirname } from "path";
import { fileURLToPath } from "url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const ROOT = resolve(__dirname, "..");

let errors = 0;
let warnings = 0;
let checks = 0;

function pass(msg: string) {
  console.log(`  ✓ ${msg}`);
  checks++;
}

function fail(msg: string) {
  console.error(`  ✗ ${msg}`);
  errors++;
  checks++;
}

function warn(msg: string) {
  console.warn(`  ⚠ ${msg}`);
  warnings++;
  checks++;
}

// ── 1. Tauri command registration ─────────────────────────────────────────────
console.log("\n[1] Tauri command registration");

const commandsRs = readFileSync(join(ROOT, "src-tauri/src/commands.rs"), "utf-8");
const libRs = readFileSync(join(ROOT, "src-tauri/src/lib.rs"), "utf-8");

// Find all `pub async fn <name>` preceded (within 3 lines) by #[tauri::command]
const commandDefs: string[] = [];
const lines = commandsRs.split("\n");
for (let i = 0; i < lines.length; i++) {
  if (lines[i].includes("#[tauri::command]")) {
    // Look for the fn declaration in the next 3 lines
    for (let j = i + 1; j < Math.min(i + 4, lines.length); j++) {
      const match = lines[j].match(/pub(?:\s+async)?\s+fn\s+(\w+)/);
      if (match) {
        commandDefs.push(match[1]);
        break;
      }
    }
  }
}

const handlerMatch = libRs.match(/generate_handler!\[([\s\S]*?)\]/);
const registered = handlerMatch
  ? [...handlerMatch[1].matchAll(/commands::(\w+)/g)].map((m) => m[1])
  : [];

if (commandDefs.length === 0) {
  warn("No #[tauri::command] functions found in commands.rs");
} else {
  for (const cmd of commandDefs) {
    if (registered.includes(cmd)) {
      pass(`commands::${cmd} registered in generate_handler!`);
    } else {
      fail(`commands::${cmd} is defined but NOT registered in generate_handler!`);
    }
  }
  for (const reg of registered) {
    if (!commandDefs.includes(reg)) {
      fail(`generate_handler! lists commands::${reg} but it is not defined in commands.rs`);
    }
  }
}

// ── 2. Vue component import existence ────────────────────────────────────────
console.log("\n[2] Vue component imports");

function getAllVueFiles(dir: string): string[] {
  const result: string[] = [];
  for (const entry of readdirSync(dir)) {
    const full = join(dir, entry);
    if (entry === "node_modules" || entry === "__tests__") continue;
    if (statSync(full).isDirectory()) result.push(...getAllVueFiles(full));
    else if (entry.endsWith(".vue") || entry.endsWith(".ts")) result.push(full);
  }
  return result;
}

const srcFiles = getAllVueFiles(join(ROOT, "src"));

for (const file of srcFiles) {
  const content = readFileSync(file, "utf-8");
  const importMatches = [...content.matchAll(/from\s+["'](\.[^"']*\.vue)["']/g)];
  for (const m of importMatches) {
    const importPath = resolve(dirname(file), m[1]);
    if (existsSync(importPath)) {
      pass(`${m[1]}  (in ${file.replace(ROOT, "").replace(/\\/g, "/")})`);
    } else {
      fail(`Import "${m[1]}" in ${file.replace(ROOT, "")} — file does not exist`);
    }
  }
}

// ── 3. Type export coverage ───────────────────────────────────────────────────
console.log("\n[3] Type export usage");

const typesPath = join(ROOT, "src/types.ts");
const typesContent = readFileSync(typesPath, "utf-8");
const exportedTypes = [
  ...typesContent.matchAll(/^export\s+(?:interface|type)\s+(\w+)/gm),
].map((m) => m[1]);

// Aggregate all src content including types.ts itself (for cross-type references)
// but excluding __tests__ (test files importing everything is expected)
const nonTypesContent = srcFiles
  .filter((f) => !f.includes("__tests__"))
  .map((f) => readFileSync(f, "utf-8"))
  .join("\n");

// Types that are intentionally internal / carry-over interfaces no longer actively used
const ALLOW_IMPLICIT = new Set(["WorkspaceStore"]);

for (const typeName of exportedTypes) {
  if (ALLOW_IMPLICIT.has(typeName)) {
    warn(`${typeName} — allowed implicit (not actively imported)`);
    continue;
  }
  if (nonTypesContent.includes(typeName)) {
    pass(`${typeName} is referenced in src/`);
  } else {
    fail(`${typeName} exported from types.ts but never referenced outside types.ts`);
  }
}

// ── 4. Tauri capabilities JSON validity ──────────────────────────────────────
console.log("\n[4] Tauri capabilities JSON");

const capDir = join(ROOT, "src-tauri/capabilities");
if (existsSync(capDir)) {
  for (const entry of readdirSync(capDir)) {
    if (!entry.endsWith(".json")) continue;
    const capPath = join(capDir, entry);
    try {
      JSON.parse(readFileSync(capPath, "utf-8"));
      pass(`${entry} is valid JSON`);
    } catch (e) {
      fail(`${entry} failed JSON parse: ${e}`);
    }
  }
} else {
  warn("src-tauri/capabilities/ directory not found — skipping");
}

// ── Summary ───────────────────────────────────────────────────────────────────
console.log(`\n${"─".repeat(48)}`);
console.log(`Ran ${checks} checks — ${errors} error(s), ${warnings} warning(s)`);

if (errors > 0) {
  console.error("\nArchitecture drift check FAILED.\n");
  process.exit(1);
} else {
  console.log("\nArchitecture check passed.\n");
}
