import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { usePromptActions } from "@/hooks/usePromptActions";
import type { Prompt } from "@/lib/api/prompts";

const toastSuccessMock = vi.fn();
const toastErrorMock = vi.fn();

const getPromptsMock = vi.fn();
const getCurrentFileContentMock = vi.fn();
const upsertPromptMock = vi.fn();
const deletePromptMock = vi.fn();
const enablePromptMock = vi.fn();
const importFromFileMock = vi.fn();

vi.mock("sonner", () => ({
  toast: {
    success: (...args: unknown[]) => toastSuccessMock(...args),
    error: (...args: unknown[]) => toastErrorMock(...args),
  },
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) => key,
  }),
}));

vi.mock("@/lib/api", () => ({
  promptsApi: {
    getPrompts: (...args: unknown[]) => getPromptsMock(...args),
    getCurrentFileContent: (...args: unknown[]) =>
      getCurrentFileContentMock(...args),
    upsertPrompt: (...args: unknown[]) => upsertPromptMock(...args),
    deletePrompt: (...args: unknown[]) => deletePromptMock(...args),
    enablePrompt: (...args: unknown[]) => enablePromptMock(...args),
    importFromFile: (...args: unknown[]) => importFromFileMock(...args),
  },
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
    toastSuccessMock.mockReset();
    toastErrorMock.mockReset();
    getPromptsMock.mockReset();
    getCurrentFileContentMock.mockReset();
    upsertPromptMock.mockReset();
    deletePromptMock.mockReset();
    enablePromptMock.mockReset();
    importFromFileMock.mockReset();
  });

  it("treats prompt import cancellation as a no-op in web mode", async () => {
    importFromFileMock.mockResolvedValue(null);
    const { result } = renderHook(() => usePromptActions("claude"));

    await act(async () => {
      await expect(result.current.importFromFile()).resolves.toBeNull();
    });

    expect(importFromFileMock).toHaveBeenCalledWith("claude");
    expect(getPromptsMock).not.toHaveBeenCalled();
    expect(getCurrentFileContentMock).not.toHaveBeenCalled();
    expect(toastSuccessMock).not.toHaveBeenCalled();
    expect(toastErrorMock).not.toHaveBeenCalled();
  });

  it("reloads prompt data and shows success after a prompt import succeeds", async () => {
    importFromFileMock.mockResolvedValue("imported-prompt");
    getPromptsMock.mockResolvedValue({
      "imported-prompt": {
        id: "imported-prompt",
        name: "Imported Prompt",
        content: "content",
        enabled: true,
      },
    });
    getCurrentFileContentMock.mockResolvedValue("# imported");

    const { result } = renderHook(() => usePromptActions("codex"));

    await act(async () => {
      await expect(result.current.importFromFile()).resolves.toBe(
        "imported-prompt",
      );
    });

    expect(importFromFileMock).toHaveBeenCalledWith("codex");
    expect(getPromptsMock).toHaveBeenCalledWith("codex");
    expect(getCurrentFileContentMock).toHaveBeenCalledWith("codex");
    expect(toastSuccessMock).toHaveBeenCalledWith("prompts.importSuccess", {
      closeButton: true,
    });
    expect(toastErrorMock).not.toHaveBeenCalled();
  });

  it("updates local prompt state immediately after saving and refreshes silently", async () => {
    upsertPromptMock.mockResolvedValue(undefined);
    getPromptsMock.mockResolvedValue({
      "gemini-smoke": {
        id: "gemini-smoke",
        name: "Gemini Smoke Prompt",
        content: "# GEMINI.md\n\nSaved content",
        description: "Saved description",
        enabled: true,
      },
    });
    getCurrentFileContentMock.mockResolvedValue("# GEMINI.md\n\nSaved content");

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
    expect(toastSuccessMock).toHaveBeenCalledWith("prompts.saveSuccess", {
      closeButton: true,
    });

    await waitFor(() => {
      expect(getPromptsMock).toHaveBeenCalledWith("gemini");
      expect(getCurrentFileContentMock).toHaveBeenCalledWith("gemini");
    });
  });

  it("shows extracted detail when loading prompt list fails", async () => {
    getPromptsMock.mockRejectedValue(new Error("prompt db unavailable"));

    const { result } = renderHook(() => usePromptActions("claude"));

    await act(async () => {
      await result.current.reload();
    });

    expect(toastErrorMock).toHaveBeenCalledWith("prompts.loadFailed", {
      description: "prompt db unavailable",
    });
  });

  it("keeps missing current prompt file silent but surfaces unexpected current-file read failures", async () => {
    getPromptsMock.mockResolvedValue({ "prompt-1": prompt });
    getCurrentFileContentMock.mockResolvedValueOnce(null);

    const { result } = renderHook(() => usePromptActions("claude"));

    await act(async () => {
      await result.current.reload();
    });

    expect(result.current.currentFileContent).toBeNull();
    expect(toastErrorMock).not.toHaveBeenCalled();

    toastErrorMock.mockReset();
    getCurrentFileContentMock.mockRejectedValueOnce(
      new Error("permission denied"),
    );

    await act(async () => {
      await result.current.reload();
    });

    expect(result.current.currentFileContent).toBeNull();
    expect(toastErrorMock).toHaveBeenCalledWith(
      "prompts.currentFileLoadFailed",
      {
        description: "permission denied",
      },
    );
  });

  it("shows extracted detail when save fails", async () => {
    upsertPromptMock.mockRejectedValue(new Error("save denied"));

    const { result } = renderHook(() => usePromptActions("claude"));

    await act(async () => {
      await expect(result.current.savePrompt(prompt.id, prompt)).rejects.toThrow(
        "save denied",
      );
    });

    expect(toastErrorMock).toHaveBeenCalledWith("prompts.saveFailed", {
      description: "save denied",
    });
  });

  it("shows extracted detail when deleting a prompt fails", async () => {
    deletePromptMock.mockRejectedValue(new Error("delete denied"));

    const { result } = renderHook(() => usePromptActions("claude"));

    await act(async () => {
      await expect(result.current.deletePrompt(prompt.id)).rejects.toThrow(
        "delete denied",
      );
    });

    expect(toastErrorMock).toHaveBeenCalledWith("prompts.deleteFailed", {
      description: "delete denied",
    });
  });

  it("shows extracted detail and rolls back when disabling a prompt fails", async () => {
    getPromptsMock.mockResolvedValue({ [prompt.id]: prompt });
    getCurrentFileContentMock.mockResolvedValue("# Prompt");
    upsertPromptMock.mockRejectedValue(new Error("disable denied"));

    const { result } = renderHook(() => usePromptActions("claude"));

    await act(async () => {
      await result.current.reload();
    });

    await act(async () => {
      await expect(result.current.toggleEnabled(prompt.id, false)).rejects.toThrow(
        "disable denied",
      );
    });

    expect(result.current.prompts[prompt.id]?.enabled).toBe(true);
    expect(toastErrorMock).toHaveBeenCalledWith("prompts.disableFailed", {
      description: "disable denied",
    });
  });

  it("shows extracted detail when importing a prompt fails", async () => {
    importFromFileMock.mockRejectedValue(new Error("import denied"));

    const { result } = renderHook(() => usePromptActions("claude"));

    await act(async () => {
      await expect(result.current.importFromFile()).rejects.toThrow(
        "import denied",
      );
    });

    expect(toastErrorMock).toHaveBeenCalledWith("prompts.importFailed", {
      description: "import denied",
    });
  });
});
