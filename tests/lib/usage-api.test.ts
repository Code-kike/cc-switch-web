import { beforeEach, describe, expect, it, vi } from "vitest";
import { setCsrfToken } from "@/lib/api/adapter";
import { usageApi } from "@/lib/api/usage";
import "@/lib/api/web-commands";

describe("usageApi", () => {
  beforeEach(() => {
    setCsrfToken("test-csrf-token");
    Object.defineProperty(window, "__TAURI_INTERNALS__", {
      configurable: true,
      value: undefined,
    });
    Object.defineProperty(window, "__TAURI__", {
      configurable: true,
      value: undefined,
    });
  });

  it("returns a failed UsageResult when saved usage query returns a Web API error", async () => {
    const fetchMock = vi.spyOn(globalThis, "fetch").mockResolvedValueOnce(
      new Response(
        JSON.stringify({
          code: "BAD_REQUEST",
          message: "用量查询未启用",
        }),
        {
          status: 400,
          headers: { "Content-Type": "application/json" },
        },
      ),
    );

    await expect(usageApi.query("provider-1", "claude")).resolves.toEqual({
      success: false,
      data: undefined,
      error: "用量查询未启用",
    });

    expect(fetchMock).toHaveBeenCalledWith(
      "/api/providers/queryproviderusage",
      expect.objectContaining({
        method: "POST",
        body: JSON.stringify({
          providerId: "provider-1",
          app: "claude",
        }),
      }),
    );
  });

  it("returns a failed UsageResult when script testing returns a Web API error", async () => {
    vi.spyOn(globalThis, "fetch").mockResolvedValueOnce(
      new Response(
        JSON.stringify({
          code: "INTERNAL_ERROR",
          message: "Script request failed",
        }),
        {
          status: 500,
          headers: { "Content-Type": "application/json" },
        },
      ),
    );

    await expect(
      usageApi.testScript(
        "provider-1",
        "claude",
        "return { remaining: 1, unit: 'USD' }",
        10,
        "key",
        "https://api.example.com",
      ),
    ).resolves.toEqual({
      success: false,
      data: undefined,
      error: "Script request failed",
    });
  });
});
