import { beforeEach, describe, expect, it, vi } from "vitest";
import { setGlobalProxyUrl } from "@/lib/api/globalProxy";

const invokeMock = vi.fn();

vi.mock("@/lib/api/adapter", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

describe("globalProxy API", () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  it("preserves structured detail from invoke failures", async () => {
    invokeMock.mockRejectedValueOnce({ detail: "proxy payload failed" });

    await expect(setGlobalProxyUrl("http://127.0.0.1:7890")).rejects.toThrow(
      "proxy payload failed",
    );
  });

  it("falls back to a stable generic message when invoke gives no detail", async () => {
    invokeMock.mockRejectedValueOnce({});

    await expect(setGlobalProxyUrl("http://127.0.0.1:7890")).rejects.toThrow(
      "Failed to set global proxy URL",
    );
  });
});
