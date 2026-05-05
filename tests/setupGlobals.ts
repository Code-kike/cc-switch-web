// Polyfill ResizeObserver for jsdom/happy-dom
if (typeof globalThis.ResizeObserver === "undefined") {
  globalThis.ResizeObserver = class ResizeObserver {
    private readonly callback: ResizeObserverCallback;

    constructor(callback: ResizeObserverCallback) {
      this.callback = callback;
    }

    observe(target: Element) {
      this.callback(
        [
          {
            target,
            contentRect: {
              width: 960,
              height: 350,
              x: 0,
              y: 0,
              top: 0,
              left: 0,
              bottom: 350,
              right: 960,
              toJSON: () => ({}),
            } as DOMRectReadOnly,
          } as ResizeObserverEntry,
        ],
        this,
      );
    }
    unobserve() {}
    disconnect() {}
  } as unknown as typeof globalThis.ResizeObserver;
}

const storage = new Map<string, string>();

if (
  typeof globalThis.localStorage === "undefined" ||
  typeof globalThis.localStorage?.getItem !== "function"
) {
  Object.defineProperty(globalThis, "localStorage", {
    value: {
      getItem: (key: string) => storage.get(key) ?? null,
      setItem: (key: string, value: string) => {
        storage.set(key, String(value));
      },
      removeItem: (key: string) => {
        storage.delete(key);
      },
      clear: () => {
        storage.clear();
      },
      key: (index: number) => Array.from(storage.keys())[index] ?? null,
      get length() {
        return storage.size;
      },
    },
    configurable: true,
  });
}
