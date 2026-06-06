import { describe, it, expect } from "vitest";
import { assembleSettingsConfig } from "@/components/providers/forms/helpers/assembleSettingsConfig";

/**
 * 简易 .env 解析器，行为对齐 useGeminiConfigState.envStringToObj，
 * 仅用于测试时把 geminiEnv 文本转成对象。
 */
function envStringToObj(value: string): Record<string, string> {
  const env: Record<string, string> = {};
  for (const line of value.split("\n")) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith("#")) continue;
    const i = trimmed.indexOf("=");
    if (i > 0) env[trimmed.slice(0, i).trim()] = trimmed.slice(i + 1).trim();
  }
  return env;
}

describe("assembleSettingsConfig (M40: validated === submitted)", () => {
  it("codex: rebuilds {auth, config} from hook state, ignoring the stale textarea", () => {
    const result = assembleSettingsConfig({
      appId: "codex",
      category: "custom",
      isAnyOmoCategory: false,
      name: "My Codex",
      // 旧实现会把已 zod 校验过的 textarea 丢弃，这里放一个明显错误的值证明没被使用
      textareaSettingsConfig: '{"stale":"must-not-be-submitted"}',
      codexAuth: '{"OPENAI_API_KEY":"sk-test"}',
      codexConfig: 'model = "gpt-5"',
    });

    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(JSON.parse(result.settingsConfig)).toEqual({
      auth: { OPENAI_API_KEY: "sk-test" },
      config: 'model = "gpt-5"',
    });
  });

  it("codex: invalid auth JSON is rejected (codexAuthInvalid), never silently saved", () => {
    const result = assembleSettingsConfig({
      appId: "codex",
      category: "custom",
      isAnyOmoCategory: false,
      name: "My Codex",
      textareaSettingsConfig: '{"stale":true}',
      codexAuth: "{not valid json",
      codexConfig: "",
    });

    expect(result).toEqual({ ok: false, error: "codexAuthInvalid" });
  });

  it("gemini: rebuilds {env, config} from hook state, ignoring the stale textarea", () => {
    const result = assembleSettingsConfig({
      appId: "gemini",
      category: "custom",
      isAnyOmoCategory: false,
      name: "My Gemini",
      textareaSettingsConfig: '{"stale":true}',
      geminiEnv: "GEMINI_API_KEY=k\nGOOGLE_GEMINI_BASE_URL=https://x",
      geminiConfig: '{"foo":"bar"}',
      envStringToObj,
    });

    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(JSON.parse(result.settingsConfig)).toEqual({
      env: { GEMINI_API_KEY: "k", GOOGLE_GEMINI_BASE_URL: "https://x" },
      config: { foo: "bar" },
    });
  });

  it("gemini: invalid config JSON is rejected (geminiConfigInvalid) instead of silently falling back to the textarea", () => {
    const result = assembleSettingsConfig({
      appId: "gemini",
      category: "custom",
      isAnyOmoCategory: false,
      name: "My Gemini",
      // 旧实现的 bug：gemini config 解析失败时静默回退到这个 textarea 值并保存
      textareaSettingsConfig: '{"would-be-silently-saved":true}',
      geminiEnv: "",
      geminiConfig: "{not json",
      envStringToObj,
    });

    expect(result).toEqual({ ok: false, error: "geminiConfigInvalid" });
  });

  it("omo: rebuilds config from omo draft state (agents/categories/otherFields), ignoring the textarea", () => {
    const result = assembleSettingsConfig({
      appId: "opencode",
      category: "omo",
      isAnyOmoCategory: true,
      name: "ignored-for-omo",
      textareaSettingsConfig: '{"stale":true}',
      omoAgents: { build: { model: "claude-opus-4-8" } },
      omoCategories: { reasoning: { model: "gpt-5" } },
      omoOtherFieldsStr: '{"extra":1}',
    });

    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(JSON.parse(result.settingsConfig)).toEqual({
      agents: { build: { model: "claude-opus-4-8" } },
      categories: { reasoning: { model: "gpt-5" } },
      otherFields: { extra: 1 },
    });
  });

  it("claude: uses the validated textarea and syncs the provider name into ui.displayName", () => {
    const result = assembleSettingsConfig({
      appId: "claude",
      category: "custom",
      isAnyOmoCategory: false,
      name: "  My Claude  ",
      textareaSettingsConfig: '{"env":{"ANTHROPIC_AUTH_TOKEN":"tok"}}',
    });

    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(JSON.parse(result.settingsConfig)).toEqual({
      env: { ANTHROPIC_AUTH_TOKEN: "tok" },
      ui: { displayName: "My Claude" },
    });
  });

  it("rejects a non-object assembled config via the final safety net (settingsConfigInvalid)", () => {
    const result = assembleSettingsConfig({
      appId: "claude",
      category: "custom",
      isAnyOmoCategory: false,
      name: "Whatever",
      textareaSettingsConfig: "[1, 2, 3]",
    });

    expect(result).toEqual({ ok: false, error: "settingsConfigInvalid" });
  });
});
