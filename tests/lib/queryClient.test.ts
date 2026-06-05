import { describe, expect, it } from "vitest";
import { queryClient } from "@/lib/query/queryClient";

describe("queryClient defaults (M36 refresh amplification)", () => {
  it("uses a 30s global staleTime so focus/interval refetch don't double-fire", () => {
    const defaults = queryClient.getDefaultOptions();
    expect(defaults.queries?.staleTime).toBe(30_000);
  });

  it("keeps refetchOnWindowFocus enabled (gated by staleTime)", () => {
    const defaults = queryClient.getDefaultOptions();
    expect(defaults.queries?.refetchOnWindowFocus).toBe(true);
  });
});
