import { WebNotSupportedError } from "./errors"

const isWebMode = (): boolean =>
  typeof window !== "undefined" && !window.__TAURI_INTERNALS__ && !window.__TAURI__

export type OpenDialogOptions = {
  directory?: boolean
  multiple?: boolean
  defaultPath?: string
  filters?: Array<{ name: string; extensions: string[] }>
}

export async function openDialog(
  opts: OpenDialogOptions = {},
): Promise<string | string[] | null> {
  if (!isWebMode()) {
    const dialog = await import("@tauri-apps/plugin-dialog")
    return dialog.open(opts) as Promise<string | string[] | null>
  }
  throw new WebNotSupportedError(
    "dialog.open",
    "Web mode cannot browse server filesystem; please type the path manually",
  )
}

export type ServerPlatform = {
  os: "linux" | "macos" | "windows" | "unknown"
  isWsl: boolean
  defaultPaths: {
    appConfig?: string
    claude?: string
    codex?: string
    gemini?: string
    opencode?: string
    openclaw?: string
    hermes?: string
    omo?: string
  }
}

export async function getServerPlatform(): Promise<ServerPlatform> {
  if (!isWebMode()) {
    return {
      os: detectClientOs(),
      isWsl: false,
      defaultPaths: {},
    }
  }
  const base = (typeof window !== "undefined" && window.__CC_SWITCH_API_BASE__) || ""
  const r = await fetch(`${base}/api/env/platform`, { credentials: "include" })
  if (!r.ok) {
    return { os: "unknown", isWsl: false, defaultPaths: {} }
  }
  return (await r.json()) as ServerPlatform
}

function detectClientOs(): ServerPlatform["os"] {
  if (typeof navigator === "undefined") return "unknown"
  const ua = navigator.userAgent.toLowerCase()
  if (ua.includes("win")) return "windows"
  if (ua.includes("mac")) return "macos"
  if (ua.includes("linux")) return "linux"
  return "unknown"
}
