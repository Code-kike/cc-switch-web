import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import WorkspaceFilesPanel from "@/components/workspace/WorkspaceFilesPanel";
import DailyMemoryPanel from "@/components/workspace/DailyMemoryPanel";

const readFileMock = vi.fn();
const openDirectoryMock = vi.fn();
const listDailyMemoryFilesMock = vi.fn();
const readDailyMemoryFileMock = vi.fn();
const writeDailyMemoryFileMock = vi.fn();
const deleteDailyMemoryFileMock = vi.fn();
const searchDailyMemoryFilesMock = vi.fn();

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) => key,
  }),
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
    children,
  }: {
    isOpen: boolean;
    children: React.ReactNode;
  }) => (isOpen ? <div>{children}</div> : null),
}));

vi.mock("@/components/workspace/WorkspaceFileEditor", () => ({
  default: () => null,
}));

vi.mock("@/components/MarkdownEditor", () => ({
  default: () => <div>markdown-editor</div>,
}));

vi.mock("@/lib/api/workspace", () => ({
  workspaceApi: {
    readFile: (...args: unknown[]) => readFileMock(...args),
    openDirectory: (...args: unknown[]) => openDirectoryMock(...args),
    listDailyMemoryFiles: (...args: unknown[]) =>
      listDailyMemoryFilesMock(...args),
    readDailyMemoryFile: (...args: unknown[]) =>
      readDailyMemoryFileMock(...args),
    writeDailyMemoryFile: (...args: unknown[]) =>
      writeDailyMemoryFileMock(...args),
    deleteDailyMemoryFile: (...args: unknown[]) =>
      deleteDailyMemoryFileMock(...args),
    searchDailyMemoryFiles: (...args: unknown[]) =>
      searchDailyMemoryFilesMock(...args),
  },
}));

describe("Workspace Web UI parity", () => {
  beforeEach(() => {
    Object.defineProperty(window, "__TAURI_INTERNALS__", {
      configurable: true,
      value: undefined,
    });
    Object.defineProperty(window, "__TAURI__", {
      configurable: true,
      value: undefined,
    });
    readFileMock.mockReset();
    openDirectoryMock.mockReset();
    listDailyMemoryFilesMock.mockReset();
    readDailyMemoryFileMock.mockReset();
    writeDailyMemoryFileMock.mockReset();
    deleteDailyMemoryFileMock.mockReset();
    searchDailyMemoryFilesMock.mockReset();
    readFileMock.mockResolvedValue(null);
    listDailyMemoryFilesMock.mockResolvedValue([]);
    searchDailyMemoryFilesMock.mockResolvedValue([]);
  });

  it("shows a manual-path hint and does not open the workspace directory in web mode", async () => {
    render(<WorkspaceFilesPanel />);

    const workspacePath = screen.getByText("~/.openclaw/workspace/");
    expect(workspacePath).toHaveAttribute(
      "title",
      "settings.webManualPathHint",
    );

    fireEvent.click(workspacePath);

    expect(openDirectoryMock).not.toHaveBeenCalled();
    await waitFor(() => expect(readFileMock).toHaveBeenCalled());
  });

  it("shows a manual-path hint and does not open the daily memory directory in web mode", async () => {
    render(<DailyMemoryPanel isOpen onClose={vi.fn()} />);

    await waitFor(() => expect(listDailyMemoryFilesMock).toHaveBeenCalled());

    const memoryPath = screen.getByText("~/.openclaw/workspace/memory/");
    expect(memoryPath).toHaveAttribute("title", "settings.webManualPathHint");

    fireEvent.click(memoryPath);

    expect(openDirectoryMock).not.toHaveBeenCalled();
  });
});
