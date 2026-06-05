import type { ReactNode } from "react";
import { renderHook, waitFor, act } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { usageKeys } from "@/lib/query/usage";
import { useUsageEventBridge } from "@/hooks/useUsageEventBridge";

type Handler = (e: { event: string; payload: unknown }) => void;

let captured: Handler | null = null;
const unlistenMock = vi.fn();

vi.mock("@/lib/api/event-adapter", () => ({
  listen: vi.fn(async (_event: string, cb: Handler) => {
    captured = cb;
    return unlistenMock;
  }),
}));

describe("useUsageEventBridge (M38 narrowed invalidation)", () => {
  beforeEach(() => {
    captured = null;
    unlistenMock.mockReset();
  });

  it("invalidates dashboard aggregates but leaves per-provider script queries untouched", async () => {
    const queryClient = new QueryClient();
    const aggregateKey = usageKeys.summary(
      "today",
      undefined,
      undefined,
      "all",
    );
    const scriptKey = usageKeys.script("provider-1", "claude");
    queryClient.setQueryData(aggregateKey, { totalCost: 1 });
    queryClient.setQueryData(scriptKey, { success: true });

    const wrapper = ({ children }: { children: ReactNode }) => (
      <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
    );

    renderHook(() => useUsageEventBridge(), { wrapper });

    // The effect registers the listener via an async `listen()` call.
    await waitFor(() => expect(captured).not.toBeNull());

    await act(async () => {
      captured?.({ event: "usage-log-recorded", payload: null });
      await Promise.resolve();
    });

    expect(queryClient.getQueryState(aggregateKey)?.isInvalidated).toBe(true);
    expect(queryClient.getQueryState(scriptKey)?.isInvalidated).toBe(false);
  });
});
