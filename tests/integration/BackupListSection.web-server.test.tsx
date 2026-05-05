import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import {
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import { afterAll, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";

import "@/lib/api/web-commands";
import { BackupListSection } from "@/components/settings/BackupListSection";
import { setCsrfToken } from "@/lib/api/adapter";
import { server } from "../msw/server";
import {
  startTestWebServer,
  type TestWebServer,
} from "../helpers/web-server";

const toastSuccessMock = vi.fn();
const toastErrorMock = vi.fn();

vi.mock("sonner", () => ({
  toast: {
    success: (...args: unknown[]) => toastSuccessMock(...args),
    error: (...args: unknown[]) => toastErrorMock(...args),
  },
}));

type BackupEntry = {
  filename: string;
  sizeBytes: number;
  createdAt: string;
};

const renderSection = () => {
  const client = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  });

  return render(
    <QueryClientProvider client={client}>
      <BackupListSection
        backupIntervalHours={24}
        backupRetainCount={10}
        onSettingsChange={vi.fn()}
      />
    </QueryClientProvider>,
  );
};

const getBackups = async (baseUrl: string): Promise<BackupEntry[]> => {
  const response = await fetch(new URL("/api/backups/list-db-backups", baseUrl));
  if (!response.ok) {
    throw new Error(`failed to load backups: ${response.status}`);
  }
  return (await response.json()) as BackupEntry[];
};

const getRowForName = (displayName: string): HTMLElement => {
  const label = screen.getByText(displayName);
  const row = label.parentElement?.parentElement;
  if (!(row instanceof HTMLElement)) {
    throw new Error(`could not locate backup row for ${displayName}`);
  }
  return row;
};

describe.sequential("BackupListSection against real web server", () => {
  let webServer: TestWebServer;

  beforeAll(async () => {
    server.close();
    webServer = await startTestWebServer();
  }, 360_000);

  afterAll(async () => {
    await webServer.stop();
    server.listen({ onUnhandledRequest: "warn" });
  }, 20_000);

  beforeEach(() => {
    toastSuccessMock.mockReset();
    toastErrorMock.mockReset();
    setCsrfToken(null);
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
  });

  it(
    "creates, renames, restores, and deletes backups through the visible section UI",
    async () => {
      renderSection();

      await waitFor(async () => {
        expect(await getBackups(webServer.baseUrl)).toEqual([]);
      });
      expect(await screen.findByText("No backups yet")).toBeInTheDocument();

      fireEvent.click(
        screen.getByRole("button", {
          name: "Backup Now",
        }),
      );

      await waitFor(() =>
        expect(toastSuccessMock).toHaveBeenCalledWith("Backup created successfully"),
      );

      let backups: BackupEntry[] = [];
      await waitFor(async () => {
        backups = await getBackups(webServer.baseUrl);
        expect(backups).toHaveLength(1);
      });

      fireEvent.click(screen.getByTitle("Rename"));

      const renameInput = screen.getByPlaceholderText(
        "Enter new name",
      );
      fireEvent.change(renameInput, { target: { value: "backup-smoke" } });
      fireEvent.keyDown(renameInput, { key: "Enter" });

      await waitFor(() =>
        expect(toastSuccessMock).toHaveBeenCalledWith("Backup renamed"),
      );
      await waitFor(async () => {
        backups = await getBackups(webServer.baseUrl);
        expect(backups.some((entry) => entry.filename === "backup-smoke.db")).toBe(
          true,
        );
      });
      expect(screen.getByText("backup-smoke")).toBeInTheDocument();

      const renamedRow = getRowForName("backup-smoke");
      fireEvent.click(
        within(renamedRow).getByRole("button", {
          name: "Restore",
        }),
      );

      await screen.findByText("Confirm Restore");

      const restoreButtons = screen.getAllByRole("button", {
        name: "Restore",
      });
      fireEvent.click(restoreButtons[restoreButtons.length - 1]);

      await waitFor(() =>
        expect(toastSuccessMock).toHaveBeenCalledWith(
          "Restore successful! Safety backup created",
          expect.objectContaining({ closeButton: true }),
        ),
      );
      await waitFor(async () => {
        backups = await getBackups(webServer.baseUrl);
        expect(backups.length).toBeGreaterThanOrEqual(2);
      });

      const restoredRow = getRowForName("backup-smoke");
      fireEvent.click(
        within(restoredRow).getByTitle("Delete"),
      );

      await screen.findByText("Confirm Delete");

      const deleteButtons = screen.getAllByRole("button", {
        name: "Delete",
      });
      fireEvent.click(deleteButtons[deleteButtons.length - 1]);

      await waitFor(() =>
        expect(toastSuccessMock).toHaveBeenCalledWith("Backup deleted"),
      );
      await waitFor(async () => {
        backups = await getBackups(webServer.baseUrl);
        expect(backups.some((entry) => entry.filename === "backup-smoke.db")).toBe(
          false,
        );
      });
      expect(screen.queryByText("backup-smoke")).not.toBeInTheDocument();
      expect(toastErrorMock).not.toHaveBeenCalled();
    },
    20_000,
  );
});
