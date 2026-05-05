type Unlisten = () => void
type EventCallback<T = unknown> = (e: { event: string; payload: T }) => void

const isWebMode = (): boolean =>
  typeof window !== "undefined" && !window.__TAURI_INTERNALS__ && !window.__TAURI__

const apiBase = (): string =>
  (typeof window !== "undefined" && window.__CC_SWITCH_API_BASE__) || ""

let sse: EventSource | null = null
let reconnectAttempts = 0
let reconnectTimer: ReturnType<typeof setTimeout> | null = null
const subscribers = new Map<string, Set<EventCallback>>()

function clearReconnect(): void {
  if (reconnectTimer !== null) {
    clearTimeout(reconnectTimer)
    reconnectTimer = null
  }
}

function ensureSse(): void {
  if (sse !== null) return
  if (typeof EventSource === "undefined") return

  const url = `${apiBase()}/api/events`
  sse = new EventSource(url, { withCredentials: true })

  sse.onmessage = (msg) => {
    try {
      const env = JSON.parse(msg.data) as {
        event: string
        payload: unknown
        ts?: number
        seq?: number
      }
      const cbs = subscribers.get(env.event)
      cbs?.forEach((cb) => cb({ event: env.event, payload: env.payload }))
    } catch {
      // Ignore malformed messages — server is best-effort
    }
  }

  sse.addEventListener("lagged", () => {
    // Server signaled receiver lag; clients should invalidate everything
    const cbs = subscribers.get("__lagged")
    cbs?.forEach((cb) => cb({ event: "__lagged", payload: null }))
  })

  sse.onerror = () => {
    sse?.close()
    sse = null
    clearReconnect()
    const delay = Math.min(1000 * 2 ** reconnectAttempts, 30_000)
    reconnectAttempts += 1
    reconnectTimer = setTimeout(() => {
      if (
        typeof document !== "undefined" &&
        document.visibilityState === "visible" &&
        navigator.onLine
      ) {
        ensureSse()
      }
    }, delay)
  }

  sse.onopen = () => {
    reconnectAttempts = 0
  }
}

export async function listen<T = unknown>(
  event: string,
  cb: EventCallback<T>,
): Promise<Unlisten> {
  if (!isWebMode()) {
    const tauri = await import("@tauri-apps/api/event")
    return tauri.listen<T>(event, cb)
  }
  const set = subscribers.get(event) ?? new Set()
  set.add(cb as EventCallback)
  subscribers.set(event, set)
  ensureSse()
  return () => {
    set.delete(cb as EventCallback)
    if (set.size === 0) subscribers.delete(event)
    if (subscribers.size === 0) {
      sse?.close()
      sse = null
      clearReconnect()
    }
  }
}

export function closeAllSubscriptions(): void {
  sse?.close()
  sse = null
  subscribers.clear()
  clearReconnect()
  reconnectAttempts = 0
}
