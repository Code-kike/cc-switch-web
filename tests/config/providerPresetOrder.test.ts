import { describe, expect, it } from "vitest";

import { providerPresets } from "@/config/claudeProviderPresets";
import { codexProviderPresets } from "@/config/codexProviderPresets";
import { hermesProviderPresets } from "@/config/hermesProviderPresets";
import { openclawProviderPresets } from "@/config/openclawProviderPresets";
import { opencodeProviderPresets } from "@/config/opencodeProviderPresets";

const namesOf = (presets: Array<{ name: string }>) =>
  presets.map((preset) => preset.name);

const expectInOrder = (names: string[], expected: string[]) => {
  const indexes = expected.map((name) => names.indexOf(name));

  expect(indexes).not.toContain(-1);
  expect(indexes).toEqual(expected.map((_, index) => indexes[0] + index));
};

describe("provider preset order", () => {
  it("prioritizes Claude partner presets", () => {
    expectInOrder(namesOf(providerPresets), [
      "Shengsuanyun",
      "PatewayAI",
      "火山Agentplan",
      "BytePlus",
      "DouBaoSeed",
    ]);
  });

  it("places PatewayAI after Shengsuanyun for Codex", () => {
    expectInOrder(namesOf(codexProviderPresets), ["Shengsuanyun", "PatewayAI"]);
  });

  it("prioritizes OpenCode partner presets", () => {
    expectInOrder(namesOf(opencodeProviderPresets), [
      "Shengsuanyun",
      "火山Agentplan",
      "BytePlus",
      "DouBaoSeed",
    ]);
  });

  it("prioritizes OpenClaw partner presets", () => {
    expectInOrder(namesOf(openclawProviderPresets), [
      "Shengsuanyun",
      "火山Agentplan",
      "BytePlus",
      "DouBaoSeed",
    ]);
  });

  it("prioritizes Hermes partner presets", () => {
    expectInOrder(namesOf(hermesProviderPresets), [
      "Shengsuanyun",
      "火山Agentplan",
      "BytePlus",
      "DouBaoSeed",
    ]);
  });
});
