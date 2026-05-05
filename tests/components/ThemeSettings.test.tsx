import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { ThemeProvider } from "@/components/theme-provider";
import { ThemeSettings } from "@/components/settings/ThemeSettings";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) => key,
  }),
}));

vi.mock("@/lib/api/adapter", () => ({
  isWebMode: () => true,
  invoke: vi.fn(),
}));

describe("ThemeSettings", () => {
  beforeEach(() => {
    window.localStorage.clear();
    document.documentElement.classList.remove("light", "dark");
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
  });

  it("persists theme changes and updates the document class", async () => {
    render(
      <ThemeProvider defaultTheme="light" storageKey="test-theme">
        <ThemeSettings />
      </ThemeProvider>,
    );

    await waitFor(() => {
      expect(
        screen.getByRole("button", { name: "settings.themeLight" }),
      ).toHaveClass("shadow-sm");
      expect(document.documentElement).toHaveClass("light");
      expect(window.localStorage.getItem("test-theme")).toBe("light");
    });

    fireEvent.click(screen.getByRole("button", { name: "settings.themeDark" }));

    await waitFor(() => {
      expect(window.localStorage.getItem("test-theme")).toBe("dark");
    });
    expect(document.documentElement).toHaveClass("dark");
    expect(screen.getByRole("button", { name: "settings.themeDark" })).toHaveClass(
      "shadow-sm",
    );
  });
});
