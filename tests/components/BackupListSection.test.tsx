import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import "@testing-library/jest-dom";

import { BackupListSection } from "@/components/settings/BackupListSection";

const toastSuccessMock = vi.fn();
const toastErrorMock = vi.fn();

const hookState = vi.hoisted(() => ({
  useBackupManagerMock: vi.fn(),
}));

vi.mock("sonner", () => ({
  toast: {
    success: (...args: unknown[]) => toastSuccessMock(...args),
    error: (...args: unknown[]) => toastErrorMock(...args),
  },
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) => key,
  }),
}));

vi.mock("@/hooks/useBackupManager", () => ({
  useBackupManager: () => hookState.useBackupManagerMock(),
}));

vi.mock("@/components/ui/button", () => ({
  Button: ({ children, ...props }: any) => <button {...props}>{children}</button>,
}));

vi.mock("@/components/ui/input", () => ({
  Input: (props: any) => <input {...props} />,
}));

vi.mock("@/components/ui/label", () => ({
  Label: ({ children, ...props }: any) => <label {...props}>{children}</label>,
}));

vi.mock("@/components/ui/select", () => ({
  Select: ({ value, onValueChange, children }: any) => (
    <select
      value={value}
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

vi.mock("@/components/ui/dialog", () => ({
  Dialog: ({ open, children }: any) => (open ? <div>{children}</div> : null),
  DialogContent: ({ children }: any) => (
    <div data-testid="dialog-content">{children}</div>
  ),
  DialogDescription: ({ children }: any) => <div>{children}</div>,
  DialogFooter: ({ children }: any) => <div>{children}</div>,
  DialogHeader: ({ children }: any) => <div>{children}</div>,
  DialogTitle: ({ children }: any) => <h2>{children}</h2>,
}));

const createMock = vi.fn();
const restoreMock = vi.fn();
const renameMock = vi.fn();
const removeMock = vi.fn();

const defaultBackup = {
  filename: "db_backup_20260501_123045.db",
  createdAt: "2026-05-01T12:30:45Z",
  sizeBytes: 2048,
};

function renderSection(
  overrides: Partial<ReturnType<typeof createHookValue>> = {},
  onSettingsChange = vi.fn(),
) {
  hookState.useBackupManagerMock.mockReturnValue({
    ...createHookValue(),
    ...overrides,
  });

  render(
    <BackupListSection
      backupIntervalHours={24}
      backupRetainCount={10}
      onSettingsChange={onSettingsChange}
    />,
  );

  return { onSettingsChange };
}

function createHookValue() {
  return {
    backups: [defaultBackup],
    isLoading: false,
    create: createMock,
    isCreating: false,
    restore: restoreMock,
    isRestoring: false,
    rename: renameMock,
    isRenaming: false,
    remove: removeMock,
    isDeleting: false,
  };
}

describe("BackupListSection", () => {
  beforeEach(() => {
    toastSuccessMock.mockReset();
    toastErrorMock.mockReset();
    hookState.useBackupManagerMock.mockReset();
    createMock.mockReset();
    restoreMock.mockReset();
    renameMock.mockReset();
    removeMock.mockReset();

    createMock.mockResolvedValue("backup-id");
    restoreMock.mockResolvedValue("safety-backup-id");
    renameMock.mockResolvedValue("renamed-backup");
    removeMock.mockResolvedValue(undefined);
  });

  it("updates backup policy settings through the parent callback", () => {
    const { onSettingsChange } = renderSection();

    const selects = screen.getAllByRole("combobox");
    fireEvent.change(selects[0], { target: { value: "48" } });
    fireEvent.change(selects[1], { target: { value: "20" } });

    expect(onSettingsChange).toHaveBeenNthCalledWith(1, {
      backupIntervalHours: 48,
    });
    expect(onSettingsChange).toHaveBeenNthCalledWith(2, {
      backupRetainCount: 20,
    });
  });

  it("creates a backup and shows a success toast", async () => {
    renderSection();

    fireEvent.click(
      screen.getByRole("button", {
        name: "settings.backupManager.createBackup",
      }),
    );

    await waitFor(() => {
      expect(createMock).toHaveBeenCalledTimes(1);
    });
    expect(toastSuccessMock).toHaveBeenCalledWith(
      "settings.backupManager.createSuccess",
    );
  });

  it("renames a backup from the inline editor", async () => {
    renderSection();

    fireEvent.click(screen.getByTitle("settings.backupManager.rename"));

    const input = screen.getByPlaceholderText(
      "settings.backupManager.namePlaceholder",
    );
    expect(input).toHaveValue("2026-05-01 12:30:45");

    fireEvent.change(input, { target: { value: "manual snapshot" } });
    fireEvent.keyDown(input, { key: "Enter" });

    await waitFor(() => {
      expect(renameMock).toHaveBeenCalledWith({
        oldFilename: "db_backup_20260501_123045.db",
        newName: "manual snapshot",
      });
    });
    expect(toastSuccessMock).toHaveBeenCalledWith(
      "settings.backupManager.renameSuccess",
    );
  });

  it("restores a backup from the confirmation dialog", async () => {
    renderSection();

    fireEvent.click(
      screen.getByRole("button", { name: "settings.backupManager.restore" }),
    );

    const dialog = screen.getByTestId("dialog-content");
    expect(
      within(dialog).getByText("settings.backupManager.confirmTitle"),
    ).toBeInTheDocument();

    fireEvent.click(
      within(dialog).getByRole("button", {
        name: "settings.backupManager.restore",
      }),
    );

    await waitFor(() => {
      expect(restoreMock).toHaveBeenCalledWith("db_backup_20260501_123045.db");
    });
    expect(toastSuccessMock).toHaveBeenCalledWith(
      "settings.backupManager.restoreSuccess",
      expect.objectContaining({
        description:
          "settings.backupManager.safetyBackupId: safety-backup-id",
        closeButton: true,
      }),
    );
  });

  it("deletes a backup from the confirmation dialog", async () => {
    renderSection();

    fireEvent.click(screen.getByTitle("settings.backupManager.delete"));

    const dialog = screen.getByTestId("dialog-content");
    expect(
      within(dialog).getByText("settings.backupManager.deleteConfirmTitle"),
    ).toBeInTheDocument();

    fireEvent.click(
      within(dialog).getByRole("button", {
        name: "settings.backupManager.delete",
      }),
    );

    await waitFor(() => {
      expect(removeMock).toHaveBeenCalledWith("db_backup_20260501_123045.db");
    });
    expect(toastSuccessMock).toHaveBeenCalledWith(
      "settings.backupManager.deleteSuccess",
    );
  });
});
