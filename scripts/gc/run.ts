#!/usr/bin/env node
/**
 * Garbage Collection — Entropy Scanner
 *
 * Scans for two types of entropy:
 *   1. Doc drift — docs/ files that haven't been updated since src/ changed significantly
 *   2. Architecture violations — cross-layer imports in src/
 *
 * Report-only: never auto-fixes. Exit code 1 if issues found (used by CI).
 */

import { execSync } from "child_process";
import { readdirSync, readFileSync, statSync } from "fs";
import { join, relative } from "path";

const ROOT = new URL("../..", import.meta.url).pathname;

let issueCount = 0;

function log(msg: string) {
  console.log(msg);
}

function warn(msg: string) {
  console.warn(`  [ISSUE] ${msg}`);
  issueCount++;
}

// ── 1. Doc Drift Detection ──────────────────────────────────────────────────

log("\n=== Doc Drift Check ===");

function getLastGitModified(path: string): Date | null {
  try {
    const out = execSync(
      `git log -1 --format=%ci -- "${path}"`,
      { cwd: ROOT, encoding: "utf-8" }
    ).trim();
    return out ? new Date(out) : null;
  } catch {
    return null;
  }
}

function getLastSrcModified(srcDir: string): Date | null {
  try {
    const out = execSync(
      `git log -1 --format=%ci -- "${srcDir}"`,
      { cwd: ROOT, encoding: "utf-8" }
    ).trim();
    return out ? new Date(out) : null;
  } catch {
    return null;
  }
}

const DOC_SRC_PAIRS: Array<{ doc: string; src: string; label: string }> = [
  {
    doc: "docs/architecture/LAYERS.md",
    src: "src-tauri/src",
    label: "LAYERS.md vs backend src/",
  },
  {
    doc: "docs/architecture/LAYERS.md",
    src: "src",
    label: "LAYERS.md vs frontend src/",
  },
  { doc: "docs/SECURITY.md", src: "src-tauri/src", label: "SECURITY.md vs backend" },
  { doc: "AGENTS.md", src: "src", label: "AGENTS.md vs frontend src/" },
];

const DRIFT_THRESHOLD_DAYS = 30;

for (const { doc, src, label } of DOC_SRC_PAIRS) {
  const docModified = getLastGitModified(doc);
  const srcModified = getLastSrcModified(src);

  if (!docModified || !srcModified) {
    log(`  [SKIP] ${label} — git history unavailable`);
    continue;
  }

  const diffDays =
    (srcModified.getTime() - docModified.getTime()) / (1000 * 60 * 60 * 24);

  if (diffDays > DRIFT_THRESHOLD_DAYS) {
    warn(
      `${doc} not updated in ${Math.round(diffDays)} days since ${src} changed — possible doc drift`
    );
  } else {
    log(`  [OK] ${label} (${Math.round(Math.max(0, diffDays))}d drift)`);
  }
}

// ── 2. Architecture Violation Scan ─────────────────────────────────────────

log("\n=== Architecture Boundary Scan ===");

const LAYER_RULES: Record<string, string[]> = {
  types: [],
  hooks: ["types"],
  components: ["hooks", "types"],
};

const FROM_RE = /\bfrom\s+['"]([^'"]+)['"]/;

function getLayer(filePath: string): string | null {
  const match = filePath.match(/^src\/([^/]+)\//);
  if (match && match[1] in LAYER_RULES) return match[1];
  return null;
}

function resolveTargetLayer(importPath: string): string | null {
  if (!importPath.startsWith(".")) return null;
  const normalized = importPath.replace(/^\.\.\//, "").replace(/^\.\//, "");
  const segments = normalized.split("/");
  for (const layer of Object.keys(LAYER_RULES)) {
    if (segments[0] === layer) return layer;
  }
  return null;
}

function collectFiles(dir: string, exts: string[]): string[] {
  const results: string[] = [];
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const fullPath = join(dir, entry.name);
    if (entry.isDirectory() && entry.name !== "node_modules") {
      results.push(...collectFiles(fullPath, exts));
    } else if (exts.some((e) => entry.name.endsWith(e))) {
      results.push(fullPath);
    }
  }
  return results;
}

const srcDir = join(ROOT, "src");
const files = collectFiles(srcDir, [".ts", ".tsx"]);
let violationCount = 0;

for (const filePath of files) {
  const relPath = relative(ROOT, filePath);
  const fromLayer = getLayer(relPath);
  if (!fromLayer) continue;

  const lines = readFileSync(filePath, "utf-8").split("\n");
  let inTypeImport = false;

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    if (/^\s*import\s+type\s/.test(line)) inTypeImport = true;
    const match = line.match(FROM_RE);
    if (match && !inTypeImport) {
      const targetLayer = resolveTargetLayer(match[1]);
      if (
        targetLayer &&
        !LAYER_RULES[fromLayer].includes(targetLayer) &&
        targetLayer !== fromLayer
      ) {
        warn(
          `VIOLATION: ${relPath}:${i + 1} imports ${match[1]} — ${fromLayer} cannot import ${targetLayer}. See docs/architecture/LAYERS.md`
        );
        violationCount++;
      }
    }
    if (inTypeImport && FROM_RE.test(line)) inTypeImport = false;
  }
}

if (violationCount === 0) {
  log("  [OK] No architecture violations found");
}

// ── Summary ─────────────────────────────────────────────────────────────────

log("\n=== GC Summary ===");
if (issueCount === 0) {
  log("No entropy detected.");
  process.exit(0);
} else {
  log(`${issueCount} issue(s) found. Review the output above.`);
  process.exit(1);
}
