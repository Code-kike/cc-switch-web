import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import {
  act,
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { SessionManagerPage } from "@/components/sessions/SessionManagerPage";
import { sessionsApi } from "@/lib/api/sessions";
import type { SessionMessage, SessionMeta } from "@/types";
import { setSessionFixtures } from "../msw/state";

const toastSuccessMock = vi.fn();
const toastErrorMock = vi.fn();

vi.mock("sonner", () => ({
  toast: {
    success: (...args: unknown[]) => toastSuccessMock(...args),
    error: (...args: unknown[]) => toastErrorMock(...args),
  },
}));

vi.mock("@/components/sessions/SessionToc", () => ({
  SessionTocSidebar: () => null,
  SessionTocDialog: () => null,
}));

vi.mock("@/components/ConfirmDialog", () => ({
  ConfirmDialog: ({
    isOpen,
    title,
    message,
    confirmText,
    cancelText,
    onConfirm,
    onCancel,
  }: {
    isOpen: boolean;
    title: string;
    message: string;
    confirmText: string;
    cancelText: string;
    onConfirm: () => void;
    onCancel: () => void;
  }) =>
    isOpen ? (
      <div data-testid="confirm-dialog">
        <div>{title}</div>
        <div>{message}</div>
        <button onClick={onConfirm}>{confirmText}</button>
        <button onClick={onCancel}>{cancelText}</button>
      </div>
    ) : null,
}));

const launchTerminalSpy = vi
  .spyOn(sessionsApi, "launchTerminal")
  .mockResolvedValue(true);

const renderPage = () => {
  const client = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  });

  return {
    client,
    ...render(
      <QueryClientProvider client={client}>
        <SessionManagerPage appId="codex" />
      </QueryClientProvider>,
    ),
  };
};

const openSearch = () => {
  const searchButton = Array.from(screen.getAllByRole("button")).find(
    (button) => button.querySelector(".lucide-search"),
  );

  if (!searchButton) {
    throw new Error("Search button not found");
  }

  fireEvent.click(searchButton);
};

const closeSearch = () => {
  const closeButton = Array.from(screen.getAllByRole("button")).find(
    (button) => button.querySelector(".lucide-x"),
  );

  if (!closeButton) {
    throw new Error("Search close button not found");
  }

  fireEvent.click(closeButton);
};

const openProviderFilter = () => {
  const trigger = screen.getByRole("combobox");
  fireEvent.pointerDown(trigger);
  fireEvent.click(trigger);
};

describe("SessionManagerPage", () => {
  beforeEach(() => {
    toastSuccessMock.mockReset();
    toastErrorMock.mockReset();
    launchTerminalSpy.mockClear();
    Element.prototype.scrollIntoView = vi.fn();
    Object.defineProperty(HTMLElement.prototype, "hasPointerCapture", {
      configurable: true,
      value: vi.fn(() => false),
    });
    Object.defineProperty(HTMLElement.prototype, "setPointerCapture", {
      configurable: true,
      value: vi.fn(),
    });
    Object.defineProperty(HTMLElement.prototype, "releasePointerCapture", {
      configurable: true,
      value: vi.fn(),
    });
    Object.defineProperty(window, "__TAURI_INTERNALS__", {
      configurable: true,
      value: {},
    });
    Object.defineProperty(window.navigator, "clipboard", {
      configurable: true,
      value: {
        writeText: vi.fn().mockResolvedValue(undefined),
      },
    });

    const sessions: SessionMeta[] = [
      {
        providerId: "codex",
        sessionId: "codex-session-1",
        title: "Alpha Session",
        summary: "Alpha summary",
        projectDir: "/mock/codex",
        createdAt: 2,
        lastActiveAt: 20,
        sourcePath: "/mock/codex/session-1.jsonl",
        resumeCommand: "codex resume codex-session-1",
      },
      {
        providerId: "codex",
        sessionId: "codex-session-2",
        title: "Beta Session",
        summary: "Beta summary",
        projectDir: "/mock/codex",
        createdAt: 1,
        lastActiveAt: 10,
        sourcePath: "/mock/codex/session-2.jsonl",
        resumeCommand: "codex resume codex-session-2",
      },
    ];
    const messages: Record<string, SessionMessage[]> = {
      "codex:/mock/codex/session-1.jsonl": [
        { role: "user", content: "alpha", ts: 20 },
      ],
      "codex:/mock/codex/session-2.jsonl": [
        { role: "user", content: "beta", ts: 10 },
      ],
    };

    setSessionFixtures(sessions, messages);
  });

  it("deletes the selected session and selects the next visible session", async () => {
    renderPage();

    await waitFor(() =>
      expect(
        screen.getByRole("heading", { name: "Alpha Session" }),
      ).toBeInTheDocument(),
    );

    fireEvent.click(screen.getByRole("button", { name: /删除会话/i }));

    const dialog = screen.getByTestId("confirm-dialog");
    expect(dialog).toBeInTheDocument();
    expect(within(dialog).getByText(/Alpha Session/)).toBeInTheDocument();

    fireEvent.click(within(dialog).getByRole("button", { name: /删除会话/i }));

    await waitFor(() =>
      expect(
        screen.getByRole("heading", { name: "Beta Session" }),
      ).toBeInTheDocument(),
    );

    expect(screen.queryByText("Alpha Session")).not.toBeInTheDocument();
    expect(toastErrorMock).not.toHaveBeenCalled();
    expect(toastSuccessMock).toHaveBeenCalled();
  });

  it("filters the session list to the current app by default", async () => {
    setSessionFixtures(
      [
        {
          providerId: "codex",
          sessionId: "codex-session-1",
          title: "Codex Visible",
          summary: "Codex summary",
          projectDir: "/mock/codex",
          createdAt: 2,
          lastActiveAt: 20,
          sourcePath: "/mock/codex/session-1.jsonl",
          resumeCommand: "codex resume codex-session-1",
        },
        {
          providerId: "claude",
          sessionId: "claude-session-1",
          title: "Claude Hidden",
          summary: "Claude summary",
          projectDir: "/mock/claude",
          createdAt: 1,
          lastActiveAt: 10,
          sourcePath: "/mock/claude/session-1.jsonl",
          resumeCommand: "claude --resume claude-session-1",
        },
      ],
      {
        "codex:/mock/codex/session-1.jsonl": [
          { role: "user", content: "codex message", ts: 20 },
        ],
        "claude:/mock/claude/session-1.jsonl": [
          { role: "user", content: "claude message", ts: 10 },
        ],
      },
    );

    renderPage();

    await waitFor(() =>
      expect(
        screen.getByRole("heading", { name: "Codex Visible" }),
      ).toBeInTheDocument(),
    );

    expect(screen.queryByText("Claude Hidden")).not.toBeInTheDocument();
  });

  it("includes Hermes in the provider filter and narrows the list when selected", async () => {
    setSessionFixtures(
      [
        {
          providerId: "codex",
          sessionId: "codex-session-1",
          title: "Codex Visible",
          summary: "Codex summary",
          projectDir: "/mock/codex",
          createdAt: 2,
          lastActiveAt: 20,
          sourcePath: "/mock/codex/session-1.jsonl",
          resumeCommand: "codex resume codex-session-1",
        },
        {
          providerId: "hermes",
          sessionId: "hermes-session-1",
          title: "Hermes Session",
          summary: "Hermes summary",
          projectDir: "/mock/hermes",
          createdAt: 3,
          lastActiveAt: 30,
          sourcePath: "/mock/hermes/session-1.jsonl",
          resumeCommand: "hermes resume hermes-session-1",
        },
      ],
      {
        "codex:/mock/codex/session-1.jsonl": [
          { role: "user", content: "codex message", ts: 20 },
        ],
        "hermes:/mock/hermes/session-1.jsonl": [
          { role: "user", content: "hermes message", ts: 30 },
        ],
      },
    );

    renderPage();

    await waitFor(() =>
      expect(
        screen.getByRole("heading", { name: "Codex Visible" }),
      ).toBeInTheDocument(),
    );

    openProviderFilter();

    const hermesOption = await screen.findByText("Hermes");
    expect(hermesOption).toBeInTheDocument();

    fireEvent.click(hermesOption);

    await waitFor(() =>
      expect(
        screen.getByRole("heading", { name: "Hermes Session" }),
      ).toBeInTheDocument(),
    );

    expect(screen.queryByText("Codex Visible")).not.toBeInTheDocument();
  });

  it("loads messages for the selected session and refreshes the detail query when switching sessions", async () => {
    const getMessagesSpy = vi.spyOn(sessionsApi, "getMessages");

    renderPage();

    await waitFor(() =>
      expect(getMessagesSpy).toHaveBeenCalledWith(
        "codex",
        "/mock/codex/session-1.jsonl",
      ),
    );

    fireEvent.click(screen.getByText("Beta Session"));

    await waitFor(() =>
      expect(getMessagesSpy).toHaveBeenCalledWith(
        "codex",
        "/mock/codex/session-2.jsonl",
      ),
    );

    getMessagesSpy.mockRestore();
  });

  it("removes a deleted session from filtered search results", async () => {
    renderPage();

    await waitFor(() =>
      expect(
        screen.getByRole("heading", { name: "Alpha Session" }),
      ).toBeInTheDocument(),
    );

    openSearch();

    fireEvent.change(screen.getByRole("textbox"), {
      target: { value: "Alpha" },
    });

    await waitFor(() =>
      expect(
        screen.getByRole("heading", { name: "Alpha Session" }),
      ).toBeInTheDocument(),
    );

    fireEvent.click(screen.getByRole("button", { name: /删除会话/i }));

    const dialog = screen.getByTestId("confirm-dialog");
    fireEvent.click(within(dialog).getByRole("button", { name: /删除会话/i }));

    await waitFor(() =>
      expect(screen.queryByText("Alpha Session")).not.toBeInTheDocument(),
    );

    expect(
      screen.getByText("sessionManager.selectSession"),
    ).toBeInTheDocument();
    expect(
      screen.queryByText("sessionManager.emptySession"),
    ).not.toBeInTheDocument();
    expect(toastErrorMock).not.toHaveBeenCalled();
    expect(toastSuccessMock).toHaveBeenCalled();
  });

  it("restores batch delete controls when deleteMany rejects", async () => {
    const deleteManySpy = vi
      .spyOn(sessionsApi, "deleteMany")
      .mockRejectedValueOnce(new Error("network error"));

    renderPage();

    await waitFor(() =>
      expect(
        screen.getByRole("heading", { name: "Alpha Session" }),
      ).toBeInTheDocument(),
    );

    fireEvent.click(screen.getByRole("button", { name: /批量管理/i }));
    fireEvent.click(screen.getByRole("button", { name: /全选当前/i }));
    fireEvent.click(screen.getByRole("button", { name: /批量删除/i }));

    const dialog = screen.getByTestId("confirm-dialog");
    fireEvent.click(
      within(dialog).getByRole("button", { name: /删除所选会话/i }),
    );

    await waitFor(() =>
      expect(toastErrorMock).toHaveBeenCalledWith("network error"),
    );

    await waitFor(() =>
      expect(
        screen.getByRole("button", { name: /批量删除/i }),
      ).not.toBeDisabled(),
    );

    deleteManySpy.mockRestore();
  });

  it("keeps the exit batch mode button visible when search hides all sessions", async () => {
    renderPage();

    await waitFor(() =>
      expect(
        screen.getByRole("heading", { name: "Alpha Session" }),
      ).toBeInTheDocument(),
    );

    fireEvent.click(screen.getByRole("button", { name: /批量管理/i }));
    openSearch();
    fireEvent.change(screen.getByRole("textbox"), {
      target: { value: "NoSuchSession" },
    });

    await waitFor(() => expect(screen.queryByText("Alpha Session")).toBeNull());

    expect(screen.getByRole("button", { name: /退出批量管理/i })).toBeVisible();
  });

  it("drops hidden selections when search narrows the result set", async () => {
    renderPage();

    await waitFor(() =>
      expect(
        screen.getByRole("heading", { name: "Alpha Session" }),
      ).toBeInTheDocument(),
    );

    fireEvent.click(screen.getByRole("button", { name: /批量管理/i }));
    fireEvent.click(screen.getByRole("button", { name: /全选当前/i }));

    expect(screen.getByText("已选 2 项")).toBeInTheDocument();

    openSearch();
    fireEvent.change(screen.getByRole("textbox"), {
      target: { value: "Alpha" },
    });

    await waitFor(() =>
      expect(screen.queryByText("Beta Session")).not.toBeInTheDocument(),
    );

    closeSearch();

    await waitFor(() =>
      expect(screen.getByText("已选 1 项")).toBeInTheDocument(),
    );
  });

  it("removes successfully deleted sessions from the UI before refetch completes", async () => {
    const view = renderPage();
    let resolveInvalidate!: () => void;
    const invalidateSpy = vi
      .spyOn(view.client, "invalidateQueries")
      .mockImplementation(
        () =>
          new Promise((resolve) => {
            resolveInvalidate = () => resolve(undefined);
          }),
      );

    await waitFor(() =>
      expect(
        screen.getByRole("heading", { name: "Alpha Session" }),
      ).toBeInTheDocument(),
    );

    fireEvent.click(screen.getByRole("button", { name: /批量管理/i }));
    fireEvent.click(screen.getByRole("button", { name: /全选当前/i }));
    fireEvent.click(screen.getByRole("button", { name: /批量删除/i }));

    const dialog = screen.getByTestId("confirm-dialog");
    fireEvent.click(
      within(dialog).getByRole("button", { name: /删除所选会话/i }),
    );

    await waitFor(() => {
      expect(screen.queryByText("Alpha Session")).not.toBeInTheDocument();
      expect(screen.queryByText("Beta Session")).not.toBeInTheDocument();
    });

    await act(async () => {
      resolveInvalidate();
    });
    invalidateSpy.mockRestore();
  });

  it("keeps failed selections while removing successfully batch-deleted sessions", async () => {
    const view = renderPage();
    let resolveInvalidate!: () => void;
    const invalidateSpy = vi
      .spyOn(view.client, "invalidateQueries")
      .mockImplementation(
        () =>
          new Promise((resolve) => {
            resolveInvalidate = () => resolve(undefined);
          }),
      );
    const deleteManySpy = vi.spyOn(sessionsApi, "deleteMany").mockResolvedValue([
      {
        providerId: "codex",
        sessionId: "codex-session-1",
        sourcePath: "/mock/codex/session-1.jsonl",
        success: true,
      },
      {
        providerId: "codex",
        sessionId: "codex-session-2",
        sourcePath: "/mock/codex/session-2.jsonl",
        success: false,
        error: "permission denied",
      },
    ]);

    await waitFor(() =>
      expect(
        screen.getByRole("heading", { name: "Alpha Session" }),
      ).toBeInTheDocument(),
    );

    fireEvent.click(screen.getByRole("button", { name: /批量管理/i }));
    fireEvent.click(screen.getByRole("button", { name: /全选当前/i }));
    fireEvent.click(screen.getByRole("button", { name: /批量删除/i }));

    const dialog = screen.getByTestId("confirm-dialog");
    fireEvent.click(
      within(dialog).getByRole("button", { name: /删除所选会话/i }),
    );

    await waitFor(() => {
      expect(screen.queryByText("Alpha Session")).not.toBeInTheDocument();
      expect(screen.getByText("Beta Session")).toBeInTheDocument();
    });
    expect(screen.getByText("已选 1 项")).toBeInTheDocument();
    expect(toastSuccessMock).toHaveBeenCalledWith("已删除 1 个会话");
    expect(toastErrorMock).toHaveBeenCalledWith("1 个会话删除失败", {
      description: "permission denied",
    });

    await act(async () => {
      resolveInvalidate();
    });

    deleteManySpy.mockRestore();
    invalidateSpy.mockRestore();
  });

  it("copies the resume command in web mode instead of launching a terminal", async () => {
    Object.defineProperty(window, "__TAURI_INTERNALS__", {
      configurable: true,
      value: undefined,
    });

    renderPage();

    await waitFor(() =>
      expect(
        screen.getByRole("heading", { name: "Alpha Session" }),
      ).toBeInTheDocument(),
    );

    fireEvent.click(screen.getByRole("button", { name: /恢复会话/i }));

    await waitFor(() =>
      expect(window.navigator.clipboard.writeText).toHaveBeenCalledWith(
        "codex resume codex-session-1",
      ),
    );

    expect(launchTerminalSpy).not.toHaveBeenCalled();
    expect(toastSuccessMock).toHaveBeenCalled();
  });
});
