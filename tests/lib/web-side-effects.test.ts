import { beforeEach, describe, expect, it, vi } from "vitest";
import { providersApi } from "@/lib/api/providers";
import { settingsApi } from "@/lib/api/settings";

describe("web side-effect adapters", () => {
  beforeEach(() => {
    Object.defineProperty(window, "__TAURI_INTERNALS__", {
      configurable: true,
      value: undefined,
    });
    Object.defineProperty(window, "__TAURI__", {
      configurable: true,
      value: undefined,
    });
  });

  it("treats tray menu refresh as a no-op in web mode", async () => {
    await expect(providersApi.updateTrayMenu()).resolves.toBe(true);
  });

  it("opens external links in the browser in web mode", async () => {
    const openSpy = vi.spyOn(window, "open").mockReturnValue(null);

    await settingsApi.openExternal("https://example.com/docs");

    expect(openSpy).toHaveBeenCalledWith(
      "https://example.com/docs",
      "_blank",
      "noopener,noreferrer",
    );
  });
});
