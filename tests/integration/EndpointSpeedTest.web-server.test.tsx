import http from "node:http";
import net from "node:net";
import type { AddressInfo } from "node:net";
import { type ComponentProps, type ReactNode, useState } from "react";
import {
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import { afterAll, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";

import "@/lib/api/web-commands";
import EndpointSpeedTest from "@/components/providers/forms/EndpointSpeedTest";
import { setCsrfToken } from "@/lib/api/adapter";
import { providersApi } from "@/lib/api/providers";
import { vscodeApi } from "@/lib/api/vscode";
import type { Provider } from "@/types";
import { server } from "../msw/server";
import {
  startTestWebServer,
  type TestWebServer,
} from "../helpers/web-server";

vi.mock("@/components/common/FullScreenPanel", () => ({
  FullScreenPanel: ({
    isOpen,
    title,
    children,
    footer,
  }: {
    isOpen: boolean;
    title: string;
    children: ReactNode;
    footer?: ReactNode;
  }) =>
    isOpen ? (
      <div data-testid="endpoint-speed-test-panel">
        <h1>{title}</h1>
        <div>{children}</div>
        <div>{footer}</div>
      </div>
    ) : null,
}));

type LatencyServer = {
  fastUrl: string;
  slowUrl: string;
  extraUrl: string;
  stop: () => Promise<void>;
};

function buildClaudeProvider(
  id: string,
  baseUrl: string,
  savedCustomUrl: string,
): Provider {
  return {
    id,
    name: "Endpoint Speed Smoke",
    category: "custom",
    sortIndex: 0,
    settingsConfig: {
      env: {
        ANTHROPIC_AUTH_TOKEN: "sk-endpoint-speed",
        ANTHROPIC_BASE_URL: baseUrl,
      },
      ui: {
        displayName: "Endpoint Speed Smoke",
      },
    },
    meta: {
      custom_endpoints: {
        [savedCustomUrl]: {
          url: savedCustomUrl,
          addedAt: 1,
        },
      },
    },
  };
}

async function startLatencyServer(): Promise<LatencyServer> {
  const latencyServer = http.createServer((req, res) => {
    const requestUrl = new URL(req.url ?? "/", "http://127.0.0.1");

    const respond = (delayMs: number) => {
      setTimeout(() => {
        res.statusCode = 200;
        res.setHeader("Content-Type", "text/plain; charset=utf-8");
        res.end("ok");
      }, delayMs);
    };

    if (req.method === "GET" && requestUrl.pathname === "/fast") {
      respond(15);
      return;
    }

    if (req.method === "GET" && requestUrl.pathname === "/slow") {
      respond(120);
      return;
    }

    if (req.method === "GET" && requestUrl.pathname === "/extra") {
      respond(35);
      return;
    }

    res.statusCode = 404;
    res.setHeader("Content-Type", "text/plain; charset=utf-8");
    res.end("not found");
  });

  await new Promise<void>((resolve, reject) => {
    latencyServer.once("error", reject);
    latencyServer.listen(0, "127.0.0.1", () => resolve());
  });

  const address = latencyServer.address();
  if (!address || typeof address === "string") {
    throw new Error("failed to start latency server");
  }

  const baseUrl = `http://127.0.0.1:${(address as AddressInfo).port}`;

  return {
    fastUrl: `${baseUrl}/fast`,
    slowUrl: `${baseUrl}/slow`,
    extraUrl: `${baseUrl}/extra`,
    stop: async () => {
      await new Promise<void>((resolve, reject) => {
        latencyServer.close((error) => {
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

async function getClosedPortUrl(): Promise<string> {
  return await new Promise((resolve, reject) => {
    const socketServer = net.createServer();
    socketServer.unref();
    socketServer.once("error", reject);
    socketServer.listen(0, "127.0.0.1", () => {
      const address = socketServer.address();
      if (!address || typeof address === "string") {
        reject(new Error("failed to allocate closed port"));
        return;
      }

      const { port } = address;
      socketServer.close((error) => {
        if (error) {
          reject(error);
          return;
        }
        resolve(`http://127.0.0.1:${port}/offline`);
      });
    });
  });
}

function renderPanel(
  overrides: Partial<ComponentProps<typeof EndpointSpeedTest>> = {},
) {
  const onChangeSpy = vi.fn();
  const onCloseSpy = vi.fn();
  const onCustomEndpointsChangeSpy = vi.fn();

  function Harness() {
    const [value, setValue] = useState(overrides.value ?? "");
    const [autoSelect, setAutoSelect] = useState(overrides.autoSelect ?? false);

    return (
      <EndpointSpeedTest
        appId="claude"
        value={value}
        onChange={(nextValue) => {
          onChangeSpy(nextValue);
          setValue(nextValue);
        }}
        initialEndpoints={[]}
        visible
        onClose={onCloseSpy}
        autoSelect={autoSelect}
        onAutoSelectChange={setAutoSelect}
        onCustomEndpointsChange={onCustomEndpointsChangeSpy}
        {...overrides}
      />
    );
  }

  render(<Harness />);

  return { onChangeSpy, onCloseSpy, onCustomEndpointsChangeSpy };
}

function getAddButton() {
  const input = screen.getByPlaceholderText("endpointTest.addEndpointPlaceholder");
  return within(input.parentElement as HTMLElement).getByRole("button");
}

function getRemoveButtonForUrl(url: string) {
  const row = screen.getByText(url).closest(".group");
  expect(row).not.toBeNull();
  return within(row as HTMLElement).getByRole("button");
}

describe.sequential("EndpointSpeedTest against real web server", () => {
  let webServer: TestWebServer;
  let latencyServer: LatencyServer;
  let closedPortUrl: string;

  beforeAll(async () => {
    server.close();
    webServer = await startTestWebServer();
    latencyServer = await startLatencyServer();
    closedPortUrl = await getClosedPortUrl();
  }, 360_000);

  afterAll(async () => {
    await webServer.stop();
    await latencyServer.stop();
    server.listen({ onUnhandledRequest: "warn" });
  }, 20_000);

  beforeEach(() => {
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
    "runs real endpoint speed tests, auto-selects the fastest healthy URL, and surfaces server-side failures",
    async () => {
      const { onChangeSpy } = renderPanel({
        value: latencyServer.slowUrl,
        initialEndpoints: [
          { url: latencyServer.slowUrl },
          { url: latencyServer.fastUrl },
          { url: closedPortUrl },
        ],
        autoSelect: true,
      });

      fireEvent.click(
        screen.getByRole("button", { name: "endpointTest.testSpeed" }),
      );

      await waitFor(
        () => {
          expect(onChangeSpy).toHaveBeenCalledWith(latencyServer.fastUrl);
        },
        { timeout: 10_000 },
      );

      await waitFor(
        () => {
          expect(screen.getAllByText(/\d+ms/).length).toBeGreaterThanOrEqual(2);
        },
        { timeout: 10_000 },
      );

      expect(screen.getByText("连接失败")).toBeInTheDocument();
    },
    20_000,
  );

  it(
    "loads saved custom endpoints in edit mode and persists add/remove diffs through the real web API",
    async () => {
      const providerId = "endpoint-speed-real-web";

      await providersApi.add(
        buildClaudeProvider(
          providerId,
          latencyServer.fastUrl,
          latencyServer.slowUrl,
        ),
        "claude",
      );

      const { onCloseSpy } = renderPanel({
        providerId,
        value: latencyServer.fastUrl,
        initialEndpoints: [{ url: latencyServer.fastUrl }],
      });

      expect(
        await screen.findByText(latencyServer.slowUrl),
      ).toBeInTheDocument();

      fireEvent.change(
        screen.getByPlaceholderText("endpointTest.addEndpointPlaceholder"),
        {
          target: { value: `${latencyServer.extraUrl}/` },
        },
      );
      fireEvent.click(getAddButton());
      fireEvent.click(getRemoveButtonForUrl(latencyServer.slowUrl));
      fireEvent.click(screen.getByRole("button", { name: "common.save" }));

      await waitFor(() => {
        expect(onCloseSpy).toHaveBeenCalledTimes(1);
      });

      const savedEndpoints = await vscodeApi.getCustomEndpoints(
        "claude",
        providerId,
      );

      expect(savedEndpoints.map((endpoint) => endpoint.url)).toEqual([
        latencyServer.extraUrl,
      ]);
      expect(savedEndpoints).not.toEqual(
        expect.arrayContaining([
          expect.objectContaining({ url: latencyServer.fastUrl }),
          expect.objectContaining({ url: latencyServer.slowUrl }),
        ]),
      );
    },
    20_000,
  );
});
