import { invoke as tauriInvoke } from "@tauri-apps/api/core"
import { WebApiError, WebAuthError, WebNotSupportedError } from "./errors"
import type { CommandMap, CommandSpec } from "./types-internal"

export const isWebMode = (): boolean =>
  typeof window !== "undefined" && !window.__TAURI_INTERNALS__ && !window.__TAURI__

export const apiBase = (): string =>
  (typeof window !== "undefined" && window.__CC_SWITCH_API_BASE__) || ""

let csrfToken: string | null = null
let csrfRefreshPromise: Promise<string> | null = null

export const setCsrfToken = (t: string | null): void => {
  csrfToken = t
}

export const getCsrfToken = (): string | null => csrfToken

async function refreshCsrfToken(): Promise<string> {
  if (csrfRefreshPromise !== null) return csrfRefreshPromise
  csrfRefreshPromise = (async () => {
    try {
      const r = await fetch(`${apiBase()}/api/system/csrf-token`, {
        credentials: "include",
        headers: { Accept: "application/json" },
      })
      if (!r.ok) {
        throw new WebAuthError(r.status, "CSRF token refresh failed")
      }
      const data = (await r.json()) as { token: string }
      csrfToken = data.token
      return data.token
    } finally {
      csrfRefreshPromise = null
    }
  })()
  return csrfRefreshPromise
}

const SAFE_METHODS = new Set(["GET", "HEAD", "OPTIONS"])

const commandRegistry: Record<string, CommandSpec> = Object.create(null)

export function registerCommands(map: CommandMap): void {
  for (const [k, v] of Object.entries(map)) {
    if (commandRegistry[k] !== undefined) {
      if (typeof console !== "undefined" && console.warn) {
        console.warn(`[adapter] command "${k}" already registered, overriding`)
      }
    }
    commandRegistry[k] = v
  }
}

export function getRegisteredCommand(cmd: string): CommandSpec | undefined {
  return commandRegistry[cmd]
}

export function listRegisteredCommands(): string[] {
  return Object.keys(commandRegistry)
}

export async function invoke<T = unknown>(
  cmd: string,
  args?: Record<string, unknown>,
): Promise<T> {
  if (!isWebMode()) {
    return tauriInvoke<T>(cmd, args)
  }
  return httpInvoke<T>(cmd, args)
}

export async function webJsonFetch<T = unknown>(
  path: string,
  init: RequestInit = {},
): Promise<T> {
  return webFetch<T>(path, {
    ...init,
    headers: {
      Accept: "application/json",
      ...(init.headers as Record<string, string> | undefined),
    },
  })
}

export async function webUpload<T = unknown>(
  path: string,
  formData: FormData,
): Promise<T> {
  return webFetch<T>(path, {
    method: "POST",
    body: formData,
    headers: {
      Accept: "application/json",
    },
  })
}

export async function webDownload(
  path: string,
  init: RequestInit = {},
): Promise<Blob> {
  return webFetch<Blob>(path, init, "blob")
}

export async function pickWebFile(accept?: string): Promise<File | null> {
  if (typeof document === "undefined") return null
  return new Promise((resolve) => {
    const input = document.createElement("input")
    input.type = "file"
    if (accept) input.accept = accept
    input.style.display = "none"
    input.addEventListener(
      "change",
      () => {
        resolve(input.files?.[0] ?? null)
        input.remove()
      },
      { once: true },
    )
    document.body.appendChild(input)
    input.click()
  })
}

export function isBrowserFile(value: unknown): value is File {
  return typeof File !== "undefined" && value instanceof File
}

export function downloadBlob(blob: Blob, filename: string): void {
  if (typeof document === "undefined") return
  const url = URL.createObjectURL(blob)
  try {
    const anchor = document.createElement("a")
    anchor.href = url
    anchor.download = filename
    anchor.style.display = "none"
    document.body.appendChild(anchor)
    anchor.click()
    anchor.remove()
  } finally {
    URL.revokeObjectURL(url)
  }
}

function buildPath(spec: CommandSpec, args: Record<string, unknown>): {
  resolvedPath: string
  remaining: Record<string, unknown>
} {
  let resolvedPath = spec.path
  const remaining: Record<string, unknown> = { ...args }

  const pathParams =
    spec.pathParams ??
    (spec.path.match(/:[a-zA-Z_][a-zA-Z0-9_]*/g) ?? []).map((m) => m.slice(1))

  for (const p of pathParams) {
    const v = remaining[p]
    if (v === undefined || v === null) {
      throw new WebApiError(400, "BAD_PATH_PARAM", `Missing path param ${p}`)
    }
    resolvedPath = resolvedPath.replace(`:${p}`, encodeURIComponent(String(v)))
    delete remaining[p]
  }

  return { resolvedPath, remaining }
}

async function httpInvoke<T>(
  cmd: string,
  args?: Record<string, unknown>,
): Promise<T> {
  const spec = commandRegistry[cmd]
  if (spec === undefined) {
    throw new WebNotSupportedError(
      cmd,
      `Command "${cmd}" not registered in adapter`,
    )
  }
  if (spec.unsupported === true) {
    throw new WebNotSupportedError(cmd)
  }

  const { method } = spec
  const { resolvedPath, remaining } = buildPath(spec, args ?? {})

  let url = apiBase() + resolvedPath
  let body: string | undefined

  const headers: Record<string, string> = {
    Accept: "application/json",
  }

  const useQuery =
    method === "GET" ||
    (method === "DELETE" && spec.bodyParams !== true) ||
    spec.queryParams === true

  if (useQuery) {
    const qp = new URLSearchParams()
    for (const [k, v] of Object.entries(remaining)) {
      if (v === undefined || v === null) continue
      qp.set(k, typeof v === "string" ? v : JSON.stringify(v))
    }
    const qs = qp.toString()
    if (qs.length > 0) {
      url += url.includes("?") ? `&${qs}` : `?${qs}`
    }
  } else {
    headers["Content-Type"] = "application/json"
    body = JSON.stringify(remaining)
  }

  if (!SAFE_METHODS.has(method)) {
    if (csrfToken === null) {
      try {
        await refreshCsrfToken()
      } catch {
        // Continue without token; server will reject and trigger refresh on retry
      }
    }
    if (csrfToken !== null) {
      headers["X-CSRF-Token"] = csrfToken
    }
  }

  const resp = await fetchWithCsrfRetry(url, {
    method,
    headers,
    body,
    credentials: "include",
  })

  return parseWebResponse<T>(resp, cmd)
}

async function webFetch<T>(
  path: string,
  init: RequestInit,
  responseType: "json" | "blob" = "json",
): Promise<T> {
  const method = (init.method ?? "GET").toUpperCase()
  const headers: Record<string, string> = {
    ...((init.headers as Record<string, string> | undefined) ?? {}),
  }

  if (!SAFE_METHODS.has(method)) {
    if (csrfToken === null) {
      try {
        await refreshCsrfToken()
      } catch {
        // Continue without token; server will reject and trigger refresh on retry
      }
    }
    if (csrfToken !== null) {
      headers["X-CSRF-Token"] = csrfToken
    }
  }

  const resp = await fetchWithCsrfRetry(apiBase() + path, {
    ...init,
    method,
    headers,
    credentials: "include",
  })

  return parseWebResponse<T>(resp, path, responseType)
}

async function fetchWithCsrfRetry(
  url: string,
  init: RequestInit,
): Promise<Response> {
  const method = (init.method ?? "GET").toUpperCase()
  const headers: Record<string, string> = {
    ...((init.headers as Record<string, string> | undefined) ?? {}),
  }

  const fetchOnce = (): Promise<Response> =>
    fetch(url, {
      ...init,
      headers,
      credentials: "include",
    })

  let resp = await fetchOnce()

  if (resp.status === 403 && !SAFE_METHODS.has(method)) {
    csrfToken = null
    try {
      await refreshCsrfToken()
    } catch {
      throw new WebAuthError(403, "CSRF refresh failed")
    }
    if (csrfToken !== null) {
      headers["X-CSRF-Token"] = csrfToken
    }
    resp = await fetchOnce()
  }

  return resp
}

async function parseWebResponse<T>(
  resp: Response,
  commandOrPath: string,
  responseType: "json" | "blob" = "json",
): Promise<T> {
  if (resp.status === 401) {
    csrfToken = null
    throw new WebAuthError(401, "Session expired")
  }
  if (!resp.ok) {
    let errBody: { code?: string; message?: string; details?: unknown } = {}
    try {
      errBody = await resp.json()
    } catch {
      // body may be empty or non-JSON
    }
    const code = errBody.code ?? `HTTP_${resp.status}`
    if (
      resp.status === 501 ||
      code === "WEB_NOT_SUPPORTED" ||
      code === "WEB_DESKTOP_ONLY" ||
      code === "WEB_UPLOAD_REQUIRED"
    ) {
      throw new WebNotSupportedError(commandOrPath, errBody.message, code)
    }
    throw new WebApiError(resp.status, code, errBody.message, errBody.details)
  }

  if (resp.status === 204) {
    return undefined as T
  }

  if (responseType === "blob") {
    return (await resp.blob()) as T
  }

  const contentType = resp.headers.get("content-type") ?? ""
  if (!contentType.includes("application/json")) {
    return (await resp.text()) as unknown as T
  }
  return (await resp.json()) as T
}

export { WebApiError, WebAuthError, WebNotSupportedError } from "./errors"
export { defineCommands } from "./types-internal"
export type { CommandMap, CommandSpec, HttpMethod } from "./types-internal"
