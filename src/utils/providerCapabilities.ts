import type { AppId } from "@/lib/api";
import type { Provider } from "@/types";
import { isOAuthProviderType } from "@/config/constants";

export const GROKBUILD_OFFICIAL_PROVIDER_ID = "grokbuild-official";

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
  if (provider.category === "official") return false;
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
