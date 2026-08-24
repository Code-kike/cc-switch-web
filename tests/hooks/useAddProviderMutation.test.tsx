import type { ReactNode } from "react";
import { act, renderHook } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useAddProviderMutation } from "@/lib/query/mutations";

const apiMocks = vi.hoisted(() => ({
  add: vi.fn(),
  getAll: vi.fn(),
  updateTrayMenu: vi.fn(),
}));

const uuidMocks = vi.hoisted(() => ({
  generateUUID: vi.fn(),
}));

const toastMocks = vi.hoisted(() => ({
  success: vi.fn(),
  error: vi.fn(),
  warning: vi.fn(),
}));

vi.mock("@/lib/api", () => ({
  providersApi: {
    add: (...args: unknown[]) => apiMocks.add(...args),
    getAll: (...args: unknown[]) => apiMocks.getAll(...args),
    updateTrayMenu: (...args: unknown[]) => apiMocks.updateTrayMenu(...args),
  },
  sessionsApi: {},
  settingsApi: {},
}));

vi.mock("@/utils/uuid", () => ({
  generateUUID: () => uuidMocks.generateUUID(),
}));

vi.mock("sonner", () => ({
  toast: toastMocks,
}));

function createWrapper() {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  });

  const wrapper = ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
  );

  return { wrapper };
}

beforeEach(() => {
  apiMocks.add.mockReset().mockResolvedValue(true);
  apiMocks.getAll.mockReset().mockResolvedValue({});
  apiMocks.updateTrayMenu.mockReset().mockResolvedValue(true);
  uuidMocks.generateUUID.mockReset().mockReturnValue("generated-uuid");
  toastMocks.success.mockReset();
  toastMocks.error.mockReset();
  toastMocks.warning.mockReset();
});

describe("useAddProviderMutation", () => {
  it("adds a managed Codex account as a separate official card", async () => {
    const { wrapper } = createWrapper();
    const { result } = renderHook(() => useAddProviderMutation("codex"), {
      wrapper,
    });

    const persistedProvider = await act(async () =>
      result.current.mutateAsync({
        name: "OpenAI Official",
        settingsConfig: { auth: {}, config: "" },
        category: "official",
        meta: {
          providerType: "codex_oauth",
          authBinding: {
            source: "managed_account",
            authProvider: "codex_oauth",
            accountId: "acct-managed",
          },
        },
      }),
    );

    expect(apiMocks.getAll).not.toHaveBeenCalled();
    expect(apiMocks.add).toHaveBeenCalledWith(
      expect.objectContaining({
        id: "generated-uuid",
        category: "official",
        meta: {
          providerType: "codex_oauth",
          authBinding: {
            source: "managed_account",
            authProvider: "codex_oauth",
            accountId: "acct-managed",
          },
        },
      }),
      "codex",
      undefined,
    );
    expect(persistedProvider).toEqual(
      expect.objectContaining({
        id: "generated-uuid",
        meta: expect.objectContaining({
          authBinding: expect.objectContaining({
            accountId: "acct-managed",
          }),
        }),
      }),
    );
  });

  it("adds every unbound Codex Official as an independent provider", async () => {
    uuidMocks.generateUUID
      .mockReset()
      .mockReturnValueOnce("unbound-official-1")
      .mockReturnValueOnce("unbound-official-2");
    const { wrapper } = createWrapper();
    const { result } = renderHook(() => useAddProviderMutation("codex"), {
      wrapper,
    });

    const firstProvider = await act(async () =>
      result.current.mutateAsync({
        name: "OpenAI Official 1",
        settingsConfig: { auth: {}, config: "" },
        category: "official",
        meta: { providerType: "codex_oauth" },
      }),
    );
    const secondProvider = await act(async () =>
      result.current.mutateAsync({
        name: "OpenAI Official 2",
        settingsConfig: { auth: {}, config: "" },
        category: "official",
        meta: { providerType: "codex_oauth" },
      }),
    );

    expect(apiMocks.getAll).not.toHaveBeenCalled();
    expect(apiMocks.add).toHaveBeenNthCalledWith(
      1,
      expect.objectContaining({
        id: "unbound-official-1",
        meta: { providerType: "codex_oauth" },
      }),
      "codex",
      undefined,
    );
    expect(apiMocks.add).toHaveBeenNthCalledWith(
      2,
      expect.objectContaining({
        id: "unbound-official-2",
        meta: { providerType: "codex_oauth" },
      }),
      "codex",
      undefined,
    );
    expect(firstProvider.id).toBe("unbound-official-1");
    expect(secondProvider.id).toBe("unbound-official-2");
  });

  it("uses the provider key as the Pi provider id", async () => {
    const { wrapper } = createWrapper();
    const { result } = renderHook(() => useAddProviderMutation("pi"), {
      wrapper,
    });

    const provider = await act(async () =>
      result.current.mutateAsync({
        name: "Pi Native",
        providerKey: "my-pi-key",
        settingsConfig: { apiKey: "sk-test", models: [] },
      } as never),
    );

    // Pi's models.json is keyed by the provider key, so a UUID would create an
    // unreachable node.
    expect(uuidMocks.generateUUID).not.toHaveBeenCalled();
    expect(provider.id).toBe("my-pi-key");
    expect(apiMocks.add).toHaveBeenCalledWith(
      expect.objectContaining({ id: "my-pi-key" }),
      "pi",
      undefined,
    );
    expect(
      (apiMocks.add.mock.calls[0][0] as Record<string, unknown>).providerKey,
    ).toBeUndefined();
  });

  it("rejects a Pi provider without a provider key", async () => {
    const { wrapper } = createWrapper();
    const { result } = renderHook(() => useAddProviderMutation("pi"), {
      wrapper,
    });

    await expect(
      act(async () =>
        result.current.mutateAsync({
          name: "Pi Native",
          settingsConfig: { apiKey: "sk-test", models: [] },
        } as never),
      ),
    ).rejects.toThrow(/Provider key is required for pi/);
    expect(apiMocks.add).not.toHaveBeenCalled();
  });
});
