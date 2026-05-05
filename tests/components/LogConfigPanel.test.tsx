import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import "@testing-library/jest-dom";

import { LogConfigPanel } from "@/components/settings/LogConfigPanel";

const toastErrorMock = vi.fn();

const { settingsApiMock } = vi.hoisted(() => ({
  settingsApiMock: {
    getLogConfig: vi.fn(),
    setLogConfig: vi.fn(),
  },
}));

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
    error: (...args: unknown[]) => toastErrorMock(...args),
  },
}));

vi.mock("@/components/ui/switch", () => ({
  Switch: ({ checked, onCheckedChange, ...props }: any) => (
    <button
      role="switch"
      aria-checked={checked}
      onClick={() => onCheckedChange?.(!checked)}
      {...props}
    />
  ),
}));

vi.mock("@/components/ui/label", () => ({
  Label: ({ children, ...props }: any) => <label {...props}>{children}</label>,
}));

vi.mock("@/components/ui/alert", () => ({
  Alert: ({ children }: any) => <div>{children}</div>,
  AlertDescription: ({ children }: any) => <div>{children}</div>,
}));

vi.mock("@/components/ui/select", () => ({
  Select: ({ value, onValueChange, children, disabled }: any) => (
    <select
      value={value}
      disabled={disabled}
      onChange={(event) => onValueChange?.(event.target.value)}
    >
      {children}
    </select>
  ),
  SelectTrigger: ({ children }: any) => <>{children}</>,
  SelectValue: () => null,
  SelectContent: ({ children }: any) => <>{children}</>,
  SelectItem: ({ value, children }: any) => (
    <option value={value}>{children}</option>
  ),
}));

vi.mock("@/lib/api/settings", async () => {
  const actual = await vi.importActual("@/lib/api/settings");
  return {
    ...actual,
    settingsApi: settingsApiMock,
  };
});

describe("LogConfigPanel", () => {
  let consoleErrorSpy: ReturnType<typeof vi.spyOn>;

  beforeEach(() => {
    toastErrorMock.mockReset();
    settingsApiMock.getLogConfig.mockReset();
    settingsApiMock.setLogConfig.mockReset();
    consoleErrorSpy = vi.spyOn(console, "error").mockImplementation(() => {});

    settingsApiMock.getLogConfig.mockResolvedValue({
      enabled: true,
      level: "warn",
    });
    settingsApiMock.setLogConfig.mockResolvedValue(true);
  });

  afterEach(() => {
    consoleErrorSpy.mockRestore();
  });

  it("loads the saved config and persists level changes", async () => {
    render(<LogConfigPanel />);

    await screen.findByRole("switch");
    const select = screen.getByRole("combobox");

    expect(screen.getByRole("switch")).toHaveAttribute("aria-checked", "true");
    expect(select).toHaveValue("warn");

    fireEvent.change(select, { target: { value: "debug" } });

    await waitFor(() => {
      expect(settingsApiMock.setLogConfig).toHaveBeenNthCalledWith(1, {
        enabled: true,
        level: "debug",
      });
      expect(screen.getByRole("combobox")).toHaveValue("debug");
    });
  });

  it("persists toggle changes and disables the level selector when logging is off", async () => {
    render(<LogConfigPanel />);

    const toggle = await screen.findByRole("switch");
    const select = screen.getByRole("combobox");

    fireEvent.click(toggle);

    await waitFor(() => {
      expect(settingsApiMock.setLogConfig).toHaveBeenNthCalledWith(1, {
        enabled: false,
        level: "warn",
      });
      expect(screen.getByRole("switch")).toHaveAttribute(
        "aria-checked",
        "false",
      );
      expect(select).toBeDisabled();
    });
  });

  it("reverts the optimistic update and shows a toast when saving fails", async () => {
    settingsApiMock.setLogConfig.mockRejectedValueOnce({
      detail: "save failed",
    });

    render(<LogConfigPanel />);

    const select = await screen.findByRole("combobox");
    expect(select).toHaveValue("warn");

    fireEvent.change(select, { target: { value: "trace" } });

    await waitFor(() => {
      expect(toastErrorMock).toHaveBeenCalledWith(
        "settings.advanced.logConfig.saveFailed:save failed",
      );
    });
    expect(select).toHaveValue("warn");
  });

  it("shows a destructive load error instead of rendering fallback defaults", async () => {
    settingsApiMock.getLogConfig.mockRejectedValue({
      message: "load failed",
    });

    render(<LogConfigPanel />);

    expect(
      await screen.findByText(
        "settings.advanced.logConfig.loadFailed:load failed",
      ),
    ).toBeInTheDocument();
    expect(screen.queryByRole("switch")).not.toBeInTheDocument();
    expect(screen.queryByRole("combobox")).not.toBeInTheDocument();
  });
});
