import { providersApi } from "@/lib/api/providers";
import type { AppId } from "@/lib/api";

export type ImportCurrentProviderConfigResult = "imported" | "no-change";

export async function importCurrentProviderConfig(
  appId: AppId,
): Promise<ImportCurrentProviderConfigResult> {
  if (appId === "opencode") {
    const count = await providersApi.importOpenCodeFromLive();
    return count > 0 ? "imported" : "no-change";
  }
  if (appId === "openclaw") {
    const count = await providersApi.importOpenClawFromLive();
    return count > 0 ? "imported" : "no-change";
  }
  if (appId === "hermes") {
    const count = await providersApi.importHermesFromLive();
    return count > 0 ? "imported" : "no-change";
  }
  const imported = await providersApi.importDefault(appId);
  return imported ? "imported" : "no-change";
}
