import { readdirSync, readFileSync } from "fs";
import { join, relative } from "path";
import { describe, test, expect } from "vitest";
import knownViolations from "./known-violations.json";

/**
 * Architecture Boundary Test
 *
 * Validates that frontend source files only import from permitted layers.
 * Layer rules are defined in docs/architecture/LAYERS.md.
 *
 * Violation format:
 *   VIOLATION: {file}:{line} imports {target} — {layer} cannot import {target_layer}.
 *   See docs/architecture/LAYERS.md
 */

const LAYER_RULES: Record<string, string[]> = {
  types: [],
  hooks: ["types"],
  components: ["hooks", "types"],
  graph: ["components", "hooks", "types"],
  pet: ["components", "hooks", "types"],
};

const FROM_RE = /\bfrom\s+['"]([^'"]+)['"]/;

function getLayer(filePath: string): string | null {
  const match = filePath.match(/^src\/([^/]+)\//);
  if (match && match[1] in LAYER_RULES) return match[1];
  return null;
}

function resolveTargetLayer(importPath: string): string | null {
  // Only check internal relative imports (starting with ./ or ../)
  if (!importPath.startsWith(".")) return null;
  const normalized = importPath.replace(/^\.\.\//, "").replace(/^\.\//, "");
  const segments = normalized.split("/");
  for (const layer of Object.keys(LAYER_RULES)) {
    if (segments[0] === layer) return layer;
  }
  return null;
}

type Violation = {
  file: string;
  line: number;
  imports: string;
  from_layer: string;
  to_layer: string;
};

function scanFile(filePath: string): Violation[] {
  const violations: Violation[] = [];
  const content = readFileSync(filePath, "utf-8");
  const lines = content.split("\n");
  const fromLayer = getLayer(relative(process.cwd(), filePath));
  if (!fromLayer) return violations;

  let inTypeImport = false;
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];

    // Track type-only imports (erased at compile time — not runtime deps)
    if (/^\s*import\s+type\s/.test(line)) inTypeImport = true;

    const match = line.match(FROM_RE);
    if (match && !inTypeImport) {
      const importPath = match[1];
      const targetLayer = resolveTargetLayer(importPath);
      if (
        targetLayer &&
        !LAYER_RULES[fromLayer].includes(targetLayer) &&
        targetLayer !== fromLayer
      ) {
        violations.push({
          file: relative(process.cwd(), filePath),
          line: i + 1,
          imports: importPath,
          from_layer: fromLayer,
          to_layer: targetLayer,
        });
      }
    }

    // Reset type-import tracking once the from-clause closes the statement
    if (inTypeImport && FROM_RE.test(line)) inTypeImport = false;
  }
  return violations;
}

function collectFiles(dir: string, ext: string[]): string[] {
  const results: string[] = [];
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const fullPath = join(dir, entry.name);
    if (entry.isDirectory()) {
      results.push(...collectFiles(fullPath, ext));
    } else if (ext.some((e) => entry.name.endsWith(e))) {
      results.push(fullPath);
    }
  }
  return results;
}

describe("Architecture Boundary Test", () => {
  const files = collectFiles("src", [".ts", ".tsx"]);
  const allViolations = files.flatMap(scanFile);

  test("no new architecture violations", () => {
    const knownSet = new Set(
      knownViolations.map((v) => `${v.file}:${v.imports}`)
    );
    const newViolations = allViolations.filter(
      (v) => !knownSet.has(`${v.file}:${v.imports}`)
    );

    if (newViolations.length > 0) {
      const msg = newViolations
        .map(
          (v) =>
            `VIOLATION: ${v.file}:${v.line} imports ${v.imports} — ${v.from_layer} cannot import ${v.to_layer}. See docs/architecture/LAYERS.md`
        )
        .join("\n");
      throw new Error(`New architecture violations found:\n${msg}`);
    }
  });

  test("violation count only shrinks (ratchet)", () => {
    expect(allViolations.length).toBeLessThanOrEqual(knownViolations.length);
  });
});
