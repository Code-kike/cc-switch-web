import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import "@testing-library/jest-dom";
import { AboutSection } from "@/components/settings/AboutSection";

const toastSuccessMock = vi.fn();
const toastErrorMock = vi.fn();

const useUpdateMock = vi.fn();
const getCurrentVersionMock = vi.fn();
const relaunchAppMock = vi.fn();
const openExternalMock = vi.fn();
const getToolVersionsMock = vi.fn();
const isWindowsMock = vi.fn();

vi.mock("sonner", () => ({
  toast: {
    success: (...args: unknown[]) => toastSuccessMock(...args),
    error: (...args: unknown[]) => toastErrorMock(...args),
  },
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, params?: Record<string, unknown>) => {
      if (key === "settings.updateTo") {
        return `settings.updateTo:${String(params?.version ?? "")}`;
      }
      if (key === "settings.updateAvailable") {
        return `settings.updateAvailable:${String(params?.version ?? "")}`;
      }
      return key;
    },
  }),
}));

vi.mock("framer-motion", () => ({
  motion: {
    section: ({
      children,
      initial,
      animate,
      transition,
      whileHover,
      ...props
    }: any) => <section {...props}>{children}</section>,
    div: ({
      children,
      initial,
      animate,
      transition,
      whileHover,
      ...props
    }: any) => <div {...props}>{children}</div>,
  },
}));

vi.mock("@/assets/icons/app-icon.png", () => ({
  default: "app-icon.png",
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

vi.mock("@/contexts/UpdateContext", () => ({
  useUpdate: () => useUpdateMock(),
}));

vi.mock("@/lib/updater", () => ({
  getCurrentVersion: () => getCurrentVersionMock(),
  relaunchApp: () => relaunchAppMock(),
}));

vi.mock("@/lib/api", () => ({
  settingsApi: {
    openExternal: (...args: unknown[]) => openExternalMock(...args),
    getToolVersions: (...args: unknown[]) => getToolVersionsMock(...args),
    checkUpdates: vi.fn(),
  },
}));

vi.mock("@/lib/platform", () => ({
  isWindows: () => isWindowsMock(),
}));

describe("AboutSection", () => {
  beforeEach(() => {
    toastSuccessMock.mockReset();
    toastErrorMock.mockReset();
    useUpdateMock.mockReset();
    getCurrentVersionMock.mockReset();
    relaunchAppMock.mockReset();
    openExternalMock.mockReset();
    getToolVersionsMock.mockReset();
    isWindowsMock.mockReset();

    Object.defineProperty(window, "__TAURI_INTERNALS__", {
      configurable: true,
      value: undefined,
    });
    Object.defineProperty(window, "__TAURI__", {
      configurable: true,
      value: undefined,
    });

    getCurrentVersionMock.mockResolvedValue("3.14.1");
    getToolVersionsMock.mockResolvedValue([]);
    isWindowsMock.mockReturnValue(true);
  });

  it("shows an up-to-date toast when no update is available", async () => {
    useUpdateMock.mockReturnValue({
      hasUpdate: false,
      updateInfo: null,
      updateHandle: null,
      checkUpdate: vi.fn().mockResolvedValue(false),
      resetDismiss: vi.fn(),
      isChecking: false,
    });

    render(<AboutSection isPortable={false} />);

    fireEvent.click(
      await screen.findByRole("button", { name: "settings.checkForUpdates" }),
    );

    await waitFor(() => {
      expect(toastSuccessMock).toHaveBeenCalledWith("settings.upToDate", {
        closeButton: true,
      });
    });
  });

  it("shows server runtime panels in web mode even when the browser reports Windows", async () => {
    getCurrentVersionMock.mockResolvedValue("");
    getToolVersionsMock.mockResolvedValue([
      {
        name: "codex",
        version: "0.9.0",
        latest_version: "1.0.0",
        error: null,
        env_type: "linux",
        wsl_distro: null,
      },
    ]);
    useUpdateMock.mockReturnValue({
      hasUpdate: false,
      updateInfo: null,
      updateHandle: null,
      checkUpdate: vi.fn().mockResolvedValue(false),
      resetDismiss: vi.fn(),
      isChecking: false,
    });

    render(<AboutSection isPortable={false} />);

    await waitFor(() => {
      expect(getToolVersionsMock).toHaveBeenCalledWith(
        ["claude", "codex", "gemini", "opencode"],
        {},
      );
    });

    expect(screen.getByText("common.unknown")).toBeInTheDocument();
    expect(screen.getByText("settings.webUpdateHint")).toBeInTheDocument();
    expect(screen.getByText("settings.serverEnvCheck")).toBeInTheDocument();
    expect(screen.getByText("settings.serverEnvCheckHint")).toBeInTheDocument();
    expect(screen.getByText("settings.serverInstallHint")).toBeInTheDocument();
    expect(screen.getByText("0.9.0")).toBeInTheDocument();
  });

  it("shows structured server environment failures in web mode", async () => {
    getToolVersionsMock.mockRejectedValue({
      message: "tool version route unavailable",
    });
    useUpdateMock.mockReturnValue({
      hasUpdate: false,
      updateInfo: null,
      updateHandle: null,
      checkUpdate: vi.fn().mockResolvedValue(false),
      resetDismiss: vi.fn(),
      isChecking: false,
    });

    render(<AboutSection isPortable={false} />);

    const alert = await screen.findByRole("alert");
    expect(alert).toHaveTextContent("settings.serverEnvCheckFailed");
    expect(alert).toHaveTextContent("tool version route unavailable");
    expect(alert).not.toHaveTextContent("[object Object]");
  });

  it("opens the web download flow without relaunching when an update exists", async () => {
    const downloadAndInstall = vi.fn().mockResolvedValue(undefined);
    const resetDismiss = vi.fn();

    useUpdateMock.mockReturnValue({
      hasUpdate: true,
      updateInfo: {
        availableVersion: "3.15.0",
        notes: "release notes",
      },
      updateHandle: {
        downloadAndInstall,
      },
      checkUpdate: vi.fn(),
      resetDismiss,
      isChecking: false,
    });

    render(<AboutSection isPortable={false} />);

    fireEvent.click(
      await screen.findByRole("button", { name: "settings.updateTo:3.15.0" }),
    );

    await waitFor(() => {
      expect(downloadAndInstall).toHaveBeenCalledTimes(1);
      expect(resetDismiss).toHaveBeenCalledTimes(1);
      expect(relaunchAppMock).not.toHaveBeenCalled();
      expect(toastSuccessMock).toHaveBeenCalledWith(
        "settings.updateDownloadOpened",
        { closeButton: true },
      );
    });
  });

  it("opens release notes for the available version", async () => {
    useUpdateMock.mockReturnValue({
      hasUpdate: true,
      updateInfo: {
        availableVersion: "3.15.0",
        notes: "release notes",
      },
      updateHandle: null,
      checkUpdate: vi.fn(),
      resetDismiss: vi.fn(),
      isChecking: false,
    });

    render(<AboutSection isPortable={false} />);

    fireEvent.click(
      await screen.findByRole("button", { name: "settings.releaseNotes" }),
    );

    await waitFor(() => {
      expect(openExternalMock).toHaveBeenCalledWith(
        "https://github.com/farion1231/cc-switch/releases/tag/v3.15.0",
      );
    });
  });

  it("shows structured detail when opening release notes fails", async () => {
    openExternalMock.mockRejectedValueOnce({ detail: "release notes blocked" });
    useUpdateMock.mockReturnValue({
      hasUpdate: true,
      updateInfo: {
        availableVersion: "3.15.0",
        notes: "release notes",
      },
      updateHandle: null,
      checkUpdate: vi.fn(),
      resetDismiss: vi.fn(),
      isChecking: false,
    });

    render(<AboutSection isPortable={false} />);

    fireEvent.click(
      await screen.findByRole("button", { name: "settings.releaseNotes" }),
    );

    await waitFor(() => {
      expect(toastErrorMock).toHaveBeenCalledWith(
        "settings.openReleaseNotesFailed",
        {
          description: "release notes blocked",
        },
      );
    });
  });

  it("shows structured detail when checking for updates fails", async () => {
    useUpdateMock.mockReturnValue({
      hasUpdate: false,
      updateInfo: null,
      updateHandle: null,
      checkUpdate: vi.fn().mockRejectedValue({ detail: "update endpoint down" }),
      resetDismiss: vi.fn(),
      isChecking: false,
    });

    render(<AboutSection isPortable={false} />);

    fireEvent.click(
      await screen.findByRole("button", { name: "settings.checkForUpdates" }),
    );

    await waitFor(() => {
      expect(toastErrorMock).toHaveBeenCalledWith(
        "settings.checkUpdateFailed",
        {
          description: "update endpoint down",
        },
      );
    });
  });

  it("loads tool versions and refreshes a WSL tool with the selected shell", async () => {
    isWindowsMock.mockReturnValue(false);
    useUpdateMock.mockReturnValue({
      hasUpdate: false,
      updateInfo: null,
      updateHandle: null,
      checkUpdate: vi.fn().mockResolvedValue(false),
      resetDismiss: vi.fn(),
      isChecking: false,
    });
    getToolVersionsMock
      .mockResolvedValueOnce([
        {
          name: "claude",
          version: "1.0.0",
          latest_version: "1.1.0",
          error: null,
          env_type: "wsl",
          wsl_distro: "Ubuntu",
        },
      ])
      .mockResolvedValueOnce([
        {
          name: "claude",
          version: "1.0.1",
          latest_version: "1.1.0",
          error: null,
          env_type: "wsl",
          wsl_distro: "Ubuntu",
        },
      ]);

    render(<AboutSection isPortable={false} />);

    await waitFor(() => {
      expect(getToolVersionsMock).toHaveBeenCalledWith(
        ["claude", "codex", "gemini", "opencode"],
        {},
      );
    });
    expect(await screen.findByText("1.0.0")).toBeInTheDocument();

    const selects = screen.getAllByRole("combobox");
    fireEvent.change(selects[0], { target: { value: "bash" } });

    await waitFor(() => {
      expect(getToolVersionsMock).toHaveBeenLastCalledWith(["claude"], {
        claude: { wslShell: "bash" },
      });
    });
    expect(await screen.findByText("1.0.1")).toBeInTheDocument();
  });
});
