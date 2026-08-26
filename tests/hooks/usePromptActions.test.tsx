import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { usePromptActions } from "@/hooks/usePromptActions";
import type { AppId, Prompt } from "@/lib/api";

const mocks = vi.hoisted(() => ({
  getPrompts: vi.fn(),
  getCurrentFileContent: vi.fn(),
  enablePrompt: vi.fn(),
  upsertPrompt: vi.fn(),
  deletePrompt: vi.fn(),
  importFromFile: vi.fn(),
  toastError: vi.fn(),
  toastSuccess: vi.fn(),
}));

vi.mock("@/lib/api", () => ({
  promptsApi: {
    getPrompts: mocks.getPrompts,
    getCurrentFileContent: mocks.getCurrentFileContent,
    enablePrompt: mocks.enablePrompt,
    upsertPrompt: mocks.upsertPrompt,
    deletePrompt: mocks.deletePrompt,
    importFromFile: mocks.importFromFile,
  },
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) => key,
  }),
}));

describe("usePromptActions", () => {
  const prompt: Prompt = {
    id: "prompt-1",
    name: "Smoke Prompt",
    content: "# Prompt",
    description: "Prompt description",
    enabled: true,
  };

  beforeEach(() => {
    mocks.toastSuccess.mockReset();
    mocks.toastError.mockReset();
    mocks.getPrompts.mockReset();
    mocks.getCurrentFileContent.mockReset();
    mocks.upsertPrompt.mockReset();
    mocks.deletePrompt.mockReset();
    mocks.enablePrompt.mockReset();
    mocks.importFromFile.mockReset();
  });

  it("treats prompt import cancellation as a no-op in web mode", async () => {
    mocks.importFromFile.mockResolvedValue(null);
    const { result } = renderHook(() => usePromptActions("claude"));

    await act(async () => {
      await expect(result.current.importFromFile()).resolves.toBeNull();
    });

    expect(mocks.importFromFile).toHaveBeenCalledWith("claude");
    expect(mocks.getPrompts).not.toHaveBeenCalled();
    expect(mocks.getCurrentFileContent).not.toHaveBeenCalled();
    expect(mocks.toastSuccess).not.toHaveBeenCalled();
    expect(mocks.toastError).not.toHaveBeenCalled();
  });

  it("reloads prompt data and shows success after a prompt import succeeds", async () => {
    mocks.importFromFile.mockResolvedValue("imported-prompt");
    mocks.getPrompts.mockResolvedValue({
      "imported-prompt": {
        id: "imported-prompt",
        name: "Imported Prompt",
        content: "content",
        enabled: true,
      },
    });
    mocks.getCurrentFileContent.mockResolvedValue("# imported");

    const { result } = renderHook(() => usePromptActions("codex"));

    await act(async () => {
      await expect(result.current.importFromFile()).resolves.toBe(
        "imported-prompt",
      );
    });

    expect(mocks.importFromFile).toHaveBeenCalledWith("codex");
    expect(mocks.getPrompts).toHaveBeenCalledWith("codex");
    expect(mocks.getCurrentFileContent).toHaveBeenCalledWith("codex");
    expect(mocks.toastSuccess).toHaveBeenCalledWith("prompts.importSuccess", {
      closeButton: true,
    });
    expect(mocks.toastError).not.toHaveBeenCalled();
  });

  it("updates local prompt state immediately after saving and refreshes silently", async () => {
    mocks.upsertPrompt.mockResolvedValue(undefined);
    mocks.getPrompts.mockResolvedValue({
      "gemini-smoke": {
        id: "gemini-smoke",
        name: "Gemini Smoke Prompt",
        content: "# GEMINI.md\n\nSaved content",
        description: "Saved description",
        enabled: true,
      },
    });
    mocks.getCurrentFileContent.mockResolvedValue(
      "# GEMINI.md\n\nSaved content",
    );

    const { result } = renderHook(() => usePromptActions("gemini"));
    const prompt = {
      id: "gemini-smoke",
      name: "Gemini Smoke Prompt",
      content: "# GEMINI.md\n\nSaved content",
      description: "Saved description",
      enabled: true,
    };

    await act(async () => {
      await result.current.savePrompt("gemini-smoke", prompt);
    });

    expect(result.current.prompts["gemini-smoke"]).toEqual(prompt);
    expect(result.current.currentFileContent).toBe(prompt.content);
    expect(mocks.toastSuccess).toHaveBeenCalledWith("prompts.saveSuccess", {
      closeButton: true,
    });

    await waitFor(() => {
      expect(mocks.getPrompts).toHaveBeenCalledWith("gemini");
      expect(mocks.getCurrentFileContent).toHaveBeenCalledWith("gemini");
    });
  });

  it("shows extracted detail when loading prompt list fails", async () => {
    mocks.getPrompts.mockRejectedValue(new Error("prompt db unavailable"));

    const { result } = renderHook(() => usePromptActions("claude"));

    await act(async () => {
      await result.current.reload();
    });

    expect(mocks.toastError).toHaveBeenCalledWith("prompts.loadFailed", {
      description: "prompt db unavailable",
    });
  });

  it("keeps missing current prompt file silent but surfaces unexpected current-file read failures", async () => {
    mocks.getPrompts.mockResolvedValue({ "prompt-1": prompt });
    mocks.getCurrentFileContent.mockResolvedValueOnce(null);

    const { result } = renderHook(() => usePromptActions("claude"));

    await act(async () => {
      await result.current.reload();
    });

    expect(result.current.currentFileContent).toBeNull();
    expect(mocks.toastError).not.toHaveBeenCalled();

    mocks.toastError.mockReset();
    mocks.getCurrentFileContent.mockRejectedValueOnce(
      new Error("permission denied"),
    );

    await act(async () => {
      await result.current.reload();
    });

    expect(result.current.currentFileContent).toBeNull();
    expect(mocks.toastError).toHaveBeenCalledWith(
      "prompts.currentFileLoadFailed",
      {
        description: "permission denied",
      },
    );
  });

  it("shows extracted detail when save fails", async () => {
    mocks.upsertPrompt.mockRejectedValue(new Error("save denied"));

    const { result } = renderHook(() => usePromptActions("claude"));

    await act(async () => {
      await expect(
        result.current.savePrompt(prompt.id, prompt),
      ).rejects.toThrow("save denied");
    });

    expect(mocks.toastError).toHaveBeenCalledWith("prompts.saveFailed", {
      description: "save denied",
    });
  });

  it("shows extracted detail when deleting a prompt fails", async () => {
    mocks.deletePrompt.mockRejectedValue(new Error("delete denied"));

    const { result } = renderHook(() => usePromptActions("claude"));

    await act(async () => {
      await expect(result.current.deletePrompt(prompt.id)).rejects.toThrow(
        "delete denied",
      );
    });

    expect(mocks.toastError).toHaveBeenCalledWith("prompts.deleteFailed", {
      description: "delete denied",
    });
  });

  it("shows extracted detail and rolls back when disabling a prompt fails", async () => {
    mocks.getPrompts.mockResolvedValue({ [prompt.id]: prompt });
    mocks.getCurrentFileContent.mockResolvedValue("# Prompt");
    mocks.upsertPrompt.mockRejectedValue(new Error("disable denied"));

    const { result } = renderHook(() => usePromptActions("claude"));

    await act(async () => {
      await result.current.reload();
    });

    await act(async () => {
      await expect(
        result.current.toggleEnabled(prompt.id, false),
      ).rejects.toThrow("disable denied");
    });

    expect(result.current.prompts[prompt.id]?.enabled).toBe(true);
    expect(mocks.toastError).toHaveBeenCalledWith("prompts.disableFailed", {
      description: "disable denied",
    });
  });

  it("shows extracted detail when importing a prompt fails", async () => {
    mocks.importFromFile.mockRejectedValue(new Error("import denied"));

    const { result } = renderHook(() => usePromptActions("claude"));

    await act(async () => {
      await expect(result.current.importFromFile()).rejects.toThrow(
        "import denied",
      );
    });

    expect(mocks.toastError).toHaveBeenCalledWith("prompts.importFailed", {
      description: "import denied",
    });
  });
});

vi.mock("sonner", () => ({
  toast: {
    error: mocks.toastError,
    success: mocks.toastSuccess,
  },
}));

interface Deferred<T> {
  promise: Promise<T>;
  resolve: (value: T | PromiseLike<T>) => void;
  reject: (reason?: unknown) => void;
}

function createDeferred<T>(): Deferred<T> {
  let resolve!: Deferred<T>["resolve"];
  let reject!: Deferred<T>["reject"];
  const promise = new Promise<T>((resolvePromise, rejectPromise) => {
    resolve = resolvePromise;
    reject = rejectPromise;
  });
  return { promise, resolve, reject };
}

function makePrompts(id: string, name: string): Record<string, Prompt> {
  return {
    [id]: {
      id,
      name,
      content: `${name} content`,
      enabled: false,
    },
  };
}

function renderPromptActions(initialAppId: AppId) {
  return renderHook(({ appId }: { appId: AppId }) => usePromptActions(appId), {
    initialProps: { appId: initialAppId },
  });
}

describe("usePromptActions reload concurrency", () => {
  beforeEach(() => {
    mocks.getPrompts.mockReset();
    mocks.getCurrentFileContent.mockReset();
    mocks.getCurrentFileContent.mockResolvedValue(null);
    mocks.enablePrompt.mockReset();
    mocks.enablePrompt.mockResolvedValue(undefined);
    mocks.upsertPrompt.mockReset();
    mocks.upsertPrompt.mockResolvedValue(undefined);
    mocks.deletePrompt.mockReset();
    mocks.deletePrompt.mockResolvedValue(undefined);
    mocks.importFromFile.mockReset();
    mocks.importFromFile.mockResolvedValue(null);
    mocks.toastError.mockReset();
    mocks.toastSuccess.mockReset();
  });

  it("does not let an older app request overwrite the current app", async () => {
    const claudeRequest = createDeferred<Record<string, Prompt>>();
    const codexRequest = createDeferred<Record<string, Prompt>>();
    mocks.getPrompts.mockImplementation((appId: AppId) =>
      appId === "claude" ? claudeRequest.promise : codexRequest.promise,
    );
    mocks.getCurrentFileContent.mockImplementation(
      async (appId: AppId) => `${appId} live content`,
    );

    const { result, rerender } = renderPromptActions("claude");
    let claudeReload!: Promise<boolean>;
    act(() => {
      claudeReload = result.current.reload();
    });

    rerender({ appId: "codex" });
    let codexReload!: Promise<boolean>;
    act(() => {
      codexReload = result.current.reload();
    });

    codexRequest.resolve(makePrompts("codex-prompt", "Codex Prompt"));
    await act(async () => {
      await codexReload;
    });

    expect(result.current.prompts).toEqual(
      makePrompts("codex-prompt", "Codex Prompt"),
    );
    expect(result.current.currentFileContent).toBe("codex live content");

    claudeRequest.resolve(makePrompts("claude-prompt", "Claude Prompt"));
    await act(async () => {
      await claudeReload;
    });

    expect(result.current.prompts).toEqual(
      makePrompts("codex-prompt", "Codex Prompt"),
    );
    expect(result.current.currentFileContent).toBe("codex live content");
    expect(mocks.getCurrentFileContent).toHaveBeenCalledTimes(1);
    expect(mocks.getCurrentFileContent).toHaveBeenCalledWith("codex");
  });

  it("keeps the newer result when same-app reloads finish out of order", async () => {
    const olderRequest = createDeferred<Record<string, Prompt>>();
    const newerRequest = createDeferred<Record<string, Prompt>>();
    mocks.getPrompts
      .mockReturnValueOnce(olderRequest.promise)
      .mockReturnValueOnce(newerRequest.promise);
    mocks.getCurrentFileContent.mockResolvedValue("latest live content");

    const { result } = renderPromptActions("claude");
    let olderReload!: Promise<boolean>;
    let newerReload!: Promise<boolean>;
    act(() => {
      olderReload = result.current.reload();
      newerReload = result.current.reload();
    });

    newerRequest.resolve(makePrompts("newer-prompt", "Newer Prompt"));
    await act(async () => {
      await newerReload;
    });

    olderRequest.resolve(makePrompts("older-prompt", "Older Prompt"));
    await act(async () => {
      await olderReload;
    });

    expect(result.current.prompts).toEqual(
      makePrompts("newer-prompt", "Newer Prompt"),
    );
    expect(result.current.currentFileContent).toBe("latest live content");
    expect(mocks.getCurrentFileContent).toHaveBeenCalledTimes(1);
  });

  it("ignores an older request error while the current app is loading", async () => {
    const claudeRequest = createDeferred<Record<string, Prompt>>();
    const codexRequest = createDeferred<Record<string, Prompt>>();
    mocks.getPrompts.mockImplementation((appId: AppId) =>
      appId === "claude" ? claudeRequest.promise : codexRequest.promise,
    );

    const { result, rerender } = renderPromptActions("claude");
    let claudeReload!: Promise<boolean>;
    act(() => {
      claudeReload = result.current.reload();
    });

    rerender({ appId: "codex" });
    let codexReload!: Promise<boolean>;
    act(() => {
      codexReload = result.current.reload();
    });
    await waitFor(() => expect(result.current.loading).toBe(true));

    claudeRequest.reject(new Error("stale Claude failure"));
    await act(async () => {
      await claudeReload;
    });

    expect(result.current.loading).toBe(true);
    expect(mocks.toastError).not.toHaveBeenCalled();

    codexRequest.resolve(makePrompts("codex-prompt", "Codex Prompt"));
    await act(async () => {
      await codexReload;
    });

    expect(result.current.loading).toBe(false);
    expect(result.current.prompts).toEqual(
      makePrompts("codex-prompt", "Codex Prompt"),
    );
    expect(mocks.toastError).not.toHaveBeenCalled();
  });

  it("does not show an error when a pending reload fails after unmount", async () => {
    const request = createDeferred<Record<string, Prompt>>();
    mocks.getPrompts.mockReturnValue(request.promise);

    const { result, unmount } = renderPromptActions("claude");
    let reload!: Promise<boolean>;
    act(() => {
      reload = result.current.reload();
    });

    unmount();
    request.reject(new Error("failure after unmount"));
    await act(async () => {
      await reload;
    });

    expect(mocks.toastError).not.toHaveBeenCalled();
  });

  it("hides the previous app prompts when the new app reload fails", async () => {
    const claudePrompts = makePrompts("claude-prompt", "Claude Prompt");
    mocks.getPrompts
      .mockResolvedValueOnce(claudePrompts)
      .mockRejectedValueOnce(new Error("Codex load failed"));
    mocks.getCurrentFileContent.mockResolvedValueOnce("claude live content");

    const { result, rerender } = renderPromptActions("claude");
    await act(async () => {
      expect(await result.current.reload()).toBe(true);
    });
    expect(result.current.prompts).toEqual(claudePrompts);
    expect(result.current.currentFileContent).toBe("claude live content");

    rerender({ appId: "codex" });
    expect(result.current.prompts).toEqual({});
    expect(result.current.currentFileContent).toBeNull();

    await act(async () => {
      expect(await result.current.reload()).toBe(false);
    });

    expect(result.current.loading).toBe(false);
    expect(result.current.prompts).toEqual({});
    expect(result.current.currentFileContent).toBeNull();
    expect(mocks.toastError).toHaveBeenCalledWith("prompts.loadFailed", {
      description: "Codex load failed",
    });
  });

  it("does not roll back the current app when an older app toggle fails", async () => {
    const claudePrompts = makePrompts("claude-prompt", "Claude Prompt");
    const codexPrompts = makePrompts("codex-prompt", "Codex Prompt");
    const enableRequest = createDeferred<void>();
    mocks.getPrompts.mockImplementation(async (appId: AppId) =>
      appId === "claude" ? claudePrompts : codexPrompts,
    );
    mocks.enablePrompt.mockReturnValueOnce(enableRequest.promise);

    const { result, rerender } = renderPromptActions("claude");
    await act(async () => {
      expect(await result.current.reload()).toBe(true);
    });

    let togglePromise!: Promise<boolean>;
    act(() => {
      togglePromise = result.current.toggleEnabled("claude-prompt", true);
    });
    await waitFor(() => {
      expect(mocks.enablePrompt).toHaveBeenCalledWith(
        "claude",
        "claude-prompt",
      );
    });

    rerender({ appId: "codex" });
    await act(async () => {
      expect(await result.current.reload()).toBe(true);
    });
    expect(result.current.prompts).toEqual(codexPrompts);

    enableRequest.reject(new Error("stale Claude toggle failed"));
    await act(async () => {
      await expect(togglePromise).rejects.toThrow("stale Claude toggle failed");
    });

    expect(result.current.prompts).toEqual(codexPrompts);
    expect(result.current.currentFileContent).toBeNull();
  });

  it("keeps a saved prompt locally when the follow-up reload fails", async () => {
    const initialPrompts = makePrompts("existing", "Existing Prompt");
    const savedPrompt: Prompt = {
      id: "saved",
      name: "Saved Prompt",
      content: "Saved content",
      enabled: false,
    };
    mocks.getPrompts
      .mockResolvedValueOnce(initialPrompts)
      .mockRejectedValueOnce(new Error("refresh failed"));

    const { result } = renderPromptActions("claude");
    await act(async () => {
      expect(await result.current.reload()).toBe(true);
      expect(await result.current.savePrompt("saved", savedPrompt)).toBe(false);
    });

    expect(mocks.upsertPrompt).toHaveBeenCalledWith(
      "claude",
      "saved",
      savedPrompt,
    );
    expect(result.current.prompts).toEqual({
      ...initialPrompts,
      saved: savedPrompt,
    });
    expect(mocks.toastSuccess).toHaveBeenCalledWith("prompts.saveSuccess", {
      closeButton: true,
    });
  });

  it("keeps a deleted prompt removed when the follow-up reload fails", async () => {
    const initialPrompts = {
      ...makePrompts("keep", "Keep Prompt"),
      ...makePrompts("remove", "Remove Prompt"),
    };
    mocks.getPrompts
      .mockResolvedValueOnce(initialPrompts)
      .mockRejectedValueOnce(new Error("refresh failed"));

    const { result } = renderPromptActions("claude");
    await act(async () => {
      expect(await result.current.reload()).toBe(true);
      expect(await result.current.deletePrompt("remove")).toBe(false);
    });

    expect(mocks.deletePrompt).toHaveBeenCalledWith("claude", "remove");
    expect(result.current.prompts).toEqual(makePrompts("keep", "Keep Prompt"));
    expect(mocks.toastSuccess).toHaveBeenCalledWith("prompts.deleteSuccess", {
      closeButton: true,
    });
  });

  it("keeps an optimistic toggle when the follow-up reload fails", async () => {
    const initialPrompts = makePrompts("toggle", "Toggle Prompt");
    mocks.getPrompts
      .mockResolvedValueOnce(initialPrompts)
      .mockRejectedValueOnce(new Error("refresh failed"));

    const { result } = renderPromptActions("claude");
    await act(async () => {
      expect(await result.current.reload()).toBe(true);
    });
    await act(async () => {
      expect(await result.current.toggleEnabled("toggle", true)).toBe(false);
    });

    expect(mocks.enablePrompt).toHaveBeenCalledWith("claude", "toggle");
    expect(result.current.prompts.toggle.enabled).toBe(true);
    expect(mocks.toastSuccess).toHaveBeenCalledWith("prompts.enableSuccess", {
      closeButton: true,
    });
  });
});

describe("usePromptActions pi toggle", () => {
  beforeEach(() => {
    mocks.getPrompts.mockReset();
    mocks.getCurrentFileContent.mockReset();
    mocks.getCurrentFileContent.mockResolvedValue(null);
    mocks.enablePrompt.mockReset();
    mocks.upsertPrompt.mockReset();
    mocks.toastSuccess.mockReset();
    mocks.toastError.mockReset();
  });

  it("serializes the AGENTS.md write and reports the reload notice", async () => {
    const initialPrompts = makePrompts("agents", "Agents Prompt");
    const enabled = {
      agents: { ...initialPrompts.agents, enabled: true },
    };
    mocks.getPrompts
      .mockResolvedValueOnce(initialPrompts)
      .mockResolvedValueOnce(enabled);
    const gate = createDeferred<void>();
    mocks.enablePrompt.mockReturnValue(gate.promise);

    const { result } = renderPromptActions("pi");
    await act(async () => {
      expect(await result.current.reload()).toBe(true);
    });

    let toggle!: Promise<boolean>;
    act(() => {
      toggle = result.current.toggleEnabled("agents", true);
    });
    // Pi has no optimistic update: the list stays on the real file state and
    // the write is surfaced through togglingId.
    await waitFor(() => expect(result.current.togglingId).toBe("agents"));
    expect(result.current.prompts.agents.enabled).toBe(false);

    await act(async () => {
      gate.resolve();
      expect(await toggle).toBe(true);
    });

    expect(mocks.enablePrompt).toHaveBeenCalledWith("pi", "agents");
    expect(result.current.togglingId).toBeNull();
    expect(result.current.prompts.agents.enabled).toBe(true);
    expect(mocks.toastSuccess).toHaveBeenCalledWith(
      "pi.prompts.usePromptSuccess",
      { closeButton: true, description: "pi.prompts.reloadNotice" },
    );
  });

  it("stops using AGENTS.md through an upsert and clears togglingId on failure", async () => {
    const initialPrompts = {
      agents: {
        ...makePrompts("agents", "Agents Prompt").agents,
        enabled: true,
      },
    };
    mocks.getPrompts.mockResolvedValue(initialPrompts);
    mocks.upsertPrompt.mockRejectedValueOnce(new Error("models.json changed"));

    const { result } = renderPromptActions("pi");
    await act(async () => {
      expect(await result.current.reload()).toBe(true);
    });

    await act(async () => {
      await expect(
        result.current.toggleEnabled("agents", false),
      ).rejects.toThrow("models.json changed");
    });

    expect(mocks.upsertPrompt).toHaveBeenCalledWith("pi", "agents", {
      ...initialPrompts.agents,
      enabled: false,
    });
    expect(result.current.togglingId).toBeNull();
    expect(mocks.toastError).toHaveBeenCalledWith("prompts.disableFailed", {
      description: "models.json changed",
    });
  });

  it("appends the reload notice only when a pi save activates AGENTS.md", async () => {
    mocks.getPrompts.mockResolvedValue({});
    const { result } = renderPromptActions("pi");
    const active: Prompt = {
      id: "agents",
      name: "Agents Prompt",
      content: "# body",
      enabled: true,
    };

    await act(async () => {
      await result.current.savePrompt("agents", active);
    });
    expect(mocks.toastSuccess).toHaveBeenLastCalledWith("prompts.saveSuccess", {
      closeButton: true,
      description: "pi.prompts.reloadNotice",
    });

    await act(async () => {
      await result.current.savePrompt("agents", { ...active, enabled: false });
    });
    expect(mocks.toastSuccess).toHaveBeenLastCalledWith("prompts.saveSuccess", {
      closeButton: true,
      description: undefined,
    });
  });

  it("keeps the optimistic toggle for non-pi apps", async () => {
    const initialPrompts = makePrompts("toggle", "Toggle Prompt");
    mocks.getPrompts.mockResolvedValue(initialPrompts);
    const gate = createDeferred<void>();
    mocks.enablePrompt.mockReturnValue(gate.promise);

    const { result } = renderPromptActions("claude");
    await act(async () => {
      expect(await result.current.reload()).toBe(true);
    });

    let toggle!: Promise<boolean>;
    act(() => {
      toggle = result.current.toggleEnabled("toggle", true);
    });
    await waitFor(() =>
      expect(result.current.prompts.toggle.enabled).toBe(true),
    );
    expect(result.current.togglingId).toBeNull();

    await act(async () => {
      gate.resolve();
      await toggle;
    });
  });
});
