import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { AppVisibilitySettings } from "@/components/settings/AppVisibilitySettings";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) => key,
  }),
}));

vi.mock("@/components/ProviderIcon", () => ({
  ProviderIcon: () => <span data-testid="provider-icon" />,
}));

describe("AppVisibilitySettings", () => {
  it("toggles app visibility and preserves the last visible app", () => {
    const onChange = vi.fn();

    render(
      <AppVisibilitySettings
        settings={{
          showInTray: true,
          minimizeToTrayOnClose: true,
          language: "zh",
          visibleApps: {
            claude: true,
            codex: true,
            gemini: true,
            opencode: true,
            openclaw: true,
            hermes: true,
          },
        }}
        onChange={onChange}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: "apps.claude" }));

    expect(onChange).toHaveBeenCalledWith({
      visibleApps: expect.objectContaining({
        claude: false,
        codex: true,
        gemini: true,
        opencode: true,
        openclaw: true,
        hermes: true,
      }),
    });
  });

  it("disables the final visible app toggle", () => {
    const onChange = vi.fn();

    render(
      <AppVisibilitySettings
        settings={{
          showInTray: true,
          minimizeToTrayOnClose: true,
          language: "zh",
          visibleApps: {
            claude: true,
            codex: false,
            gemini: false,
            opencode: false,
            openclaw: false,
            hermes: false,
          },
        }}
        onChange={onChange}
      />,
    );

    const claudeButton = screen.getByRole("button", { name: "apps.claude" });
    expect(claudeButton).toBeDisabled();

    fireEvent.click(claudeButton);
    expect(onChange).not.toHaveBeenCalled();
  });
});
