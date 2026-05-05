import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import "@testing-library/jest-dom";

import { SkillStorageLocationSettings } from "@/components/settings/SkillStorageLocationSettings";

const toastSuccessMock = vi.fn();
const toastWarningMock = vi.fn();
const toastErrorMock = vi.fn();

const { migrateStorageMock } = vi.hoisted(() => ({
  migrateStorageMock: vi.fn(),
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
    success: (...args: unknown[]) => toastSuccessMock(...args),
    warning: (...args: unknown[]) => toastWarningMock(...args),
    error: (...args: unknown[]) => toastErrorMock(...args),
  },
}));

vi.mock("@/components/ui/button", () => ({
  Button: ({ children, onClick, ...props }: any) => (
    <button onClick={onClick} {...props}>
      {children}
    </button>
  ),
}));

vi.mock("@/components/ui/dialog", () => ({
  Dialog: ({ open, children }: any) => (open ? <div>{children}</div> : null),
  DialogContent: ({ children }: any) => <div>{children}</div>,
  DialogDescription: ({ children }: any) => <div>{children}</div>,
  DialogFooter: ({ children }: any) => <div>{children}</div>,
  DialogHeader: ({ children }: any) => <div>{children}</div>,
  DialogTitle: ({ children }: any) => <div>{children}</div>,
}));

vi.mock("@/lib/api/skills", async () => {
  const actual =
    await vi.importActual<typeof import("@/lib/api/skills")>(
      "@/lib/api/skills",
    );
  return {
    ...actual,
    skillsApi: {
      ...actual.skillsApi,
      migrateStorage: (...args: unknown[]) => migrateStorageMock(...args),
    },
  };
});

describe("SkillStorageLocationSettings", () => {
  beforeEach(() => {
    toastSuccessMock.mockReset();
    toastWarningMock.mockReset();
    toastErrorMock.mockReset();
    migrateStorageMock.mockReset();
    migrateStorageMock.mockResolvedValue({
      migratedCount: 2,
      skippedCount: 0,
      errors: [],
    });
  });

  it("asks for confirmation before migrating installed skills", async () => {
    const onMigrated = vi.fn();

    render(
      <SkillStorageLocationSettings
        value="cc_switch"
        installedCount={2}
        onMigrated={onMigrated}
      />,
    );

    fireEvent.click(
      screen.getByRole("button", { name: "settings.skillStorage.unified" }),
    );

    expect(
      screen.getByText("settings.skillStorage.confirmTitle"),
    ).toBeInTheDocument();
    expect(migrateStorageMock).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole("button", { name: "common.confirm" }));

    await waitFor(() => {
      expect(migrateStorageMock).toHaveBeenCalledWith("unified");
    });
    expect(onMigrated).toHaveBeenCalledWith("unified");
    expect(toastSuccessMock).toHaveBeenCalledWith(
      "settings.skillStorage.migrationSuccess",
    );
  });

  it("migrates immediately when there are no installed skills", async () => {
    const onMigrated = vi.fn();

    render(
      <SkillStorageLocationSettings
        value="cc_switch"
        installedCount={0}
        onMigrated={onMigrated}
      />,
    );

    fireEvent.click(
      screen.getByRole("button", { name: "settings.skillStorage.unified" }),
    );

    await waitFor(() => {
      expect(migrateStorageMock).toHaveBeenCalledWith("unified");
    });
    expect(onMigrated).toHaveBeenCalledWith("unified");
    expect(
      screen.queryByText("settings.skillStorage.confirmTitle"),
    ).not.toBeInTheDocument();
  });

  it("shows a partial migration warning when some skills fail", async () => {
    migrateStorageMock.mockResolvedValueOnce({
      migratedCount: 1,
      skippedCount: 0,
      errors: ["one failed"],
    });

    render(
      <SkillStorageLocationSettings
        value="cc_switch"
        installedCount={0}
        onMigrated={vi.fn()}
      />,
    );

    fireEvent.click(
      screen.getByRole("button", { name: "settings.skillStorage.unified" }),
    );

    await waitFor(() => {
      expect(toastWarningMock).toHaveBeenCalledWith(
        "settings.skillStorage.migrationPartial",
      );
    });
  });

  it("shows extracted error detail when migration fails", async () => {
    migrateStorageMock.mockRejectedValueOnce({
      detail: "permission denied",
    });

    render(
      <SkillStorageLocationSettings
        value="cc_switch"
        installedCount={0}
        onMigrated={vi.fn()}
      />,
    );

    fireEvent.click(
      screen.getByRole("button", { name: "settings.skillStorage.unified" }),
    );

    await waitFor(() => {
      expect(toastErrorMock).toHaveBeenCalledWith(
        "settings.skillStorage.migrationFailed:permission denied",
      );
    });
  });
});
