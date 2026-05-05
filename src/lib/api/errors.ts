export class WebNotSupportedError extends Error {
  readonly code: string
  constructor(
    public command: string,
    message?: string,
    code = "WEB_NOT_SUPPORTED",
  ) {
    super(message ?? `Command "${command}" is not supported in Web mode`)
    this.name = "WebNotSupportedError"
    this.code = code
  }
}

export class WebAuthError extends Error {
  readonly code = "AUTH_ERROR" as const
  constructor(public status: number, message?: string) {
    super(message ?? `Authentication failed (${status})`)
    this.name = "WebAuthError"
  }
}

export class WebApiError extends Error {
  constructor(
    public status: number,
    public code: string,
    message?: string,
    public details?: unknown,
  ) {
    super(message ?? `API error ${status}: ${code}`)
    this.name = "WebApiError"
  }
}

export type WebUnavailableResult = {
  status: "unavailable"
  reason: "web-mode-disabled"
}

export const isWebUnavailable = (v: unknown): v is WebUnavailableResult =>
  typeof v === "object" &&
  v !== null &&
  (v as { status?: string }).status === "unavailable" &&
  (v as { reason?: string }).reason === "web-mode-disabled"

export const isWebNotSupported = (e: unknown): e is WebNotSupportedError =>
  e instanceof Error && (e as { code?: string }).code === "WEB_NOT_SUPPORTED"

export const isWebAuthError = (e: unknown): e is WebAuthError =>
  e instanceof Error && (e as { code?: string }).code === "AUTH_ERROR"
