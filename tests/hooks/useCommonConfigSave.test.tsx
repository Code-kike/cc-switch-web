import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useCommonConfigSnippet } from "@/components/providers/forms/hooks/useCommonConfigSnippet";
import { useCodexCommonConfig } from "@/components/providers/forms/hooks/useCodexCommonConfig";
import { useGeminiCommonConfig } from "@/components/providers/forms/hooks/useGeminiCommonConfig";

const getCommonConfigSnippetMock = vi.fn();
const setCommonConfigSnippetMock = vi.fn();
const extractCommonConfigSnippetMock = vi.fn();
const updateTomlCommonConfigSnippetMock = vi.fn();

vi.mock("@/lib/api", () => ({
  configApi: {
    getCommonConfigSnippet: (...args: unknown[]) =>
      getCommonConfigSnippetMock(...args),
    setCommonConfigSnippet: (...args: unknown[]) =>
      setCommonConfigSnippetMock(...args),
    extractCommonConfigSnippet: (...args: unknown[]) =>
      extractCommonConfigSnippetMock(...args),
    updateTomlCommonConfigSnippet: (...args: unknown[]) =>
      updateTomlCommonConfigSnippetMock(...args),
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
    updateTomlCommonConfigSnippetMock.mockReset();
    getCommonConfigSnippetMock.mockResolvedValue("");
    setCommonConfigSnippetMock.mockResolvedValue(undefined);
    extractCommonConfigSnippetMock.mockResolvedValue("");
    updateTomlCommonConfigSnippetMock.mockImplementation(
      async (configToml: string) => configToml,
    );
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
        codexConfig: 'model = "gpt-5"',
        onConfigChange,
      }),
    );

    await waitFor(() => expect(result.current.isLoading).toBe(false));

    let saved = true;
    await act(async () => {
      saved = await result.current.handleCommonConfigSnippetChange(
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
        codexConfig: 'model = "gpt-5"',
        onConfigChange,
      }),
    );

    await waitFor(() => expect(result.current.isLoading).toBe(false));

    setCommonConfigSnippetMock.mockRejectedValueOnce({
      message: "codex save failed",
    });
    await act(async () => {
      await result.current.handleCommonConfigSnippetChange(
        'model = "gpt-5"\nbase_url = "https://api.example.com"',
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

  it("keeps the latest Codex toggle when backend TOML operations resolve out of order", async () => {
    getCommonConfigSnippetMock.mockResolvedValue(
      "[tui]\nnotifications = true\n",
    );
    const onConfigChange = vi.fn();
    const { result } = renderHook(() =>
      useCodexCommonConfig({
        codexConfig: 'model = "gpt-5"',
        onConfigChange,
        initialData: { settingsConfig: { config: 'model = "gpt-5"' } },
        initialEnabled: false,
      }),
    );

    await waitFor(() => expect(result.current.isLoading).toBe(false));
    await waitFor(() => expect(result.current.useCommonConfig).toBe(false));

    let resolveMerge: ((value: string) => void) | undefined;
    updateTomlCommonConfigSnippetMock
      .mockImplementationOnce(
        () =>
          new Promise<string>((resolve) => {
            resolveMerge = resolve;
          }),
      )
      .mockImplementationOnce(async (configToml: string) => configToml);

    await act(async () => {
      const mergePending = result.current.handleCommonConfigToggle(true);
      const removeDone = result.current.handleCommonConfigToggle(false);
      await removeDone;
      resolveMerge?.('model = "gpt-5"\n\n[tui]\nnotifications = true\n');
      await mergePending;
    });

    expect(result.current.useCommonConfig).toBe(false);
    expect(onConfigChange.mock.calls.at(-1)?.[0]).not.toContain("[tui]");
  });

  it("does not overwrite a manual Codex config edit with an in-flight merge", async () => {
    getCommonConfigSnippetMock.mockResolvedValue(
      "[tui]\nnotifications = true\n",
    );
    const initialData = { settingsConfig: { config: 'model = "gpt-5"' } };
    const onConfigChange = vi.fn();
    const { result, rerender } = renderHook(
      ({ config }: { config: string }) =>
        useCodexCommonConfig({
          codexConfig: config,
          onConfigChange,
          initialData,
          initialEnabled: false,
        }),
      { initialProps: { config: 'model = "gpt-5"' } },
    );

    await waitFor(() => expect(result.current.isLoading).toBe(false));
    await waitFor(() => expect(result.current.useCommonConfig).toBe(false));

    let resolveMerge: ((value: string) => void) | undefined;
    updateTomlCommonConfigSnippetMock.mockImplementationOnce(
      () =>
        new Promise<string>((resolve) => {
          resolveMerge = resolve;
        }),
    );

    let pending: Promise<void> = Promise.resolve();
    act(() => {
      pending = result.current.handleCommonConfigToggle(true);
    });
    rerender({ config: 'model = "gpt-6-user-edit"' });

    await act(async () => {
      resolveMerge?.('model = "gpt-5"\n\n[tui]\nnotifications = true\n');
      await pending;
    });

    expect(onConfigChange).not.toHaveBeenCalled();
    expect(result.current.useCommonConfig).toBe(false);
  });

  it("serializes durable Codex snippet saves so the latest invocation wins", async () => {
    const onConfigChange = vi.fn();
    const { result } = renderHook(() =>
      useCodexCommonConfig({
        codexConfig: 'model = "gpt-5"',
        onConfigChange,
        initialData: { settingsConfig: { config: 'model = "gpt-5"' } },
        initialEnabled: false,
      }),
    );
    await waitFor(() => expect(result.current.isLoading).toBe(false));
    await waitFor(() => expect(result.current.useCommonConfig).toBe(false));

    let resolveFirst: (() => void) | undefined;
    setCommonConfigSnippetMock
      .mockImplementationOnce(
        () =>
          new Promise<void>((resolve) => {
            resolveFirst = resolve;
          }),
      )
      .mockResolvedValueOnce(undefined);

    let firstSave: Promise<boolean> = Promise.resolve(false);
    let secondSave: Promise<boolean> = Promise.resolve(false);
    act(() => {
      firstSave = result.current.handleCommonConfigSnippetChange(
        "request_max_retries = 1\n",
      );
      secondSave = result.current.handleCommonConfigSnippetChange(
        "request_max_retries = 2\n",
      );
    });

    await waitFor(() =>
      expect(setCommonConfigSnippetMock).toHaveBeenCalledTimes(1),
    );
    expect(setCommonConfigSnippetMock).toHaveBeenNthCalledWith(
      1,
      "codex",
      "request_max_retries = 1\n",
    );

    await act(async () => {
      resolveFirst?.();
      await firstSave;
      await secondSave;
    });

    expect(await firstSave).toBe(false);
    expect(await secondSave).toBe(true);
    expect(setCommonConfigSnippetMock).toHaveBeenNthCalledWith(
      2,
      "codex",
      "request_max_retries = 2\n",
    );
    expect(result.current.commonConfigSnippet).toBe(
      "request_max_retries = 2\n",
    );
  });

  it("invalidates an old merge when switching presets with identical TOML", async () => {
    getCommonConfigSnippetMock.mockResolvedValue(
      "[tui]\nnotifications = true\n",
    );
    const initialData = { settingsConfig: { config: 'model = "gpt-5"' } };
    const onConfigChange = vi.fn();
    let resolveOldMerge: ((value: string) => void) | undefined;
    updateTomlCommonConfigSnippetMock
      .mockImplementationOnce(
        () =>
          new Promise<string>((resolve) => {
            resolveOldMerge = resolve;
          }),
      )
      .mockResolvedValueOnce(
        'model = "gpt-5"\n\n[tui]\nnotifications = true\n# preset-b',
      );

    const { result, rerender } = renderHook(
      ({ preset }: { preset: string }) =>
        useCodexCommonConfig({
          codexConfig: 'model = "gpt-5"',
          onConfigChange,
          initialData,
          initialEnabled: true,
          selectedPresetId: preset,
        }),
      { initialProps: { preset: "preset-a" } },
    );

    await waitFor(() => expect(result.current.isLoading).toBe(false));
    await waitFor(() =>
      expect(updateTomlCommonConfigSnippetMock).toHaveBeenCalledTimes(1),
    );

    rerender({ preset: "preset-b" });
    await waitFor(() =>
      expect(updateTomlCommonConfigSnippetMock).toHaveBeenCalledTimes(2),
    );
    await waitFor(() => expect(onConfigChange).toHaveBeenCalledTimes(1));

    await act(async () => {
      resolveOldMerge?.(
        'model = "gpt-5"\n\n[tui]\nnotifications = true\n# stale-preset-a',
      );
    });

    expect(onConfigChange).toHaveBeenCalledTimes(1);
    expect(onConfigChange).toHaveBeenCalledWith(
      expect.stringContaining("# preset-b"),
    );
    expect(onConfigChange).not.toHaveBeenCalledWith(
      expect.stringContaining("# stale-preset-a"),
    );
  });

  it("invalidates an in-flight Codex merge on unmount", async () => {
    getCommonConfigSnippetMock.mockResolvedValue(
      "[tui]\nnotifications = true\n",
    );
    let resolveMerge: ((value: string) => void) | undefined;
    updateTomlCommonConfigSnippetMock.mockImplementationOnce(
      () =>
        new Promise<string>((resolve) => {
          resolveMerge = resolve;
        }),
    );
    const onConfigChange = vi.fn();
    const { result, unmount } = renderHook(() =>
      useCodexCommonConfig({
        codexConfig: 'model = "gpt-5"',
        onConfigChange,
        initialData: { settingsConfig: { config: 'model = "gpt-5"' } },
        initialEnabled: true,
      }),
    );

    await waitFor(() => expect(result.current.isLoading).toBe(false));
    await waitFor(() =>
      expect(updateTomlCommonConfigSnippetMock).toHaveBeenCalledTimes(1),
    );
    unmount();

    await act(async () => {
      resolveMerge?.('model = "gpt-5"\n\n[tui]\nnotifications = true\n# stale');
    });
    expect(onConfigChange).not.toHaveBeenCalled();
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
