import type { ReactNode } from "react";
import { act, renderHook, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useManagedAuth } from "@/components/providers/forms/hooks/useManagedAuth";

const authGetStatusMock = vi.fn();
const authStartLoginMock = vi.fn();
const authPollForAccountMock = vi.fn();
const authLogoutMock = vi.fn();
const authRemoveAccountMock = vi.fn();
const authSetDefaultAccountMock = vi.fn();
const openExternalMock = vi.fn();
const copyTextMock = vi.fn();

vi.mock("@/lib/api", () => ({
  authApi: {
    authGetStatus: (...args: unknown[]) => authGetStatusMock(...args),
    authStartLogin: (...args: unknown[]) => authStartLoginMock(...args),
    authPollForAccount: (...args: unknown[]) => authPollForAccountMock(...args),
    authLogout: (...args: unknown[]) => authLogoutMock(...args),
    authRemoveAccount: (...args: unknown[]) => authRemoveAccountMock(...args),
    authSetDefaultAccount: (...args: unknown[]) =>
      authSetDefaultAccountMock(...args),
  },
  settingsApi: {
    openExternal: (...args: unknown[]) => openExternalMock(...args),
  },
}));

vi.mock("@/lib/clipboard", () => ({
  copyText: (...args: unknown[]) => copyTextMock(...args),
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

describe("useManagedAuth", () => {
  beforeEach(() => {
    authGetStatusMock.mockReset();
    authStartLoginMock.mockReset();
    authPollForAccountMock.mockReset();
    authLogoutMock.mockReset();
    authRemoveAccountMock.mockReset();
    authSetDefaultAccountMock.mockReset();
    openExternalMock.mockReset();
    copyTextMock.mockReset();

    authGetStatusMock.mockResolvedValue({
      provider: "github_copilot",
      authenticated: false,
      default_account_id: null,
      accounts: [],
    });
    openExternalMock.mockResolvedValue(undefined);
    copyTextMock.mockResolvedValue(undefined);
  });

  it("extracts structured detail from login failures instead of leaking raw objects", async () => {
    authStartLoginMock.mockRejectedValueOnce({
      detail: "login denied by backend",
    });

    const { result } = renderHook(() => useManagedAuth("github_copilot"), {
      wrapper: createWrapper(),
    });

    await waitFor(() => {
      expect(result.current.isLoadingStatus).toBe(false);
    });

    await act(async () => {
      result.current.startAuth();
    });

    await waitFor(() => {
      expect(result.current.pollingState).toBe("error");
    });

    expect(result.current.error).toBe("login denied by backend");
    expect(result.current.error).not.toBe("[object Object]");
    expect(authStartLoginMock).toHaveBeenCalledWith(
      "github_copilot",
      undefined,
    );
  });

  it("extracts structured detail from polling failures", async () => {
    authStartLoginMock.mockResolvedValueOnce({
      provider: "codex_oauth",
      device_code: "dev-123",
      user_code: "CODE-1234",
      verification_uri: "https://example.com/device",
      expires_in: 600,
      interval: 1,
    });
    authPollForAccountMock.mockRejectedValueOnce({
      detail: "device authorization denied",
    });

    const { result } = renderHook(() => useManagedAuth("codex_oauth"), {
      wrapper: createWrapper(),
    });

    await waitFor(() => {
      expect(result.current.isLoadingStatus).toBe(false);
    });

    await act(async () => {
      result.current.startAuth();
    });

    await waitFor(() => {
      expect(result.current.pollingState).toBe("error");
    });

    expect(result.current.error).toBe("device authorization denied");
    expect(result.current.error).not.toBe("[object Object]");
    expect(copyTextMock).toHaveBeenCalledWith("CODE-1234");
    expect(openExternalMock).toHaveBeenCalledWith("https://example.com/device");
  });
});
