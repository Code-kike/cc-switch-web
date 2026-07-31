import { beforeEach, describe, expect, it, vi } from "vitest";
import { profilesApi } from "@/lib/api/profiles";
import "@/lib/api/web-commands";

describe("profilesApi Web adapter parity", () => {
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

  it("maps list/create/update/apply to their JSON Web routes", async () => {
    const fetchMock = vi
      .spyOn(globalThis, "fetch")
      .mockResolvedValueOnce(
        new Response(
          JSON.stringify({
            profiles: [],
            currentIds: { claude: null, codex: null },
          }),
          { status: 200, headers: { "Content-Type": "application/json" } },
        ),
      )
      .mockResolvedValueOnce(
        new Response(
          JSON.stringify({
            id: "p1",
            name: "Project 1",
            payload: {
              providers: { claude: null, codex: null },
              mcp: { claude: [], codex: null },
              skills: { claude: [], codex: null },
              prompts: { claude: null, codex: null },
            },
          }),
          { status: 200, headers: { "Content-Type": "application/json" } },
        ),
      )
      .mockResolvedValueOnce(
        new Response(
          JSON.stringify({
            id: "p1",
            name: "Renamed",
            payload: {
              providers: { claude: null, codex: null },
              mcp: { claude: [], codex: null },
              skills: { claude: [], codex: null },
              prompts: { claude: null, codex: null },
            },
          }),
          { status: 200, headers: { "Content-Type": "application/json" } },
        ),
      )
      .mockResolvedValueOnce(
        new Response(JSON.stringify(["warning"]), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        }),
      );

    await profilesApi.list();
    await profilesApi.create("Project 1", "claude");
    await profilesApi.update("p1", { name: "Renamed" });
    await expect(profilesApi.apply("p1", "codex")).resolves.toEqual([
      "warning",
    ]);

    expect(fetchMock).toHaveBeenNthCalledWith(
      1,
      "/api/profiles/list-profiles",
      expect.objectContaining({ method: "GET" }),
    );
    expect(fetchMock).toHaveBeenNthCalledWith(
      2,
      "/api/profiles/create-profile",
      expect.objectContaining({
        method: "POST",
        body: JSON.stringify({ name: "Project 1", scope: "claude" }),
      }),
    );
    expect(fetchMock).toHaveBeenNthCalledWith(
      3,
      "/api/profiles/update-profile",
      expect.objectContaining({
        method: "PUT",
        body: JSON.stringify({ id: "p1", name: "Renamed" }),
      }),
    );
    expect(fetchMock).toHaveBeenNthCalledWith(
      4,
      "/api/profiles/apply-profile",
      expect.objectContaining({
        method: "POST",
        body: JSON.stringify({ id: "p1", scope: "codex" }),
      }),
    );
  });

  it("encodes delete and scoped clear arguments as DELETE query parameters", async () => {
    const fetchMock = vi
      .spyOn(globalThis, "fetch")
      .mockResolvedValueOnce(
        new Response("true", {
          status: 200,
          headers: { "Content-Type": "application/json" },
        }),
      )
      .mockResolvedValueOnce(
        new Response("true", {
          status: 200,
          headers: { "Content-Type": "application/json" },
        }),
      );

    await profilesApi.delete("project with spaces");
    await profilesApi.clearCurrent("codex");

    expect(fetchMock).toHaveBeenNthCalledWith(
      1,
      "/api/profiles/delete-profile?id=project+with+spaces",
      expect.objectContaining({ method: "DELETE", body: undefined }),
    );
    expect(fetchMock).toHaveBeenNthCalledWith(
      2,
      "/api/profiles/clear-current-profile?scope=codex",
      expect.objectContaining({ method: "DELETE", body: undefined }),
    );
  });
});
