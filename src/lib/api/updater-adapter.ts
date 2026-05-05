import { isWebMode, webJsonFetch } from "./adapter"

export type UpdateInfo = {
  available: boolean
  version?: string
  notes?: string
  downloadUrl?: string
  isWebMode?: boolean
}

export async function checkForUpdates(): Promise<UpdateInfo> {
  if (!isWebMode()) {
    const updater = await import("@tauri-apps/plugin-updater")
    const u = await updater.check()
    return u
      ? {
          available: true,
          version: u.version,
          notes: u.body,
        }
      : { available: false }
  }

  try {
    const data = await webJsonFetch<UpdateInfo>("/api/system/get_update_info")
    return {
      available: data.available,
      version: data.version,
      notes: data.notes,
      downloadUrl: data.downloadUrl,
      isWebMode: true,
    }
  } catch {
    return { available: false, isWebMode: true }
  }
}
