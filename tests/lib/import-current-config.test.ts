import { beforeEach, describe, expect, it, vi } from "vitest";
import { importCurrentProviderConfig } from "@/lib/providers/import-current-config";
import { providersApi } from "@/lib/api/providers";

describe("importCurrentProviderConfig", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  it("uses generic default import for Claude/Codex/Gemini", async () => {
    const importDefaultSpy = vi
      .spyOn(providersApi, "importDefault")
      .mockResolvedValue(true);

    await expect(importCurrentProviderConfig("claude")).resolves.toBe(
      "imported",
    );
    await expect(importCurrentProviderConfig("codex")).resolves.toBe(
      "imported",
    );
    await expect(importCurrentProviderConfig("gemini")).resolves.toBe(
      "imported",
    );

    expect(importDefaultSpy).toHaveBeenNthCalledWith(1, "claude");
    expect(importDefaultSpy).toHaveBeenNthCalledWith(2, "codex");
    expect(importDefaultSpy).toHaveBeenNthCalledWith(3, "gemini");
  });

  it("uses OpenCode live import for additive-mode OpenCode", async () => {
    const importOpenCodeSpy = vi
      .spyOn(providersApi, "importOpenCodeFromLive")
      .mockResolvedValue(2);
    const importDefaultSpy = vi.spyOn(providersApi, "importDefault");

    await expect(importCurrentProviderConfig("opencode")).resolves.toBe(
      "imported",
    );

    expect(importOpenCodeSpy).toHaveBeenCalledTimes(1);
    expect(importDefaultSpy).not.toHaveBeenCalled();
  });

  it("uses OpenClaw live import for additive-mode OpenClaw", async () => {
    const importOpenClawSpy = vi
      .spyOn(providersApi, "importOpenClawFromLive")
      .mockResolvedValue(1);
    const importDefaultSpy = vi.spyOn(providersApi, "importDefault");

    await expect(importCurrentProviderConfig("openclaw")).resolves.toBe(
      "imported",
    );

    expect(importOpenClawSpy).toHaveBeenCalledTimes(1);
    expect(importDefaultSpy).not.toHaveBeenCalled();
  });

  it("uses Hermes live import for additive-mode Hermes", async () => {
    const importHermesSpy = vi
      .spyOn(providersApi, "importHermesFromLive")
      .mockResolvedValue(1);
    const importDefaultSpy = vi.spyOn(providersApi, "importDefault");

    await expect(importCurrentProviderConfig("hermes")).resolves.toBe(
      "imported",
    );

    expect(importHermesSpy).toHaveBeenCalledTimes(1);
    expect(importDefaultSpy).not.toHaveBeenCalled();
  });

  it("returns false when additive-mode imports find nothing", async () => {
    vi.spyOn(providersApi, "importOpenCodeFromLive").mockResolvedValue(0);
    vi.spyOn(providersApi, "importOpenClawFromLive").mockResolvedValue(0);
    vi.spyOn(providersApi, "importHermesFromLive").mockResolvedValue(0);

    await expect(importCurrentProviderConfig("opencode")).resolves.toBe(
      "no-change",
    );
    await expect(importCurrentProviderConfig("openclaw")).resolves.toBe(
      "no-change",
    );
    await expect(importCurrentProviderConfig("hermes")).resolves.toBe(
      "no-change",
    );
  });
});
