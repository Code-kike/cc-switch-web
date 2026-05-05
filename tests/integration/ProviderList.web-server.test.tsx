import http from "node:http";
import fs from "node:fs/promises";
import { execFile } from "node:child_process";
import type { AddressInfo } from "node:net";
import path from "node:path";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { useState } from "react";
import {
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import { afterAll, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";

import "@/lib/api/web-commands";
import { ProviderList } from "@/components/providers/ProviderList";
import { EditProviderDialog } from "@/components/providers/EditProviderDialog";
import { setCsrfToken } from "@/lib/api/adapter";
import { configApi } from "@/lib/api";
import { failoverApi } from "@/lib/api/failover";
import { providersApi } from "@/lib/api/providers";
import { settingsApi } from "@/lib/api/settings";
import { useProviderActions } from "@/hooks/useProviderActions";
import { useProvidersQuery } from "@/lib/query";
import type { AppId } from "@/lib/api";
import type { Provider } from "@/types";
import { server } from "../msw/server";
import {
  startTestWebServer,
  type TestWebServer,
} from "../helpers/web-server";

const toastSuccessMock = vi.fn();
const toastErrorMock = vi.fn();
const toastInfoMock = vi.fn();

vi.mock("sonner", () => ({
  toast: {
    success: (...args: unknown[]) => toastSuccessMock(...args),
    error: (...args: unknown[]) => toastErrorMock(...args),
    info: (...args: unknown[]) => toastInfoMock(...args),
  },
}));

const claudeSettingsPath = (homeDir: string): string =>
  path.join(homeDir, ".claude", "settings.json");

const opencodeSettingsPath = (homeDir: string): string =>
  path.join(homeDir, ".config", "opencode", "opencode.json");

const openclawSettingsPath = (homeDir: string): string =>
  path.join(homeDir, ".openclaw", "openclaw.json");

const hermesSettingsPath = (homeDir: string): string =>
  path.join(homeDir, ".hermes", "config.yaml");

const buildClaudeProvider = (
  id: string,
  name: string,
  token: string,
  baseUrl: string,
  sortIndex: number,
): Provider => ({
  id,
  name,
  category: "custom",
  sortIndex,
  settingsConfig: {
    env: {
      ANTHROPIC_AUTH_TOKEN: token,
      ANTHROPIC_BASE_URL: baseUrl,
    },
    ui: {
      displayName: name,
    },
  },
});

async function writeJsonFixture(filePath: string, value: unknown): Promise<void> {
  await fs.mkdir(path.dirname(filePath), { recursive: true });
  await fs.writeFile(filePath, JSON.stringify(value, null, 2), "utf8");
}

async function writeTextFixture(filePath: string, value: string): Promise<void> {
  await fs.mkdir(path.dirname(filePath), { recursive: true });
  await fs.writeFile(filePath, value, "utf8");
}

async function readJsonFixture<T>(filePath: string): Promise<T> {
  return JSON.parse(await fs.readFile(filePath, "utf8")) as T;
}

async function execSqlite(dbPath: string, sql: string): Promise<void> {
  await new Promise<void>((resolve, reject) => {
    execFile("sqlite3", [dbPath, sql], (error, _stdout, stderr) => {
      if (error) {
        reject(
          new Error(
            `sqlite3 failed for ${dbPath}: ${stderr || error.message}`,
          ),
        );
        return;
      }
      resolve();
    });
  });
}

async function insertProxyUsageLog(
  dataDir: string,
  {
    requestId,
    providerId,
    appType,
    totalCostUsd,
    latencyMs,
    createdAt,
  }: {
    requestId: string;
    providerId: string;
    appType: AppId;
    totalCostUsd: string;
    latencyMs: number;
    createdAt: number;
  },
): Promise<void> {
  const dbPath = path.join(dataDir, "cc-switch.db");
  await execSqlite(
    dbPath,
    [
      "INSERT INTO proxy_request_logs (",
      "request_id, provider_id, app_type, model, input_tokens, output_tokens, total_cost_usd, latency_ms, status_code, created_at",
      ") VALUES (",
      `'${requestId}', '${providerId}', '${appType}', 'claude-3-5-sonnet', 100, 50, '${totalCostUsd}', ${latencyMs}, 200, ${createdAt}`,
      ");",
    ].join(" "),
  );
}

function mockApiFailure(
  pathname: string,
  message: string,
  baseUrl: string,
): { restore: () => void } {
  const nativeFetch = globalThis.fetch.bind(globalThis);
  const fetchSpy = vi
    .spyOn(globalThis, "fetch")
    .mockImplementation(async (input: RequestInfo | URL, init?: RequestInit) => {
      const rawUrl =
        typeof input === "string"
          ? input
          : input instanceof URL
            ? input.toString()
            : input.url;
      const requestUrl = new URL(rawUrl, baseUrl);
      if (requestUrl.pathname === pathname) {
        return new Response(JSON.stringify({ message }), {
          status: 500,
          headers: { "Content-Type": "application/json; charset=utf-8" },
        });
      }
      return nativeFetch(input, init);
    });

  return {
    restore: () => fetchSpy.mockRestore(),
  };
}

function ProviderPageHarness({
  appId,
  isProxyRunning = false,
  isProxyTakeover = false,
}: {
  appId: AppId;
  isProxyRunning?: boolean;
  isProxyTakeover?: boolean;
}) {
  const { data, isLoading } = useProvidersQuery(appId);
  const providers = data?.providers ?? {};
  const currentProviderId = data?.currentProviderId ?? "";
  const { switchProvider, updateProvider } = useProviderActions(appId);
  const [editingProvider, setEditingProvider] = useState<Provider | null>(null);

  return (
    <>
      <ProviderList
        providers={providers}
        currentProviderId={currentProviderId}
        appId={appId}
        isLoading={isLoading}
        isProxyRunning={isProxyRunning}
        isProxyTakeover={isProxyTakeover}
        onSwitch={switchProvider}
        onEdit={setEditingProvider}
        onDelete={() => undefined}
        onDuplicate={() => undefined}
        onOpenWebsite={() => undefined}
      />
      <EditProviderDialog
        open={Boolean(editingProvider)}
        provider={editingProvider}
        onOpenChange={(open) => {
          if (!open) {
            setEditingProvider(null);
          }
        }}
        onSubmit={async ({ provider, originalId }) => {
          await updateProvider(provider, originalId);
          setEditingProvider(null);
        }}
        appId={appId}
      />
    </>
  );
}

function renderProviderPage(
  appId: AppId,
  options: { isProxyRunning?: boolean; isProxyTakeover?: boolean } = {},
) {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  });

  render(
    <QueryClientProvider client={queryClient}>
      <ProviderPageHarness
        appId={appId}
        isProxyRunning={options.isProxyRunning}
        isProxyTakeover={options.isProxyTakeover}
      />
    </QueryClientProvider>,
  );

  return { queryClient };
}

function getProviderCard(providerName: string): HTMLElement {
  const label = screen.getByText(providerName);
  let current: HTMLElement | null = label instanceof HTMLElement ? label : null;

  while (
    current &&
    !current.className.includes("relative overflow-hidden rounded-xl border")
  ) {
    current = current.parentElement;
  }

  if (!(current instanceof HTMLElement)) {
    throw new Error(`could not locate provider card for ${providerName}`);
  }

  return current;
}

const importCurrentRegex = /^(provider\.importCurrent|导入当前配置|Import Current)$/;
const enableRegex = /^(provider\.enable|启用|Enable)$/;
const editRegex = /^(common\.edit|编辑|Edit)$/;
const saveRegex = /^(common\.save|保存|Save)$/;
const providerNameRegex = /^(provider\.name|名称|Name)$/;
const claudeBaseUrlRegex =
  /^(providerForm\.apiEndpoint|API 端点|API Endpoint)$/;
const opencodeBaseUrlRegex = /^(opencode\.baseUrl|Base URL)$/;
const commonConfigConfirmRegex =
  /^(confirm\.commonConfig\.confirm|我知道了|Got it)$/;
const fetchModelsRegex =
  /^(providerForm\.fetchModels|获取模型|Fetch Models)$/;
const advancedOptionsRegex =
  /^(providerForm\.advancedOptionsToggle|高级选项|Advanced Options)$/;
const editCommonConfigRegex =
  /^(claudeConfig\.editCommonConfig|编辑通用配置|Edit Common Config)$/;
const extractCommonConfigRegex =
  /^(claudeConfig\.extractFromCurrent|从编辑内容提取|Extract From Current)$/;
const liveConfigRegex =
  /^(provider\.liveConfigPresent|Live Config|已在 Live 配置)$/;
const dbOnlyRegex = /^(provider\.liveConfigMissing|DB Only|仅数据库)$/;

type ModelServer = {
  baseUrl: string;
  requestCount: () => number;
  stop: () => Promise<void>;
};

async function startModelServer(): Promise<ModelServer> {
  let requests = 0;
  const server = http.createServer((req, res) => {
    const requestUrl = new URL(req.url ?? "/", "http://127.0.0.1");

    if (req.method === "GET" && requestUrl.pathname === "/v1/models") {
      requests += 1;
      res.statusCode = 200;
      res.setHeader("Content-Type", "application/json; charset=utf-8");
      res.end(
        JSON.stringify({
          object: "list",
          data: [
            { id: "claude-3-5-sonnet", owned_by: "anthropic" },
            { id: "openai/gpt-4.1-mini", owned_by: "openai" },
          ],
        }),
      );
      return;
    }

    res.statusCode = 404;
    res.setHeader("Content-Type", "text/plain; charset=utf-8");
    res.end("not found");
  });

  await new Promise<void>((resolve, reject) => {
    server.once("error", reject);
    server.listen(0, "127.0.0.1", () => resolve());
  });

  const address = server.address();
  if (!address || typeof address === "string") {
    throw new Error("failed to start model server");
  }

  return {
    baseUrl: `http://127.0.0.1:${(address as AddressInfo).port}`,
    requestCount: () => requests,
    stop: async () => {
      await new Promise<void>((resolve, reject) => {
        server.close((error) => {
          if (error) {
            reject(error);
            return;
          }
          resolve();
        });
      });
    },
  };
}

describe.sequential("ProviderList against real web server", () => {
  let webServer: TestWebServer;
  let modelServer: ModelServer;

  beforeAll(async () => {
    server.close();
    webServer = await startTestWebServer();
    modelServer = await startModelServer();
  }, 360_000);

  afterAll(async () => {
    await webServer.stop();
    await modelServer.stop();
    server.listen({ onUnhandledRequest: "warn" });
  }, 20_000);

  beforeEach(() => {
    toastSuccessMock.mockReset();
    toastErrorMock.mockReset();
    toastInfoMock.mockReset();
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
    Object.defineProperty(window, "scrollTo", {
      configurable: true,
      value: vi.fn(),
    });

    const emptyRectList = {
      length: 0,
      item: () => null,
      [Symbol.iterator]: function* () {
        yield* [];
      },
    };
    Object.defineProperty(Range.prototype, "getClientRects", {
      configurable: true,
      value: () => emptyRectList,
    });
    Object.defineProperty(Range.prototype, "getBoundingClientRect", {
      configurable: true,
      value: () => ({
        x: 0,
        y: 0,
        top: 0,
        right: 0,
        bottom: 0,
        left: 0,
        width: 0,
        height: 0,
        toJSON: () => ({}),
      }),
    });
  });

  it(
    "imports the live Claude config from the rendered empty state, edits the current provider, and switches live writeback to another provider",
    async () => {
      await writeJsonFixture(claudeSettingsPath(webServer.homeDir), {
        env: {
          ANTHROPIC_AUTH_TOKEN: "claude-live-key",
          ANTHROPIC_BASE_URL: "https://claude-live.example.com",
        },
        ui: {
          displayName: "Live Claude",
        },
      });
      const settings = await settingsApi.get();
      await settingsApi.save({ ...settings, commonConfigConfirmed: true });

      const { queryClient } = renderProviderPage("claude");

      const importButton = await screen.findByRole("button", {
        name: importCurrentRegex,
      });
      fireEvent.click(
        importButton,
      );

      expect(await screen.findByText("default")).toBeInTheDocument();

      const importedLiveSettings = await readJsonFixture<{
        env?: { ANTHROPIC_BASE_URL?: string };
      }>(claudeSettingsPath(webServer.homeDir));
      expect(importedLiveSettings.env?.ANTHROPIC_BASE_URL).toBe(
        "https://claude-live.example.com",
      );

      const liveClaudeCard = getProviderCard("default");
      fireEvent.click(within(liveClaudeCard).getByTitle(editRegex));

      const nameInput = await screen.findByLabelText(providerNameRegex);
      const baseUrlInput = await screen.findByLabelText(claudeBaseUrlRegex);

      fireEvent.change(nameInput, { target: { value: "Live Claude Edited" } });
      fireEvent.change(baseUrlInput, {
        target: { value: "https://claude-edited.example.com" },
      });
      fireEvent.click(screen.getByRole("button", { name: saveRegex }));

      expect(await screen.findByText("Live Claude Edited")).toBeInTheDocument();

      await waitFor(async () => {
        const liveSettings = await readJsonFixture<{
          env?: { ANTHROPIC_BASE_URL?: string };
          ui?: { displayName?: string };
        }>(claudeSettingsPath(webServer.homeDir));
        expect(liveSettings.env?.ANTHROPIC_BASE_URL).toBe(
          "https://claude-edited.example.com",
        );
        expect(liveSettings.ui?.displayName).toBe("Live Claude Edited");
      });

      await providersApi.add(
        buildClaudeProvider(
          "claude-alt-page",
          "Claude Alt",
          "claude-alt-key",
          "https://claude-alt.example.com",
          1,
        ),
        "claude",
      );
      await queryClient.invalidateQueries({ queryKey: ["providers", "claude"] });

      const altCard = await waitFor(() => getProviderCard("Claude Alt"));
      fireEvent.click(
        within(altCard).getByRole("button", {
          name: enableRegex,
        }),
      );

      await waitFor(async () => {
        expect(await providersApi.getCurrent("claude")).toBe("claude-alt-page");
        const liveSettings = await readJsonFixture<{
          env?: {
            ANTHROPIC_AUTH_TOKEN?: string;
            ANTHROPIC_BASE_URL?: string;
          };
        }>(claudeSettingsPath(webServer.homeDir));
        expect(liveSettings.env?.ANTHROPIC_BASE_URL).toBe(
          "https://claude-alt.example.com",
        );
        expect(liveSettings.env?.ANTHROPIC_AUTH_TOKEN).toBe("claude-alt-key");
      });
    },
    25_000,
  );

  it(
    "imports the live OpenCode config from the rendered empty state and edits the live-managed provider through the real web API",
    async () => {
      await writeJsonFixture(opencodeSettingsPath(webServer.homeDir), {
        $schema: "https://opencode.ai/config.json",
        provider: {
          "page-opencode": {
            npm: "@ai-sdk/openai-compatible",
            name: "Live OpenCode",
            options: {
              baseURL: "https://opencode-live.example.com/v1",
              apiKey: "opencode-live-key",
            },
            models: {
              "gpt-4o": {
                name: "GPT-4o",
              },
            },
          },
        },
      });
      const settings = await settingsApi.get();
      await settingsApi.save({ ...settings, commonConfigConfirmed: true });

      renderProviderPage("opencode");

      const importButton = await screen.findByRole("button", {
        name: importCurrentRegex,
      });
      fireEvent.click(
        importButton,
      );

      expect(await screen.findByText("Live OpenCode")).toBeInTheDocument();

      const liveOpenCodeCard = getProviderCard("Live OpenCode");
      expect(within(liveOpenCodeCard).getByText(liveConfigRegex)).toBeInTheDocument();
      fireEvent.click(within(liveOpenCodeCard).getByTitle(editRegex));

      const confirmButton = screen.queryByRole("button", {
        name: commonConfigConfirmRegex,
      });
      if (confirmButton) {
        fireEvent.click(confirmButton);
      }

      const nameInput = await screen.findByLabelText(providerNameRegex);
      const baseUrlInput = await screen.findByLabelText(opencodeBaseUrlRegex);

      fireEvent.change(nameInput, {
        target: { value: "Live OpenCode Edited" },
      });
      fireEvent.change(baseUrlInput, {
        target: { value: "https://opencode-edited.example.com/v1" },
      });
      fireEvent.click(screen.getByRole("button", { name: saveRegex }));

      expect(
        await screen.findByText("Live OpenCode Edited"),
      ).toBeInTheDocument();

      await waitFor(async () => {
        const liveConfig = await readJsonFixture<{
          provider?: Record<
            string,
            {
              name?: string;
              options?: { baseURL?: string };
            }
          >;
        }>(opencodeSettingsPath(webServer.homeDir));
        expect(liveConfig.provider?.["page-opencode"]?.name).toBe(
          "Live OpenCode Edited",
        );
        expect(
          liveConfig.provider?.["page-opencode"]?.options?.baseURL,
        ).toBe("https://opencode-edited.example.com/v1");
      });
    },
    20_000,
  );

  it(
    "imports the live OpenClaw config from the rendered empty state and marks the provider as live-managed in the rendered card",
    async () => {
      await writeJsonFixture(openclawSettingsPath(webServer.homeDir), {
        models: {
          mode: "merge",
          providers: {
            "page-openclaw-live": {
              baseUrl: "https://openclaw-live.example.com/v1",
              apiKey: "openclaw-live-key",
              api: "openai-completions",
              models: [
                {
                  id: "claude-sonnet-4",
                  name: "OpenClaw Live Model",
                },
              ],
            },
          },
        },
        agents: {
          defaults: {
            model: "page-openclaw-live/claude-sonnet-4",
          },
        },
      });

      renderProviderPage("openclaw");

      fireEvent.click(
        await screen.findByRole("button", {
          name: importCurrentRegex,
        }),
      );

      const liveCard = await waitFor(() => getProviderCard("OpenClaw Live Model"));
      expect(within(liveCard).getByText(liveConfigRegex)).toBeInTheDocument();

      await waitFor(async () => {
        const providers = await providersApi.getAll("openclaw");
        expect(providers["page-openclaw-live"]).toBeDefined();
        expect(providers["page-openclaw-live"]?.name).toBe("OpenClaw Live Model");
      });
    },
    20_000,
  );

  it(
    "imports the live Hermes config from the rendered empty state and highlights the current live provider in the rendered card",
    async () => {
      await writeTextFixture(
        hermesSettingsPath(webServer.homeDir),
        [
          "model:",
          '  default: "anthropic/claude-sonnet-4"',
          '  provider: "page-hermes-live"',
          "custom_providers:",
          '  - name: "page-hermes-live"',
          '    base_url: "https://hermes-live.example.com/v1"',
          '    api_key: "hermes-live-key"',
          '    model: "anthropic/claude-sonnet-4"',
          "    models:",
          '      anthropic/claude-sonnet-4: {}',
          "",
        ].join("\n"),
      );

      renderProviderPage("hermes");

      fireEvent.click(
        await screen.findByRole("button", {
          name: importCurrentRegex,
        }),
      );

      const liveCard = await waitFor(() => getProviderCard("page-hermes-live"));
      expect(within(liveCard).getByText(liveConfigRegex)).toBeInTheDocument();
      expect(liveCard.className).toContain("border-blue-500/60");

      await waitFor(async () => {
        const providers = await providersApi.getAll("hermes");
        expect(providers["page-hermes-live"]).toBeDefined();
        expect(providers["page-hermes-live"]?.name).toBe("page-hermes-live");
      });
    },
    20_000,
  );

  it(
    "shows db-only status for additive providers that are not present in the live config",
    async () => {
      await providersApi.add(
        {
          id: "opencode-db-only-page",
          name: "OpenCode DB Only",
          category: "custom",
          sortIndex: 200,
          settingsConfig: {
            name: "OpenCode DB Only",
            options: {
              baseURL: "https://db-only.example.com/v1",
              apiKey: "db-only-key",
            },
          },
        },
        "opencode",
        false,
      );

      renderProviderPage("opencode");

      const dbOnlyCard = await waitFor(() =>
        getProviderCard("OpenCode DB Only"),
      );
      expect(within(dbOnlyCard).getByText(dbOnlyRegex)).toBeInTheDocument();
    },
    20_000,
  );

  it(
    "fetches models and extracts the Claude common config snippet from the rendered edit dialog through the real web API",
    async () => {
      await configApi.setCommonConfigSnippet("claude", "");
      const settings = await settingsApi.get();
      await settingsApi.save({ ...settings, commonConfigConfirmed: true });

      await providersApi.add(
        {
          id: "claude-diagnostics-page",
          name: "Claude Diagnostics",
          category: "custom",
          sortIndex: 99,
          settingsConfig: {
            env: {
              ANTHROPIC_AUTH_TOKEN: "claude-model-fetch-key",
              ANTHROPIC_BASE_URL: modelServer.baseUrl,
            },
            includeCoAuthoredBy: false,
          },
        },
        "claude",
      );

      renderProviderPage("claude");

      const diagnosticsCard = await waitFor(() =>
        getProviderCard("Claude Diagnostics"),
      );
      fireEvent.click(within(diagnosticsCard).getByTitle(editRegex));
      fireEvent.click(
        await screen.findByRole("button", {
          name: advancedOptionsRegex,
        }),
      );

      toastSuccessMock.mockReset();
      toastErrorMock.mockReset();

      const fetchModelsButton = await screen.findByRole("button", {
        name: fetchModelsRegex,
      });
      fireEvent.click(fetchModelsButton);

      await waitFor(() => {
        expect(modelServer.requestCount()).toBeGreaterThan(0);
        expect(toastSuccessMock).toHaveBeenCalled();
      });
      expect(toastErrorMock).not.toHaveBeenCalled();

      fireEvent.click(
        screen.getByRole("button", {
          name: editCommonConfigRegex,
        }),
      );
      fireEvent.click(
        await screen.findByRole("button", {
          name: extractCommonConfigRegex,
        }),
      );

      await waitFor(async () => {
        const snippet = await configApi.getCommonConfigSnippet("claude");
        expect(snippet).toBeTruthy();

        const parsed = JSON.parse(snippet ?? "{}") as Record<string, unknown>;
        expect(parsed.includeCoAuthoredBy).toBe(false);
        expect(parsed.env).toBeUndefined();
        expect(parsed.apiBaseUrl).toBeUndefined();
      });
    },
    20_000,
  );

  it(
    "renders provider usage limit diagnostics on the card through the real web API",
    async () => {
      await providersApi.add(
        {
          id: "claude-limits-page",
          name: "Claude Limits",
          category: "custom",
          sortIndex: 100,
          meta: {
            limitDailyUsd: "1.25",
            limitMonthlyUsd: "9.5",
          },
          settingsConfig: {
            env: {
              ANTHROPIC_AUTH_TOKEN: "claude-limits-key",
            },
          },
        },
        "claude",
      );

      renderProviderPage("claude");

      const limitsCard = await waitFor(() => getProviderCard("Claude Limits"));

      await waitFor(() => {
        expect(
          within(limitsCard).getByText(/日限额 \$0\.0000 \/ \$1\.25/),
        ).toBeInTheDocument();
        expect(
          within(limitsCard).getByText(/月限额 \$0\.0000 \/ \$9\.50/),
        ).toBeInTheDocument();
      });
    },
    20_000,
  );

  it(
    "renders provider usage stats on the card through the real web API",
    async () => {
      const createdAt = Math.floor(Date.now() / 1000);

      await providersApi.add(
        {
          id: "claude-usage-page",
          name: "Claude Usage",
          category: "custom",
          sortIndex: 101,
          settingsConfig: {
            env: {
              ANTHROPIC_AUTH_TOKEN: "claude-usage-key",
            },
          },
        },
        "claude",
      );

      await insertProxyUsageLog(webServer.dataDir, {
        requestId: "usage-page-1",
        providerId: "claude-usage-page",
        appType: "claude",
        totalCostUsd: "0.01",
        latencyMs: 120,
        createdAt,
      });
      await insertProxyUsageLog(webServer.dataDir, {
        requestId: "usage-page-2",
        providerId: "claude-usage-page",
        appType: "claude",
        totalCostUsd: "0.02",
        latencyMs: 180,
        createdAt,
      });

      renderProviderPage("claude");

      const usageCard = await waitFor(() => getProviderCard("Claude Usage"));
      await waitFor(() => {
        expect(
          within(usageCard).getByText(/30天 2 次 \/ \$0\.0300/),
        ).toBeInTheDocument();
      });
    },
    20_000,
  );

  it(
    "splits provider diagnostics errors into separate inline badges through the real web API",
    async () => {
      await providersApi.add(
        {
          id: "claude-error-diagnostics-page",
          name: "Claude Error Diagnostics",
          category: "custom",
          sortIndex: 102,
          meta: {
            limitDailyUsd: "1.25",
          },
          settingsConfig: {
            env: {
              ANTHROPIC_AUTH_TOKEN: "claude-error-key",
            },
          },
        },
        "claude",
      );

      await failoverApi.addToFailoverQueue(
        "claude",
        "claude-error-diagnostics-page",
      );
      await failoverApi.setAutoFailoverEnabled("claude", true);

      const statsFailure = mockApiFailure(
        "/api/providers/get-provider-stats",
        "provider stats failed",
        webServer.baseUrl,
      );
      const limitsFailure = mockApiFailure(
        "/api/providers/check-provider-limits",
        "provider limits failed",
        webServer.baseUrl,
      );
      const healthFailure = mockApiFailure(
        "/api/providers/get-provider-health",
        "provider health failed",
        webServer.baseUrl,
      );

      try {
        renderProviderPage("claude", {
          isProxyRunning: true,
          isProxyTakeover: true,
        });

        const errorCard = await waitFor(() =>
          getProviderCard("Claude Error Diagnostics"),
        );

        await waitFor(() => {
          expect(
            within(errorCard).getByText("健康状态不可用"),
          ).toBeInTheDocument();
          expect(
            within(errorCard).getByText("限额状态不可用"),
          ).toBeInTheDocument();
          expect(
            within(errorCard).getByText("用量摘要不可用"),
          ).toBeInTheDocument();
        });
      } finally {
        healthFailure.restore();
        limitsFailure.restore();
        statsFailure.restore();
      }
    },
    20_000,
  );
});
