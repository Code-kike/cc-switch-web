import type { ReactNode } from "react";
import { renderHook, act, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  useProxyConfig,
  useSwitchProxyProvider,
  useUpdateAppProxyConfig,
  useUpdateGlobalProxyConfig,
} from "@/lib/query/proxy";

const toastErrorMock = vi.fn();
const toastSuccessMock = vi.fn();

const getProxyConfigMock = vi.fn();
const updateProxyConfigMock = vi.fn();
const updateGlobalProxyConfigMock = vi.fn();
const updateProxyConfigForAppMock = vi.fn();
const switchProxyProviderMock = vi.fn();
const getGlobalProxyConfigMock = vi.fn();

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
      return key;
    },
  }),
}));

vi.mock("@/lib/api/proxy", () => ({
  proxyApi: {
    getProxyConfig: (...args: unknown[]) => getProxyConfigMock(...args),
    updateProxyConfig: (...args: unknown[]) => updateProxyConfigMock(...args),
    getGlobalProxyConfig: (...args: unknown[]) =>
      getGlobalProxyConfigMock(...args),
    updateGlobalProxyConfig: (...args: unknown[]) =>
      updateGlobalProxyConfigMock(...args),
    updateProxyConfigForApp: (...args: unknown[]) =>
      updateProxyConfigForAppMock(...args),
    switchProxyProvider: (...args: unknown[]) =>
      switchProxyProviderMock(...args),
  },
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

describe("proxy query hooks", () => {
  beforeEach(() => {
    toastErrorMock.mockReset();
    toastSuccessMock.mockReset();
    getProxyConfigMock.mockReset();
    updateProxyConfigMock.mockReset();
    updateGlobalProxyConfigMock.mockReset();
    updateProxyConfigForAppMock.mockReset();
    switchProxyProviderMock.mockReset();
    getGlobalProxyConfigMock.mockReset();
    getProxyConfigMock.mockResolvedValue({});
    getGlobalProxyConfigMock.mockResolvedValue({});
  });

  it("shows structured detail when switching proxy provider fails", async () => {
    switchProxyProviderMock.mockRejectedValueOnce({ detail: "switch exploded" });
    const { result } = renderHook(() => useSwitchProxyProvider(), {
      wrapper: createWrapper(),
    });

    await act(async () => {
      await result.current
        .mutateAsync({ appType: "claude", providerId: "provider-1" })
        .catch(() => undefined);
    });

    await waitFor(() => {
      expect(toastErrorMock).toHaveBeenCalledWith(
        "proxy.switchFailed:switch exploded",
      );
    });
  });

  it("shows structured detail when saving legacy proxy config fails", async () => {
    updateProxyConfigMock.mockRejectedValueOnce({ detail: "legacy save failed" });
    const { result } = renderHook(() => useProxyConfig(), {
      wrapper: createWrapper(),
    });

    await act(async () => {
      await result.current.updateConfig({} as never).catch(() => undefined);
    });

    await waitFor(() => {
      expect(toastErrorMock).toHaveBeenCalledWith(
        "proxy.settings.toast.saveFailed:legacy save failed",
      );
    });
  });

  it("shows structured detail when saving global proxy config fails", async () => {
    updateGlobalProxyConfigMock.mockRejectedValueOnce({
      detail: "global save failed",
    });
    const { result } = renderHook(() => useUpdateGlobalProxyConfig(), {
      wrapper: createWrapper(),
    });

    await act(async () => {
      await result.current.mutateAsync({} as never).catch(() => undefined);
    });

    await waitFor(() => {
      expect(toastErrorMock).toHaveBeenCalledWith(
        "proxy.settings.toast.saveFailed:global save failed",
      );
    });
  });

  it("shows structured detail when saving app proxy config fails", async () => {
    updateProxyConfigForAppMock.mockRejectedValueOnce({
      detail: "app save failed",
    });
    const { result } = renderHook(() => useUpdateAppProxyConfig(), {
      wrapper: createWrapper(),
    });

    await act(async () => {
      await result.current.mutateAsync({ appType: "claude" } as never).catch(
        () => undefined,
      );
    });

    await waitFor(() => {
      expect(toastErrorMock).toHaveBeenCalledWith(
        "proxy.settings.toast.saveFailed:app save failed",
      );
    });
  });
});
