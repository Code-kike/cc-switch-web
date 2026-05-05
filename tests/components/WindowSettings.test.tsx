import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi, beforeEach } from "vitest";
import { WindowSettings } from "@/components/settings/WindowSettings";
import type { SettingsFormState } from "@/hooks/useSettingsForm";

const isWebModeMock = vi.fn();
const isLinuxMock = vi.fn();

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) => key,
  }),
}));

vi.mock("@/lib/api/adapter", () => ({
  isWebMode: () => isWebModeMock(),
}));

vi.mock("@/lib/platform", () => ({
  isLinux: () => isLinuxMock(),
}));

const baseSettings = (overrides: Partial<SettingsFormState> = {}): SettingsFormState =>
  ({
    showInTray: true,
    minimizeToTrayOnClose: true,
    useAppWindowControls: false,
    enableClaudePluginIntegration: false,
    skipClaudeOnboarding: false,
    language: "zh",
    ...overrides,
  }) as SettingsFormState;

describe("WindowSettings", () => {
  beforeEach(() => {
    isWebModeMock.mockReset();
    isLinuxMock.mockReset();
  });

  it("hides desktop-only toggles in web mode", () => {
    const onChange = vi.fn();
    isWebModeMock.mockReturnValue(true);
    isLinuxMock.mockReturnValue(false);

    render(
      <WindowSettings settings={baseSettings()} onChange={onChange} />,
    );

    expect(
      screen.queryByRole("switch", { name: "settings.launchOnStartup" }),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByRole("switch", { name: "settings.minimizeToTray" }),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByRole("switch", { name: "settings.useAppWindowControls" }),
    ).not.toBeInTheDocument();

    fireEvent.click(
      screen.getByRole("switch", {
        name: "settings.enableClaudePluginIntegration",
      }),
    );
    expect(onChange).toHaveBeenCalledWith({
      enableClaudePluginIntegration: true,
    });
  });

  it("shows desktop-only toggles on Linux and exposes silent startup when enabled", () => {
    const onChange = vi.fn();
    isWebModeMock.mockReturnValue(false);
    isLinuxMock.mockReturnValue(true);

    render(
      <WindowSettings
        settings={baseSettings({
          launchOnStartup: true,
          silentStartup: false,
        })}
        onChange={onChange}
      />,
    );

    expect(
      screen.getByRole("switch", { name: "settings.launchOnStartup" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("switch", { name: "settings.minimizeToTray" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("switch", { name: "settings.useAppWindowControls" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("switch", { name: "settings.silentStartup" }),
    ).toBeInTheDocument();

    fireEvent.click(
      screen.getByRole("switch", { name: "settings.silentStartup" }),
    );
    expect(onChange).toHaveBeenCalledWith({ silentStartup: true });
  });
});
