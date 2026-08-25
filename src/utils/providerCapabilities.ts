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

/**
 * Keep the UI capability rule aligned with the Rust takeover policy.
 *
 * Single source of truth on the Rust side is
 * `Provider::blocked_by_proxy_takeover` (src-tauri/src/provider.rs), pinned by
 * `blocked_by_proxy_takeover_opens_only_managed_codex_official_cards`. This
 * fork opens **only managed** Codex Official cards under takeover: the
 * forwarder resolves the bound account's token and injects
 * `chatgpt-account-id`. Unbound native-login (and api-key) Official cards have
 * no server-side credential here, because this fork does not carry upstream's
 * inbound Authorization passthrough.
 *
 * Degradation direction: this gates an **authorization** decision, so it must
 * stay **fail-closed** — return false whenever the card is not a managed
 * account. Do NOT widen it back to "any non-api-key Official identity"
 * (upstream's shape, which ended in an unreachable `return true`): the UI would
 * stop emitting the explicit `notifications.officialBlockedByProxy` refusal and
 * the switch would instead fail deeper in the Rust service layer, turning a
 * clear switch-time refusal into a failing Codex session.
 */
export function supportsOfficialProxyTakeover(
  appId: AppId,
  provider: Pick<Provider, "id" | "category" | "meta" | "settingsConfig">,
): boolean {
  return resolveCodexOfficialIdentity(appId, provider) === "managed_account";
}

/**
 * Whether switching to this provider must be refused because local routing
 * takeover is active — the UI mirror of Rust
 * `Provider::blocked_by_proxy_takeover`.
 *
 * Single definition for every frontend surface (card switch button, switch
 * action hook). Do not re-derive the three-part condition at call sites: an
 * earlier copy in `ProviderCard` missed the managed carve-out and made the
 * server-side-supported switch unreachable from the card UI.
 */
export function isOfficialBlockedByTakeover(
  appId: AppId,
  provider: Pick<Provider, "id" | "category" | "meta" | "settingsConfig">,
  isProxyTakeover: boolean | undefined,
): boolean {
  return (
    isProxyTakeover === true &&
    provider.category === "official" &&
    !supportsOfficialProxyTakeover(appId, provider)
  );
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
