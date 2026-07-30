import fs from "node:fs/promises";
import { readFileSync } from "node:fs";
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
import { server } from "../msw/server";
import { startTestWebServer, type TestWebServer } from "../helpers/web-server";

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

const TOOL_NAMES = ["claude", "codex", "gemini", "grok", "opencode"] as const;
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
  setToolBehavior: (
    tool: ToolName,
    behavior: FakeToolBehavior,
  ) => Promise<void>;
  stop: () => Promise<void>;
};

const currentRelease: LatestReleaseFixture = {
  tag_name: "v3.16.2",
  body: "Current release notes",
  html_url: "https://github.com/farion1231/cc-switch/releases/tag/v3.16.2",
};

// The version badge shows the running binary's own version (from /api/health =
// CARGO_PKG_VERSION), not any release-server tag. Read it from Cargo.toml so the
// assertion tracks the real app version across bumps instead of a stale literal.
const APP_VERSION = ((): string => {
  const cargoToml = readFileSync(
    path.join(__dirname, "../../src-tauri/Cargo.toml"),
    "utf8",
  );
  const match = cargoToml.match(/^version\s*=\s*"([^"]+)"/m);
  if (!match)
    throw new Error("could not read version from src-tauri/Cargo.toml");
  return match[1];
})();

const newerRelease: LatestReleaseFixture = {
  tag_name: "v3.99.0",
  body: "Newer release notes from test server",
  html_url: "https://github.com/farion1231/cc-switch/releases/tag/v3.99.0",
};

const latestToolVersions: Record<ToolName, string> = {
  claude: "9.9.9",
  codex: "7.7.7",
  gemini: "8.8.8",
  grok: "5.5.5",
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

    if (
      req.method === "GET" &&
      requestUrl.pathname === "/@anthropic-ai/claude-code"
    ) {
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

    if (req.method === "GET" && requestUrl.pathname === "/@xai-official/grok") {
      sendJson({ "dist-tags": { latest: latestToolVersions.grok } });
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
        SHELL: "sh",
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
    await fakeToolBin.setToolBehavior("grok", { stdout: "grok 0.5.0" });
    await fakeToolBin.setToolBehavior("opencode", { stdout: "opencode 2.4.0" });
    localStorage.clear();

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

  it("renders real server-side tool versions, latest metadata, and refreshes the runtime cards", async () => {
    renderAboutSection();

    await expectVersionBadge(`v${APP_VERSION}`);
    await waitFor(
      () => {
        expect(screen.getByText("1.0.0")).toBeInTheDocument();
        expect(screen.getByText("0.9.1")).toBeInTheDocument();
        expect(screen.getByText("5.0.0")).toBeInTheDocument();
        expect(screen.getByText("0.5.0")).toBeInTheDocument();
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
  }, 20_000);

  it("renders server version/runtime info and opens the current release notes link", async () => {
    renderAboutSection();

    expect(await screen.findByText("CC Switch")).toBeInTheDocument();
    await expectVersionBadge(`v${APP_VERSION}`);
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

    await waitFor(
      () => {
        expect(
          screen.getByRole("button", {
            name: /^(settings\.checkForUpdates|Check for Updates|检查更新)$/,
          }),
        ).toBeEnabled();
      },
      { timeout: 10_000 },
    );

    fireEvent.click(
      await screen.findByRole("button", {
        name: /^(settings\.releaseNotes|Release Notes|发行说明)$/,
      }),
    );

    await waitFor(() => {
      // L11: with the web update-check disabled, the Release Notes link uses the
      // running app's own version tag (from /api/health), not a release-server tag.
      expect(window.open).toHaveBeenCalledWith(
        `https://github.com/farion1231/cc-switch/releases/tag/v${APP_VERSION}`,
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
  }, 15_000);

  it("does not surface an available update in web mode even when the server reports a newer release", async () => {
    // L11 / H3: the web build has no independent release channel and must not
    // steer users to upstream desktop releases. Even when a newer release is
    // published upstream, web mode never shows an "Update to" button and
    // reports up-to-date on an explicit check.
    releaseServer.setRelease(newerRelease);

    renderAboutSection();

    await expectVersionBadge(`v${APP_VERSION}`);

    // No "Update to <newer>" button appears.
    await waitFor(
      () => {
        expect(
          screen.getByRole("button", {
            name: /^(settings\.checkForUpdates|Check for Updates|检查更新)$/,
          }),
        ).toBeEnabled();
      },
      { timeout: 10_000 },
    );
    expect(
      screen.queryByRole("button", {
        name: /^(settings\.updateTo|Update to|更新到)/,
      }),
    ).not.toBeInTheDocument();

    // An explicit check reports up-to-date and never opens an upstream URL.
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
    expect(window.open).not.toHaveBeenCalledWith(
      newerRelease.html_url,
      "_blank",
      "noopener,noreferrer",
    );
  });
});
