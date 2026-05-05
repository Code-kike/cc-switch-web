import fs from "node:fs/promises";
import path from "node:path";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
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
import { AuthCenterPanel } from "@/components/settings/AuthCenterPanel";
import { setCsrfToken } from "@/lib/api/adapter";
import { server } from "../msw/server";
import {
  startTestWebServer,
  type TestWebServer,
} from "../helpers/web-server";
import {
  startTestAuthServer,
  type TestAuthServer,
} from "../helpers/auth-server";

const remoteHintRegex =
  /^(settings\.authCenter\.webRemoteHint|远程 Web 模式会把 OAuth 账号保存到服务端机器|Remote Web mode stores OAuth accounts on the server host)$/;
const remoteHintDescriptionRegex =
  /^(settings\.authCenter\.webRemoteHintDescription|授权页面会在当前浏览器打开，但登录成功后的 Copilot \/ ChatGPT 账号会绑定到运行 cc-switch Web 的那台机器。仅在您信任该服务端时继续。|The authorization page opens in this browser, but successful Copilot \/ ChatGPT logins are saved on the machine running cc-switch Web\. Continue only if you trust that server\.)$/;
const copilotLoginRegex =
  /^(copilot\.loginWithGitHub|使用 GitHub 登录|Login with GitHub)$/;
const codexLoginRegex =
  /^(codexOauth\.loginWithChatGPT|使用 ChatGPT 登录|Login with ChatGPT)$/;
const addAnotherRegex =
  /(copilot\.addAnotherAccount|codexOauth\.addAnotherAccount|添加其他账号|Add another account)/;
const cancelRegex = /^(common\.cancel|取消|Cancel)$/;
const retryRegex = /(copilot\.retry|codexOauth\.retry|重试|Retry)/;
const setDefaultRegex =
  /(copilot\.setAsDefault|codexOauth\.setAsDefault|设为默认|Set as default)/;
const removeAccountRegex =
  /(copilot\.removeAccount|codexOauth\.removeAccount|移除账号|Remove account)/;
const logoutAllRegex =
  /(copilot\.logoutAll|codexOauth\.logoutAll|注销所有账号|Log out all accounts)/;
const copilotHeadingRegex = /GitHub Copilot/;
const codexHeadingRegex = /ChatGPT \(Codex OAuth\)/;

type CopilotAuthStore = {
  version: number;
  accounts: Record<
    string,
    {
      github_token: string;
      user: {
        id: number;
        login: string;
      };
      github_domain: string;
    }
  >;
  default_account_id: string | null;
};

type CodexAuthStore = {
  version: number;
  accounts: Record<
    string,
    {
      account_id: string;
      email?: string | null;
      refresh_token: string;
    }
  >;
  default_account_id: string | null;
};

function renderPanel() {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  });

  return render(
    <QueryClientProvider client={queryClient}>
      <AuthCenterPanel />
    </QueryClientProvider>,
  );
}

function getSection(titlePattern: RegExp): HTMLElement {
  const heading = screen.getByRole("heading", { name: titlePattern, level: 4 });
  const section = heading.closest("section");
  if (!(section instanceof HTMLElement)) {
    throw new Error(`could not locate section for ${titlePattern}`);
  }
  return section;
}

function getAccountRow(section: HTMLElement, accountLabel: string): HTMLElement {
  const label = within(section).getByText(accountLabel);
  let current: HTMLElement | null = label instanceof HTMLElement ? label : null;

  while (
    current &&
    !current.className.includes("flex items-center justify-between p-2 rounded-md border bg-muted/30")
  ) {
    current = current.parentElement;
  }

  if (!(current instanceof HTMLElement)) {
    throw new Error(`could not locate account row for ${accountLabel}`);
  }

  return current;
}

async function readJsonFile<T>(filePath: string): Promise<T | null> {
  try {
    return JSON.parse(await fs.readFile(filePath, "utf8")) as T;
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code === "ENOENT") {
      return null;
    }
    throw error;
  }
}

describe.sequential("AuthCenterPanel against real web server", () => {
  let webServer: TestWebServer;
  let authServer: TestAuthServer;

  beforeAll(async () => {
    server.close();
    authServer = await startTestAuthServer();
    webServer = await startTestWebServer({
      env: {
        CC_SWITCH_TEST_GITHUB_DEVICE_CODE_URL_TEMPLATE: `${authServer.baseUrl}/copilot/{domain}/login/device/code`,
        CC_SWITCH_TEST_GITHUB_OAUTH_TOKEN_URL_TEMPLATE: `${authServer.baseUrl}/copilot/{domain}/login/oauth/access_token`,
        CC_SWITCH_TEST_GITHUB_API_BASE_URL_TEMPLATE: `${authServer.baseUrl}/copilot/{domain}/api`,
        CC_SWITCH_TEST_CODEX_DEVICE_AUTH_USERCODE_URL: `${authServer.baseUrl}/codex/deviceauth/usercode`,
        CC_SWITCH_TEST_CODEX_DEVICE_AUTH_TOKEN_URL: `${authServer.baseUrl}/codex/deviceauth/token`,
        CC_SWITCH_TEST_CODEX_OAUTH_TOKEN_URL: `${authServer.baseUrl}/codex/oauth/token`,
        CC_SWITCH_TEST_CODEX_DEVICE_VERIFICATION_URL: `${authServer.baseUrl}/codex/verify`,
      },
    });
  }, 360_000);

  afterAll(async () => {
    await webServer.stop();
    await authServer.stop();
    server.listen({ onUnhandledRequest: "warn" });
  }, 20_000);

  beforeEach(async () => {
    authServer.reset();
    setCsrfToken(null);

    await fs.rm(path.join(webServer.dataDir, "copilot_auth.json"), {
      force: true,
    });
    await fs.rm(path.join(webServer.dataDir, "codex_oauth_auth.json"), {
      force: true,
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
    Object.defineProperty(window, "open", {
      configurable: true,
      value: vi.fn(),
    });
    Object.defineProperty(window, "scrollTo", {
      configurable: true,
      value: vi.fn(),
    });
    Object.defineProperty(HTMLElement.prototype, "scrollIntoView", {
      configurable: true,
      value: vi.fn(),
    });
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: {
        writeText: vi.fn().mockResolvedValue(undefined),
      },
    });
  });

  it("shows the remote safety hint and drives Copilot device login through pending, cancel, and success states", async () => {
    authServer.queueCopilotFlow({
      mode: "pending",
      userCode: "GH-PEND",
      verificationUri: `${authServer.baseUrl}/copilot/verify/pending`,
    });
    authServer.queueCopilotFlow({
      mode: "success",
      userCode: "GH-SUCC",
      verificationUri: `${authServer.baseUrl}/copilot/verify/success`,
      account: {
        id: 20001,
        login: "copilot-octo",
        avatarUrl: null,
      },
    });

    renderPanel();

    expect(await screen.findByText(remoteHintRegex)).toBeInTheDocument();
    expect(screen.getByText(remoteHintDescriptionRegex)).toBeInTheDocument();

    const copilotSection = getSection(copilotHeadingRegex);
    fireEvent.click(
      within(copilotSection).getByRole("button", { name: copilotLoginRegex }),
    );

    expect(
      await within(copilotSection).findByText("GH-PEND"),
    ).toBeInTheDocument();
    expect(window.open).toHaveBeenCalledWith(
      `${authServer.baseUrl}/copilot/verify/pending`,
      "_blank",
      "noopener,noreferrer",
    );

    fireEvent.click(
      within(copilotSection).getByRole("button", { name: cancelRegex }),
    );

    await waitFor(() => {
      expect(
        within(copilotSection).getByRole("button", { name: copilotLoginRegex }),
      ).toBeInTheDocument();
    });

    fireEvent.click(
      within(copilotSection).getByRole("button", { name: copilotLoginRegex }),
    );

    expect(
      await within(copilotSection).findByText("copilot-octo"),
    ).toBeInTheDocument();
    expect(window.open).toHaveBeenCalledWith(
      `${authServer.baseUrl}/copilot/verify/success`,
      "_blank",
      "noopener,noreferrer",
    );

    await waitFor(async () => {
      const store = await readJsonFile<CopilotAuthStore>(
        path.join(webServer.dataDir, "copilot_auth.json"),
      );
      expect(store?.default_account_id).toBe("20001");
      expect(store?.accounts["20001"]?.user.login).toBe("copilot-octo");
      expect(store?.accounts["20001"]?.github_domain).toBe("github.com");
    });
  });

  it("drives Codex OAuth login, default switching, removal, failure retry, and logout through the rendered panel", async () => {
    authServer.queueCodexFlow({
      mode: "success",
      userCode: "OA-1001",
      account: {
        accountId: "codex-acc-1",
        email: "plus-user@example.com",
      },
    });
    authServer.queueCodexFlow({
      mode: "success",
      userCode: "OA-1002",
      account: {
        accountId: "codex-acc-2",
        email: "pro-user@example.com",
      },
    });

    renderPanel();

    const codexSection = getSection(codexHeadingRegex);

    fireEvent.click(
      within(codexSection).getByRole("button", { name: codexLoginRegex }),
    );
    expect(
      await within(codexSection).findByText("plus-user@example.com"),
    ).toBeInTheDocument();

    fireEvent.click(
      within(codexSection).getByRole("button", { name: addAnotherRegex }),
    );
    expect(
      await within(codexSection).findByText("pro-user@example.com"),
    ).toBeInTheDocument();

    fireEvent.click(
      within(getAccountRow(codexSection, "pro-user@example.com")).getByRole(
        "button",
        { name: setDefaultRegex },
      ),
    );

    await waitFor(async () => {
      const store = await readJsonFile<CodexAuthStore>(
        path.join(webServer.dataDir, "codex_oauth_auth.json"),
      );
      expect(store?.default_account_id).toBe("codex-acc-2");
      expect(Object.keys(store?.accounts ?? {})).toHaveLength(2);
    });

    fireEvent.click(
      within(getAccountRow(codexSection, "plus-user@example.com")).getByTitle(
        removeAccountRegex,
      ),
    );

    await waitFor(async () => {
      const store = await readJsonFile<CodexAuthStore>(
        path.join(webServer.dataDir, "codex_oauth_auth.json"),
      );
      expect(Object.keys(store?.accounts ?? {})).toEqual(["codex-acc-2"]);
      expect(store?.default_account_id).toBe("codex-acc-2");
    });

    authServer.queueCodexFlow({
      mode: "error",
      userCode: "OA-ERR",
    });

    fireEvent.click(
      within(codexSection).getByRole("button", { name: addAnotherRegex }),
    );

    expect(
      await within(codexSection).findByRole("button", { name: retryRegex }),
    ).toBeInTheDocument();

    authServer.queueCodexFlow({
      mode: "success",
      userCode: "OA-1003",
      account: {
        accountId: "codex-acc-3",
        email: "team-user@example.com",
      },
    });

    fireEvent.click(
      within(codexSection).getByRole("button", { name: retryRegex }),
    );

    expect(
      await within(codexSection).findByText("team-user@example.com"),
    ).toBeInTheDocument();

    fireEvent.click(
      within(codexSection).getByRole("button", { name: logoutAllRegex }),
    );

    await waitFor(async () => {
      const store = await readJsonFile<CodexAuthStore>(
        path.join(webServer.dataDir, "codex_oauth_auth.json"),
      );
      expect(store?.default_account_id ?? null).toBeNull();
      expect(Object.keys(store?.accounts ?? {})).toHaveLength(0);
    });
  });
});
