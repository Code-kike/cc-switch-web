import type { ReactNode } from "react";
import { renderHook, act, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  useScanProxies,
  useSetGlobalProxyUrl,
  useTestProxy,
} from "@/hooks/useGlobalProxy";

const toastErrorMock = vi.fn();
const toastSuccessMock = vi.fn();

const setGlobalProxyUrlMock = vi.fn();
const testProxyUrlMock = vi.fn();
const scanLocalProxiesMock = vi.fn();
const getGlobalProxyUrlMock = vi.fn();
const getUpstreamProxyStatusMock = vi.fn();

vi.mock("sonner", () => ({
  toast: {
    error: (...args: unknown[]) => toastErrorMock(...args),
    success: (...args: unknown[]) => toastSuccessMock(...args),
  },
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, options?: Record<string, unknown>) => {
      if (typeof options?.error === "string") {
        return `${key}:${options.error}`;
      }
      if (typeof options?.latency === "number") {
        return `${key}:${options.latency}`;
      }
      return key;
    },
  }),
}));

vi.mock("@/lib/api/globalProxy", () => ({
  getGlobalProxyUrl: (...args: unknown[]) => getGlobalProxyUrlMock(...args),
  setGlobalProxyUrl: (...args: unknown[]) => setGlobalProxyUrlMock(...args),
  testProxyUrl: (...args: unknown[]) => testProxyUrlMock(...args),
  getUpstreamProxyStatus: (...args: unknown[]) =>
    getUpstreamProxyStatusMock(...args),
  scanLocalProxies: (...args: unknown[]) => scanLocalProxiesMock(...args),
}));

function createWrapper() {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  });

  return ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
  );
}

describe("useGlobalProxy", () => {
  beforeEach(() => {
    toastErrorMock.mockReset();
    toastSuccessMock.mockReset();
    setGlobalProxyUrlMock.mockReset();
    testProxyUrlMock.mockReset();
    scanLocalProxiesMock.mockReset();
    getGlobalProxyUrlMock.mockReset();
    getUpstreamProxyStatusMock.mockReset();
  });

  it("shows structured detail when saving the global proxy fails", async () => {
    setGlobalProxyUrlMock.mockRejectedValueOnce({ detail: "proxy save failed" });
    const { result } = renderHook(() => useSetGlobalProxyUrl(), {
      wrapper: createWrapper(),
    });

    await act(async () => {
      await result.current
        .mutateAsync("http://127.0.0.1:7890")
        .catch(() => undefined);
    });

    await waitFor(() => {
      expect(toastErrorMock).toHaveBeenCalledWith(
        "settings.globalProxy.saveFailed:proxy save failed",
      );
    });
  });

  it("shows structured detail when testing the global proxy throws", async () => {
    testProxyUrlMock.mockRejectedValueOnce({ detail: "proxy test failed" });
    const { result } = renderHook(() => useTestProxy(), {
      wrapper: createWrapper(),
    });

    await act(async () => {
      await result.current
        .mutateAsync("http://127.0.0.1:7890")
        .catch(() => undefined);
    });

    await waitFor(() => {
      expect(toastErrorMock).toHaveBeenCalledWith(
        "settings.globalProxy.testFailed:proxy test failed",
      );
    });
  });

  it("shows structured detail when scanning local proxies fails", async () => {
    scanLocalProxiesMock.mockRejectedValueOnce({ error: "scan failed" });
    const { result } = renderHook(() => useScanProxies(), {
      wrapper: createWrapper(),
    });

    await act(async () => {
      await result.current.mutateAsync().catch(() => undefined);
    });

    await waitFor(() => {
      expect(toastErrorMock).toHaveBeenCalledWith(
        "settings.globalProxy.scanFailed:scan failed",
      );
    });
  });
});
