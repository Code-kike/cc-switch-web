import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke, setCsrfToken } from "@/lib/api/adapter";
import { WebNotSupportedError } from "@/lib/api/errors";
import "@/lib/api/web-commands";

describe("web adapter DELETE encoding", () => {
  beforeEach(() => {
    setCsrfToken("test-csrf-token");
    Object.defineProperty(window, "__TAURI_INTERNALS__", {
      configurable: true,
      value: undefined,
    });
  });

  it("sends delete_sessions as JSON body in web mode", async () => {
    const fetchMock = vi
      .spyOn(globalThis, "fetch")
      .mockResolvedValueOnce(
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
          "X-CSRF-Token": "test-csrf-token",
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
    await expect(invoke("open_config_folder", { app: "claude" })).rejects.toBeInstanceOf(
      WebNotSupportedError,
    );
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
