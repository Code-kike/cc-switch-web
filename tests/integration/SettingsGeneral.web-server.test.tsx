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
const chineseLanguageRegex = /^(settings\.languageOptionChinese|中文)$/;
const englishLanguageRegex = /^(settings\.languageOptionEnglish|English)$/;

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
          defaultTab="general"
          onOpenChange={() => undefined}
        />
      </ThemeProvider>
    </QueryClientProvider>,
  );
}

describe.sequential("Settings general tab against real web server", () => {
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
    });
    window.localStorage.setItem("language", "zh");
  });

  it("auto-saves the selected language and reloads it through the rendered general settings page", async () => {
    const { unmount } = renderSettingsPage();

    const chineseButton = await screen.findByRole("button", {
      name: chineseLanguageRegex,
    });
    const englishButton = screen.getByRole("button", {
      name: englishLanguageRegex,
    });

    expect(chineseButton).toHaveClass("shadow-sm");
    expect(englishButton).not.toHaveClass("shadow-sm");

    fireEvent.click(englishButton);

    await waitFor(async () => {
      expect((await settingsApi.get()).language).toBe("en");
    });
    await waitFor(() => {
      expect(window.localStorage.getItem("language")).toBe("en");
    });
    await waitFor(() => {
      expect(
        screen.getByRole("button", { name: englishLanguageRegex }),
      ).toHaveClass(
        "shadow-sm",
      );
    });

    unmount();
    renderSettingsPage();

    expect(
      await screen.findByRole("button", { name: englishLanguageRegex }),
    ).toHaveClass("shadow-sm");
    expect(
      screen.getByRole("button", { name: chineseLanguageRegex }),
    ).not.toHaveClass("shadow-sm");
  });
});
