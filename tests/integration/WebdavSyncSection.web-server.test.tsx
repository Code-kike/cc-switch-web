import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterAll, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";

import "@/lib/api/web-commands";
import { WebdavSyncSection } from "@/components/settings/WebdavSyncSection";
import { setCsrfToken } from "@/lib/api/adapter";
import { providersApi } from "@/lib/api/providers";
import { settingsApi } from "@/lib/api/settings";
import { useSettingsQuery } from "@/lib/query";
import type { Provider } from "@/types";
import type { SettingsFormState } from "@/hooks/useSettings";
import { server } from "../msw/server";
import {
  startTestWebServer,
  type TestWebServer,
} from "../helpers/web-server";
import {
  startTestWebDavServer,
  type TestWebDavServer,
} from "../helpers/webdav-server";

const toastSuccessMock = vi.fn();
const toastErrorMock = vi.fn();
const toastWarningMock = vi.fn();
const toastInfoMock = vi.fn();

const saveRegex = /^(settings\.webdavSync\.save|保存配置|Save Config)$/;
const uploadRegex = /^(settings\.webdavSync\.upload|上传到云端|Upload to Cloud)$/;
const downloadRegex =
  /^(settings\.webdavSync\.download|从云端下载|Download from Cloud)$/;
const saveAndTestSuccessRegex =
  /^(settings\.webdavSync\.saveAndTestSuccess|配置已保存，连接正常|Config saved, connection OK)$/;
const uploadSuccessRegex =
  /^(settings\.webdavSync\.uploadSuccess|已上传到 WebDAV|Uploaded to WebDAV)$/;
const downloadSuccessRegex =
  /^(settings\.webdavSync\.downloadSuccess|已从 WebDAV 下载并恢复|Downloaded and restored from WebDAV)$/;
const fetchRemoteFailedRegex =
  /^(settings\.webdavSync\.fetchRemoteFailed|获取远端信息失败，请检查配置和网络后重试。|Failed to fetch remote info\. Please check configuration and network\.)$/;
const confirmUploadTitleRegex =
  /^(settings\.webdavSync\.confirmUpload\.title|上传到云端|Upload to Cloud)$/;
const confirmDownloadTitleRegex =
  /^(settings\.webdavSync\.confirmDownload\.title|从云端恢复|Restore from Cloud)$/;
const confirmUploadRegex =
  /^(settings\.webdavSync\.confirmUpload\.confirm|确认上传|Confirm Upload)$/;
const confirmDownloadRegex =
  /^(settings\.webdavSync\.confirmDownload\.confirm|确认恢复|Confirm Restore)$/;
const uploadedByRegex =
  /^(settings\.webdavSync\.(confirmUpload|confirmDownload)\.deviceName|上传设备|Uploaded by)$/;

vi.mock("sonner", () => ({
  toast: {
    success: (...args: unknown[]) => toastSuccessMock(...args),
    error: (...args: unknown[]) => toastErrorMock(...args),
    warning: (...args: unknown[]) => toastWarningMock(...args),
    info: (...args: unknown[]) => toastInfoMock(...args),
  },
}));

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

function toastMessages(mock: ReturnType<typeof vi.fn>): string[] {
  return mock.mock.calls.map(([message]) => String(message));
}

function expectToastMessage(
  mock: ReturnType<typeof vi.fn>,
  pattern: RegExp,
): void {
  expect(toastMessages(mock).some((message) => pattern.test(message))).toBe(true);
}

async function removeClaudeProviderIfPresent(id: string): Promise<void> {
  const providers = await providersApi.getAll("claude");
  if (!providers[id]) {
    return;
  }

  const currentProviderId = await providersApi.getCurrent("claude");
  if (currentProviderId === id) {
    const fallbackProviderId = Object.keys(providers).find(
      (providerId) => providerId !== id,
    );
    if (fallbackProviderId) {
      await providersApi.switch(fallbackProviderId, "claude");
    } else {
      return;
    }
  }

  await providersApi.delete(id, "claude");
}

function WebdavSectionHarness() {
  const { data: settings } = useSettingsQuery();

  if (!settings) {
    return <div>loading</div>;
  }

  const normalizedSettings: SettingsFormState = {
    ...settings,
    language: settings.language ?? "zh",
  };

  return (
    <WebdavSyncSection
      config={settings.webdavSync}
      settings={normalizedSettings}
    />
  );
}

function renderSection() {
  const client = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  });

  return render(
    <QueryClientProvider client={client}>
      <WebdavSectionHarness />
    </QueryClientProvider>,
  );
}

describe.sequential("WebdavSyncSection against real web server", () => {
  let webServer: TestWebServer;
  let webDavServer: TestWebDavServer;

  beforeAll(async () => {
    server.close();
    webDavServer = await startTestWebDavServer({
      rootPath: "/dav",
      username: "alice",
      password: "secret",
    });
    webServer = await startTestWebServer();
  }, 360_000);

  afterAll(async () => {
    await webServer.stop();
    await webDavServer.stop();
    server.listen({ onUnhandledRequest: "warn" });
  }, 20_000);

  beforeEach(async () => {
    toastSuccessMock.mockReset();
    toastErrorMock.mockReset();
    toastWarningMock.mockReset();
    toastInfoMock.mockReset();
    webDavServer.reset();
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

    const currentSettings = await settingsApi.get();
    await settingsApi.save({
      ...currentSettings,
      commonConfigConfirmed: true,
    });
    await settingsApi.webdavSyncSaveSettings(
      {
        enabled: true,
        baseUrl: webDavServer.baseUrl,
        username: "alice",
        password: "secret",
        remoteRoot: "reset-root",
        profile: "reset-profile",
        autoSync: false,
      },
      true,
    );

    await removeClaudeProviderIfPresent("webdav-page-provider");
  });

  it(
    "saves WebDAV config, uploads a snapshot, shows remote info, and restores database state through the rendered section UI",
    async () => {
      renderSection();

      expect(await screen.findByText(/loading|settings\.webdavSync\.title|WebDAV/)).toBeInTheDocument();

      const baseUrlInput = await screen.findByDisplayValue(webDavServer.baseUrl);
      const usernameInput = screen.getByDisplayValue("alice");
      const passwordInput = screen.getByPlaceholderText(
        /^(settings\.webdavSync\.passwordPlaceholder|应用密码（坚果云请使用「第三方应用密码」）|App password)$/,
      );
      const remoteRootInput = screen.getByDisplayValue("reset-root");
      const profileInput = screen.getByDisplayValue("reset-profile");

      fireEvent.change(baseUrlInput, { target: { value: webDavServer.baseUrl } });
      fireEvent.change(usernameInput, { target: { value: "alice" } });
      fireEvent.change(passwordInput, { target: { value: "secret" } });
      fireEvent.change(remoteRootInput, { target: { value: "page-smoke-root" } });
      fireEvent.change(profileInput, { target: { value: "page-smoke-profile" } });

      fireEvent.click(screen.getByRole("button", { name: saveRegex }));

      await waitFor(() => {
        expectToastMessage(toastSuccessMock, saveAndTestSuccessRegex);
      });

      const savedSettings = await settingsApi.get();
      expect(savedSettings.webdavSync).toEqual(
        expect.objectContaining({
          baseUrl: webDavServer.baseUrl,
          username: "alice",
          password: "",
          remoteRoot: "page-smoke-root",
          profile: "page-smoke-profile",
          autoSync: false,
        }),
      );

      const provider = buildClaudeProvider(
        "webdav-page-provider",
        "WebDAV Page Provider",
        "webdav-page-token",
        "https://webdav-page.example.com",
        777,
      );
      await providersApi.add(provider, "claude");

      await waitFor(async () => {
        expect((await providersApi.getAll("claude"))["webdav-page-provider"]).toBeDefined();
      });

      const uploadButton = screen.getByRole("button", { name: uploadRegex });
      const downloadButton = screen.getByRole("button", { name: downloadRegex });
      expect(uploadButton).not.toBeDisabled();
      expect(downloadButton).not.toBeDisabled();

      fireEvent.click(uploadButton);

      expect(await screen.findByText(confirmUploadTitleRegex)).toBeInTheDocument();
      expect(screen.getByText("/page-smoke-root/v2/db-v6/page-smoke-profile")).toBeInTheDocument();

      fireEvent.click(screen.getByRole("button", { name: confirmUploadRegex }));

      await waitFor(() => {
        expectToastMessage(toastSuccessMock, uploadSuccessRegex);
      });

      const manifestPath =
        "/dav/page-smoke-root/v2/db-v6/page-smoke-profile/manifest.json";
      const manifestBytes = webDavServer.readFile(manifestPath);
      expect(manifestBytes).toBeDefined();

      const manifest = JSON.parse(String(manifestBytes)) as {
        deviceName: string;
        artifacts: Record<string, { sha256: string; size: number }>;
      };
      expect(Object.keys(manifest.artifacts)).toEqual(["db.sql", "skills.zip"]);
      expect(
        webDavServer.readFile("/dav/page-smoke-root/v2/db-v6/page-smoke-profile/db.sql"),
      ).toBeDefined();
      expect(
        webDavServer.readFile("/dav/page-smoke-root/v2/db-v6/page-smoke-profile/skills.zip"),
      ).toBeDefined();

      await waitFor(async () => {
        expect((await settingsApi.get()).webdavSync?.status?.lastSyncAt).toBeTruthy();
      });

      const mutatedProvider = buildClaudeProvider(
        "webdav-page-provider",
        "Mutated WebDAV Page Provider",
        "webdav-page-token-mutated",
        "https://webdav-mutated.example.com",
        777,
      );
      await providersApi.update(mutatedProvider, "claude");
      await waitFor(async () => {
        expect(
          (await providersApi.getAll("claude"))["webdav-page-provider"]?.name,
        ).toBe("Mutated WebDAV Page Provider");
      });

      fireEvent.click(downloadButton);

      expect(await screen.findByText(confirmDownloadTitleRegex)).toBeInTheDocument();
      expect(screen.getByText(uploadedByRegex)).toBeInTheDocument();
      expect(screen.getByText(manifest.deviceName)).toBeInTheDocument();
      expect(screen.getByText("/page-smoke-root/v2/db-v6/page-smoke-profile")).toBeInTheDocument();
      expect(screen.getByText("db.sql, skills.zip")).toBeInTheDocument();

      fireEvent.click(screen.getByRole("button", { name: confirmDownloadRegex }));

      await waitFor(() => {
        expectToastMessage(toastSuccessMock, downloadSuccessRegex);
      });
      await waitFor(async () => {
        expect(
          (await providersApi.getAll("claude"))["webdav-page-provider"],
        ).toEqual(
          expect.objectContaining({
            name: "WebDAV Page Provider",
            settingsConfig: expect.objectContaining({
              env: expect.objectContaining({
                ANTHROPIC_BASE_URL: "https://webdav-page.example.com",
              }),
            }),
          }),
        );
      });

      expect(toastErrorMock).not.toHaveBeenCalled();
    },
    30_000,
  );

  it("shows a rendered-page error toast when remote info fetch fails", async () => {
    renderSection();

    await screen.findByText(/loading|settings\.webdavSync\.title|WebDAV/);

    fireEvent.change(await screen.findByDisplayValue(webDavServer.baseUrl), {
      target: { value: webDavServer.baseUrl },
    });
    fireEvent.change(screen.getByDisplayValue("alice"), {
      target: { value: "alice" },
    });
    fireEvent.change(
      screen.getByPlaceholderText(
        /^(settings\.webdavSync\.passwordPlaceholder|应用密码（坚果云请使用「第三方应用密码」）|App password)$/,
      ),
      {
        target: { value: "secret" },
      },
    );
    fireEvent.change(screen.getByDisplayValue("reset-root"), {
      target: { value: "page-smoke-failure-root" },
    });
    fireEvent.change(screen.getByDisplayValue("reset-profile"), {
      target: { value: "page-smoke-failure-profile" },
    });

    fireEvent.click(screen.getByRole("button", { name: saveRegex }));

    await waitFor(() => {
      expectToastMessage(toastSuccessMock, saveAndTestSuccessRegex);
    });

    webDavServer.failNext(
      "GET",
      "/dav/page-smoke-failure-root/v2/db-v6/page-smoke-failure-profile/manifest.json",
      500,
    );

    fireEvent.click(screen.getByRole("button", { name: uploadRegex }));

    await waitFor(() => {
      expectToastMessage(toastErrorMock, fetchRemoteFailedRegex);
    });
    expect(screen.queryByText(confirmUploadTitleRegex)).not.toBeInTheDocument();
  });
});
