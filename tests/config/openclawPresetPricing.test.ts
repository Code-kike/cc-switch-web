import { describe, expect, it } from "vitest";
import fs from "node:fs";
import path from "node:path";

// L19: the OpenClaw provider presets embed a per-model `cost` that is written
// into the OpenClaw config and used by OpenClaw's own cost accounting, while
// cc-switch's usage dashboard prices the same models from the Rust seed table
// (src-tauri/src/database/schema.rs). If the two diverge, a user sees two
// conflicting cost numbers for identical traffic. This test enforces that every
// preset model whose id also exists in the seed shares the seed's input/output
// price, so the seed stays the single source of truth for pricing.

const repoRoot = path.resolve(__dirname, "../..");

function parsePresetCosts(): Map<string, { input: number; output: number }> {
  const src = fs.readFileSync(
    path.join(repoRoot, "src/config/openclawProviderPresets.ts"),
    "utf8",
  );
  const re =
    /id:\s*"([^"]+)"[\s\S]{0,220}?cost:\s*\{\s*input:\s*([\d.]+),\s*output:\s*([\d.]+)\s*\}/g;
  const out = new Map<string, { input: number; output: number }>();
  let m: RegExpExecArray | null;
  while ((m = re.exec(src)) !== null) {
    if (!out.has(m[1]))
      out.set(m[1], { input: Number(m[2]), output: Number(m[3]) });
  }
  return out;
}

function parseSeedCosts(): Map<string, { input: number; output: number }> {
  const src = fs.readFileSync(
    path.join(repoRoot, "src-tauri/src/database/schema.rs"),
    "utf8",
  );
  // Seed tuples: ("model-id", "Display Name", "input", "output", ...)
  const re =
    /\(\s*"([a-z0-9.\-]+)"\s*,\s*"[^"]*"\s*,\s*"([\d.]+)"\s*,\s*"([\d.]+)"/g;
  const out = new Map<string, { input: number; output: number }>();
  let m: RegExpExecArray | null;
  while ((m = re.exec(src)) !== null) {
    if (!out.has(m[1]))
      out.set(m[1], { input: Number(m[2]), output: Number(m[3]) });
  }
  return out;
}

describe("OpenClaw preset pricing consistency with the Rust seed (L19)", () => {
  it("matches seed input/output cost for every shared model id", () => {
    const presetCosts = parsePresetCosts();
    const seedCosts = parseSeedCosts();
    // Sanity: both parsers found data (guards against a format change silently
    // making this test vacuous).
    expect(presetCosts.size).toBeGreaterThan(0);
    expect(seedCosts.size).toBeGreaterThan(0);

    const mismatches: string[] = [];
    for (const [id, preset] of presetCosts) {
      const seed = seedCosts.get(id);
      if (!seed) continue; // preset-only models have no seed price to compare
      if (preset.input !== seed.input || preset.output !== seed.output) {
        mismatches.push(
          `${id}: preset ${preset.input}/${preset.output} vs seed ${seed.input}/${seed.output}`,
        );
      }
    }
    expect(mismatches).toEqual([]);
  });
});
