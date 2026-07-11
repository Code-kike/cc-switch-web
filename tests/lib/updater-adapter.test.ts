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

  // L11 / H3: this fork has no independent release channel, and the upstream
  // update-check would steer users to upstream desktop binaries. In web mode
  // checkForUpdates must report "no update available" WITHOUT querying the
  // upstream repo.
  it("reports no update and does not query the upstream update API", async () => {
    const { checkForUpdates } = await import("@/lib/api/updater-adapter");

    await expect(checkForUpdates()).resolves.toEqual({
      available: false,
      isWebMode: true,
    });
    expect(webJsonFetchMock).not.toHaveBeenCalled();
  });
});
