import type { ReactNode } from "react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

const {
  getOpenCodeLiveProviderIdsMock,
  getOpenCodeModelsMock,
  toastWarningMock,
  translateMock,
  useProvidersQueryMock,
} = vi.hoisted(() => ({
  getOpenCodeLiveProviderIdsMock: vi.fn(),
  getOpenCodeModelsMock: vi.fn(),
  toastWarningMock: vi.fn(),
  translateMock: vi.fn((key: string) => key),
  useProvidersQueryMock: vi.fn(),
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: translateMock }),
}));

vi.mock("sonner", () => ({
  toast: { warning: toastWarningMock },
}));

vi.mock("@/lib/api", () => ({
  providersApi: {
    getOpenCodeLiveProviderIds: (...args: unknown[]) =>
      getOpenCodeLiveProviderIdsMock(...args),
  },
}));

vi.mock("@/lib/api/model-fetch", () => ({
  getOpenCodeModels: (...args: unknown[]) => getOpenCodeModelsMock(...args),
}));

vi.mock("@/lib/query/queries", () => ({
  useProvidersQuery: (...args: unknown[]) => useProvidersQueryMock(...args),
}));

import { useOmoModelSource } from "@/components/providers/forms/hooks/useOmoModelSource";

const configuredProvider = {
  id: "configured",
  name: "Configured Provider",
  category: "third_party" as const,
  settingsConfig: {
    npm: "@ai-sdk/openai-compatible",
    options: {},
    models: {
      "shared-model": {
        name: "Configured Label",
        variants: {
          fast: {},
          deep: {},
        },
      },
      "configured-only": {
        name: "Configured Only",
      },
    },
  },
};

function createWrapper() {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: {
        retry: false,
        retryDelay: 0,
      },
    },
  });

  return ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
  );
}

describe("useOmoModelSource", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    getOpenCodeLiveProviderIdsMock.mockResolvedValue(["configured"]);
    getOpenCodeModelsMock.mockResolvedValue([]);
    useProvidersQueryMock.mockReturnValue({
      data: {
        providers: { configured: configuredProvider },
        currentProviderId: "configured",
      },
    });
  });

  it("merges OAuth and Zen runtime models with configured provider models", async () => {
    getOpenCodeModelsMock.mockResolvedValue([
      { providerId: "oauth", modelId: "oauth-only" },
      { providerId: "opencode", modelId: "zen-free" },
    ]);

    const { result } = renderHook(
      () => useOmoModelSource({ isOmoCategory: true }),
      { wrapper: createWrapper() },
    );

    await waitFor(() => {
      expect(
        result.current.omoModelOptions.map((option) => option.value),
      ).toEqual(
        expect.arrayContaining([
          "configured/configured-only",
          "oauth/oauth-only",
          "opencode/zen-free",
        ]),
      );
    });
  });

  it("keeps configured labels and variants when runtime discovery returns a duplicate", async () => {
    getOpenCodeModelsMock.mockResolvedValue([
      { providerId: "configured", modelId: "shared-model" },
    ]);

    const { result } = renderHook(
      () => useOmoModelSource({ isOmoCategory: true }),
      { wrapper: createWrapper() },
    );

    await waitFor(() => {
      expect(
        result.current.omoModelOptions.find(
          (option) => option.value === "configured/shared-model",
        ),
      ).toEqual({
        value: "configured/shared-model",
        label: "Configured Provider / Configured Label (shared-model)",
      });
    });

    expect(
      result.current.omoModelVariantsMap["configured/shared-model"],
    ).toEqual(["fast", "deep"]);
    expect(
      result.current.omoModelOptions.filter(
        (option) => option.value === "configured/shared-model",
      ),
    ).toHaveLength(1);
  });

  it("warns on runtime discovery failure and preserves configured fallback models", async () => {
    getOpenCodeModelsMock.mockRejectedValue(new Error("opencode unavailable"));

    const { result } = renderHook(
      () => useOmoModelSource({ isOmoCategory: true }),
      { wrapper: createWrapper() },
    );

    await waitFor(() => {
      expect(
        result.current.omoModelOptions.map((option) => option.value),
      ).toContain("configured/configured-only");
    });
    await waitFor(() => {
      expect(toastWarningMock).toHaveBeenCalledWith(
        "omo.runtimeModelsFailedWarning",
      );
    });
  });
});
