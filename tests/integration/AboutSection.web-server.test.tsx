import fs from "node:fs/promises";
import http from "node:http";
import type { AddressInfo } from "node:net";
import os from "node:os";
import path from "node:path";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import {
  afterAll,
  beforeAll,
  beforeEach,
  describe,
  expect,
  it,
  vi,
} from "vitest";

import "@/lib/api/web-commands";
import { AboutSection } from "@/components/settings/AboutSection";
import { UpdateProvider } from "@/contexts/UpdateContext";
import { setCsrfToken } from "@/lib/api/adapter";
import { server } from "../msw/server";
import {
  startTestWebServer,
  type TestWebServer,
} from "../helpers/web-server";

const toastSuccessMock = vi.fn();
const toastErrorMock = vi.fn();

vi.mock("sonner", () => ({
  toast: {
    success: (...args: unknown[]) => toastSuccessMock(...args),
    error: (...args: unknown[]) => toastErrorMock(...args),
  },
}));

type LatestReleaseFixture = {
  tag_name: string;
  body: string;
  html_url: string;
};

type ReleaseServer = {
  baseUrl: string;
  setRelease: (release: LatestReleaseFixture) => void;
  stop: () => Promise<void>;
};

const TOOL_NAMES = ["claude", "codex", "gemini", "opencode"] as const;
type ToolName = (typeof TOOL_NAMES)[number];

type ToolMetadataServer = {
  baseUrl: string;
  stop: () => Promise<void>;
};

type FakeToolBehavior = {
  stdout?: string;
  stderr?: string;
  exitCode?: number;
};

type FakeToolBin = {
  binDir: string;
  setToolBehavior: (tool: ToolName, behavior: FakeToolBehavior) => Promise<void>;
  stop: () => Promise<void>;
};

const currentRelease: LatestReleaseFixture = {
  tag_name: "v3.14.1",
  body: "Current release notes",
  html_url: "https://github.com/farion1231/cc-switch/releases/tag/v3.14.1",
};

const newerRelease: LatestReleaseFixture = {
  tag_name: "v3.15.0",
  body: "Newer release notes from test server",
  html_url: "https://github.com/farion1231/cc-switch/releases/tag/v3.15.0",
};

const latestToolVersions: Record<ToolName, string> = {
  claude: "9.9.9",
  codex: "7.7.7",
  gemini: "8.8.8",
  opencode: "6.6.6",
};

async function startReleaseServer(): Promise<ReleaseServer> {
  let currentFixture = currentRelease;

  const releaseServer = http.createServer((req, res) => {
    const requestUrl = new URL(req.url ?? "/", "http://127.0.0.1");

    if (
      req.method === "GET" &&
      requestUrl.pathname === "/repos/farion1231/cc-switch/releases/latest"
    ) {
      res.statusCode = 200;
      res.setHeader("Content-Type", "application/json; charset=utf-8");
      res.end(JSON.stringify(currentFixture));
      return;
    }

    res.statusCode = 404;
    res.setHeader("Content-Type", "text/plain; charset=utf-8");
    res.end("not found");
  });

  await new Promise<void>((resolve, reject) => {
    releaseServer.once("error", reject);
    releaseServer.listen(0, "127.0.0.1", () => resolve());
  });

  const address = releaseServer.address();
  if (!address || typeof address === "string") {
    throw new Error("failed to start release server");
  }

  return {
    baseUrl: `http://127.0.0.1:${(address as AddressInfo).port}`,
    setRelease: (release) => {
      currentFixture = release;
    },
    stop: async () => {
      await new Promise<void>((resolve, reject) => {
        releaseServer.close((error) => {
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

async function startToolMetadataServer(): Promise<ToolMetadataServer> {
  const metadataServer = http.createServer((req, res) => {
    const requestUrl = new URL(req.url ?? "/", "http://127.0.0.1");

    const sendJson = (payload: unknown) => {
      res.statusCode = 200;
      res.setHeader("Content-Type", "application/json; charset=utf-8");
      res.end(JSON.stringify(payload));
    };

    if (req.method === "GET" && requestUrl.pathname === "/@anthropic-ai/claude-code") {
      sendJson({ "dist-tags": { latest: latestToolVersions.claude } });
      return;
    }

    if (req.method === "GET" && requestUrl.pathname === "/@openai/codex") {
      sendJson({ "dist-tags": { latest: latestToolVersions.codex } });
      return;
    }

    if (req.method === "GET" && requestUrl.pathname === "/@google/gemini-cli") {
      sendJson({ "dist-tags": { latest: latestToolVersions.gemini } });
      return;
    }

    if (
      req.method === "GET" &&
      requestUrl.pathname === "/repos/anomalyco/opencode/releases/latest"
    ) {
      sendJson({ tag_name: `v${latestToolVersions.opencode}` });
      return;
    }

    res.statusCode = 404;
    res.setHeader("Content-Type", "text/plain; charset=utf-8");
    res.end("not found");
  });

  await new Promise<void>((resolve, reject) => {
    metadataServer.once("error", reject);
    metadataServer.listen(0, "127.0.0.1", () => resolve());
  });

  const address = metadataServer.address();
  if (!address || typeof address === "string") {
    throw new Error("failed to start tool metadata server");
  }

  return {
    baseUrl: `http://127.0.0.1:${(address as AddressInfo).port}`,
    stop: async () => {
      await new Promise<void>((resolve, reject) => {
        metadataServer.close((error) => {
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

function shellQuote(value: string): string {
  return `'${value.replace(/'/g, `'\"'\"'`)}'`;
}

async function writeFakeToolBehavior(
  binDir: string,
  tool: ToolName,
  behavior: FakeToolBehavior,
): Promise<void> {
  const stateFile = path.join(binDir, `${tool}.state`);
  const state = [
    `stdout_output=${shellQuote(behavior.stdout ?? "")}`,
    `stderr_output=${shellQuote(behavior.stderr ?? "")}`,
    `exit_code=${behavior.exitCode ?? 0}`,
    "",
  ].join("\n");

  await fs.writeFile(stateFile, state, "utf8");
}

async function startFakeToolBin(): Promise<FakeToolBin> {
  const binDir = await fs.mkdtemp(
    path.join(os.tmpdir(), "cc-switch-about-tools-"),
  );

  for (const tool of TOOL_NAMES) {
    const scriptPath = path.join(binDir, tool);
    const script = [
      "#!/bin/sh",
      "set -eu",
      'state_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)',
      `. "$state_dir/${tool}.state"`,
      'if [ -n "${stderr_output:-}" ]; then',
      '  printf "%s\\n" "$stderr_output" >&2',
      "fi",
      'if [ -n "${stdout_output:-}" ]; then',
      '  printf "%s\\n" "$stdout_output"',
      "fi",
      'exit "${exit_code:-0}"',
      "",
    ].join("\n");

    await fs.writeFile(scriptPath, script, "utf8");
    await fs.chmod(scriptPath, 0o755);
    await writeFakeToolBehavior(binDir, tool, {});
  }

  return {
    binDir,
    setToolBehavior: (tool, behavior) =>
      writeFakeToolBehavior(binDir, tool, behavior),
    stop: async () => {
      await fs.rm(binDir, { recursive: true, force: true });
    },
  };
}

function renderAboutSection() {
  render(
    <UpdateProvider>
      <AboutSection isPortable={false} />
    </UpdateProvider>,
  );
}

async function expectVersionBadge(versionText: string): Promise<void> {
  await waitFor(
    () => {
      expect(screen.getByText(versionText)).toBeInTheDocument();
    },
    { timeout: 10_000 },
  );
}

describe.sequential("AboutSection against real web server", () => {
  let webServer: TestWebServer;
  let releaseServer: ReleaseServer;
  let toolMetadataServer: ToolMetadataServer;
  let fakeToolBin: FakeToolBin;

  beforeAll(async () => {
    server.close();
    releaseServer = await startReleaseServer();
    toolMetadataServer = await startToolMetadataServer();
    fakeToolBin = await startFakeToolBin();
    webServer = await startTestWebServer({
      env: {
        CC_SWITCH_RELEASES_API_BASE_URL: releaseServer.baseUrl,
        CC_SWITCH_NPM_REGISTRY_BASE_URL: toolMetadataServer.baseUrl,
        CC_SWITCH_GITHUB_API_BASE_URL: toolMetadataServer.baseUrl,
        PATH: `${fakeToolBin.binDir}:${process.env.PATH ?? ""}`,
      },
    });
  }, 360_000);

  afterAll(async () => {
    await webServer?.stop();
    await releaseServer?.stop();
    await toolMetadataServer?.stop();
    await fakeToolBin?.stop();
    server.listen({ onUnhandledRequest: "warn" });
  }, 20_000);

  beforeEach(async () => {
    toastSuccessMock.mockReset();
    toastErrorMock.mockReset();
    releaseServer.setRelease(currentRelease);
    await fakeToolBin.setToolBehavior("claude", { stdout: "claude 1.0.0" });
    await fakeToolBin.setToolBehavior("codex", { stdout: "codex 0.9.1" });
    await fakeToolBin.setToolBehavior("gemini", { stdout: "gemini 5.0.0" });
    await fakeToolBin.setToolBehavior("opencode", { stdout: "opencode 2.4.0" });
    localStorage.clear();
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
    Object.defineProperty(window, "open", {
      configurable: true,
      value: vi.fn(),
    });
    Object.defineProperty(window, "scrollTo", {
      configurable: true,
      value: vi.fn(),
    });
  });

  it(
    "renders real server-side tool versions, latest metadata, and refreshes the runtime cards",
    async () => {
      renderAboutSection();

      await expectVersionBadge("v3.14.1");
      await waitFor(
        () => {
          expect(screen.getByText("1.0.0")).toBeInTheDocument();
          expect(screen.getByText("0.9.1")).toBeInTheDocument();
          expect(screen.getByText("5.0.0")).toBeInTheDocument();
          expect(screen.getByText("2.4.0")).toBeInTheDocument();
          expect(screen.getByText(latestToolVersions.claude)).toBeInTheDocument();
        },
        { timeout: 15_000 },
      );

      await fakeToolBin.setToolBehavior("claude", { stdout: "claude 1.0.1" });

      fireEvent.click(
        await screen.findByRole("button", {
          name: /^(common\.refresh|Refresh|刷新)$/,
        }),
      );

      await waitFor(
        () => {
          expect(screen.getByText("1.0.1")).toBeInTheDocument();
          expect(screen.queryByText("1.0.0")).not.toBeInTheDocument();
        },
        { timeout: 10_000 },
      );
    },
    20_000,
  );

  it(
    "renders server version/runtime info and opens the current release notes link",
    async () => {
      renderAboutSection();

      expect(await screen.findByText("CC Switch")).toBeInTheDocument();
      await expectVersionBadge("v3.14.1");
      expect(
        await screen.findByText(
          /^(settings\.serverEnvCheck|服务端环境检查|Server Environment Check)$/,
        ),
      ).toBeInTheDocument();
      expect(
        await screen.findByText(
          /^(settings\.serverInstallHint|在服务端执行安装命令|Run the install commands on the server)$/,
        ),
      ).toBeInTheDocument();

      await waitFor(() => {
        expect(
          screen.getByRole("button", {
            name: /^(settings\.checkForUpdates|Check for Updates|检查更新)$/,
          }),
        ).toBeEnabled();
      }, { timeout: 10_000 });

      fireEvent.click(
        await screen.findByRole("button", {
          name: /^(settings\.releaseNotes|Release Notes|发行说明)$/,
        }),
      );

      await waitFor(() => {
        expect(window.open).toHaveBeenCalledWith(
          currentRelease.html_url,
          "_blank",
          "noopener,noreferrer",
        );
      });

      fireEvent.click(
        await screen.findByRole("button", {
          name: /^(settings\.checkForUpdates|Check for Updates|检查更新)$/,
        }),
      );

      await waitFor(() => {
        expect(toastSuccessMock).toHaveBeenCalledWith(
          expect.stringMatching(/^(settings\.upToDate|已是最新版本|Up to date)$/),
          expect.objectContaining({ closeButton: true }),
        );
      });
    },
    15_000,
  );

  it("uses server-backed update metadata and opens the newer release url", async () => {
    releaseServer.setRelease(newerRelease);

    renderAboutSection();

    await expectVersionBadge("v3.14.1");

    const updateButton = await screen.findByRole("button", {
      name: /^(settings\.updateTo|Update to|更新到)/,
    }, {
      timeout: 10_000,
    });
    fireEvent.click(updateButton);

    await waitFor(() => {
      expect(window.open).toHaveBeenCalledWith(
        newerRelease.html_url,
        "_blank",
        "noopener,noreferrer",
      );
      expect(toastSuccessMock).toHaveBeenCalledWith(
        expect.stringMatching(
          /^(settings\.updateDownloadOpened|已打开下载页面|Opened download page)$/,
        ),
        expect.objectContaining({ closeButton: true }),
      );
    });
  });
});
