import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterAll, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";

import "@/lib/api/web-commands";
import { LogConfigPanel } from "@/components/settings/LogConfigPanel";
import { ModelTestConfigPanel } from "@/components/usage/ModelTestConfigPanel";
import { setCsrfToken } from "@/lib/api/adapter";
import {
  getStreamCheckConfig,
  saveStreamCheckConfig,
  type StreamCheckConfig,
} from "@/lib/api/model-test";
import { settingsApi } from "@/lib/api/settings";
import { server } from "../msw/server";
import {
  startTestWebServer,
  type TestWebServer,
} from "../helpers/web-server";

const timeoutRegex = /^(streamCheck\.timeout|超时时间（秒）|Timeout \(seconds\))$/;
const claudeModelRegex =
  /^(streamCheck\.claudeModel|Claude 模型|Claude Model)$/;
const testPromptRegex =
  /^(streamCheck\.testPrompt|检查提示词|Test Prompt)$/;
const saveRegex = /^(common\.save|保存|Save)$/;
const logDebugRegex =
  /^(settings\.advanced\.logConfig\.levels\.debug|调试|Debug)$/;

const toastSuccessMock = vi.fn();
const toastErrorMock = vi.fn();

vi.mock("sonner", () => ({
  toast: {
    success: (...args: unknown[]) => toastSuccessMock(...args),
    error: (...args: unknown[]) => toastErrorMock(...args),
  },
}));

const initialStreamCheckConfig: StreamCheckConfig = {
  timeoutSecs: 45,
  maxRetries: 2,
  degradedThresholdMs: 6000,
  claudeModel: "claude-haiku-4-5-20251001",
  codexModel: "gpt-5.4@low",
  geminiModel: "gemini-3-flash-preview",
  testPrompt: "Who are you?",
};

describe.sequential("Advanced config panels against real web server", () => {
  let webServer: TestWebServer;

  beforeAll(async () => {
    server.close();
    webServer = await startTestWebServer();
  }, 360_000);

  afterAll(async () => {
    await webServer.stop();
    server.listen({ onUnhandledRequest: "warn" });
  }, 20_000);

  beforeEach(async () => {
    toastSuccessMock.mockReset();
    toastErrorMock.mockReset();
    setCsrfToken(null);

    Object.defineProperty(window, "__TAURI_INTERNALS__", {
      configurable: true,
      value: undefined,
    });
    Object.defineProperty(window, "__TAURI__", {
      configurable: true,
      value: undefined,
    });
    Object.defineProperty(window, "__CC_SWITCH_API_BASE__", {
      configurable: true,
      value: webServer.baseUrl,
    });
    Object.defineProperty(window, "scrollTo", {
      configurable: true,
      value: vi.fn(),
    });
    Object.defineProperty(HTMLElement.prototype, "scrollIntoView", {
      configurable: true,
      value: vi.fn(),
    });

    await saveStreamCheckConfig(initialStreamCheckConfig);
    await settingsApi.setLogConfig({ enabled: true, level: "info" });
  });

  it("loads, saves, and reloads the stream-check config through the rendered model test panel", async () => {
    const { unmount } = render(<ModelTestConfigPanel />);

    const timeout = await screen.findByLabelText(timeoutRegex);
    const claudeModel = screen.getByLabelText(claudeModelRegex);
    const testPrompt = screen.getByLabelText(testPromptRegex);

    expect(timeout).toHaveValue(45);
    expect(claudeModel).toHaveValue("claude-haiku-4-5-20251001");
    expect(testPrompt).toHaveValue("Who are you?");

    fireEvent.change(timeout, { target: { value: "77" } });
    fireEvent.change(claudeModel, {
      target: { value: "rendered-smoke-claude-model" },
    });
    fireEvent.change(testPrompt, {
      target: { value: "Return the rendered smoke marker." },
    });
    fireEvent.click(screen.getByRole("button", { name: saveRegex }));

    await waitFor(async () => {
      expect(await getStreamCheckConfig()).toEqual({
        ...initialStreamCheckConfig,
        timeoutSecs: 77,
        claudeModel: "rendered-smoke-claude-model",
        testPrompt: "Return the rendered smoke marker.",
      });
    });

    unmount();
    render(<ModelTestConfigPanel />);

    expect(await screen.findByLabelText(timeoutRegex)).toHaveValue(77);
    expect(screen.getByLabelText(claudeModelRegex)).toHaveValue(
      "rendered-smoke-claude-model",
    );
    expect(screen.getByLabelText(testPromptRegex)).toHaveValue(
      "Return the rendered smoke marker.",
    );
  });

  it("loads, saves, and reloads the log config through the rendered log config panel", async () => {
    const { unmount } = render(<LogConfigPanel />);

    const enabledSwitch = await screen.findByRole("switch");
    const levelSelect = screen.getByRole("combobox");

    expect(enabledSwitch).toHaveAttribute("aria-checked", "true");

    fireEvent.click(enabledSwitch);

    await waitFor(async () => {
      expect(await settingsApi.getLogConfig()).toEqual({
        enabled: false,
        level: "info",
      });
    });

    fireEvent.click(enabledSwitch);

    await waitFor(async () => {
      expect(await settingsApi.getLogConfig()).toEqual({
        enabled: true,
        level: "info",
      });
    });

    fireEvent.click(levelSelect);
    fireEvent.click(await screen.findByText(logDebugRegex));

    await waitFor(async () => {
      expect(await settingsApi.getLogConfig()).toEqual({
        enabled: true,
        level: "debug",
      });
    });

    unmount();
    render(<LogConfigPanel />);

    expect(await screen.findByRole("switch")).toHaveAttribute(
      "aria-checked",
      "true",
    );
    expect(screen.getByRole("combobox")).toHaveTextContent(logDebugRegex);
  });
});
