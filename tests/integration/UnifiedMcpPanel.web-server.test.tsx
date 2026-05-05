import fs from "node:fs/promises";
import path from "node:path";
import { createRef } from "react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import {
  act,
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import { afterAll, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";

import "@/lib/api/web-commands";
import UnifiedMcpPanel, {
  type UnifiedMcpPanelHandle,
} from "@/components/mcp/UnifiedMcpPanel";
import { MCP_APP_IDS } from "@/config/appConfig";
import { setCsrfToken } from "@/lib/api/adapter";
import type { McpServer } from "@/types";
import { server } from "../msw/server";
import {
  startTestWebServer,
  type TestWebServer,
} from "../helpers/web-server";

const toastSuccessMock = vi.fn();
const toastErrorMock = vi.fn();

const smokeMcpIds = {
  shared: "page-smoke-shared",
  claudeOnly: "page-smoke-claude-only",
};

vi.mock("sonner", () => ({
  toast: {
    success: (...args: unknown[]) => toastSuccessMock(...args),
    error: (...args: unknown[]) => toastErrorMock(...args),
  },
}));

vi.mock("@/components/JsonEditor", () => ({
  default: ({
    value,
    onChange,
    placeholder,
  }: {
    value: string;
    onChange?: (value: string) => void;
    placeholder?: string;
  }) => (
    <textarea
      aria-label="json-editor"
      placeholder={placeholder}
      value={value}
      onChange={(event) => onChange?.(event.target.value)}
    />
  ),
}));

async function writeFixtureFile(
  filePath: string,
  content: string,
): Promise<void> {
  await fs.mkdir(path.dirname(filePath), { recursive: true });
  await fs.writeFile(filePath, content, "utf8");
}

async function writeJsonFixture(
  filePath: string,
  value: unknown,
): Promise<void> {
  await writeFixtureFile(filePath, `${JSON.stringify(value, null, 2)}\n`);
}

async function seedMcpLiveFixtures(homeDir: string): Promise<void> {
  await writeJsonFixture(path.join(homeDir, ".claude.json"), {
    mcpServers: {
      [smokeMcpIds.shared]: {
        type: "stdio",
        command: "sh",
        args: ["-lc", "echo shared"],
      },
      [smokeMcpIds.claudeOnly]: {
        type: "stdio",
        command: "echo",
        args: ["claude-only"],
      },
    },
  });

  await writeFixtureFile(
    path.join(homeDir, ".codex", "config.toml"),
    [
      `[mcp_servers.${smokeMcpIds.shared}]`,
      'type = "stdio"',
      'command = "sh"',
      'args = ["-lc", "echo shared"]',
      "",
    ].join("\n"),
  );
}

const renderPanel = () => {
  const client = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  });
  const ref = createRef<UnifiedMcpPanelHandle>();

  render(
    <QueryClientProvider client={client}>
      <UnifiedMcpPanel ref={ref} onOpenChange={vi.fn()} />
    </QueryClientProvider>,
  );

  return { ref };
};

const getMcpServers = async (
  baseUrl: string,
): Promise<Record<string, McpServer>> => {
  const response = await fetch(new URL("/api/mcp/get-mcp-servers", baseUrl));
  if (!response.ok) {
    throw new Error(`failed to load MCP servers: ${response.status}`);
  }
  return (await response.json()) as Record<string, McpServer>;
};

const getRowForId = (serverId: string): HTMLElement => {
  const label = screen.getByText(serverId);
  let current: HTMLElement | null = label;

  while (current && !current.classList.contains("group")) {
    current = current.parentElement;
  }

  if (!(current instanceof HTMLElement)) {
    throw new Error(`could not locate MCP row for ${serverId}`);
  }

  return current;
};

const getJsonEditor = (): HTMLTextAreaElement => {
  const editor = screen.getByLabelText("json-editor");
  if (!(editor instanceof HTMLTextAreaElement)) {
    throw new Error("could not locate MCP json editor");
  }
  return editor;
};

const getAppToggleButton = (
  row: HTMLElement,
  appId: (typeof MCP_APP_IDS)[number],
): HTMLButtonElement => {
  const buttons = Array.from(row.querySelectorAll("button")).filter(
    (button) => !button.getAttribute("title"),
  );
  const index = MCP_APP_IDS.indexOf(appId);
  const button = buttons[index];

  if (!(button instanceof HTMLButtonElement)) {
    throw new Error(`could not locate MCP toggle button for ${appId}`);
  }

  return button;
};

describe.sequential("UnifiedMcpPanel against real web server", () => {
  let webServer: TestWebServer;

  beforeAll(async () => {
    server.close();
    webServer = await startTestWebServer();
    await seedMcpLiveFixtures(webServer.homeDir);
  }, 360_000);

  afterAll(async () => {
    await webServer.stop();
    server.listen({ onUnhandledRequest: "warn" });
  }, 20_000);

  beforeEach(() => {
    toastSuccessMock.mockReset();
    toastErrorMock.mockReset();
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
    "imports, toggles, and deletes MCP servers through the rendered panel UI",
    async () => {
      const { ref } = renderPanel();

      await waitFor(async () => {
        expect(await getMcpServers(webServer.baseUrl)).toEqual({});
      });
      expect(screen.getByText("mcp.unifiedPanel.noServers")).toBeInTheDocument();

      await act(async () => {
        await ref.current?.openImport();
      });

      await waitFor(() =>
        expect(toastSuccessMock).toHaveBeenCalledWith(
          "mcp.unifiedPanel.importSuccess",
          { closeButton: true },
        ),
      );
      await waitFor(async () => {
        const servers = await getMcpServers(webServer.baseUrl);
        expect(Object.keys(servers).sort()).toEqual(
          [smokeMcpIds.claudeOnly, smokeMcpIds.shared].sort(),
        );
        expect(servers[smokeMcpIds.shared]?.apps?.claude).toBe(true);
        expect(servers[smokeMcpIds.shared]?.apps?.codex).toBe(true);
        expect(servers[smokeMcpIds.claudeOnly]?.apps?.claude).toBe(true);
      });

      expect(screen.getByText(smokeMcpIds.shared)).toBeInTheDocument();
      expect(screen.getByText(smokeMcpIds.claudeOnly)).toBeInTheDocument();

      const sharedRow = getRowForId(smokeMcpIds.shared);
      fireEvent.click(getAppToggleButton(sharedRow, "codex"));

      await waitFor(async () => {
        const servers = await getMcpServers(webServer.baseUrl);
        expect(servers[smokeMcpIds.shared]?.apps?.codex).toBe(false);
      });
      await waitFor(async () => {
        const codexConfig = await fs.readFile(
          path.join(webServer.homeDir, ".codex", "config.toml"),
          "utf8",
        );
        expect(codexConfig).not.toContain(
          `[mcp_servers.${smokeMcpIds.shared}]`,
        );
      });

      const claudeOnlyRow = getRowForId(smokeMcpIds.claudeOnly);
      fireEvent.click(within(claudeOnlyRow).getByTitle("common.delete"));

      await screen.findByText("mcp.unifiedPanel.deleteServer");
      fireEvent.click(screen.getByRole("button", { name: "common.confirm" }));

      await waitFor(() =>
        expect(toastSuccessMock).toHaveBeenCalledWith("common.success", {
          closeButton: true,
        }),
      );
      await waitFor(async () => {
        const servers = await getMcpServers(webServer.baseUrl);
        expect(servers[smokeMcpIds.claudeOnly]).toBeUndefined();
        expect(servers[smokeMcpIds.shared]).toBeDefined();
      });
      await waitFor(async () => {
        const claudeConfig = JSON.parse(
          await fs.readFile(path.join(webServer.homeDir, ".claude.json"), "utf8"),
        ) as {
          mcpServers?: Record<string, unknown>;
        };
        expect(claudeConfig.mcpServers?.[smokeMcpIds.claudeOnly]).toBeUndefined();
      });

      expect(
        screen.queryByText(smokeMcpIds.claudeOnly),
      ).not.toBeInTheDocument();
      expect(toastErrorMock).not.toHaveBeenCalled();
    },
    20_000,
  );

  it(
    "adds and edits an MCP server through the rendered panel UI",
    async () => {
      const { ref } = renderPanel();

      await act(async () => {
        ref.current?.openAdd();
      });

      fireEvent.change(
        await screen.findByPlaceholderText("mcp.form.titlePlaceholder"),
        {
          target: { value: "page-added-server" },
        },
      );
      fireEvent.change(screen.getByPlaceholderText("mcp.form.namePlaceholder"), {
        target: { value: "Page Added Server" },
      });
      fireEvent.change(getJsonEditor(), {
        target: {
          value: '{"type":"stdio","command":"echo","args":["added-v1"]}',
        },
      });
      fireEvent.click(screen.getByRole("button", { name: "common.add" }));

      await waitFor(() =>
        expect(toastSuccessMock).toHaveBeenCalledWith("common.success", {
          closeButton: true,
        }),
      );
      await waitFor(async () => {
        const servers = await getMcpServers(webServer.baseUrl);
        expect(servers["page-added-server"]?.name).toBe("Page Added Server");
        expect(servers["page-added-server"]?.server.command).toBe("echo");
      });
      await waitFor(async () => {
        const claudeConfig = JSON.parse(
          await fs.readFile(path.join(webServer.homeDir, ".claude.json"), "utf8"),
        ) as {
          mcpServers?: Record<string, { command?: string }>;
        };
        expect(
          claudeConfig.mcpServers?.["page-added-server"]?.command,
        ).toBe("echo");

        const codexConfig = await fs.readFile(
          path.join(webServer.homeDir, ".codex", "config.toml"),
          "utf8",
        );
        expect(codexConfig).toContain("[mcp_servers.page-added-server]");
        expect(codexConfig).toContain('command = "echo"');
      });

      const addedRow = getRowForId("Page Added Server");
      fireEvent.click(within(addedRow).getByTitle("common.edit"));

      const nameInput = await screen.findByPlaceholderText(
        "mcp.form.namePlaceholder",
      );
      expect(nameInput).toHaveValue("Page Added Server");
      expect(getJsonEditor().value).toContain('"command": "echo"');

      fireEvent.change(nameInput, {
        target: { value: "Page Added Server Edited" },
      });
      fireEvent.change(getJsonEditor(), {
        target: {
          value: '{"type":"stdio","command":"printf","args":["added-v2"]}',
        },
      });
      fireEvent.click(screen.getByRole("button", { name: "common.save" }));

      await waitFor(async () => {
        const servers = await getMcpServers(webServer.baseUrl);
        expect(servers["page-added-server"]?.name).toBe(
          "Page Added Server Edited",
        );
        expect(servers["page-added-server"]?.server.command).toBe("printf");
      });
      await waitFor(async () => {
        const claudeConfig = JSON.parse(
          await fs.readFile(path.join(webServer.homeDir, ".claude.json"), "utf8"),
        ) as {
          mcpServers?: Record<string, { command?: string; args?: string[] }>;
        };
        expect(
          claudeConfig.mcpServers?.["page-added-server"]?.command,
        ).toBe("printf");
        expect(
          claudeConfig.mcpServers?.["page-added-server"]?.args,
        ).toContain("added-v2");

        const codexConfig = await fs.readFile(
          path.join(webServer.homeDir, ".codex", "config.toml"),
          "utf8",
        );
        expect(codexConfig).toContain('command = "printf"');
        expect(codexConfig).toContain('"added-v2"');
      });

      expect(screen.getByText("Page Added Server Edited")).toBeInTheDocument();
      expect(toastErrorMock).not.toHaveBeenCalled();
    },
    20_000,
  );

  it(
    "keeps an MCP server when rendered delete confirmation is canceled",
    async () => {
      const { ref } = renderPanel();

      await act(async () => {
        ref.current?.openAdd();
      });

      fireEvent.change(
        await screen.findByPlaceholderText("mcp.form.titlePlaceholder"),
        {
          target: { value: "page-cancel-delete" },
        },
      );
      fireEvent.change(screen.getByPlaceholderText("mcp.form.namePlaceholder"), {
        target: { value: "Page Cancel Delete" },
      });
      fireEvent.change(getJsonEditor(), {
        target: {
          value: '{"type":"stdio","command":"echo","args":["cancel-delete"]}',
        },
      });
      fireEvent.click(screen.getByRole("button", { name: "common.add" }));

      await waitFor(async () => {
        const servers = await getMcpServers(webServer.baseUrl);
        expect(servers["page-cancel-delete"]?.name).toBe("Page Cancel Delete");
      });

      const successCallsAfterAdd = toastSuccessMock.mock.calls.length;
      const row = getRowForId("Page Cancel Delete");
      fireEvent.click(within(row).getByTitle("common.delete"));

      await screen.findByText("mcp.unifiedPanel.deleteServer");
      fireEvent.click(screen.getByRole("button", { name: "common.cancel" }));

      await waitFor(() =>
        expect(
          screen.queryByText("mcp.unifiedPanel.deleteServer"),
        ).not.toBeInTheDocument(),
      );
      await waitFor(async () => {
        const servers = await getMcpServers(webServer.baseUrl);
        expect(servers["page-cancel-delete"]?.name).toBe("Page Cancel Delete");
      });
      expect(screen.getByText("Page Cancel Delete")).toBeInTheDocument();
      expect(toastSuccessMock.mock.calls).toHaveLength(successCallsAfterAdd);
      expect(toastErrorMock).not.toHaveBeenCalled();
    },
    20_000,
  );
});
