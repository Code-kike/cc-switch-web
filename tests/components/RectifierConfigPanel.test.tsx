import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import type { HTMLAttributes, ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { RectifierConfigPanel } from "@/components/settings/RectifierConfigPanel";

const getRectifierConfigMock = vi.fn();
const setRectifierConfigMock = vi.fn();
const getOptimizerConfigMock = vi.fn();
const setOptimizerConfigMock = vi.fn();
const toastErrorMock = vi.fn();
const translateMock = vi.fn(
  (key: string, options?: { error?: string }) =>
    options?.error ? `${key}: ${options.error}` : key,
);

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: translateMock,
  }),
}));

vi.mock("sonner", () => ({
  toast: {
    error: (...args: unknown[]) => toastErrorMock(...args),
  },
}));

vi.mock("@/lib/api/settings", () => ({
  settingsApi: {
    getRectifierConfig: (...args: unknown[]) => getRectifierConfigMock(...args),
    setRectifierConfig: (...args: unknown[]) => setRectifierConfigMock(...args),
    getOptimizerConfig: (...args: unknown[]) => getOptimizerConfigMock(...args),
    setOptimizerConfig: (...args: unknown[]) => setOptimizerConfigMock(...args),
  },
}));

vi.mock("@/components/ui/switch", () => ({
  Switch: ({
    checked,
    disabled,
    onCheckedChange,
  }: {
    checked: boolean;
    disabled?: boolean;
    onCheckedChange: (checked: boolean) => void;
  }) => (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      disabled={disabled}
      onClick={() => onCheckedChange(!checked)}
    >
      {String(checked)}
    </button>
  ),
}));

vi.mock("@/components/ui/alert", () => ({
  Alert: ({ children }: { children: ReactNode }) => (
    <div role="alert">{children}</div>
  ),
  AlertDescription: ({ children, className }: HTMLAttributes<HTMLDivElement>) => (
    <div className={className}>{children}</div>
  ),
}));

const defaultRectifierConfig = {
  enabled: true,
  requestThinkingSignature: true,
  requestThinkingBudget: true,
};

const defaultOptimizerConfig = {
  enabled: false,
  thinkingOptimizer: true,
  cacheInjection: true,
  cacheTtl: "1h",
};

describe("RectifierConfigPanel", () => {
  beforeEach(() => {
    getRectifierConfigMock.mockReset();
    setRectifierConfigMock.mockReset();
    getOptimizerConfigMock.mockReset();
    setOptimizerConfigMock.mockReset();
    toastErrorMock.mockReset();
    translateMock.mockClear();

    getRectifierConfigMock.mockResolvedValue(defaultRectifierConfig);
    getOptimizerConfigMock.mockResolvedValue(defaultOptimizerConfig);
  });

  it("shows a visible alert when loading either config fails", async () => {
    getOptimizerConfigMock.mockRejectedValue({
      payload: { detail: "optimizer load exploded" },
    });

    render(<RectifierConfigPanel />);

    await waitFor(() =>
      expect(screen.getByRole("alert")).toHaveTextContent(
        "settings.advanced.optimizer.loadFailed: optimizer load exploded",
      ),
    );
  });

  it("surfaces structured rectifier save errors and restores the previous state", async () => {
    setRectifierConfigMock.mockRejectedValue({
      payload: { detail: "rectifier save exploded" },
    });

    render(<RectifierConfigPanel />);

    await screen.findByText("settings.advanced.rectifier.enabled");

    const enabledSwitch = screen.getAllByRole("switch")[0];
    expect(enabledSwitch).toHaveAttribute("aria-checked", "true");

    fireEvent.click(enabledSwitch);

    await waitFor(() =>
      expect(setRectifierConfigMock).toHaveBeenCalledWith({
        enabled: false,
        requestThinkingSignature: true,
        requestThinkingBudget: true,
      }),
    );
    await waitFor(() =>
      expect(toastErrorMock).toHaveBeenCalledWith(
        "settings.advanced.rectifier.saveFailed: rectifier save exploded",
      ),
    );
    await waitFor(() =>
      expect(enabledSwitch).toHaveAttribute("aria-checked", "true"),
    );
  });

  it("surfaces structured optimizer save errors", async () => {
    setOptimizerConfigMock.mockRejectedValue({
      message: "optimizer save exploded",
    });

    render(<RectifierConfigPanel />);

    await screen.findByText("settings.advanced.rectifier.enabled");

    const optimizerEnabledSwitch = screen.getAllByRole("switch")[3];
    fireEvent.click(optimizerEnabledSwitch);

    await waitFor(() =>
      expect(setOptimizerConfigMock).toHaveBeenCalledWith({
        enabled: true,
        thinkingOptimizer: true,
        cacheInjection: true,
        cacheTtl: "1h",
      }),
    );
    await waitFor(() =>
      expect(toastErrorMock).toHaveBeenCalledWith(
        "settings.advanced.optimizer.saveFailed: optimizer save exploded",
      ),
    );
  });
});
