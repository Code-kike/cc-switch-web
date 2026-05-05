import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { useStreamCheck } from "@/hooks/useStreamCheck";

const toastSuccessMock = vi.fn();
const toastErrorMock = vi.fn();
const toastWarningMock = vi.fn();
const streamCheckProviderMock = vi.fn();
const resetCircuitBreakerMutateMock = vi.fn();

vi.mock("sonner", () => ({
  toast: {
    success: (...args: unknown[]) => toastSuccessMock(...args),
    error: (...args: unknown[]) => toastErrorMock(...args),
    warning: (...args: unknown[]) => toastWarningMock(...args),
  },
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (
      key: string,
      options?: { providerName?: string; error?: string; defaultValue?: string },
    ) => {
      if (options?.error) return `${key}:${options.error}`;
      return options?.defaultValue ?? key;
    },
  }),
}));

vi.mock("@/lib/api/model-test", () => ({
  streamCheckProvider: (...args: unknown[]) => streamCheckProviderMock(...args),
}));

vi.mock("@/lib/query/failover", () => ({
  useResetCircuitBreaker: () => ({
    mutate: (...args: unknown[]) => resetCircuitBreakerMutateMock(...args),
  }),
}));

describe("useStreamCheck", () => {
  beforeEach(() => {
    toastSuccessMock.mockReset();
    toastErrorMock.mockReset();
    toastWarningMock.mockReset();
    streamCheckProviderMock.mockReset();
    resetCircuitBreakerMutateMock.mockReset();
  });

  it("surfaces structured details when the provider check throws", async () => {
    streamCheckProviderMock.mockRejectedValue({
      payload: { detail: "network exploded" },
    });

    const { result } = renderHook(() => useStreamCheck("claude"));

    await act(async () => {
      await result.current.checkProvider("provider-1", "Provider A");
    });

    await waitFor(() =>
      expect(toastErrorMock).toHaveBeenCalledWith(
        "streamCheck.error:network exploded",
      ),
    );
    expect(toastErrorMock.mock.calls[0]?.[0]).not.toContain("[object Object]");
  });
});
