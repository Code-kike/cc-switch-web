import fs from "node:fs/promises";
import path from "node:path";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import {
  afterAll,
  afterEach,
  beforeAll,
  beforeEach,
  describe,
  expect,
  it,
  vi,
} from "vitest";

import "@/lib/api/web-commands";
import { UsageDashboard } from "@/components/usage/UsageDashboard";
import { setCsrfToken } from "@/lib/api/adapter";
import { server } from "../msw/server";
import {
  startTestWebServer,
  type TestWebServer,
} from "../helpers/web-server";

const toastSuccessMock = vi.fn();
const toastErrorMock = vi.fn();

const smokeUsage = {
  codexSessionId: "019cc36c-bd7c-7891-b371-7b20b4fe0b21",
  archivedFileName: "page-usage-session.jsonl",
  model: "openai/gpt-5.4",
  sessionMetaTimestamp: new Date(Date.now() - 4_000).toISOString(),
  turnContextTimestamp: new Date(Date.now() - 3_000).toISOString(),
  eventTimestamp: new Date(Date.now() - 2_000).toISOString(),
  inputTokens: 1200,
  cachedInputTokens: 300,
  outputTokens: 450,
} as const;

type SmokeUsageFixture = {
  codexSessionId: string;
  archivedFileName: string;
  model: string;
  sessionMetaTimestamp: string;
  turnContextTimestamp: string;
  eventTimestamp: string;
  inputTokens: number;
  cachedInputTokens: number;
  outputTokens: number;
};

type FetchSpy = {
  mock: {
    calls: unknown[][];
  };
  mockRestore: () => void;
};

type UsageDataSource = {
  dataSource: string;
  requestCount: number;
};

type RequestLog = {
  requestId: string;
};

type RequestLogsResponse = {
  data: RequestLog[];
  total: number;
};

type ProviderStats = {
  providerName: string;
  requestCount: number;
};

type ModelStats = {
  model: string;
  requestCount: number;
};

const viewButtonName = /^(common\.view|View)$/;
const noDataText = /^(usage\.noData|No data)$/;
const todayRangeName = /^(usage\.presetToday|Today|当天)$/;
const confirmButtonName = /^(common\.confirm|Confirm)$/;
const claudeFilterName = /^(usage\.appFilter\.claude|Claude Code)$/;
const codexFilterName = /^(usage\.appFilter\.codex|Codex)$/;
const providerStatsTabName = /^(usage\.providerStats|Provider Stats)$/;
const modelStatsTabName = /^(usage\.modelStats|Model Stats)$/;

vi.mock("sonner", () => ({
  toast: {
    success: (...args: unknown[]) => toastSuccessMock(...args),
    error: (...args: unknown[]) => toastErrorMock(...args),
    info: vi.fn(),
  },
}));

async function writeFixtureFile(
  filePath: string,
  content: string,
): Promise<void> {
  await fs.mkdir(path.dirname(filePath), { recursive: true });
  await fs.writeFile(filePath, content, "utf8");
}

function createSmokeUsageFixture(suffix: string): SmokeUsageFixture {
  const now = Date.now();
  return {
    codexSessionId: `019cc36c-${suffix}-7891-b371-7b20b4fe0b21`,
    archivedFileName: `page-usage-session-${suffix}.jsonl`,
    model: smokeUsage.model,
    sessionMetaTimestamp: new Date(now - 4_000).toISOString(),
    turnContextTimestamp: new Date(now - 3_000).toISOString(),
    eventTimestamp: new Date(now - 2_000).toISOString(),
    inputTokens: smokeUsage.inputTokens,
    cachedInputTokens: smokeUsage.cachedInputTokens,
    outputTokens: smokeUsage.outputTokens,
  };
}

function getArchivedUsagePathForFixture(
  homeDir: string,
  usage: SmokeUsageFixture,
): string {
  return path.join(
    homeDir,
    ".codex",
    "archived_sessions",
    usage.archivedFileName,
  );
}

async function seedUsageFixture(
  homeDir: string,
  usage: SmokeUsageFixture = smokeUsage,
): Promise<void> {
  const usageLines = [
    JSON.stringify({
      timestamp: usage.sessionMetaTimestamp,
      type: "session_meta",
      payload: {
        session_id: usage.codexSessionId,
      },
    }),
    JSON.stringify({
      timestamp: usage.turnContextTimestamp,
      type: "turn_context",
      payload: {
        model: usage.model,
      },
    }),
    JSON.stringify({
      timestamp: usage.eventTimestamp,
      type: "event_msg",
      payload: {
        type: "token_count",
        info: {
          model: usage.model,
          total_token_usage: {
            input_tokens: usage.inputTokens,
            cached_input_tokens: usage.cachedInputTokens,
            output_tokens: usage.outputTokens,
          },
        },
      },
    }),
  ];

  await writeFixtureFile(
    getArchivedUsagePathForFixture(homeDir, usage),
    `${usageLines.join("\n")}\n`,
  );
}

const renderDashboard = () => {
  const client = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  });

  render(
    <QueryClientProvider client={client}>
      <UsageDashboard />
    </QueryClientProvider>,
  );
};

const requestJson = async <T,>(
  input: string | URL,
  init?: RequestInit,
): Promise<T> => {
  const response = await fetch(input, init);
  if (!response.ok) {
    throw new Error(`request failed: ${response.status} ${String(input)}`);
  }
  return (await response.json()) as T;
};

const buildUrl = (
  pathname: string,
  baseUrl: string,
  query?: Record<string, string | number | undefined>,
): URL => {
  const url = new URL(pathname, baseUrl);
  for (const [key, value] of Object.entries(query ?? {})) {
    if (value == null || value === "") continue;
    url.searchParams.set(key, String(value));
  }
  return url;
};

const getUsageDataSources = async (
  baseUrl: string,
): Promise<UsageDataSource[]> =>
  await requestJson(new URL("/api/usage/get-usage-data-sources", baseUrl));

const getRequestLogs = async (
  baseUrl: string,
  appType: "claude" | "codex" | "gemini" = "codex",
): Promise<RequestLogsResponse> =>
  await requestJson(new URL("/api/system/get_request_logs", baseUrl), {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
    },
    body: JSON.stringify({
      filters: {
        appType,
      },
      page: 0,
      pageSize: 20,
    }),
  });

const getProviderStats = async (
  baseUrl: string,
  appType?: "claude" | "codex" | "gemini",
): Promise<ProviderStats[]> =>
  await requestJson(
    buildUrl("/api/providers/get-provider-stats", baseUrl, { appType }),
  );

const getModelStats = async (
  baseUrl: string,
  appType?: "claude" | "codex" | "gemini",
): Promise<ModelStats[]> =>
  await requestJson(new URL("/api/system/get_model_stats", baseUrl), {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
    },
    body: JSON.stringify({ appType }),
  });

function formatDateInputValue(date: Date): string {
  return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(
    2,
    "0",
  )}-${String(date.getDate()).padStart(2, "0")}`;
}

function normalizeFetchInput(input: unknown): string {
  if (typeof input === "string") return input;
  if (input instanceof URL) return input.toString();
  if (typeof Request !== "undefined" && input instanceof Request) {
    return input.url;
  }
  return String(input);
}

function countFetchCalls(fetchSpy: FetchSpy, path: string): number {
  return fetchSpy.mock.calls.filter(([input]) =>
    normalizeFetchInput(input).includes(path),
  ).length;
}

describe.sequential("UsageDashboard against real web server", () => {
  let webServer: TestWebServer;
  let fetchSpy: FetchSpy;

  beforeAll(async () => {
    server.close();
    webServer = await startTestWebServer();
    await seedUsageFixture(webServer.homeDir);
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
    fetchSpy = vi.spyOn(globalThis, "fetch") as unknown as FetchSpy;
  });

  afterEach(() => {
    fetchSpy.mockRestore();
  });

  it(
    "keeps session import visible in the empty state and hydrates logs after sync",
    async () => {
      renderDashboard();

      expect(await screen.findByText("usage.title")).toBeInTheDocument();
      expect(
        await screen.findByText(
          "No usage data yet. Import session logs to populate this dashboard.",
        ),
      ).toBeInTheDocument();

      fireEvent.click(
        screen.getByRole("button", {
          name: /Import Sessions/,
        }),
      );

      await waitFor(() => expect(toastSuccessMock).toHaveBeenCalledTimes(1));

      let requestLogs: RequestLogsResponse | undefined;
      await waitFor(async () => {
        const sources = await getUsageDataSources(webServer.baseUrl);
        expect(sources.some((source) => source.requestCount > 0)).toBe(true);
        requestLogs = await getRequestLogs(webServer.baseUrl);
        expect(requestLogs.total).toBeGreaterThan(0);
      });

      const codexSessionMatches = await screen.findAllByText(
        /^(codex_session|usage\.dataSource\.codex_session)$/,
      );
      expect(codexSessionMatches.length).toBeGreaterThan(0);
      expect(
        screen.getByRole("button", {
          name: /Sync/,
        }),
      ).toBeInTheDocument();

      fireEvent.click(
        screen.getByRole("button", {
          name: /^(common\.view|View)$/,
        }),
      );

      expect(
        await screen.findByText(/^(usage\.requestDetail|请求详情)$/),
      ).toBeInTheDocument();
      expect(
        await screen.findByText(requestLogs!.data[0]!.requestId),
      ).toBeInTheDocument();
      expect(toastErrorMock).not.toHaveBeenCalled();
    },
    20_000,
  );

  it(
    "links app filter, refresh cadence, date range, and logs/providers/models tabs to real web data",
    async () => {
      await seedUsageFixture(
        webServer.homeDir,
        createSmokeUsageFixture("filter-refresh-range"),
      );

      renderDashboard();

      fireEvent.click(
        screen.getByRole("button", {
          name: /Import Sessions/,
        }),
      );

      await waitFor(() => expect(toastSuccessMock).toHaveBeenCalledTimes(1));
      await waitFor(async () => {
        const codexLogs = await getRequestLogs(webServer.baseUrl, "codex");
        expect(codexLogs.total).toBeGreaterThan(0);
      });
      await waitFor(() =>
        expect(
          screen.getAllByRole("button", { name: viewButtonName }).length,
        ).toBeGreaterThan(0),
      );

      const codexProviderStats = await getProviderStats(webServer.baseUrl, "codex");
      const codexModelStats = await getModelStats(webServer.baseUrl, "codex");
      const claudeRequestLogs = await getRequestLogs(webServer.baseUrl, "claude");
      const claudeProviderStats = await getProviderStats(webServer.baseUrl, "claude");
      const claudeModelStats = await getModelStats(webServer.baseUrl, "claude");

      expect(codexProviderStats.length).toBeGreaterThan(0);
      expect(codexModelStats.length).toBeGreaterThan(0);
      expect(claudeRequestLogs.total).toBe(0);
      expect(claudeProviderStats).toHaveLength(0);
      expect(claudeModelStats).toHaveLength(0);

      const summaryFetchesBeforeRefresh = countFetchCalls(
        fetchSpy,
        "/api/usage/get-usage-summary",
      );
      fireEvent.click(screen.getByRole("button", { name: "30s" }));

      await waitFor(() =>
        expect(screen.getByRole("button", { name: "60s" })).toBeInTheDocument(),
      );
      await waitFor(() =>
        expect(
          countFetchCalls(fetchSpy, "/api/usage/get-usage-summary"),
        ).toBeGreaterThan(summaryFetchesBeforeRefresh),
      );

      expect(
        within(screen.getByRole("tabpanel")).getAllByRole("button", {
          name: viewButtonName,
        }).length,
      ).toBeGreaterThan(0);

      fireEvent.click(screen.getByRole("button", { name: claudeFilterName }));

      await waitFor(() =>
        expect(
          within(screen.getByRole("tabpanel")).getByText(noDataText),
        ).toBeInTheDocument(),
      );

      fireEvent.click(screen.getByRole("tab", { name: providerStatsTabName }));
      await waitFor(() =>
        expect(
          within(screen.getByRole("tabpanel")).getByText(noDataText),
        ).toBeInTheDocument(),
      );

      fireEvent.click(screen.getByRole("tab", { name: modelStatsTabName }));
      await waitFor(() =>
        expect(
          within(screen.getByRole("tabpanel")).getByText(noDataText),
        ).toBeInTheDocument(),
      );

      fireEvent.click(screen.getByRole("button", { name: codexFilterName }));
      fireEvent.click(screen.getByRole("tab", { name: modelStatsTabName }));

      await waitFor(() =>
        expect(
          within(screen.getByRole("tabpanel")).getAllByText(
            codexModelStats[0]!.model,
          ).length,
        ).toBeGreaterThan(0),
      );

      fireEvent.click(screen.getByRole("tab", { name: providerStatsTabName }));
      await waitFor(() =>
        expect(
          within(screen.getByRole("tabpanel")).getAllByText(
            codexProviderStats[0]!.providerName,
          ).length,
        ).toBeGreaterThan(0),
      );

      fireEvent.click(screen.getAllByRole("button", { name: todayRangeName })[0]!);

      const twoDaysAgo = new Date(Date.now() - 2 * 24 * 60 * 60 * 1000);
      const emptyDate = formatDateInputValue(twoDaysAgo);
      const dateInputs = Array.from(
        document.querySelectorAll<HTMLInputElement>('input[type="date"]'),
      );
      expect(dateInputs).toHaveLength(2);

      for (const input of dateInputs) {
        fireEvent.change(input, {
          target: { value: emptyDate },
        });
      }

      fireEvent.click(screen.getByRole("button", { name: confirmButtonName }));

      await waitFor(() =>
        expect(
          within(screen.getByRole("tabpanel")).getByText(noDataText),
        ).toBeInTheDocument(),
      );

      fireEvent.click(screen.getByRole("tab", { name: modelStatsTabName }));
      await waitFor(() =>
        expect(
          within(screen.getByRole("tabpanel")).getByText(noDataText),
        ).toBeInTheDocument(),
      );

      expect(toastErrorMock).not.toHaveBeenCalled();
    },
    20_000,
  );
});
