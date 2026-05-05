import { beforeEach, describe, expect, it, vi } from "vitest";

const webJsonFetchMock = vi.fn();

vi.mock("@/lib/api/adapter", () => ({
  isWebMode: () => true,
  webJsonFetch: (...args: unknown[]) => webJsonFetchMock(...args),
}));

vi.mock("@/lib/api/updater-adapter", () => ({
  checkForUpdates: vi.fn(),
}));

describe("getCurrentVersion in web mode", () => {
  beforeEach(() => {
    webJsonFetchMock.mockReset();
  });

  it("reads the version from /api/health", async () => {
    webJsonFetchMock.mockResolvedValue({ version: "3.14.1" });

    const { getCurrentVersion } = await import("@/lib/updater");

    await expect(getCurrentVersion()).resolves.toBe("3.14.1");
    expect(webJsonFetchMock).toHaveBeenCalledWith("/api/health");
  });

  it("returns an empty string when the health probe fails", async () => {
    webJsonFetchMock.mockRejectedValue(new Error("network error"));

    const { getCurrentVersion } = await import("@/lib/updater");

    await expect(getCurrentVersion()).resolves.toBe("");
  });
});
