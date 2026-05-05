import { createRef } from "react";
import { render, screen, waitFor, act, fireEvent } from "@testing-library/react";
import { describe, expect, it, vi, beforeEach } from "vitest";

import UnifiedSkillsPanel, {
  type UnifiedSkillsPanelHandle,
} from "@/components/skills/UnifiedSkillsPanel";

const toastSuccessMock = vi.fn();
const toastErrorMock = vi.fn();
const toastInfoMock = vi.fn();
const scanUnmanagedMock = vi.fn();
const toggleSkillAppMock = vi.fn();
const uninstallSkillMock = vi.fn();
const importSkillsMock = vi.fn();
const installFromZipMock = vi.fn();
const openZipFileDialogMock = vi.fn();
const deleteSkillBackupMock = vi.fn();
const restoreSkillBackupMock = vi.fn();
let installedSkillsFixture: any[] = [];
let unmanagedSkillsFixture = [
  {
    directory: "shared-skill",
    name: "Shared Skill",
    description: "Imported from Claude",
    foundIn: ["claude"],
    path: "/tmp/shared-skill",
  },
];

vi.mock("sonner", () => ({
  toast: {
    success: (...args: unknown[]) => toastSuccessMock(...args),
    error: (...args: unknown[]) => toastErrorMock(...args),
    info: (...args: unknown[]) => toastInfoMock(...args),
  },
}));

vi.mock("@/hooks/useSkills", () => ({
  useInstalledSkills: () => ({
    data: installedSkillsFixture,
    isLoading: false,
  }),
  useSkillBackups: () => ({
    data: [],
    refetch: vi.fn(),
    isFetching: false,
  }),
  useDeleteSkillBackup: () => ({
    mutateAsync: deleteSkillBackupMock,
    isPending: false,
  }),
  useToggleSkillApp: () => ({
    mutateAsync: toggleSkillAppMock,
  }),
  useRestoreSkillBackup: () => ({
    mutateAsync: restoreSkillBackupMock,
    isPending: false,
  }),
  useUninstallSkill: () => ({
    mutateAsync: uninstallSkillMock,
  }),
  useScanUnmanagedSkills: () => ({
    data: unmanagedSkillsFixture,
    refetch: scanUnmanagedMock,
  }),
  useImportSkillsFromApps: () => ({
    mutateAsync: importSkillsMock,
  }),
  useInstallSkillsFromZip: () => ({
    mutateAsync: installFromZipMock,
  }),
  useCheckSkillUpdates: () => ({
    data: [],
    refetch: vi.fn(),
    isFetching: false,
  }),
  useUpdateSkill: () => ({
    mutateAsync: vi.fn(),
    isPending: false,
  }),
}));

vi.mock("@/components/common/AppToggleGroup", () => ({
  AppToggleGroup: ({
    apps,
    onToggle,
    appIds,
  }: {
    apps: Record<string, boolean>;
    onToggle: (app: string, enabled: boolean) => void;
    appIds: string[];
  }) => (
    <div>
      {appIds.map((app) => (
        <button
          key={app}
          type="button"
          aria-label={app}
          onClick={() => onToggle(app, !apps[app])}
        >
          {app}
        </button>
      ))}
    </div>
  ),
}));

vi.mock("@/lib/api", () => ({
  settingsApi: {
    openExternal: vi.fn(),
  },
  skillsApi: {
    openZipFileDialog: (...args: unknown[]) => openZipFileDialogMock(...args),
  },
}));

describe("UnifiedSkillsPanel", () => {
  beforeEach(() => {
    installedSkillsFixture = [];
    unmanagedSkillsFixture = [
      {
        directory: "shared-skill",
        name: "Shared Skill",
        description: "Imported from Claude",
        foundIn: ["claude"],
        path: "/tmp/shared-skill",
      },
    ];
    scanUnmanagedMock.mockResolvedValue({
      data: unmanagedSkillsFixture,
    });
    toastSuccessMock.mockReset();
    toastErrorMock.mockReset();
    toastInfoMock.mockReset();
    toggleSkillAppMock.mockReset();
    uninstallSkillMock.mockReset();
    importSkillsMock.mockReset();
    installFromZipMock.mockReset();
    openZipFileDialogMock.mockReset();
    deleteSkillBackupMock.mockReset();
    restoreSkillBackupMock.mockReset();
  });

  it("opens the import dialog without crashing when app toggles render", async () => {
    const ref = createRef<UnifiedSkillsPanelHandle>();

    render(
      <UnifiedSkillsPanel
        ref={ref}
        onOpenDiscovery={() => {}}
        currentApp="claude"
      />,
    );

    await act(async () => {
      await ref.current?.openImport();
    });

    await waitFor(() => {
      expect(screen.getByText("skills.import")).toBeInTheDocument();
      expect(screen.getByText("Shared Skill")).toBeInTheDocument();
      expect(screen.getByText("/tmp/shared-skill")).toBeInTheDocument();
    });
  });

  it("passes the selected browser File directly to the ZIP install mutation", async () => {
    const ref = createRef<UnifiedSkillsPanelHandle>();
    const zipFile = new File(["zip"], "skills.zip", {
      type: "application/zip",
    });
    openZipFileDialogMock.mockResolvedValue(zipFile);
    installFromZipMock.mockResolvedValue([]);

    render(
      <UnifiedSkillsPanel
        ref={ref}
        onOpenDiscovery={() => {}}
        currentApp="claude"
      />,
    );

    await act(async () => {
      await ref.current?.openInstallFromZip();
    });

    expect(openZipFileDialogMock).toHaveBeenCalledTimes(1);
    expect(installFromZipMock).toHaveBeenCalledWith({
      filePath: zipFile,
      currentApp: "claude",
    });
  });

  it("treats canceling the ZIP picker as a silent no-op", async () => {
    const ref = createRef<UnifiedSkillsPanelHandle>();
    openZipFileDialogMock.mockResolvedValue(null);

    render(
      <UnifiedSkillsPanel
        ref={ref}
        onOpenDiscovery={() => {}}
        currentApp="claude"
      />,
    );

    await act(async () => {
      await ref.current?.openInstallFromZip();
    });

    expect(openZipFileDialogMock).toHaveBeenCalledTimes(1);
    expect(installFromZipMock).not.toHaveBeenCalled();
    expect(toastErrorMock).not.toHaveBeenCalled();
    expect(toastSuccessMock).not.toHaveBeenCalled();
    expect(toastInfoMock).not.toHaveBeenCalled();
  });

  it("defaults OpenClaw imports to enabled when the unmanaged skill comes from OpenClaw", async () => {
    const ref = createRef<UnifiedSkillsPanelHandle>();
    unmanagedSkillsFixture = [
      {
        directory: "openclaw-skill",
        name: "OpenClaw Skill",
        description: "Imported from OpenClaw",
        foundIn: ["openclaw"],
        path: "/tmp/openclaw-skill",
      },
    ];
    scanUnmanagedMock.mockResolvedValue({
      data: unmanagedSkillsFixture,
    });
    importSkillsMock.mockResolvedValue([]);

    render(
      <UnifiedSkillsPanel
        ref={ref}
        onOpenDiscovery={() => {}}
        currentApp="openclaw"
      />,
    );

    await act(async () => {
      await ref.current?.openImport();
    });

    await waitFor(() =>
      expect(screen.getByText("OpenClaw Skill")).toBeInTheDocument(),
    );

    fireEvent.click(screen.getByRole("button", { name: "skills.importSelected" }));

    await waitFor(() =>
      expect(importSkillsMock).toHaveBeenCalledWith([
        {
          directory: "openclaw-skill",
          apps: {
            claude: false,
            codex: false,
            gemini: false,
            opencode: false,
            openclaw: true,
            hermes: false,
          },
        },
      ]),
    );
  });

  it("formats structured ZIP install errors into user-facing skill messages", async () => {
    const ref = createRef<UnifiedSkillsPanelHandle>();
    const zipFile = new File(["zip"], "skills.zip", {
      type: "application/zip",
    });
    openZipFileDialogMock.mockResolvedValue(zipFile);
    installFromZipMock.mockRejectedValueOnce(
      JSON.stringify({
        code: "NO_SKILLS_IN_ZIP",
        context: {},
        suggestion: "checkZipContent",
      }),
    );

    render(
      <UnifiedSkillsPanel
        ref={ref}
        onOpenDiscovery={() => {}}
        currentApp="claude"
      />,
    );

    await act(async () => {
      await ref.current?.openInstallFromZip();
    });

    await waitFor(() =>
      expect(toastErrorMock).toHaveBeenCalledWith("skills.installFailed", {
        description:
          "skills.error.noSkillsInZip\n\nskills.error.suggestion.checkZipContent",
        duration: 6000,
      }),
    );
  });

  it("falls back to a skills-specific toggle error when no detail is available", async () => {
    installedSkillsFixture = [
      {
        id: "skill-1",
        name: "Installed Skill",
        directory: "installed-skill",
        description: "Installed skill description",
        apps: {
          claude: true,
          codex: false,
          gemini: false,
          opencode: false,
          openclaw: false,
          hermes: false,
        },
      },
    ];
    toggleSkillAppMock.mockRejectedValueOnce({});

    render(
      <UnifiedSkillsPanel
        onOpenDiscovery={() => {}}
        currentApp="claude"
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: "codex" }));

    await waitFor(() =>
      expect(toastErrorMock).toHaveBeenCalledWith("skills.toggleFailed", {
        description: "common.error",
        duration: 4000,
      }),
    );
  });
});
