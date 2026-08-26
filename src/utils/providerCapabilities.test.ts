import { describe, expect, it } from "vitest";
import type { AppId } from "@/lib/api";
import type { Provider } from "@/types";
import {
  CODEX_OFFICIAL_PROVIDER_ID,
  GROKBUILD_OFFICIAL_PROVIDER_ID,
  isOfficialBlockedByTakeover,
  providerNeedsRouting,
  resolveCodexOfficialIdentity,
  supportsOfficialProxyTakeover,
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

function codexOfficialCard(overrides: Partial<Provider> = {}): Provider {
  return provider({
    id: CODEX_OFFICIAL_PROVIDER_ID,
    name: "OpenAI Official",
    category: "official",
    settingsConfig: { auth: {}, config: "" },
    ...overrides,
  });
}

function managedBinding(accountId = "acct-managed"): Provider["meta"] {
  return {
    providerType: "codex_oauth",
    authBinding: {
      source: "managed_account",
      authProvider: "codex_oauth",
      accountId,
    },
  };
}

describe("resolveCodexOfficialIdentity", () => {
  it("classifies the fixed Official card by whether an account is bound", () => {
    expect(resolveCodexOfficialIdentity("codex", codexOfficialCard())).toBe(
      "native_login",
    );
    expect(
      resolveCodexOfficialIdentity(
        "codex",
        codexOfficialCard({ meta: managedBinding() }),
      ),
    ).toBe("managed_account");
  });

  it("classifies a UUID Official copy with a stored key as api_key", () => {
    expect(
      resolveCodexOfficialIdentity(
        "codex",
        codexOfficialCard({
          id: "generated-uuid",
          settingsConfig: { auth: { OPENAI_API_KEY: "sk-live" }, config: "" },
        }),
      ),
    ).toBe("api_key");
  });

  it("returns null for non-codex apps and third-party upstreams", () => {
    expect(
      resolveCodexOfficialIdentity("claude", codexOfficialCard()),
    ).toBeNull();
    expect(
      resolveCodexOfficialIdentity(
        "codex",
        codexOfficialCard({
          id: "generated-uuid",
          settingsConfig: { auth: {}, config: "", baseUrl: "https://x.test" },
        }),
      ),
    ).toBeNull();
  });
});

/**
 * Cross-layer contract: this predicate must stay in lockstep with Rust
 * `Provider::blocked_by_proxy_takeover` (src-tauri/src/provider.rs, pinned by
 * `blocked_by_proxy_takeover_opens_only_managed_codex_official_cards`).
 * The Rust side opens ONLY managed Codex Official cards; this fork has no
 * inbound Authorization passthrough, so unbound native-login cards carry no
 * server-side credential and must be refused at switch time (fail-closed).
 */
describe("supportsOfficialProxyTakeover", () => {
  it("opens managed Codex Official cards", () => {
    expect(
      supportsOfficialProxyTakeover(
        "codex",
        codexOfficialCard({ meta: managedBinding() }),
      ),
    ).toBe(true);
    expect(
      supportsOfficialProxyTakeover(
        "codex",
        codexOfficialCard({ id: "generated-uuid", meta: managedBinding() }),
      ),
    ).toBe(true);
  });

  it("keeps the unbound native-login Official card closed", () => {
    expect(supportsOfficialProxyTakeover("codex", codexOfficialCard())).toBe(
      false,
    );
  });

  it("keeps api-key and non-codex Official cards closed", () => {
    expect(
      supportsOfficialProxyTakeover(
        "codex",
        codexOfficialCard({
          id: "generated-uuid",
          settingsConfig: { auth: { OPENAI_API_KEY: "sk-live" }, config: "" },
        }),
      ),
    ).toBe(false);
    expect(
      supportsOfficialProxyTakeover(
        "claude",
        codexOfficialCard({ id: "claude-official", meta: managedBinding() }),
      ),
    ).toBe(false);
  });

  it("ignores an empty-string account binding", () => {
    expect(
      supportsOfficialProxyTakeover(
        "codex",
        codexOfficialCard({ meta: managedBinding("") }),
      ),
    ).toBe(false);
  });
});

describe("isOfficialBlockedByTakeover", () => {
  it("only blocks while takeover is active", () => {
    expect(
      isOfficialBlockedByTakeover("codex", codexOfficialCard(), false),
    ).toBe(false);
    expect(
      isOfficialBlockedByTakeover("codex", codexOfficialCard(), undefined),
    ).toBe(false);
    expect(
      isOfficialBlockedByTakeover("codex", codexOfficialCard(), true),
    ).toBe(true);
  });

  it("opens managed Official cards and keeps other Official cards blocked", () => {
    expect(
      isOfficialBlockedByTakeover(
        "codex",
        codexOfficialCard({ meta: managedBinding() }),
        true,
      ),
    ).toBe(false);
    expect(
      isOfficialBlockedByTakeover(
        "claude",
        codexOfficialCard({ id: "claude-official" }),
        true,
      ),
    ).toBe(true);
  });

  it("never blocks a non-official provider", () => {
    expect(
      isOfficialBlockedByTakeover(
        "codex",
        codexOfficialCard({ id: "third-party", category: "third_party" }),
        true,
      ),
    ).toBe(false);
  });
});
