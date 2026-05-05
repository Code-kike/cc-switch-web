import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi, beforeEach } from "vitest";
import { DirectorySettings } from "@/components/settings/DirectorySettings";

describe("DirectorySettings", () => {
  beforeEach(() => {
    Object.defineProperty(window, "__TAURI_INTERNALS__", {
      configurable: true,
      value: undefined,
    });
  });

  it("disables browse buttons in web mode and keeps reset actions available", () => {
    const onBrowseAppConfig = vi.fn();
    const onResetAppConfig = vi.fn();
    const onBrowseDirectory = vi.fn();
    const onResetDirectory = vi.fn();

    render(
      <DirectorySettings
        appConfigDir="/app-config"
        resolvedDirs={{
          appConfig: "/app-config",
          claude: "/claude",
          codex: "/codex",
          gemini: "/gemini",
          opencode: "/opencode",
          openclaw: "/openclaw",
          hermes: "/hermes",
        }}
        onAppConfigChange={vi.fn()}
        onBrowseAppConfig={onBrowseAppConfig}
        onResetAppConfig={onResetAppConfig}
        claudeDir="/claude"
        codexDir="/codex"
        geminiDir="/gemini"
        opencodeDir="/opencode"
        openclawDir="/openclaw"
        hermesDir="/hermes"
        onDirectoryChange={vi.fn()}
        onBrowseDirectory={onBrowseDirectory}
        onResetDirectory={onResetDirectory}
      />,
    );

    const iconButtons = screen.getAllByRole("button");
    const browseButtons = iconButtons.filter(
      (button): button is HTMLButtonElement => button instanceof HTMLButtonElement,
    ).filter((button) => button.disabled);
    expect(browseButtons.length).toBeGreaterThanOrEqual(7);

    fireEvent.click(iconButtons[1]);
    expect(onResetAppConfig).toHaveBeenCalledTimes(1);

    fireEvent.click(iconButtons[3]);
    expect(onResetDirectory).toHaveBeenCalledTimes(1);

    expect(onBrowseAppConfig).not.toHaveBeenCalled();
    expect(onBrowseDirectory).not.toHaveBeenCalled();
  });
});
