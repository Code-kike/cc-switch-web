import fs from "node:fs/promises";
import path from "node:path";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterAll, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";

import "@/lib/api/web-commands";
import { SessionManagerPage } from "@/components/sessions/SessionManagerPage";
import { setCsrfToken } from "@/lib/api/adapter";
import type { SessionMeta } from "@/types";
import { server } from "../msw/server";
import {
  startTestWebServer,
  type TestWebServer,
} from "../helpers/web-server";

const toastSuccessMock = vi.fn();
const toastErrorMock = vi.fn();

const smokeSessions = [
  {
    sessionId: "019cc369-bd7c-7891-b371-7b20b4fe0b18",
    subdir: "project-alpha",
    fileName: "alpha-session.jsonl",
    projectDir: "/tmp/smoke-alpha",
    userMessage: "Alpha smoke request",
    assistantMessage: "Alpha smoke response",
    metaTimestamp: "2026-03-06T21:50:12Z",
    userTimestamp: "2026-03-06T21:50:13Z",
    assistantTimestamp: "2026-03-06T21:50:14Z",
  },
  {
    sessionId: "019cc36a-bd7c-7891-b371-7b20b4fe0b19",
    subdir: "project-beta",
    fileName: "beta-session.jsonl",
    projectDir: "/tmp/smoke-beta",
    userMessage: "Beta smoke request",
    assistantMessage: "Beta smoke response",
    metaTimestamp: "2026-03-06T21:51:12Z",
    userTimestamp: "2026-03-06T21:51:13Z",
    assistantTimestamp: "2026-03-06T21:51:14Z",
  },
  {
    sessionId: "019cc36b-bd7c-7891-b371-7b20b4fe0b20",
    subdir: "project-gamma",
    fileName: "gamma-session.jsonl",
    projectDir: "/tmp/smoke-gamma",
    userMessage: "Gamma smoke request",
    assistantMessage: "Gamma smoke response",
    metaTimestamp: "2026-03-06T21:52:12Z",
    userTimestamp: "2026-03-06T21:52:13Z",
    assistantTimestamp: "2026-03-06T21:52:14Z",
  },
] as const;

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

async function writeFixtureFile(
  filePath: string,
  content: string,
): Promise<void> {
  await fs.mkdir(path.dirname(filePath), { recursive: true });
  await fs.writeFile(filePath, content, "utf8");
}

function getSmokeSessionPath(
  homeDir: string,
  session: (typeof smokeSessions)[number],
): string {
  return path.join(homeDir, ".codex", "sessions", session.subdir, session.fileName);
}

async function seedSessionFixtures(homeDir: string): Promise<void> {
  for (const session of smokeSessions) {
    const lines = [
      JSON.stringify({
        timestamp: session.metaTimestamp,
        type: "session_meta",
        payload: {
          id: session.sessionId,
          cwd: session.projectDir,
        },
      }),
      JSON.stringify({
        timestamp: session.userTimestamp,
        type: "response_item",
        payload: {
          type: "message",
          role: "user",
          content: session.userMessage,
        },
      }),
      JSON.stringify({
        timestamp: session.assistantTimestamp,
        type: "response_item",
        payload: {
          type: "message",
          role: "assistant",
          content: session.assistantMessage,
        },
      }),
    ];

    await writeFixtureFile(
      getSmokeSessionPath(homeDir, session),
      `${lines.join("\n")}\n`,
    );
  }
}

const renderPage = () => {
  const client = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  });

  return render(
    <QueryClientProvider client={client}>
      <SessionManagerPage appId="codex" />
    </QueryClientProvider>,
  );
};

const getSessions = async (baseUrl: string): Promise<SessionMeta[]> => {
  const response = await fetch(new URL("/api/sessions/list-sessions", baseUrl));
  if (!response.ok) {
    throw new Error(`failed to load sessions: ${response.status}`);
  }
  return (await response.json()) as SessionMeta[];
};

const sortSessions = (sessions: SessionMeta[]): SessionMeta[] =>
  [...sessions].sort(
    (a, b) =>
      (b.lastActiveAt ?? b.createdAt ?? 0) - (a.lastActiveAt ?? a.createdAt ?? 0),
  );

describe.sequential("SessionManagerPage against real web server", () => {
  let webServer: TestWebServer;

  beforeAll(async () => {
    server.close();
    webServer = await startTestWebServer();
    await seedSessionFixtures(webServer.homeDir);
  }, 360_000);

  afterAll(async () => {
    await webServer.stop();
    server.listen({ onUnhandledRequest: "warn" });
  }, 20_000);

  beforeEach(() => {
    toastSuccessMock.mockReset();
    toastErrorMock.mockReset();
    setCsrfToken(null);
    Element.prototype.scrollIntoView = vi.fn();
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
    Object.defineProperty(window.navigator, "clipboard", {
      configurable: true,
      value: {
        writeText: vi.fn().mockResolvedValue(undefined),
      },
    });
  });

  it(
    "lists sessions, loads messages, copies resume commands, and batch deletes through the rendered page UI",
    async () => {
      renderPage();

      let sessions: SessionMeta[] = [];
      await waitFor(async () => {
        sessions = sortSessions(await getSessions(webServer.baseUrl));
        expect(sessions).toHaveLength(smokeSessions.length);
      });

      expect(
        await screen.findByRole("heading", { name: sessions[0].title! }),
      ).toBeInTheDocument();
      expect(screen.getAllByText("Gamma smoke request").length).toBeGreaterThan(0);
      expect(screen.getByText("Beta smoke request")).toBeInTheDocument();
      expect(screen.getByText("Alpha smoke request")).toBeInTheDocument();

      fireEvent.click(screen.getByText("Alpha smoke request"));

      await waitFor(() =>
        expect(
          screen.getByRole("heading", { name: "Alpha smoke request" }),
        ).toBeInTheDocument(),
      );
      expect(
        screen.getByText(`codex resume ${smokeSessions[0].sessionId}`),
      ).toBeInTheDocument();

      fireEvent.click(screen.getByRole("button", { name: /恢复会话/i }));

      await waitFor(() =>
        expect(window.navigator.clipboard.writeText).toHaveBeenCalledWith(
          `codex resume ${smokeSessions[0].sessionId}`,
        ),
      );
      expect(toastSuccessMock).toHaveBeenCalledWith(
        "sessionManager.resumeCommandCopied",
      );

      fireEvent.click(screen.getByRole("button", { name: /批量管理/i }));
      fireEvent.click(screen.getByRole("button", { name: /全选当前/i }));
      fireEvent.click(screen.getByRole("button", { name: /批量删除/i }));

      await screen.findByText("批量删除会话");
      fireEvent.click(screen.getByRole("button", { name: /删除所选会话/i }));

      await waitFor(async () => {
        expect(await getSessions(webServer.baseUrl)).toEqual([]);
      });
      await waitFor(async () => {
        await Promise.all(
          smokeSessions.map(async (session) => {
            await expect(
              fs.access(getSmokeSessionPath(webServer.homeDir, session)),
            ).rejects.toThrow();
          }),
        );
      });

      expect(screen.getByText("sessionManager.noSessions")).toBeInTheDocument();
      expect(screen.getByText("sessionManager.selectSession")).toBeInTheDocument();
      expect(toastErrorMock).not.toHaveBeenCalled();
    },
    20_000,
  );
});
