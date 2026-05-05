import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterAll, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";

import "@/lib/api/web-commands";
import { SettingsPage } from "@/components/settings/SettingsPage";
import { ThemeProvider } from "@/components/theme-provider";
import { setCsrfToken } from "@/lib/api/adapter";
import { settingsApi } from "@/lib/api/settings";
import { server } from "../msw/server";
import {
  startTestWebServer,
  type TestWebServer,
} from "../helpers/web-server";

const toastSuccessMock = vi.fn();
const toastErrorMock = vi.fn();

const configDirTitleRegex =
  /^(settings\.advanced\.configDir\.title|配置目录|Config Directory)$/;
const saveRegex = /^(common\.save|保存|Save)$/;
const restartRequiredRegex =
  /^(settings\.restartRequired|需要重启|Restart Required)$/;
const restartLaterRegex = /^(settings\.restartLater|稍后重启|Restart Later)$/;
const webManualPathHintRegex =
  /(settings\.webManualPathHint|手动输入路径|enter the path manually)/i;

vi.mock("sonner", () => ({
  toast: {
    success: (...args: unknown[]) => toastSuccessMock(...args),
    error: (...args: unknown[]) => toastErrorMock(...args),
  },
}));

function renderSettingsPage() {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  });

  return render(
    <QueryClientProvider client={queryClient}>
      <ThemeProvider defaultTheme="light">
        <SettingsPage
          open
          defaultTab="advanced"
          onOpenChange={() => undefined}
        />
      </ThemeProvider>
    </QueryClientProvider>,
  );
}

describe.sequential("DirectorySettings against real web server", () => {
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
    window.localStorage.clear();

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
    Object.defineProperty(window, "matchMedia", {
      configurable: true,
      value: vi.fn().mockImplementation(() => ({
        matches: false,
        media: "(prefers-color-scheme: dark)",
        onchange: null,
        addEventListener: vi.fn(),
        removeEventListener: vi.fn(),
        addListener: vi.fn(),
        removeListener: vi.fn(),
        dispatchEvent: vi.fn(),
      })),
    });

    const currentSettings = await settingsApi.get();
    await settingsApi.save({
      ...currentSettings,
      language: "zh",
      claudeConfigDir: undefined,
      codexConfigDir: undefined,
      geminiConfigDir: undefined,
      opencodeConfigDir: undefined,
      openclawConfigDir: undefined,
      hermesConfigDir: undefined,
    });
    await settingsApi.setAppConfigDirOverride(null);
    window.localStorage.setItem("language", "zh");
  });

  it(
    "keeps browse disabled in web mode and persists manual directory overrides through the rendered advanced page",
    async () => {
      const { unmount } = renderSettingsPage();

      fireEvent.click(await screen.findByText(configDirTitleRegex));

      const browseButtons = await screen.findAllByTitle(webManualPathHintRegex);
      expect(browseButtons.length).toBeGreaterThanOrEqual(7);
      for (const button of browseButtons) {
        expect(button).toBeDisabled();
      }

      const inputs = await screen.findAllByRole("textbox");
      const appInput = inputs[0] as HTMLInputElement;
      const claudeInput = inputs[1] as HTMLInputElement;

      expect(appInput.value).not.toBe("");
      expect(claudeInput.value).not.toBe("");

      fireEvent.change(appInput, {
        target: { value: "  /tmp/rendered-app-config  " },
      });
      fireEvent.change(claudeInput, {
        target: { value: "  /tmp/rendered-claude-config  " },
      });
      fireEvent.click(screen.getByRole("button", { name: saveRegex }));

      await waitFor(() => {
        expect(toastSuccessMock).toHaveBeenCalled();
      });
      await waitFor(async () => {
        expect(await settingsApi.getAppConfigDirOverride()).toBe(
          "/tmp/rendered-app-config",
        );
      });
      await waitFor(async () => {
        expect((await settingsApi.get()).claudeConfigDir).toBe(
          "/tmp/rendered-claude-config",
        );
      });

      expect(await screen.findByText(restartRequiredRegex)).toBeInTheDocument();
      fireEvent.click(
        screen.getByRole("button", { name: restartLaterRegex }),
      );
      await waitFor(() => {
        expect(
          screen.queryByText(restartRequiredRegex),
        ).not.toBeInTheDocument();
      });

      unmount();
      renderSettingsPage();

      fireEvent.click(await screen.findByText(configDirTitleRegex));

      await waitFor(() => {
        expect(
          screen.getByDisplayValue("/tmp/rendered-app-config"),
        ).toBeInTheDocument();
      });
      expect(
        screen.getByDisplayValue("/tmp/rendered-claude-config"),
      ).toBeInTheDocument();
    },
    20_000,
  );
});
