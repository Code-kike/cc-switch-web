import { describe, expect, it } from "vitest";

import { openclawProviderPresets } from "@/config/openclawProviderPresets";

// Runtime seed parity belongs to the real Web-server integration test, which
// reads the actual initialized database. This focused test intentionally checks
// only the imported TypeScript objects so source formatting cannot make the
// acceptance vacuous (the former regex parser skipped reordered cost fields).
describe("OpenClaw preset pricing objects", () => {
  it("contains only finite, non-negative tuples with non-zero input/output", () => {
    const pricedModels = openclawProviderPresets.flatMap((preset) =>
      (preset.settingsConfig.models ?? [])
        .filter((model) => model.cost !== undefined)
        .map((model) => ({
          preset: preset.name,
          modelId: model.id,
          cost: model.cost!,
        })),
    );

    expect(pricedModels.length).toBeGreaterThan(0);
    for (const { preset, modelId, cost } of pricedModels) {
      const label = `${preset}/${modelId}`;
      expect(Number.isFinite(cost.input), `${label} input must be finite`).toBe(
        true,
      );
      expect(
        Number.isFinite(cost.output),
        `${label} output must be finite`,
      ).toBe(true);
      expect(cost.input, `${label} input must be non-zero`).toBeGreaterThan(0);
      expect(cost.output, `${label} output must be non-zero`).toBeGreaterThan(
        0,
      );
      for (const [name, value] of [
        ["cacheRead", cost.cacheRead],
        ["cacheWrite", cost.cacheWrite],
      ] as const) {
        if (value === undefined) continue;
        expect(Number.isFinite(value), `${label} ${name} must be finite`).toBe(
          true,
        );
        expect(
          value,
          `${label} ${name} must be non-negative`,
        ).toBeGreaterThanOrEqual(0);
      }
    }
  });
});
