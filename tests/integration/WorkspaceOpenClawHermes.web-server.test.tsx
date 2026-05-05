import fs from "node:fs/promises";
import path from "node:path";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterAll, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import type { ReactNode } from "react";

import "@/lib/api/web-commands";
import WorkspaceFilesPanel from "@/components/workspace/WorkspaceFilesPanel";
import DailyMemoryPanel from "@/components/workspace/DailyMemoryPanel";
import EnvPanel from "@/components/openclaw/EnvPanel";
import ToolsPanel from "@/components/openclaw/ToolsPanel";
import AgentsDefaultsPanel from "@/components/openclaw/AgentsDefaultsPanel";
import HermesMemoryPanel from "@/components/hermes/HermesMemoryPanel";
import { setCsrfToken } from "@/lib/api/adapter";
import { openclawApi } from "@/lib/api/openclaw";
import { server } from "../msw/server";
import {
  startTestWebServer,
  type TestWebServer,
} from "../helpers/web-server";

const toastSuccessMock = vi.fn();
const toastErrorMock = vi.fn();

const saveRegex = /^(common\.save|保存|Save)$/;
const hermesOpenConfigRegex =
  /^(hermes\.memory\.openConfig|打开配置|Open Config)$/;
const webManualPathTitleRegex =
  /(settings\.webManualPathHint|Web 模式无法浏览服务端文件系统|server file system)/;
const legacyTimeoutRegex =
  /^(openclaw\.agents\.legacyTimeoutTitle|Legacy timeout detected)$/;
const hermesDisabledHintRegex =
  /^(hermes\.memory\.disabledHint|已禁用|Disabled)$/;
const hermesRemoteHintRegex =
  /^(hermes\.webui\.remoteHint|Hermes Web UI 仅在服务端本机可访问)$/;
const hermesRemoteHintDescriptionRegex =
  /(hermes\.webui\.remoteHintDescription|127\.0\.0\.1:9119\/config)/;

vi.mock("sonner", () => ({
  toast: {
    success: (...args: unknown[]) => toastSuccessMock(...args),
    error: (...args: unknown[]) => toastErrorMock(...args),
  },
}));

vi.mock("@/components/MarkdownEditor", () => ({
  default: ({
    value,
    onChange,
  }: {
    value: string;
    onChange?: (value: string) => void;
  }) => (
    <textarea
      aria-label="markdown-editor"
      value={value}
      onChange={(event) => onChange?.(event.target.value)}
    />
  ),
}));

vi.mock("@/components/JsonEditor", () => ({
  default: ({
    value,
    onChange,
  }: {
    value: string;
    onChange: (value: string) => void;
  }) => (
    <textarea
      aria-label="json-editor"
      value={value}
      onChange={(event) => onChange(event.target.value)}
    />
  ),
}));

const workspaceFilePath = (homeDir: string, filename: string): string =>
  path.join(homeDir, ".openclaw", "workspace", filename);

const dailyMemoryFilePath = (homeDir: string, filename: string): string =>
  path.join(homeDir, ".openclaw", "workspace", "memory", filename);

const openClawConfigPath = (homeDir: string): string =>
  path.join(homeDir, ".openclaw", "openclaw.json");

const hermesConfigPath = (homeDir: string): string =>
  path.join(homeDir, ".hermes", "config.yaml");

const hermesMemoryPath = (homeDir: string, filename: string): string =>
  path.join(homeDir, ".hermes", "memories", filename);

async function writeTextFixture(
  filePath: string,
  content: string,
): Promise<void> {
  await fs.mkdir(path.dirname(filePath), { recursive: true });
  await fs.writeFile(filePath, content, "utf8");
}

async function writeJsonFixture(filePath: string, value: unknown): Promise<void> {
  await writeTextFixture(filePath, JSON.stringify(value, null, 2));
}

function renderWithQueryClient(ui: ReactNode) {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  });

  return render(
    <QueryClientProvider client={queryClient}>{ui}</QueryClientProvider>,
  );
}

describe.sequential(
  "Workspace, OpenClaw, and Hermes panels against real web server",
  () => {
    let webServer: TestWebServer;

    beforeAll(async () => {
      server.close();
      webServer = await startTestWebServer();
    }, 360_000);

    afterAll(async () => {
      await webServer.stop();
      server.listen({ onUnhandledRequest: "warn" });
    }, 20_000);

    beforeEach(async () => {
      toastSuccessMock.mockReset();
      toastErrorMock.mockReset();
      setCsrfToken(null);
      window.localStorage.clear();

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
      Object.defineProperty(HTMLElement.prototype, "scrollIntoView", {
        configurable: true,
        value: vi.fn(),
      });
      Object.defineProperty(window, "matchMedia", {
        configurable: true,
        value: vi.fn().mockImplementation(() => ({
          matches: false,
          media: "(prefers-color-scheme: dark)",
          onchange: null,
          addEventListener: vi.fn(),
          removeEventListener: vi.fn(),
          addListener: vi.fn(),
          removeListener: vi.fn(),
          dispatchEvent: vi.fn(),
        })),
      });

      await fs.rm(path.join(webServer.homeDir, ".openclaw"), {
        recursive: true,
        force: true,
      });
      await fs.rm(path.join(webServer.homeDir, ".hermes"), {
        recursive: true,
        force: true,
      });

      await writeTextFixture(
        workspaceFilePath(webServer.homeDir, "AGENTS.md"),
        "# Seed AGENTS\n\nOriginal workspace content.\n",
      );
      await writeTextFixture(
        dailyMemoryFilePath(webServer.homeDir, "2026-03-04.md"),
        "# 2026-03-04\n\nInitial daily memory marker.\n",
      );
      await writeJsonFixture(openClawConfigPath(webServer.homeDir), {
        models: {
          mode: "merge",
          providers: {},
        },
        env: {
          vars: {
            SEED_TOKEN: "abc",
          },
          shellEnv: {
            OPENCLAW_ENV: "seed",
          },
        },
        tools: {
          profile: "minimal",
          allow: ["allow:read"],
          deny: ["deny:write"],
          passthrough: true,
        },
        agents: {
          defaults: {
            workspace: "~/seed-projects",
            timeout: 300,
            contextTokens: 200000,
            maxConcurrent: 4,
            unknownFlag: true,
          },
        },
      });
      await writeTextFixture(
        hermesConfigPath(webServer.homeDir),
        [
          "memory:",
          "  memory_char_limit: 2200",
          "  user_char_limit: 1375",
          "  memory_enabled: true",
          "  user_profile_enabled: false",
          "",
        ].join("\n"),
      );
      await writeTextFixture(
        hermesMemoryPath(webServer.homeDir, "MEMORY.md"),
        "# Hermes MEMORY\n\nInitial Hermes memory.\n",
      );
      await writeTextFixture(
        hermesMemoryPath(webServer.homeDir, "USER.md"),
        "# Hermes USER\n\nInitial Hermes user profile.\n",
      );
    });

    it(
      "persists workspace and daily-memory edits through the rendered panels while keeping web-only manual path hints",
      async () => {
        const workspaceRender = render(<WorkspaceFilesPanel />);

        const workspacePath = await screen.findByText("~/.openclaw/workspace/");
        expect(workspacePath).toHaveAttribute("title");
        expect(workspacePath.getAttribute("title")).toMatch(webManualPathTitleRegex);

        fireEvent.click(screen.getByText("AGENTS.md"));

        const workspaceEditor = await screen.findByLabelText("markdown-editor");
        fireEvent.change(workspaceEditor, {
          target: { value: "# Seed AGENTS\n\nUpdated workspace content.\n" },
        });
        fireEvent.click(screen.getByRole("button", { name: saveRegex }));

        await waitFor(async () => {
          expect(
            await fs.readFile(
              workspaceFilePath(webServer.homeDir, "AGENTS.md"),
              "utf8",
            ),
          ).toContain("Updated workspace content.");
        });
        expect(toastSuccessMock).toHaveBeenCalled();

        workspaceRender.unmount();

        const dailyMemoryRender = render(
          <DailyMemoryPanel isOpen onClose={() => undefined} />,
        );

        const memoryPath = await screen.findByText(
          "~/.openclaw/workspace/memory/",
        );
        expect(memoryPath).toHaveAttribute("title");
        expect(memoryPath.getAttribute("title")).toMatch(webManualPathTitleRegex);
        expect(await screen.findByText("2026-03-04")).toBeInTheDocument();

        fireEvent.click(screen.getByText("2026-03-04"));

        const dailyMemoryEditor = await screen.findByLabelText("markdown-editor");
        fireEvent.change(dailyMemoryEditor, {
          target: {
            value: "# 2026-03-04\n\nUpdated daily memory marker.\n",
          },
        });
        fireEvent.click(screen.getByRole("button", { name: saveRegex }));

        await waitFor(async () => {
          expect(
            await fs.readFile(
              dailyMemoryFilePath(webServer.homeDir, "2026-03-04.md"),
              "utf8",
            ),
          ).toContain("Updated daily memory marker.");
        });

        dailyMemoryRender.unmount();

        render(<DailyMemoryPanel isOpen onClose={() => undefined} />);
        fireEvent.click(await screen.findByText("2026-03-04"));
        expect(await screen.findByDisplayValue(/Updated daily memory marker\./)).toBeInTheDocument();
      },
    );

    it(
      "persists OpenClaw env and tools through rendered panels and reloads saved state",
      async () => {
        const envRender = renderWithQueryClient(<EnvPanel />);

        const envEditor = await screen.findByLabelText("json-editor");
        await waitFor(() => {
          expect((envEditor as HTMLTextAreaElement).value).toContain(
            "SEED_TOKEN",
          );
        });
        fireEvent.change(envEditor, {
          target: {
            value: JSON.stringify(
              {
                vars: {
                  SEED_TOKEN: "updated",
                  NEW_VALUE: "visible",
                },
                shellEnv: {
                  OPENCLAW_ENV: "rendered",
                },
              },
              null,
              2,
            ),
          },
        });
        fireEvent.click(screen.getByRole("button", { name: saveRegex }));

        await waitFor(async () => {
          expect(await openclawApi.getEnv()).toEqual({
            vars: {
              SEED_TOKEN: "updated",
              NEW_VALUE: "visible",
            },
            shellEnv: {
              OPENCLAW_ENV: "rendered",
            },
          });
        });
        expect(await fs.readFile(openClawConfigPath(webServer.homeDir), "utf8")).toContain(
          '"NEW_VALUE": "visible"',
        );

        envRender.unmount();

        const envReload = renderWithQueryClient(<EnvPanel />);
        expect(await screen.findByDisplayValue(/NEW_VALUE/)).toBeInTheDocument();
        envReload.unmount();

        const toolsRender = renderWithQueryClient(<ToolsPanel />);

        expect(await screen.findByDisplayValue("allow:read")).toBeInTheDocument();
        expect(screen.getByDisplayValue("deny:write")).toBeInTheDocument();

        fireEvent.click(
          screen.getByRole("button", {
            name: /^(openclaw\.tools\.addAllow|添加允许|Add Allow)$/,
          }),
        );
        fireEvent.click(
          screen.getByRole("button", {
            name: /^(openclaw\.tools\.addDeny|添加拒绝|Add Deny)$/,
          }),
        );

        const patternInputs = screen.getAllByPlaceholderText(
          /^(openclaw\.tools\.patternPlaceholder|工具名称或模式|Tool name or pattern)$/,
        );
        fireEvent.change(patternInputs[1], {
          target: { value: "allow:network" },
        });
        fireEvent.change(patternInputs[3], {
          target: { value: "deny:shell" },
        });
        fireEvent.click(screen.getByRole("button", { name: saveRegex }));

        await waitFor(async () => {
          expect(await openclawApi.getTools()).toEqual({
            profile: "minimal",
            allow: ["allow:read", "allow:network"],
            deny: ["deny:write", "deny:shell"],
            passthrough: true,
          });
        });
        expect(await fs.readFile(openClawConfigPath(webServer.homeDir), "utf8")).toContain(
          '"allow:network"',
        );

        toolsRender.unmount();

        renderWithQueryClient(<ToolsPanel />);
        expect(await screen.findByDisplayValue("allow:network")).toBeInTheDocument();
        expect(screen.getByDisplayValue("deny:shell")).toBeInTheDocument();
      },
    );

    it(
      "migrates and reloads OpenClaw agents.defaults runtime fields through the rendered panel",
      async () => {
        const firstRender = renderWithQueryClient(<AgentsDefaultsPanel />);

        expect(await screen.findByText(legacyTimeoutRegex)).toBeInTheDocument();

        fireEvent.change(screen.getByDisplayValue("~/seed-projects"), {
          target: { value: "~/page-smoke-workspace" },
        });
        fireEvent.change(screen.getByDisplayValue("300"), {
          target: { value: "480" },
        });
        fireEvent.change(screen.getByDisplayValue("200000"), {
          target: { value: "250000" },
        });
        fireEvent.change(screen.getByDisplayValue("4"), {
          target: { value: "6" },
        });
        fireEvent.click(screen.getByRole("button", { name: saveRegex }));

        await waitFor(async () => {
          expect(await openclawApi.getAgentsDefaults()).toEqual(
            expect.objectContaining({
              workspace: "~/page-smoke-workspace",
              timeoutSeconds: 480,
              contextTokens: 250000,
              maxConcurrent: 6,
              unknownFlag: true,
            }),
          );
        });

        const persistedConfig = await fs.readFile(
          openClawConfigPath(webServer.homeDir),
          "utf8",
        );
        expect(persistedConfig).toContain("~/page-smoke-workspace");
        expect(persistedConfig).toContain("timeoutSeconds");
        expect(persistedConfig).toContain("250000");
        expect(persistedConfig).toContain("unknownFlag");
        expect(persistedConfig).not.toMatch(/"timeout"\s*:/);

        firstRender.unmount();

        renderWithQueryClient(<AgentsDefaultsPanel />);
        expect(
          await screen.findByDisplayValue("~/page-smoke-workspace"),
        ).toBeInTheDocument();
        expect(screen.getByDisplayValue("480")).toBeInTheDocument();
      },
    );

    it(
      "shows the remote Hermes hint, saves memory content, toggles enablement, and reloads persisted state",
      async () => {
        const firstRender = renderWithQueryClient(<HermesMemoryPanel />);

        const openConfigButton = await screen.findByRole("button", {
          name: hermesOpenConfigRegex,
        });
        expect(openConfigButton).toBeDisabled();
        expect(openConfigButton.getAttribute("title")).toMatch(
          hermesRemoteHintDescriptionRegex,
        );
        expect(await screen.findByText(hermesRemoteHintRegex)).toBeInTheDocument();
        expect(
          await screen.findByText(hermesRemoteHintDescriptionRegex),
        ).toBeInTheDocument();

        const memoryEditor = await screen.findByLabelText("markdown-editor");
        fireEvent.change(memoryEditor, {
          target: { value: "# Hermes MEMORY\n\nUpdated Hermes memory.\n" },
        });
        fireEvent.click(screen.getByRole("switch"));
        fireEvent.click(screen.getByRole("button", { name: saveRegex }));

        await waitFor(async () => {
          expect(
            await fs.readFile(
              hermesMemoryPath(webServer.homeDir, "MEMORY.md"),
              "utf8",
            ),
          ).toContain("Updated Hermes memory.");
        });
        await waitFor(async () => {
          expect(await fs.readFile(hermesConfigPath(webServer.homeDir), "utf8")).toContain(
            "memory_enabled: false",
          );
        });

        firstRender.unmount();

        renderWithQueryClient(<HermesMemoryPanel />);
        expect(await screen.findByDisplayValue(/Updated Hermes memory\./)).toBeInTheDocument();
        expect(await screen.findByText(hermesDisabledHintRegex)).toBeInTheDocument();
      },
    );
  },
);
