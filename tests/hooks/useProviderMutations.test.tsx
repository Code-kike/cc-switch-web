import type { ReactNode } from "react";
import { act, renderHook } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useAddProviderMutation } from "@/lib/query/mutations";
import type { Provider } from "@/types";

const ensureGrokBuildOfficialProviderMock = vi.fn();
const getAllProvidersMock = vi.fn();
const addProviderMock = vi.fn();
const updateTrayMenuMock = vi.fn();

vi.mock("sonner", () => ({
  toast: {
    error: vi.fn(),
    success: vi.fn(),
  },
}));

vi.mock("@/lib/api", () => ({
  providersApi: {
    ensureGrokBuildOfficialProvider: (...args: unknown[]) =>
      ensureGrokBuildOfficialProviderMock(...args),
    getAll: (...args: unknown[]) => getAllProvidersMock(...args),
    add: (...args: unknown[]) => addProviderMock(...args),
    updateTrayMenu: (...args: unknown[]) => updateTrayMenuMock(...args),
  },
  sessionsApi: {},
  settingsApi: {},
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

describe("provider mutations", () => {
  beforeEach(() => {
    ensureGrokBuildOfficialProviderMock.mockReset();
    getAllProvidersMock.mockReset();
    addProviderMock.mockReset();
    updateTrayMenuMock.mockReset();
    updateTrayMenuMock.mockResolvedValue(true);
  });

  it("reuses the canonical Grok Official seed without adding a duplicate provider", async () => {
    const officialProvider: Provider = {
      id: "grokbuild-official",
      name: "Grok Official",
      category: "official",
      settingsConfig: { config: "" },
    };
    ensureGrokBuildOfficialProviderMock.mockResolvedValue(true);
    getAllProvidersMock.mockResolvedValue({
      [officialProvider.id]: officialProvider,
    });

    const { result } = renderHook(() => useAddProviderMutation("grokbuild"), {
      wrapper: createWrapper(),
    });
    let returnedProvider: Provider | undefined;

    await act(async () => {
      returnedProvider = await result.current.mutateAsync({
        name: officialProvider.name,
        category: officialProvider.category,
        settingsConfig: officialProvider.settingsConfig,
        ensureGrokBuildOfficialSeed: true,
      });
    });

    expect(ensureGrokBuildOfficialProviderMock).toHaveBeenCalledTimes(1);
    expect(getAllProvidersMock).toHaveBeenCalledWith("grokbuild");
    expect(addProviderMock).not.toHaveBeenCalled();
    expect(returnedProvider).toBe(officialProvider);
  });
});
