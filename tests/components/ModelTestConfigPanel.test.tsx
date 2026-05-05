import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import "@testing-library/jest-dom";

import { ModelTestConfigPanel } from "@/components/usage/ModelTestConfigPanel";

const toastSuccessMock = vi.fn();
const toastErrorMock = vi.fn();

const { getStreamCheckConfigMock, saveStreamCheckConfigMock } = vi.hoisted(
  () => ({
    getStreamCheckConfigMock: vi.fn(),
    saveStreamCheckConfigMock: vi.fn(),
  }),
);

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, params?: Record<string, unknown>) =>
      params && typeof params.error === "string"
        ? `${key}:${params.error}`
        : key,
  }),
}));

vi.mock("sonner", () => ({
  toast: {
    success: (...args: unknown[]) => toastSuccessMock(...args),
    error: (...args: unknown[]) => toastErrorMock(...args),
  },
}));

vi.mock("@/components/ui/button", () => ({
  Button: ({ children, ...props }: any) => <button {...props}>{children}</button>,
}));

vi.mock("@/components/ui/input", () => ({
  Input: (props: any) => <input {...props} />,
}));

vi.mock("@/components/ui/textarea", () => ({
  Textarea: (props: any) => <textarea {...props} />,
}));

vi.mock("@/components/ui/label", () => ({
  Label: ({ children, ...props }: any) => <label {...props}>{children}</label>,
}));

vi.mock("@/components/ui/alert", () => ({
  Alert: ({ children }: any) => <div>{children}</div>,
  AlertDescription: ({ children }: any) => <div>{children}</div>,
}));

vi.mock("@/lib/api/model-test", () => ({
  getStreamCheckConfig: () => getStreamCheckConfigMock(),
  saveStreamCheckConfig: (...args: unknown[]) =>
    saveStreamCheckConfigMock(...args),
}));

describe("ModelTestConfigPanel", () => {
  beforeEach(() => {
    toastSuccessMock.mockReset();
    toastErrorMock.mockReset();
    getStreamCheckConfigMock.mockReset();
    saveStreamCheckConfigMock.mockReset();

    getStreamCheckConfigMock.mockResolvedValue({
      timeoutSecs: 60,
      maxRetries: 3,
      degradedThresholdMs: 8000,
      claudeModel: "claude-test-model",
      codexModel: "codex-test-model",
      geminiModel: "gemini-test-model",
      testPrompt: "Reply with smoke-test",
    });
    saveStreamCheckConfigMock.mockResolvedValue(undefined);
  });

  it("loads the current config and saves edited values", async () => {
    render(<ModelTestConfigPanel />);

    const claudeModel = await screen.findByLabelText("streamCheck.claudeModel");
    const timeout = screen.getByLabelText("streamCheck.timeout");
    const prompt = screen.getByLabelText("streamCheck.testPrompt");

    expect(claudeModel).toHaveValue("claude-test-model");
    expect(timeout).toHaveValue(60);
    expect(prompt).toHaveValue("Reply with smoke-test");

    fireEvent.change(timeout, { target: { value: "75" } });
    fireEvent.change(prompt, { target: { value: "Updated prompt" } });

    fireEvent.click(screen.getByRole("button", { name: "common.save" }));

    await waitFor(() => {
      expect(saveStreamCheckConfigMock).toHaveBeenCalledWith({
        timeoutSecs: 75,
        maxRetries: 3,
        degradedThresholdMs: 8000,
        claudeModel: "claude-test-model",
        codexModel: "codex-test-model",
        geminiModel: "gemini-test-model",
        testPrompt: "Updated prompt",
      });
    });
    expect(toastSuccessMock).toHaveBeenCalledWith("streamCheck.configSaved", {
      closeButton: true,
    });
  });

  it("falls back to defaults when numeric fields are cleared and prompt is empty", async () => {
    render(<ModelTestConfigPanel />);

    const timeout = await screen.findByLabelText("streamCheck.timeout");
    const retries = screen.getByLabelText("streamCheck.maxRetries");
    const degraded = screen.getByLabelText("streamCheck.degradedThreshold");
    const prompt = screen.getByLabelText("streamCheck.testPrompt");

    fireEvent.change(timeout, { target: { value: "" } });
    fireEvent.change(retries, { target: { value: "" } });
    fireEvent.change(degraded, { target: { value: "" } });
    fireEvent.change(prompt, { target: { value: "" } });

    fireEvent.click(screen.getByRole("button", { name: "common.save" }));

    await waitFor(() => {
      expect(saveStreamCheckConfigMock).toHaveBeenCalledWith({
        timeoutSecs: 45,
        maxRetries: 2,
        degradedThresholdMs: 6000,
        claudeModel: "claude-test-model",
        codexModel: "codex-test-model",
        geminiModel: "gemini-test-model",
        testPrompt: "Who are you?",
      });
    });
  });

  it("shows extracted detail when loading the config fails", async () => {
    getStreamCheckConfigMock.mockRejectedValueOnce({
      detail: "config missing",
    });

    render(<ModelTestConfigPanel />);

    expect(
      await screen.findByText("streamCheck.loadFailed:config missing"),
    ).toBeInTheDocument();
  });

  it("shows extracted detail when saving fails", async () => {
    saveStreamCheckConfigMock.mockRejectedValueOnce({
      message: "cannot save",
    });

    render(<ModelTestConfigPanel />);

    await screen.findByLabelText("streamCheck.claudeModel");
    fireEvent.click(screen.getByRole("button", { name: "common.save" }));

    await waitFor(() => {
      expect(toastErrorMock).toHaveBeenCalledWith(
        "streamCheck.configSaveFailedDetail:cannot save",
      );
    });
  });
});
