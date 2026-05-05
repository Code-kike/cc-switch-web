import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useCommonConfigSnippet } from "@/components/providers/forms/hooks/useCommonConfigSnippet";
import { useCodexCommonConfig } from "@/components/providers/forms/hooks/useCodexCommonConfig";
import { useGeminiCommonConfig } from "@/components/providers/forms/hooks/useGeminiCommonConfig";

const getCommonConfigSnippetMock = vi.fn();
const setCommonConfigSnippetMock = vi.fn();
const extractCommonConfigSnippetMock = vi.fn();

vi.mock("@/lib/api", () => ({
  configApi: {
    getCommonConfigSnippet: (...args: unknown[]) =>
      getCommonConfigSnippetMock(...args),
    setCommonConfigSnippet: (...args: unknown[]) =>
      setCommonConfigSnippetMock(...args),
    extractCommonConfigSnippet: (...args: unknown[]) =>
      extractCommonConfigSnippetMock(...args),
  },
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (
      key: string,
      options?: {
        error?: string;
      },
    ) => (options?.error ? `${key}:${options.error}` : key),
  }),
}));

describe("common config snippet saving", () => {
  beforeEach(() => {
    getCommonConfigSnippetMock.mockReset();
    setCommonConfigSnippetMock.mockReset();
    extractCommonConfigSnippetMock.mockReset();
    getCommonConfigSnippetMock.mockResolvedValue("");
    setCommonConfigSnippetMock.mockResolvedValue(undefined);
    extractCommonConfigSnippetMock.mockResolvedValue("");
  });

  it("shows structured details when Claude common config save or extract fails", async () => {
    const onConfigChange = vi.fn();
    const { result } = renderHook(() =>
      useCommonConfigSnippet({
        settingsConfig: JSON.stringify({}),
        onConfigChange,
      }),
    );

    await waitFor(() => expect(result.current.isLoading).toBe(false));

    setCommonConfigSnippetMock.mockRejectedValueOnce({
      message: "claude save failed",
    });
    act(() => {
      result.current.handleCommonConfigSnippetChange(
        JSON.stringify({ includeCoAuthoredBy: true }),
      );
    });

    await waitFor(() =>
      expect(result.current.commonConfigError).toBe(
        "claudeConfig.saveFailed:claude save failed",
      ),
    );

    extractCommonConfigSnippetMock.mockRejectedValueOnce({
      message: "claude extract failed",
    });
    await act(async () => {
      await result.current.handleExtract();
    });

    await waitFor(() =>
      expect(result.current.commonConfigError).toBe(
        "claudeConfig.extractFailed:claude extract failed",
      ),
    );
  });

  it("does not persist an invalid Codex common config snippet", async () => {
    const onConfigChange = vi.fn();
    const { result } = renderHook(() =>
      useCodexCommonConfig({
        codexConfig: "model = \"gpt-5\"",
        onConfigChange,
      }),
    );

    await waitFor(() => expect(result.current.isLoading).toBe(false));

    let saved = false;
    act(() => {
      saved = result.current.handleCommonConfigSnippetChange(
        "base_url = https://bad.example/v1",
      );
    });

    expect(saved).toBe(false);
    expect(setCommonConfigSnippetMock).not.toHaveBeenCalled();
    expect(onConfigChange).not.toHaveBeenCalled();
    expect(result.current.commonConfigError).toContain("invalid value");
  });

  it("shows structured details when Codex common config save or extract fails", async () => {
    const onConfigChange = vi.fn();
    const { result } = renderHook(() =>
      useCodexCommonConfig({
        codexConfig: "model = \"gpt-5\"",
        onConfigChange,
      }),
    );

    await waitFor(() => expect(result.current.isLoading).toBe(false));

    setCommonConfigSnippetMock.mockRejectedValueOnce({
      message: "codex save failed",
    });
    act(() => {
      result.current.handleCommonConfigSnippetChange(
        "model = \"gpt-5\"\nbase_url = \"https://api.example.com\"",
      );
    });

    await waitFor(() =>
      expect(result.current.commonConfigError).toBe(
        "codexConfig.saveFailed:codex save failed",
      ),
    );

    extractCommonConfigSnippetMock.mockRejectedValueOnce({
      message: "codex extract failed",
    });
    await act(async () => {
      await result.current.handleExtract();
    });

    await waitFor(() =>
      expect(result.current.commonConfigError).toBe(
        "codexConfig.extractFailed:codex extract failed",
      ),
    );
  });

  it("does not persist an invalid Gemini common config snippet", async () => {
    const onEnvChange = vi.fn();
    const { result } = renderHook(() =>
      useGeminiCommonConfig({
        envValue: "",
        onEnvChange,
        envStringToObj: () => ({}),
        envObjToString: () => "",
      }),
    );

    await waitFor(() => expect(result.current.isLoading).toBe(false));

    let saved = false;
    act(() => {
      saved = result.current.handleCommonConfigSnippetChange(
        JSON.stringify({ GEMINI_MODEL: 123 }),
      );
    });

    expect(saved).toBe(false);
    expect(setCommonConfigSnippetMock).not.toHaveBeenCalled();
    expect(onEnvChange).not.toHaveBeenCalled();
    expect(result.current.commonConfigError).toBe(
      "geminiConfig.commonConfigInvalidValues",
    );
  });

  it("shows structured details when Gemini common config save or extract fails", async () => {
    const onEnvChange = vi.fn();
    const { result } = renderHook(() =>
      useGeminiCommonConfig({
        envValue: "",
        onEnvChange,
        envStringToObj: () => ({}),
        envObjToString: () => "",
      }),
    );

    await waitFor(() => expect(result.current.isLoading).toBe(false));

    setCommonConfigSnippetMock.mockRejectedValueOnce({
      message: "gemini save failed",
    });
    act(() => {
      result.current.handleCommonConfigSnippetChange(
        JSON.stringify({ GEMINI_MODEL: "gemini-3-pro" }),
      );
    });

    await waitFor(() =>
      expect(result.current.commonConfigError).toBe(
        "geminiConfig.saveFailed:gemini save failed",
      ),
    );

    extractCommonConfigSnippetMock.mockRejectedValueOnce({
      message: "gemini extract failed",
    });
    await act(async () => {
      await result.current.handleExtract();
    });

    await waitFor(() =>
      expect(result.current.commonConfigError).toBe(
        "geminiConfig.extractFailed:gemini extract failed",
      ),
    );
  });
});
