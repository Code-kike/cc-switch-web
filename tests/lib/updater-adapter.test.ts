import { beforeEach, describe, expect, it, vi } from "vitest";

const webJsonFetchMock = vi.fn();

vi.mock("@/lib/api/adapter", () => ({
  isWebMode: () => true,
  webJsonFetch: (...args: unknown[]) => webJsonFetchMock(...args),
}));

describe("checkForUpdates in web mode", () => {
  beforeEach(() => {
    vi.resetModules();
    webJsonFetchMock.mockReset();
  });

  it("reads structured update metadata from the local web API", async () => {
    webJsonFetchMock.mockResolvedValue({
      available: true,
      version: "3.15.0",
      notes: "latest notes",
      downloadUrl: "https://downloads.example.com/cc-switch/v3.15.0",
    });

    const { checkForUpdates } = await import("@/lib/api/updater-adapter");

    await expect(checkForUpdates()).resolves.toEqual({
      available: true,
      version: "3.15.0",
      notes: "latest notes",
      downloadUrl: "https://downloads.example.com/cc-switch/v3.15.0",
      isWebMode: true,
    });
    expect(webJsonFetchMock).toHaveBeenCalledWith("/api/system/get_update_info");
  });

  it("falls back to unavailable when the local web API probe fails", async () => {
    webJsonFetchMock.mockRejectedValue(new Error("request failed"));

    const { checkForUpdates } = await import("@/lib/api/updater-adapter");

    await expect(checkForUpdates()).resolves.toEqual({
      available: false,
      isWebMode: true,
    });
  });
});
