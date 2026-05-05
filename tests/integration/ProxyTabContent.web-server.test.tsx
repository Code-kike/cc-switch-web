import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterAll, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";

import "@/lib/api/web-commands";
import { ProxyTabContent } from "@/components/settings/ProxyTabContent";
import { setCsrfToken } from "@/lib/api/adapter";
import { providersApi } from "@/lib/api/providers";
import { server } from "../msw/server";
import {
  startTestWebServer,
  type TestWebServer,
} from "../helpers/web-server";
import type { AppProxyConfig, FailoverQueueItem } from "@/types/proxy";
import type { Provider } from "@/types";
import type { SettingsFormState } from "@/hooks/useSettings";

const toastSuccessMock = vi.fn();
const toastErrorMock = vi.fn();

vi.mock("sonner", () => ({
  toast: {
    success: (...args: unknown[]) => toastSuccessMock(...args),
    error: (...args: unknown[]) => toastErrorMock(...args),
  },
}));

const baseSettings = {
  language: "zh",
  enableLocalProxy: false,
  proxyConfirmed: true,
  failoverConfirmed: true,
  enableFailoverToggle: false,
} as SettingsFormState;

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

const renderPanel = () => {
  const client = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  });

  render(
    <QueryClientProvider client={client}>
      <ProxyTabContent settings={baseSettings} onAutoSave={vi.fn()} />
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

const getFailoverQueue = async (
  baseUrl: string,
  appType: string,
): Promise<FailoverQueueItem[]> =>
  await requestJson(
    new URL(`/api/failover/get-failover-queue?appType=${appType}`, baseUrl),
  );

const getAutoFailoverEnabled = async (
  baseUrl: string,
  appType: string,
): Promise<boolean> =>
  await requestJson(
    new URL(
      `/api/failover/get-auto-failover-enabled?appType=${appType}`,
      baseUrl,
    ),
  );

const getAppProxyConfig = async (
  baseUrl: string,
  appType: string,
): Promise<AppProxyConfig> =>
  await requestJson(
    new URL(`/api/config/get-proxy-config-for-app?appType=${appType}`, baseUrl),
  );

const getNearestSwitch = (labelText: string | RegExp): HTMLElement => {
  const label = screen.getByText(labelText);
  let current: HTMLElement | null = label instanceof HTMLElement ? label : null;

  while (current) {
    const toggle = within(current).queryByRole("switch");
    if (toggle) {
      return toggle;
    }
    current = current.parentElement;
  }

  throw new Error(`could not find switch for label ${labelText}`);
};

const getProviderRow = (providerName: string): HTMLElement => {
  const label = screen.getByText(providerName);
  let current: HTMLElement | null = label instanceof HTMLElement ? label : null;

  while (current && !current.className.includes("rounded-lg border bg-card")) {
    current = current.parentElement;
  }

  if (!(current instanceof HTMLElement)) {
    throw new Error(`could not locate provider row for ${providerName}`);
  }

  return current;
};

const getSelectTrigger = (): HTMLElement => {
  const trigger = screen
    .getAllByRole("combobox")
    .find(
      (element) =>
        element instanceof HTMLButtonElement &&
        element.getAttribute("aria-expanded") !== null,
    );

  if (!(trigger instanceof HTMLElement)) {
    throw new Error("could not locate failover provider select trigger");
  }

  return trigger;
};

const getQueueActionButton = (name: "add" | "delete"): HTMLElement => {
  const accessibleName =
    name === "add" ? /^(common\.add|添加)$/ : /^(common\.delete|删除)$/;

  return screen.getByRole("button", { name: accessibleName });
};

const getConfigSaveButton = (): HTMLElement => {
  const saveButtons = screen.getAllByRole("button", {
    name: /^(common\.save|保存)$/,
  });

  const button = saveButtons.at(-1);
  if (!(button instanceof HTMLElement)) {
    throw new Error("could not locate failover config save button");
  }

  return button;
};

const expectSuccessToast = async (message: RegExp): Promise<void> => {
  await waitFor(() =>
    expect(toastSuccessMock).toHaveBeenCalledWith(
      expect.stringMatching(message),
      expect.objectContaining({ closeButton: true }),
    ),
  );
};

describe.sequential("ProxyTabContent against real web server", () => {
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
    Object.defineProperty(Element.prototype, "scrollIntoView", {
      configurable: true,
      value: vi.fn(),
    });
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
    "manages failover queue, auto switch, and app config through the rendered page",
    async () => {
      const primaryProvider = buildClaudeProvider(
        "proxy-page-primary",
        "Proxy Page Primary",
        "proxy-page-primary-token",
        "https://primary.example.com",
        1,
      );
      const backupProvider = buildClaudeProvider(
        "proxy-page-backup",
        "Proxy Page Backup",
        "proxy-page-backup-token",
        "https://backup.example.com",
        2,
      );

      await providersApi.add(primaryProvider, "claude", false);
      await providersApi.add(backupProvider, "claude", false);

      renderPanel();

      fireEvent.click(screen.getByText("settings.advanced.proxy.title"));
      expect(
        await screen.findByText("Web mode does not expose proxy runtime control"),
      ).toBeInTheDocument();
      expect(
        screen.getByText(
          "You can still edit proxy and failover settings here, but starting the local proxy runtime and app takeover stays desktop-only for now.",
        ),
      ).toBeInTheDocument();

      fireEvent.click(screen.getByText("settings.advanced.failover.title"));

      expect(
        await screen.findByText(
          "proxy.failover.runtimeStatsUnavailableTitle",
        ),
      ).toBeInTheDocument();
      expect(
        screen.getByText("proxy.failover.runtimeStatsUnavailableDescription"),
      ).toBeInTheDocument();
      expect(
        screen.getByText(
          "Web mode keeps failover in configuration-only mode",
        ),
      ).toBeInTheDocument();
      expect(
        screen.getByText(
          "Queue order and thresholds can still be edited remotely. Runtime counters and local proxy execution remain unavailable in web-server mode.",
        ),
      ).toBeInTheDocument();

      expect(
        await screen.findByText(
          "队列顺序与首页供应商列表顺序一致。当请求失败时，系统会按顺序依次尝试队列中的供应商。",
        ),
      ).toBeInTheDocument();

      const providerSelect = getSelectTrigger();
      providerSelect.focus();
      fireEvent.keyDown(providerSelect, { key: "ArrowDown" });
      await waitFor(() =>
        expect(providerSelect).toHaveAttribute("aria-expanded", "true"),
      );
      const backupOption = await screen.findByRole("option", {
        name: "Proxy Page Backup",
      });
      fireEvent.click(backupOption);
      fireEvent.click(getQueueActionButton("add"));

      await expectSuccessToast(
        /^(proxy\.failoverQueue\.addSuccess|已添加到故障转移队列)$/,
      );
      expect(await screen.findByText("Proxy Page Backup")).toBeInTheDocument();
      await waitFor(async () => {
        const queue = await getFailoverQueue(webServer.baseUrl, "claude");
        expect(queue.map((item) => item.providerId)).toContain(backupProvider.id);
      });

      fireEvent.click(
        getNearestSwitch(/^(proxy\.failover\.autoSwitch|自动故障转移)$/),
      );

      await expectSuccessToast(
        /^(failover\.enabled|Claude 故障转移已启用)$/,
      );
      await waitFor(async () => {
        expect(await getAutoFailoverEnabled(webServer.baseUrl, "claude")).toBe(
          true,
        );
      });

      fireEvent.change(
        screen.getByLabelText(/^(proxy\.autoFailover\.maxRetries|最大重试次数)$/),
        {
          target: { value: "4" },
        },
      );
      fireEvent.click(getConfigSaveButton());

      await expectSuccessToast(
        /^(proxy\.autoFailover\.configSaved|自动故障转移配置已保存)$/,
      );
      await waitFor(async () => {
        expect((await getAppProxyConfig(webServer.baseUrl, "claude")).maxRetries).toBe(4);
      });

      fireEvent.click(
        getNearestSwitch(/^(proxy\.failover\.autoSwitch|自动故障转移)$/),
      );

      await expectSuccessToast(
        /^(failover\.disabled|Claude 故障转移已关闭)$/,
      );
      await waitFor(async () => {
        expect(await getAutoFailoverEnabled(webServer.baseUrl, "claude")).toBe(
          false,
        );
      });

      fireEvent.click(
        within(getProviderRow("Proxy Page Backup")).getByRole("button", {
          name: /^(common\.delete|删除)$/,
        }),
      );

      expect(
        await screen.findByText(
          /^(proxy\.failoverQueue\.empty|故障转移队列为空。添加供应商以启用自动故障转移。)$/,
        ),
      ).toBeInTheDocument();
      await waitFor(async () => {
        expect(await getFailoverQueue(webServer.baseUrl, "claude")).toHaveLength(0);
      });
    },
    180_000,
  );
});
