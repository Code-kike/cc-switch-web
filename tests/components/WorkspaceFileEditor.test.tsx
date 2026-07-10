import {
  act,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
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

function createDeferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason?: unknown) => void;
  const promise = new Promise<T>((resolvePromise, rejectPromise) => {
    resolve = resolvePromise;
    reject = rejectPromise;
  });
  return { promise, resolve, reject };
}

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

  it("keeps the latest file content and saves it to the matching filename", async () => {
    const firstRead = createDeferred<string | null>();
    const secondRead = createDeferred<string | null>();
    readFileMock.mockImplementation((filename: string) =>
      filename === "AGENTS.md" ? firstRead.promise : secondRead.promise,
    );

    const { rerender } = render(
      <WorkspaceFileEditor
        filename="AGENTS.md"
        isOpen={true}
        onClose={() => undefined}
      />,
    );

    rerender(
      <WorkspaceFileEditor
        filename="CLAUDE.md"
        isOpen={true}
        onClose={() => undefined}
      />,
    );

    await waitFor(() => {
      expect(readFileMock).toHaveBeenCalledWith("CLAUDE.md");
    });
    expect(screen.getByRole("button", { name: "common.save" })).toBeDisabled();

    await act(async () => {
      secondRead.resolve("# Claude");
    });
    expect(screen.getByLabelText("workspace-editor")).toHaveValue("# Claude");

    await act(async () => {
      firstRead.resolve("# Agents");
    });
    expect(screen.getByLabelText("workspace-editor")).toHaveValue("# Claude");

    fireEvent.change(screen.getByLabelText("workspace-editor"), {
      target: { value: "# Claude updated" },
    });
    fireEvent.click(screen.getByRole("button", { name: "common.save" }));

    await waitFor(() => {
      expect(writeFileMock).toHaveBeenCalledWith(
        "CLAUDE.md",
        "# Claude updated",
      );
    });
  });

  it("ignores a stale read failure without unlocking or failing the new file", async () => {
    const firstRead = createDeferred<string | null>();
    const secondRead = createDeferred<string | null>();
    readFileMock.mockImplementation((filename: string) =>
      filename === "AGENTS.md" ? firstRead.promise : secondRead.promise,
    );

    const { rerender } = render(
      <WorkspaceFileEditor
        filename="AGENTS.md"
        isOpen={true}
        onClose={() => undefined}
      />,
    );
    rerender(
      <WorkspaceFileEditor
        filename="CLAUDE.md"
        isOpen={true}
        onClose={() => undefined}
      />,
    );

    await act(async () => {
      firstRead.reject(new Error("stale failure"));
    });

    expect(screen.queryByLabelText("workspace-editor")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "common.save" })).toBeDisabled();
    expect(toastErrorMock).not.toHaveBeenCalled();

    await act(async () => {
      secondRead.resolve("# Claude");
    });
    expect(screen.getByLabelText("workspace-editor")).toHaveValue("# Claude");
    expect(screen.getByRole("button", { name: "common.save" })).toBeEnabled();
  });
});
