import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { invoke, pickWebFile } from "@/lib/api/adapter";
import { WebNotSupportedError } from "@/lib/api/errors";
import "@/lib/api/web-commands";

afterEach(() => {
  vi.useRealTimers();
  document
    .querySelectorAll('input[type="file"]')
    .forEach((input) => input.remove());
});

describe("web adapter DELETE encoding", () => {
  beforeEach(() => {
    Object.defineProperty(window, "__TAURI_INTERNALS__", {
      configurable: true,
      value: undefined,
    });
  });

  it("sends delete_sessions as JSON body in web mode", async () => {
    const fetchMock = vi.spyOn(globalThis, "fetch").mockResolvedValueOnce(
      new Response(JSON.stringify([{ sessionId: "s1", success: true }]), {
        status: 200,
        headers: { "Content-Type": "application/json" },
      }),
    );

    await invoke("delete_sessions", {
      items: [
        {
          providerId: "codex",
          sessionId: "s1",
          sourcePath: "/tmp/s1.jsonl",
        },
      ],
    });

    expect(fetchMock).toHaveBeenCalledWith(
      "/api/sessions/delete-sessions",
      expect.objectContaining({
        method: "DELETE",
        body: JSON.stringify({
          items: [
            {
              providerId: "codex",
              sessionId: "s1",
              sourcePath: "/tmp/s1.jsonl",
            },
          ],
        }),
        credentials: "include",
        headers: expect.objectContaining({
          Accept: "application/json",
          "Content-Type": "application/json",
        }),
      }),
    );
  });

  it("throws structured WebNotSupportedError before fetch for desktop-only commands", async () => {
    const fetchMock = vi.spyOn(globalThis, "fetch").mockImplementation(() => {
      throw new Error("fetch should not be called");
    });

    await expect(invoke("open_app_config_folder")).rejects.toMatchObject({
      name: "WebNotSupportedError",
      command: "open_app_config_folder",
      code: "WEB_NOT_SUPPORTED",
    });
    await expect(
      invoke("open_config_folder", { app: "claude" }),
    ).rejects.toBeInstanceOf(WebNotSupportedError);
    await expect(
      invoke("open_provider_terminal", {
        providerId: "provider-1",
        app: "claude",
      }),
    ).rejects.toBeInstanceOf(WebNotSupportedError);
    await expect(
      invoke("open_workspace_directory", { subdir: "workspace" }),
    ).rejects.toBeInstanceOf(WebNotSupportedError);
    await expect(
      invoke("pick_directory", { defaultPath: "/tmp" }),
    ).rejects.toBeInstanceOf(WebNotSupportedError);

    expect(fetchMock).not.toHaveBeenCalled();
  });
});

describe("pickWebFile", () => {
  it("resolves null and removes the temporary input when the picker is canceled", async () => {
    const result = pickWebFile(".sql");
    const input =
      document.querySelector<HTMLInputElement>('input[type="file"]');

    expect(input).not.toBeNull();
    input?.dispatchEvent(new Event("cancel"));

    await expect(result).resolves.toBeNull();
    expect(document.querySelector('input[type="file"]')).toBeNull();
  });

  it("resolves null after focus returns without a selected file", async () => {
    vi.useFakeTimers();

    const result = pickWebFile(".sql");
    expect(document.querySelector('input[type="file"]')).not.toBeNull();

    window.dispatchEvent(new Event("focus"));
    await vi.advanceTimersByTimeAsync(100);

    await expect(result).resolves.toBeNull();
    expect(document.querySelector('input[type="file"]')).toBeNull();
  });
});
