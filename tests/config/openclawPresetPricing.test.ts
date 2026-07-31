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
  // The cost object may carry extra fields after output (cacheRead/cacheWrite
  // since e356fc6e); do NOT anchor on the closing brace or those entries
  // silently drop out of the comparison and the test goes vacuous.
  const re =
    /id:\s*"([^"]+)"[\s\S]{0,220}?cost:\s*\{\s*input:\s*([\d.]+),\s*output:\s*([\d.]+)[\s\S]{0,120}?\}/g;
  const out = new Map<string, { input: number; output: number }>();
  let m: RegExpExecArray | null;
  while ((m = re.exec(src)) !== null) {
    // Runtime pricing lookup lowercases the incoming model id before hitting
    // the all-lowercase seed table, so mixed-case preset ids (KAT-Coder-Pro,
    // MiniMax-M2.7, ...) must be compared under the same normalization.
    const id = m[1].toLowerCase();
    if (!out.has(id))
      out.set(id, { input: Number(m[2]), output: Number(m[3]) });
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
    // Sentinels: these ids exist on both sides and MUST be compared. They pin
    // the parser against regressions like cacheRead-bearing cost objects or
    // mixed-case ids escaping the regex/lookup (the exact gaps that once hid
    // a 150x kat-coder-pro divergence).
    for (const sentinel of ["kat-coder-pro", "minimax-m2.7", "kimi-k3"]) {
      expect(presetCosts.has(sentinel), `preset parser lost ${sentinel}`).toBe(
        true,
      );
      expect(seedCosts.has(sentinel), `seed parser lost ${sentinel}`).toBe(
        true,
      );
    }

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
