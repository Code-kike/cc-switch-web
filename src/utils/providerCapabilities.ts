import type { AppId } from "@/lib/api";
import type { Provider } from "@/types";
import { isOAuthProviderType } from "@/config/constants";
import { resolveManagedAccountId } from "@/lib/authBinding";
import {
  extractCodexBaseUrl,
  extractCodexExperimentalBearerToken,
  hasExplicitNonOpenAiCodexModelProvider,
} from "@/utils/providerConfigUtils";

export const GROKBUILD_OFFICIAL_PROVIDER_ID = "grokbuild-official";
export const CODEX_OFFICIAL_PROVIDER_ID = "codex-official";

export type CodexOfficialIdentity =
  | "native_login"
  | "managed_account"
  | "api_key";

const nonEmptyString = (value: unknown): boolean =>
  typeof value === "string" && value.trim().length > 0;

function hasExplicitCodexThirdPartyUpstream(
  settings: Record<string, unknown>,
): boolean {
  const config = typeof settings.config === "string" ? settings.config : "";

  return (
    nonEmptyString(settings.baseUrl) ||
    nonEmptyString(settings.baseURL) ||
    nonEmptyString(settings.base_url) ||
    Boolean(extractCodexExperimentalBearerToken(config)) ||
    Boolean(extractCodexBaseUrl(config)) ||
    hasExplicitNonOpenAiCodexModelProvider(config)
  );
}

function hasStoredCodexApiKey(settings: Record<string, unknown>): boolean {
  const auth = settings.auth as Record<string, unknown> | undefined;
  return nonEmptyString(auth?.OPENAI_API_KEY);
}

export function resolveCodexOfficialIdentity(
  appId: AppId,
  provider: Pick<Provider, "id" | "category" | "meta" | "settingsConfig">,
): CodexOfficialIdentity | null {
  if (appId !== "codex") return null;

  const managedAccountId = resolveManagedAccountId(
    provider.meta,
    "codex_oauth",
  )?.trim();
  const hasFixedOfficialId = provider.id === CODEX_OFFICIAL_PROVIDER_ID;
  if (hasFixedOfficialId && provider.category === "official") {
    return managedAccountId ? "managed_account" : "native_login";
  }

  const settings = provider.settingsConfig as Record<string, unknown>;
  const auth = settings?.auth;
  const config = settings?.config;
  if (
    !auth ||
    typeof auth !== "object" ||
    Array.isArray(auth) ||
    (config != null && typeof config !== "string")
  ) {
    return null;
  }

  if (hasExplicitCodexThirdPartyUpstream(settings)) {
    return null;
  }

  if (managedAccountId) {
    return "managed_account";
  }
  if (hasStoredCodexApiKey(settings)) {
    return provider.category === "official" ? "api_key" : null;
  }
  return hasFixedOfficialId || provider.category === "official"
    ? "native_login"
    : null;
}

/** Keep the UI capability rule aligned with the Rust takeover policy. */
export function supportsOfficialProxyTakeover(
  appId: AppId,
  provider: Pick<Provider, "id" | "category" | "meta" | "settingsConfig">,
): boolean {
  const identity = resolveCodexOfficialIdentity(appId, provider);
  if (!identity || identity === "api_key") return false;
  if (
    provider.id === CODEX_OFFICIAL_PROVIDER_ID ||
    identity === "managed_account"
  ) {
    return true;
  }
  return true;
}

/**
 * Whether a provider must use the local routing takeover for the selected app.
 *
 * Managed OAuth is authoritative: its real credential exists only inside the
 * proxy runtime, so editable or legacy-missing apiFormat metadata cannot make
 * it safe to route directly. Non-OAuth providers follow each client's native
 * wire format and full-URL behavior.
 */
export function providerNeedsRouting(
  appId: AppId,
  provider: Provider,
): boolean {
  if (
    provider.category === "official" ||
    resolveCodexOfficialIdentity(appId, provider)
  )
    return false;

  if (appId !== "claude" && appId !== "codex" && appId !== "grokbuild") {
    return false;
  }

  if (isOAuthProviderType(provider.meta?.providerType)) return true;

  if (provider.meta?.isFullUrl === true) return true;

  const apiFormat = provider.meta?.apiFormat;
  if (appId === "claude") {
    return apiFormat != null && apiFormat !== "anthropic";
  }

  // Codex and Grok Build natively speak Responses. Chat conversion requires
  // local routing; a missing format remains the native/direct baseline.
  return apiFormat === "openai_chat" || apiFormat === "anthropic";
}
