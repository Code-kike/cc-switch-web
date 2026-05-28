import { describe, expect, it } from "vitest";
import {
  extractCodexBaseUrl,
  extractCodexExperimentalBearerToken,
  extractCodexModelName,
  isCodexGoalModeEnabled,
  setCodexBaseUrl,
  setCodexGoalMode,
  setCodexModelName,
  updateCodexExperimentalBearerToken,
} from "@/utils/providerConfigUtils";

describe("Codex TOML utils", () => {
  it("removes base_url line when set to empty", () => {
    const input = [
      'model_provider = "openai"',
      'base_url = "https://api.example.com/v1"',
      'model = "gpt-5-codex"',
      "",
    ].join("\n");

    const output = setCodexBaseUrl(input, "");

    expect(output).not.toMatch(/^\s*base_url\s*=/m);
    expect(extractCodexBaseUrl(output)).toBeUndefined();
    expect(extractCodexModelName(output)).toBe("gpt-5-codex");
  });

  it("removes only the top-level model line when set to empty", () => {
    const input = [
      'model_provider = "openai"',
      'base_url = "https://api.example.com/v1"',
      'model = "gpt-5-codex"',
      "",
      "[profiles.default]",
      'model = "profile-model"',
      "",
    ].join("\n");

    const output = setCodexModelName(input, "");

    expect(output).not.toMatch(/^model\s*=\s*"gpt-5-codex"$/m);
    expect(output).toMatch(/^\[profiles\.default\]\nmodel = "profile-model"$/m);
    expect(extractCodexModelName(output)).toBeUndefined();
    expect(extractCodexBaseUrl(output)).toBe("https://api.example.com/v1");
  });

  it("updates existing values when non-empty", () => {
    const input = [
      'model_provider = "openai"',
      "base_url = 'https://old.example/v1'",
      'model = "old-model"',
      "",
    ].join("\n");

    const output1 = setCodexBaseUrl(input, " https://new.example/v1 \n");
    expect(extractCodexBaseUrl(output1)).toBe("https://new.example/v1");

    const output2 = setCodexModelName(output1, " new-model \n");
    expect(extractCodexModelName(output2)).toBe("new-model");
  });

  it("reads and writes base_url in the active provider section", () => {
    const input = [
      'model_provider = "custom"',
      'model = "gpt-5.4"',
      "",
      "[model_providers.custom]",
      'name = "custom"',
      'wire_api = "responses"',
      "",
      "[profiles.default]",
      'approval_policy = "never"',
      "",
    ].join("\n");

    const output = setCodexBaseUrl(input, "https://api.example.com/v1");

    expect(output).toContain(
      '[model_providers.custom]\nname = "custom"\nwire_api = "responses"\nbase_url = "https://api.example.com/v1"',
    );
    expect(extractCodexBaseUrl(output)).toBe("https://api.example.com/v1");
  });

  it("recovers a single misplaced base_url from another section", () => {
    const input = [
      'model_provider = "custom"',
      'model = "gpt-5.4"',
      "",
      "[model_providers.custom]",
      'name = "custom"',
      'wire_api = "responses"',
      "",
      "[profiles.default]",
      'approval_policy = "never"',
      'base_url = "https://wrong.example/v1"',
      "",
    ].join("\n");

    expect(extractCodexBaseUrl(input)).toBe("https://wrong.example/v1");

    const output = setCodexBaseUrl(input, "https://fixed.example/v1");

    expect(output).toContain(
      '[model_providers.custom]\nname = "custom"\nwire_api = "responses"\nbase_url = "https://fixed.example/v1"',
    );
    expect(output).not.toContain("https://wrong.example/v1");
    expect(output.match(/base_url\s*=/g)).toHaveLength(1);
  });

  it("does not treat mcp_servers base_url as provider base_url", () => {
    const input = [
      'model_provider = "azure"',
      'model = "gpt-4"',
      "",
      "[model_providers.azure]",
      'name = "Azure OpenAI"',
      'wire_api = "responses"',
      "",
      "[mcp_servers.my_server]",
      'base_url = "http://localhost:8080"',
      "",
    ].join("\n");

    expect(extractCodexBaseUrl(input)).toBeUndefined();

    const output = setCodexBaseUrl(input, "https://new.azure/v1");

    expect(output).toContain(
      '[model_providers.azure]\nname = "Azure OpenAI"\nwire_api = "responses"\nbase_url = "https://new.azure/v1"',
    );
    expect(output).toContain(
      '[mcp_servers.my_server]\nbase_url = "http://localhost:8080"',
    );
  });

  it("reads model only from the top-level config", () => {
    const input = [
      'model_provider = "custom"',
      "",
      "[profiles.default]",
      'model = "profile-model"',
      "",
    ].join("\n");

    expect(extractCodexModelName(input)).toBeUndefined();
  });

  it("handles single-quoted values", () => {
    const input = "base_url = 'https://api.example.com/v1'\nmodel = 'gpt-5'\n";

    expect(extractCodexBaseUrl(input)).toBe("https://api.example.com/v1");
    expect(extractCodexModelName(input)).toBe("gpt-5");
  });

  it("adds Goal mode under the top-level features table", () => {
    const input = [
      'model_provider = "custom"',
      'model = "gpt-5.4"',
      "",
      "[model_providers.custom]",
      'name = "custom"',
      "",
    ].join("\n");

    const output = setCodexGoalMode(input, true);

    expect(isCodexGoalModeEnabled(output)).toBe(true);
    expect(output).toContain(
      'model = "gpt-5.4"\n\n[features]\ngoals = true\n\n[model_providers.custom]',
    );
  });

  it("removes Goal mode without deleting other feature flags", () => {
    const input = [
      'model_provider = "custom"',
      "",
      "[features]",
      "goals = true",
      "experimental_resume = true",
      "",
      "[model_providers.custom]",
      'name = "custom"',
      "",
    ].join("\n");

    const output = setCodexGoalMode(input, false);

    expect(isCodexGoalModeEnabled(output)).toBe(false);
    expect(output).toContain("[features]\nexperimental_resume = true");
    expect(output).not.toMatch(/^\s*goals\s*=/m);
  });

  it("removes the features table when disabling the only Goal mode flag", () => {
    const input = [
      'model_provider = "custom"',
      "",
      "[features]",
      "goals = true",
      "",
      "[model_providers.custom]",
      'name = "custom"',
      "",
    ].join("\n");

    const output = setCodexGoalMode(input, false);

    expect(isCodexGoalModeEnabled(output)).toBe(false);
    expect(output).not.toContain("[features]");
    expect(output).toContain("[model_providers.custom]");
  });

  it("preserves feature-section comments when disabling Goal mode", () => {
    const input = [
      'model_provider = "custom"',
      "",
      "[features]",
      "# Keep this note",
      "goals = true",
      "",
      "[model_providers.custom]",
      'name = "custom"',
      "",
    ].join("\n");

    const output = setCodexGoalMode(input, false);

    expect(isCodexGoalModeEnabled(output)).toBe(false);
    expect(output).toContain("[features]\n# Keep this note");
    expect(output).not.toMatch(/^\s*goals\s*=/m);
  });

  it("extracts experimental_bearer_token from the active custom provider section", () => {
    const input = [
      'model_provider = "thirdparty"',
      "",
      "[model_providers.thirdparty]",
      'experimental_bearer_token = "sk-provider"',
      "",
    ].join("\n");

    expect(extractCodexExperimentalBearerToken(input)).toBe("sk-provider");
  });

  it("extractCodexExperimentalBearerToken ignores reserved provider tables", () => {
    const input = [
      'model_provider = "openai"',
      'experimental_bearer_token = "top-level-key"',
      "",
      "[model_providers.openai]",
      'experimental_bearer_token = "stale-table-key"',
      "",
    ].join("\n");

    expect(extractCodexExperimentalBearerToken(input)).toBe("top-level-key");
  });

  it("extractCodexExperimentalBearerToken reads only top-level model_provider", () => {
    const input = [
      'experimental_bearer_token = "top-level-key"',
      "",
      "[profiles.work]",
      'model_provider = "fake"',
      "",
      "[model_providers.fake]",
      'experimental_bearer_token = "wrong-key"',
      "",
    ].join("\n");

    expect(extractCodexExperimentalBearerToken(input)).toBe("top-level-key");
  });

  it("updateCodexExperimentalBearerToken leaves config without the token alone", () => {
    const input = [
      'model_provider = "openai"',
      'base_url = "https://api.example.com/v1"',
      "",
    ].join("\n");

    expect(updateCodexExperimentalBearerToken(input, "new-key")).toBe(input);
    expect(updateCodexExperimentalBearerToken(input, "")).toBe(input);
  });

  it("updateCodexExperimentalBearerToken removes the token line when set to empty", () => {
    const input = [
      'model_provider = "thirdparty"',
      "",
      "[model_providers.thirdparty]",
      'name = "Thirdparty"',
      'base_url = "https://thirdparty.example/v1"',
      'experimental_bearer_token = "old-key"',
      "requires_openai_auth = true",
      "",
    ].join("\n");

    const cleared = updateCodexExperimentalBearerToken(input, "");

    expect(extractCodexExperimentalBearerToken(cleared)).toBeUndefined();
    expect(cleared).toMatch(/requires_openai_auth = true/);
    expect(cleared).toMatch(/base_url = "https:\/\/thirdparty\.example\/v1"/);
  });

  it("updateCodexExperimentalBearerToken replaces the active provider token and keeps comments", () => {
    const input = [
      'model_provider = "thirdparty"',
      "",
      "[model_providers.thirdparty]",
      'experimental_bearer_token = "old-key" # vendor token',
      "",
    ].join("\n");

    const updated = updateCodexExperimentalBearerToken(input, 'abc"def\\ghi');

    expect(updated).toContain(
      'experimental_bearer_token = "abc\\"def\\\\ghi" # vendor token',
    );
    expect(extractCodexExperimentalBearerToken(updated)).toBe('abc"def\\ghi');
  });
});
