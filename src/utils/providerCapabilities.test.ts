import { describe, expect, it } from "vitest";
import type { AppId } from "@/lib/api";
import type { Provider } from "@/types";
import {
  GROKBUILD_OFFICIAL_PROVIDER_ID,
  providerNeedsRouting,
} from "@/utils/providerCapabilities";

it("keeps the Grok Official seed id stable across UI flows", () => {
  expect(GROKBUILD_OFFICIAL_PROVIDER_ID).toBe("grokbuild-official");
});

function provider(overrides: Partial<Provider> = {}): Provider {
  return {
    id: "provider-1",
    name: "Test Provider",
    settingsConfig: {},
    category: "third_party",
    ...overrides,
  };
}

describe("providerNeedsRouting", () => {
  it("never routes explicit official providers", () => {
    const apps: AppId[] = ["claude", "codex", "grokbuild"];
    for (const appId of apps) {
      expect(
        providerNeedsRouting(
          appId,
          provider({
            category: "official",
            meta: { providerType: "xai_oauth" },
          }),
        ),
      ).toBe(false);
    }
  });

  it.each([
    ["claude", "github_copilot"],
    ["claude", "codex_oauth"],
    ["claude", "xai_oauth"],
    ["codex", "xai_oauth"],
    ["grokbuild", "xai_oauth"],
  ] as const)("routes managed OAuth for %s/%s", (appId, providerType) => {
    expect(
      providerNeedsRouting(
        appId,
        provider({
          meta: { providerType, apiFormat: "openai_responses" },
        }),
      ),
    ).toBe(true);
  });

  it("keeps managed OAuth routing-required when apiFormat is missing or edited", () => {
    expect(
      providerNeedsRouting(
        "claude",
        provider({ meta: { providerType: "codex_oauth" } }),
      ),
    ).toBe(true);
    expect(
      providerNeedsRouting(
        "claude",
        provider({
          meta: { providerType: "codex_oauth", apiFormat: "anthropic" },
        }),
      ),
    ).toBe(true);
  });

  it("uses the native wire format for non-OAuth providers", () => {
    expect(
      providerNeedsRouting(
        "claude",
        provider({ meta: { apiFormat: "anthropic" } }),
      ),
    ).toBe(false);
    expect(
      providerNeedsRouting(
        "claude",
        provider({ meta: { apiFormat: "openai_responses" } }),
      ),
    ).toBe(true);
    expect(
      providerNeedsRouting(
        "codex",
        provider({ meta: { apiFormat: "openai_responses" } }),
      ),
    ).toBe(false);
    expect(
      providerNeedsRouting(
        "codex",
        provider({ meta: { apiFormat: "openai_chat" } }),
      ),
    ).toBe(true);
    expect(
      providerNeedsRouting(
        "grokbuild",
        provider({ meta: { apiFormat: "openai_chat" } }),
      ),
    ).toBe(true);
  });

  it("routes full-URL providers but ignores unsupported app families", () => {
    expect(
      providerNeedsRouting("codex", provider({ meta: { isFullUrl: true } })),
    ).toBe(true);
    expect(
      providerNeedsRouting(
        "gemini",
        provider({ meta: { providerType: "xai_oauth", isFullUrl: true } }),
      ),
    ).toBe(false);
  });
});
