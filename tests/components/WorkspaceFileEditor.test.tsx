import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import WorkspaceFileEditor from "@/components/workspace/WorkspaceFileEditor";

const toastErrorMock = vi.fn();
const toastSuccessMock = vi.fn();
const readFileMock = vi.fn();
const writeFileMock = vi.fn();
const tMock = (key: string, options?: Record<string, unknown>) =>
  typeof options?.filename === "string" ? `${key}:${options.filename}` : key;

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
      aria-label="workspace-editor"
      value={value}
      placeholder={placeholder}
      onChange={(event) => onChange(event.target.value)}
    />
  ),
}));

vi.mock("@/lib/api/workspace", () => ({
  workspaceApi: {
    readFile: (...args: unknown[]) => readFileMock(...args),
    writeFile: (...args: unknown[]) => writeFileMock(...args),
  },
}));

describe("WorkspaceFileEditor", () => {
  beforeEach(() => {
    toastErrorMock.mockReset();
    toastSuccessMock.mockReset();
    readFileMock.mockReset();
    writeFileMock.mockReset();
    readFileMock.mockResolvedValue(null);
    writeFileMock.mockResolvedValue(undefined);
  });

  it("shows structured detail when loading a workspace file fails", async () => {
    readFileMock.mockRejectedValueOnce({ detail: "workspace read denied" });

    render(
      <WorkspaceFileEditor
        filename="AGENTS.md"
        isOpen={true}
        onClose={() => undefined}
      />,
    );

    await waitFor(() => {
      expect(toastErrorMock).toHaveBeenCalledWith("workspace.loadFailed", {
        description: "workspace read denied",
      });
    });
  });

  it("shows structured detail when saving a workspace file fails", async () => {
    readFileMock.mockResolvedValue("# Workspace");
    writeFileMock.mockRejectedValueOnce({ detail: "workspace save denied" });

    render(
      <WorkspaceFileEditor
        filename="AGENTS.md"
        isOpen={true}
        onClose={() => undefined}
      />,
    );

    await waitFor(() => {
      expect(screen.getByLabelText("workspace-editor")).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole("button", { name: "common.save" }));

    await waitFor(() => {
      expect(toastErrorMock).toHaveBeenCalledWith("workspace.saveFailed", {
        description: "workspace save denied",
      });
    });
  });
});
