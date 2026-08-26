import { describe, expect, it } from "vitest";
import { providerPresets } from "@/config/claudeProviderPresets";
import { opencodeProviderPresets } from "@/config/opencodeProviderPresets";
import { hermesProviderPresets } from "@/config/hermesProviderPresets";
import {
  openclawProviderPresets,
  rebaseOpenClawSuggestedDefaults,
} from "@/config/openclawProviderPresets";

// 千帆 Token Plan 个人版（2026-07-13 起替代 Coding Plan 发售；存量 Coding
// Plan 可用至到期，旧预设保留并存）。Codex 侧口径由 codexChatProviderPresets
// 与 codexReasoningLevelPresets 两个测试锁定，此处覆盖其余四应用（本 fork 无
// claudeDesktopProviderPresets 表面，桌面用例已省略）。
const PRESET_NAME = "Baidu Qianfan Token Plan";
const OPENAI_BASE = "https://qianfan.baidubce.com/v2/tokenplan/personal";
const ANTHROPIC_BASE =
  "https://qianfan.baidubce.com/anthropic/tokenplan/personal";
// 阵容=Token Plan 主文档（2026-08-14 版）；ernie-5.1 官方标注 8/20 下线不收
const MODEL_IDS = [
  "deepseek-v4-pro",
  "deepseek-v4-flash",
  "deepseek-v4-flash-0731",
  "glm-5.2",
  "glm-5.1",
  "kimi-k2.6",
];

describe("Baidu Qianfan Token Plan presets", () => {
  it("Claude preset points every model role at deepseek-v4-pro", () => {
    const preset = providerPresets.find((item) => item.name === PRESET_NAME);
    expect(preset).toBeDefined();

    const env = (preset?.settingsConfig as { env: Record<string, string> }).env;
    expect(env.ANTHROPIC_BASE_URL).toBe(ANTHROPIC_BASE);
    // 官方 Claude Code 接入页（2026-07-30 版）全角色 deepseek-v4-pro
    for (const key of [
      "ANTHROPIC_MODEL",
      "ANTHROPIC_DEFAULT_HAIKU_MODEL",
      "ANTHROPIC_DEFAULT_SONNET_MODEL",
      "ANTHROPIC_DEFAULT_OPUS_MODEL",
    ]) {
      expect(env[key], key).toBe("deepseek-v4-pro");
    }
  });

  it("OpenCode preset carries the full Token Plan lineup", () => {
    const preset = opencodeProviderPresets.find(
      (item) => item.name === PRESET_NAME,
    );
    expect(preset).toBeDefined();
    expect(preset?.settingsConfig.npm).toBe("@ai-sdk/openai-compatible");
    expect(
      (preset?.settingsConfig.options as { baseURL: string }).baseURL,
    ).toBe(OPENAI_BASE);
    expect(Object.keys(preset?.settingsConfig.models ?? {})).toEqual(MODEL_IDS);
  });

  it("Hermes preset uses the OpenAI-compatible endpoint with v4-pro default", () => {
    const preset = hermesProviderPresets.find(
      (item) => item.name === PRESET_NAME,
    );
    expect(preset).toBeDefined();
    expect(preset?.settingsConfig.base_url).toBe(OPENAI_BASE);
    expect(preset?.settingsConfig.api_mode).toBe("chat_completions");
    expect(
      (preset?.settingsConfig.models ?? []).map((model) => model.id),
    ).toEqual(MODEL_IDS);
    expect(preset?.suggestedDefaults?.model).toEqual({
      default: "deepseek-v4-pro",
      provider: "qianfan_tokenplan",
    });
  });

  it("OpenClaw preset mirrors the official OpenClaw integration page", () => {
    const preset = openclawProviderPresets.find(
      (item) => item.name === PRESET_NAME,
    );
    expect(preset).toBeDefined();
    expect(preset?.settingsConfig.baseUrl).toBe(OPENAI_BASE);
    expect(preset?.settingsConfig.api).toBe("openai-completions");

    // 模型条目=官方 OpenClaw 接入页（2026-07-22 版）原样。窗口 98304 是官方
    // 钦定的 OpenClaw 口径，≠平台模型列表页 1M——勿按平台口径"修正"
    // id 为 provider-scoped：百度 Token Plan 转售档位的 cost 0.0025/0.01 与
    // DeepSeek 裸种子不等，裸 id 会触发 FE-preset↔Rust-seed 一致性门禁
    // （grilling #7），实际 API 模型名仍由 suggestedDefaults.model.primary
    // 后缀 deepseek-v4-pro 传入百度端点。
    const model = preset?.settingsConfig.models?.[0];
    expect(model?.id).toBe("qianfan-tokenplan/deepseek-v4-pro");
    expect(model?.name).toBe("deepseek-v4-pro");
    expect(model?.contextWindow).toBe(98304);
    expect(model?.maxTokens).toBe(65536);
    expect(model?.cost).toEqual({
      input: 0.0025,
      output: 0.01,
      cacheRead: 0,
      cacheWrite: 0,
    });
  });

  it("rebases OpenClaw defaults to the submitted provider key", () => {
    const preset = openclawProviderPresets.find(
      (item) => item.name === PRESET_NAME,
    );
    expect(preset?.suggestedDefaults).toBeDefined();

    const rebased = rebaseOpenClawSuggestedDefaults(
      preset!.suggestedDefaults!,
      "my-qianfan",
    );
    expect(rebased.model?.primary).toBe("my-qianfan/deepseek-v4-pro");
    expect(rebased.modelCatalog).toHaveProperty("my-qianfan/deepseek-v4-pro");
  });
});
