import type { ReactNode } from "react";
import { renderHook, act, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  useUpdateAppProxyConfig,
  useUpdateGlobalProxyConfig,
} from "@/lib/query/proxy";

const toastErrorMock = vi.fn();
const toastSuccessMock = vi.fn();

const updateGlobalProxyConfigMock = vi.fn();
const updateProxyConfigForAppMock = vi.fn();
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
    getGlobalProxyConfig: (...args: unknown[]) =>
      getGlobalProxyConfigMock(...args),
    updateGlobalProxyConfig: (...args: unknown[]) =>
      updateGlobalProxyConfigMock(...args),
    updateProxyConfigForApp: (...args: unknown[]) =>
      updateProxyConfigForAppMock(...args),
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
    updateGlobalProxyConfigMock.mockReset();
    updateProxyConfigForAppMock.mockReset();
    getGlobalProxyConfigMock.mockReset();
    getGlobalProxyConfigMock.mockResolvedValue({});
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

  it("preserves the dedicated failover toggle when saving app config", async () => {
    const queryClient = new QueryClient({
      defaultOptions: {
        queries: { retry: false },
        mutations: { retry: false },
      },
    });
    queryClient.setQueryData(["autoFailoverEnabled", "claude"], true);
    updateProxyConfigForAppMock.mockResolvedValueOnce(undefined);
    const wrapper = ({ children }: { children: ReactNode }) => (
      <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
    );
    const { result } = renderHook(() => useUpdateAppProxyConfig(), { wrapper });

    await act(async () => {
      await result.current.mutateAsync({
        appType: "claude",
        autoFailoverEnabled: false,
      } as never);
    });

    expect(updateProxyConfigForAppMock).toHaveBeenCalledWith(
      expect.objectContaining({
        appType: "claude",
        autoFailoverEnabled: true,
      }),
    );
  });

  it("shows structured detail when saving app proxy config fails", async () => {
    updateProxyConfigForAppMock.mockRejectedValueOnce({
      detail: "app save failed",
    });
    const { result } = renderHook(() => useUpdateAppProxyConfig(), {
      wrapper: createWrapper(),
    });

    await act(async () => {
      await result.current
        .mutateAsync({ appType: "claude" } as never)
        .catch(() => undefined);
    });

    await waitFor(() => {
      expect(toastErrorMock).toHaveBeenCalledWith(
        "proxy.settings.toast.saveFailed:app save failed",
      );
    });
  });
});
