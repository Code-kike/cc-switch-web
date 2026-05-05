import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import DailyMemoryPanel from "@/components/workspace/DailyMemoryPanel";

const toastErrorMock = vi.fn();
const toastSuccessMock = vi.fn();
const listDailyMemoryFilesMock = vi.fn();
const readDailyMemoryFileMock = vi.fn();
const writeDailyMemoryFileMock = vi.fn();
const deleteDailyMemoryFileMock = vi.fn();
const searchDailyMemoryFilesMock = vi.fn();
const openDirectoryMock = vi.fn();
const tMock = (key: string) => key;

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: tMock,
  }),
}));

vi.mock("sonner", () => ({
  toast: {
    error: (...args: unknown[]) => toastErrorMock(...args),
    success: (...args: unknown[]) => toastSuccessMock(...args),
  },
}));

vi.mock("framer-motion", () => ({
  motion: {
    div: ({ children, ...props }: React.HTMLAttributes<HTMLDivElement>) => (
      <div {...props}>{children}</div>
    ),
  },
  AnimatePresence: ({ children }: { children: React.ReactNode }) => (
    <>{children}</>
  ),
}));

vi.mock("@/components/common/FullScreenPanel", () => ({
  FullScreenPanel: ({
    isOpen,
    title,
    children,
    footer,
  }: {
    isOpen: boolean;
    title?: string;
    children: React.ReactNode;
    footer?: React.ReactNode;
  }) =>
    isOpen ? (
      <div>
        <div>{title}</div>
        <div>{children}</div>
        <div>{footer}</div>
      </div>
    ) : null,
}));

vi.mock("@/components/ConfirmDialog", () => ({
  ConfirmDialog: ({
    isOpen,
    onConfirm,
    onCancel,
  }: {
    isOpen: boolean;
    onConfirm: () => void;
    onCancel: () => void;
  }) =>
    isOpen ? (
      <div data-testid="confirm-dialog">
        <button type="button" onClick={onConfirm}>
          confirm-delete
        </button>
        <button type="button" onClick={onCancel}>
          cancel-delete
        </button>
      </div>
    ) : null,
}));

vi.mock("@/components/MarkdownEditor", () => ({
  default: ({
    value,
    onChange,
    placeholder,
  }: {
    value: string;
    onChange: (value: string) => void;
    placeholder?: string;
  }) => (
    <textarea
      aria-label="daily-memory-editor"
      value={value}
      placeholder={placeholder}
      onChange={(event) => onChange(event.target.value)}
    />
  ),
}));

vi.mock("@/lib/api/workspace", () => ({
  workspaceApi: {
    listDailyMemoryFiles: (...args: unknown[]) => listDailyMemoryFilesMock(...args),
    readDailyMemoryFile: (...args: unknown[]) => readDailyMemoryFileMock(...args),
    writeDailyMemoryFile: (...args: unknown[]) => writeDailyMemoryFileMock(...args),
    deleteDailyMemoryFile: (...args: unknown[]) => deleteDailyMemoryFileMock(...args),
    searchDailyMemoryFiles: (...args: unknown[]) => searchDailyMemoryFilesMock(...args),
    openDirectory: (...args: unknown[]) => openDirectoryMock(...args),
  },
}));

function getDeleteTriggerForItem(text: string): HTMLElement {
  const item = Array.from(screen.getAllByRole("button")).find((button) =>
    button.textContent?.includes(text),
  );
  if (!item?.lastElementChild || !(item.lastElementChild instanceof HTMLElement)) {
    throw new Error(`Delete trigger not found for ${text}`);
  }
  return item.lastElementChild;
}

describe("DailyMemoryPanel", () => {
  beforeEach(() => {
    Object.defineProperty(window, "__TAURI_INTERNALS__", {
      configurable: true,
      value: undefined,
    });
    Object.defineProperty(window, "__TAURI__", {
      configurable: true,
      value: undefined,
    });

    toastErrorMock.mockReset();
    toastSuccessMock.mockReset();
    listDailyMemoryFilesMock.mockReset();
    readDailyMemoryFileMock.mockReset();
    writeDailyMemoryFileMock.mockReset();
    deleteDailyMemoryFileMock.mockReset();
    searchDailyMemoryFilesMock.mockReset();
    openDirectoryMock.mockReset();

    listDailyMemoryFilesMock.mockResolvedValue([]);
    readDailyMemoryFileMock.mockResolvedValue(null);
    writeDailyMemoryFileMock.mockResolvedValue(undefined);
    deleteDailyMemoryFileMock.mockResolvedValue(undefined);
    searchDailyMemoryFilesMock.mockResolvedValue([]);
    openDirectoryMock.mockResolvedValue(undefined);
  });

  it("shows structured detail when loading the daily memory list fails", async () => {
    listDailyMemoryFilesMock.mockRejectedValueOnce({
      detail: "memory list unavailable",
    });

    render(<DailyMemoryPanel isOpen={true} onClose={() => undefined} />);

    await waitFor(() => {
      expect(toastErrorMock).toHaveBeenCalledWith(
        "workspace.dailyMemory.loadFailed",
        {
          description: "memory list unavailable",
        },
      );
    });
  });

  it("shows structured detail when saving a daily memory file fails", async () => {
    listDailyMemoryFilesMock.mockResolvedValue([
      {
        filename: "2026-03-04.md",
        date: "2026-03-04",
        sizeBytes: 128,
        modifiedAt: 0,
        preview: "hello",
      },
    ]);
    readDailyMemoryFileMock.mockResolvedValue("# Daily Memory");
    writeDailyMemoryFileMock.mockRejectedValueOnce({
      detail: "daily memory save denied",
    });

    render(<DailyMemoryPanel isOpen={true} onClose={() => undefined} />);

    await waitFor(() => {
      expect(screen.getByText("2026-03-04")).toBeInTheDocument();
    });

    fireEvent.click(screen.getByText("2026-03-04"));

    await waitFor(() => {
      expect(screen.getByLabelText("daily-memory-editor")).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole("button", { name: "common.save" }));

    await waitFor(() => {
      expect(toastErrorMock).toHaveBeenCalledWith("workspace.saveFailed", {
        description: "daily memory save denied",
      });
    });
  });

  it("deletes a search result, reloads the list, and refreshes the active search", async () => {
    listDailyMemoryFilesMock.mockResolvedValue([
      {
        filename: "2026-03-04.md",
        date: "2026-03-04",
        sizeBytes: 128,
        modifiedAt: 0,
        preview: "needle before delete",
      },
    ]);
    searchDailyMemoryFilesMock.mockResolvedValue([
      {
        filename: "2026-03-04.md",
        date: "2026-03-04",
        sizeBytes: 128,
        modifiedAt: 0,
        snippet: "needle before delete",
        matchCount: 1,
      },
    ]);

    render(<DailyMemoryPanel isOpen={true} onClose={() => undefined} />);

    await screen.findByText("2026-03-04");

    fireEvent.click(screen.getByTitle("workspace.dailyMemory.searchScopeHint"));
    fireEvent.change(
      screen.getByPlaceholderText("workspace.dailyMemory.searchPlaceholder"),
      {
        target: { value: "needle" },
      },
    );

    await waitFor(() => {
      expect(searchDailyMemoryFilesMock).toHaveBeenCalledWith("needle");
    });
    await screen.findByText("needle before delete");

    fireEvent.click(getDeleteTriggerForItem("needle before delete"));
    fireEvent.click(screen.getByText("confirm-delete"));

    await waitFor(() => {
      expect(deleteDailyMemoryFileMock).toHaveBeenCalledWith("2026-03-04.md");
    });
    await waitFor(() => {
      expect(listDailyMemoryFilesMock).toHaveBeenCalledTimes(2);
    });
    await waitFor(() => {
      expect(searchDailyMemoryFilesMock).toHaveBeenCalledTimes(2);
    });
    expect(toastSuccessMock).toHaveBeenCalledWith(
      "workspace.dailyMemory.deleteSuccess",
    );
  });

  it("shows structured detail when deleting a daily memory file fails", async () => {
    listDailyMemoryFilesMock.mockResolvedValue([
      {
        filename: "2026-03-04.md",
        date: "2026-03-04",
        sizeBytes: 128,
        modifiedAt: 0,
        preview: "delete target",
      },
    ]);
    deleteDailyMemoryFileMock.mockRejectedValueOnce({
      detail: "daily memory delete denied",
    });

    render(<DailyMemoryPanel isOpen={true} onClose={() => undefined} />);

    await screen.findByText("2026-03-04");

    fireEvent.click(getDeleteTriggerForItem("delete target"));
    fireEvent.click(screen.getByText("confirm-delete"));

    await waitFor(() => {
      expect(toastErrorMock).toHaveBeenCalledWith(
        "workspace.dailyMemory.deleteFailed",
        {
          description: "daily memory delete denied",
        },
      );
    });
  });
});
