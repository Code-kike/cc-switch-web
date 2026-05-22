import { describe, expect, it } from "vitest";

import { providerPresets } from "@/config/claudeProviderPresets";
import { codexProviderPresets } from "@/config/codexProviderPresets";
import { geminiProviderPresets } from "@/config/geminiProviderPresets";
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

const findPreset = <T extends { name: string }>(
  presets: T[],
  name: string,
): T => {
  const preset = presets.find((item) => item.name === name);
  expect(preset).toBeDefined();
  return preset as T;
};

const envOf = (preset: { settingsConfig: unknown }) =>
  (preset.settingsConfig as { env: Record<string, string> }).env;

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

  it("keeps OpenClaw ClaudeCN and RunAPI before OpenRouter", () => {
    expectInOrder(namesOf(openclawProviderPresets), [
      "ClaudeCN",
      "RunAPI",
      "OpenRouter",
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

  it("does not expose the removed DDSHub partner preset", () => {
    expect(namesOf(providerPresets)).not.toContain("DDSHub");
    expect(namesOf(codexProviderPresets)).not.toContain("DDSHub");
    expect(namesOf(hermesProviderPresets)).not.toContain("DDSHub");
  });

  it("uses current endpoints for migrated partner presets", () => {
    expect(
      envOf(findPreset(providerPresets, "CrazyRouter")).ANTHROPIC_BASE_URL,
    ).toBe("https://cn.crazyrouter.com");
    expect(findPreset(geminiProviderPresets, "CrazyRouter").baseURL).toBe(
      "https://cn.crazyrouter.com",
    );
    expect(envOf(findPreset(providerPresets, "Micu")).ANTHROPIC_BASE_URL).toBe(
      "https://www.micuapi.ai",
    );
    expect(
      envOf(findPreset(providerPresets, "Compshare Coding Plan"))
        .ANTHROPIC_BASE_URL,
    ).toBe("https://cp.compshare.cn");
    expect(envOf(findPreset(providerPresets, "GitHub Copilot"))).toMatchObject({
      ANTHROPIC_MODEL: "claude-sonnet-4.6",
      ANTHROPIC_DEFAULT_OPUS_MODEL: "claude-sonnet-4.6",
    });
  });
});
