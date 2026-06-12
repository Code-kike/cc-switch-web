import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { closeAllSubscriptions, listen } from "@/lib/api/event-adapter";

const originalEventSource = globalThis.EventSource;

class MockEventSource {
  static instances: MockEventSource[] = [];

  readonly url: string;
  readonly withCredentials: boolean;
  onmessage: ((event: MessageEvent) => void) | null = null;
  onerror: ((event: Event) => void) | null = null;
  onopen: ((event: Event) => void) | null = null;
  close = vi.fn();

  constructor(url: string | URL, init?: EventSourceInit) {
    this.url = String(url);
    this.withCredentials = init?.withCredentials ?? false;
    MockEventSource.instances.push(this);
  }

  addEventListener(
    _type: string,
    _listener: EventListenerOrEventListenerObject,
  ): void {}

  emitError(): void {
    this.onerror?.(new Event("error"));
  }

  emitMessage(data: unknown): void {
    this.onmessage?.({ data: JSON.stringify(data) } as MessageEvent);
  }
}

describe("web event adapter SSE reconnect", () => {
  let visibilityState: DocumentVisibilityState;
  let online: boolean;

  beforeEach(() => {
    vi.useFakeTimers();
    MockEventSource.instances = [];
    visibilityState = "visible";
    online = true;

    Object.defineProperty(window, "__TAURI_INTERNALS__", {
      configurable: true,
      value: undefined,
    });
    Object.defineProperty(window, "__TAURI__", {
      configurable: true,
      value: undefined,
    });
    Object.defineProperty(window, "__CC_SWITCH_API_BASE__", {
      configurable: true,
      value: "",
    });
    Object.defineProperty(document, "visibilityState", {
      configurable: true,
      get: () => visibilityState,
    });
    Object.defineProperty(navigator, "onLine", {
      configurable: true,
      get: () => online,
    });
    Object.defineProperty(globalThis, "EventSource", {
      configurable: true,
      value: MockEventSource as unknown as typeof EventSource,
    });
  });

  afterEach(() => {
    closeAllSubscriptions();
    Object.defineProperty(globalThis, "EventSource", {
      configurable: true,
      value: originalEventSource,
    });
    vi.useRealTimers();
    vi.restoreAllMocks();
  });

  it("reconnects after a retry tick fires while the document is hidden", async () => {
    const unlisten = await listen("usage-cache-updated", vi.fn());

    expect(MockEventSource.instances).toHaveLength(1);
    expect(MockEventSource.instances[0].url).toBe("/api/events");

    visibilityState = "hidden";
    MockEventSource.instances[0].emitError();
    await vi.advanceTimersByTimeAsync(1000);

    expect(MockEventSource.instances).toHaveLength(1);

    visibilityState = "visible";
    document.dispatchEvent(new Event("visibilitychange"));

    expect(MockEventSource.instances).toHaveLength(2);
    unlisten();
  });

  it("reconnects when the browser comes back online after a missed retry", async () => {
    const unlisten = await listen("usage-cache-updated", vi.fn());

    online = false;
    MockEventSource.instances[0].emitError();
    await vi.advanceTimersByTimeAsync(1000);

    expect(MockEventSource.instances).toHaveLength(1);

    online = true;
    window.dispatchEvent(new Event("online"));

    expect(MockEventSource.instances).toHaveLength(2);
    unlisten();
  });

  it("delivers proxy failover SSE events to their subscribers", async () => {
    // S4 (06-11 web proxy port): the web proxy runtime emits
    // `provider-switched` (failover hot-switch) and `proxy-official-warning`
    // (takeover targets an official provider) through the SSE bridge; the
    // adapter must route each envelope to the matching subscriber only.
    const onSwitched = vi.fn();
    const onWarning = vi.fn();
    const offSwitched = await listen("provider-switched", onSwitched);
    const offWarning = await listen("proxy-official-warning", onWarning);

    expect(MockEventSource.instances).toHaveLength(1);
    const source = MockEventSource.instances[0];

    source.emitMessage({
      event: "provider-switched",
      payload: { appType: "claude", providerId: "p-2", source: "failover" },
    });
    source.emitMessage({
      event: "proxy-official-warning",
      payload: { appType: "codex", providerName: "Official Codex" },
    });

    expect(onSwitched).toHaveBeenCalledTimes(1);
    expect(onSwitched).toHaveBeenCalledWith({
      event: "provider-switched",
      payload: { appType: "claude", providerId: "p-2", source: "failover" },
    });
    expect(onWarning).toHaveBeenCalledTimes(1);
    expect(onWarning).toHaveBeenCalledWith({
      event: "proxy-official-warning",
      payload: { appType: "codex", providerName: "Official Codex" },
    });

    offSwitched();
    offWarning();
  });
});
