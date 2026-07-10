import { isWebMode } from "./adapter";

export type UpdateInfo = {
  available: boolean;
  version?: string;
  notes?: string;
  downloadUrl?: string;
  isWebMode?: boolean;
};

export async function checkForUpdates(): Promise<UpdateInfo> {
  if (!isWebMode()) {
    const updater = await import("@tauri-apps/plugin-updater");
    const u = await updater.check();
    return u
      ? {
          available: true,
          version: u.version,
          notes: u.body,
        }
      : { available: false };
  }

  // Web mode: this fork has no independent release channel, and the upstream
  // update-check would steer users to upstream desktop binaries that lack this
  // fork's changes. Report "no update available" instead of querying upstream.
  // See docs/audit/2026-07-10-full-audit.md (L11 / H3).
  return { available: false, isWebMode: true };
}
