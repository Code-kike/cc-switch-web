import fs from "node:fs/promises";
import path from "node:path";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterAll, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("@/lib/api/adapter", async () => {
  const actual = await vi.importActual<typeof import("@/lib/api/adapter")>(
    "@/lib/api/adapter",
  );

  const readUploadBytes = async (value: unknown): Promise<Uint8Array> => {
    if (value instanceof Blob) {
      if (typeof (value as Blob & { arrayBuffer?: unknown }).arrayBuffer === "function") {
        return new Uint8Array(await (value as Blob).arrayBuffer());
      }
      return await new Promise<Uint8Array>((resolve, reject) => {
        const reader = new FileReader();
        reader.onerror = () => reject(reader.error ?? new Error("failed to read blob"));
        reader.onload = () => {
          const result = reader.result;
          if (result instanceof ArrayBuffer) {
            resolve(new Uint8Array(result));
            return;
          }
          resolve(new TextEncoder().encode(String(result ?? "")));
        };
        reader.readAsArrayBuffer(value);
      });
    }
    return new Uint8Array(
      await new Response(String(value), {
        headers: { "Content-Type": "text/plain; charset=utf-8" },
      }).arrayBuffer(),
    );
  };

  return {
    ...actual,
    pickWebFile: vi.fn(),
    webUpload: vi.fn(async (uploadPath: string, formData: FormData) => {
      const source = formData.get("file");
      if (source === null) {
        throw new Error("missing upload field");
      }

      let token = actual.getCsrfToken();
      if (!token) {
        const csrfResponse = await fetch(
          `${actual.apiBase()}/api/system/csrf-token`,
          {
            credentials: "include",
            headers: { Accept: "application/json" },
          },
        );
        const payload = (await csrfResponse.json()) as { token: string };
        token = payload.token;
        actual.setCsrfToken(token);
      }

      const fileName =
        source instanceof Blob && "name" in source && typeof source.name === "string"
          ? source.name
          : "upload.sql";
      const contentType =
        source instanceof Blob && source.type.length > 0
          ? source.type
          : "application/sql";
      const boundary = `----vitest-config-import-${Date.now()}`;
      const body = Buffer.concat([
        Buffer.from(
          `--${boundary}\r\nContent-Disposition: form-data; name="file"; filename="${fileName}"\r\nContent-Type: ${contentType}\r\n\r\n`,
          "utf8",
        ),
        Buffer.from(await readUploadBytes(source)),
        Buffer.from(`\r\n--${boundary}--\r\n`, "utf8"),
      ]);

      const response = await fetch(`${actual.apiBase()}${uploadPath}`, {
        method: "POST",
        body,
        credentials: "include",
        headers: {
          Accept: "application/json",
          "Content-Type": `multipart/form-data; boundary=${boundary}`,
          ...(token ? { "X-CSRF-Token": token } : {}),
        },
      });
      if (!response.ok) {
        throw new Error(`upload failed: ${response.status}`);
      }
      return await response.json();
    }),
  };
});

import "@/lib/api/web-commands";
import { ImportExportSection } from "@/components/settings/ImportExportSection";
import { pickWebFile, setCsrfToken } from "@/lib/api/adapter";
import { useImportExport } from "@/hooks/useImportExport";
import { providersApi } from "@/lib/api/providers";
import type { Provider } from "@/types";
import { server } from "../msw/server";
import {
  startTestWebServer,
  type TestWebServer,
} from "../helpers/web-server";

const toastSuccessMock = vi.fn();
const toastErrorMock = vi.fn();
const toastWarningMock = vi.fn();

vi.mock("sonner", () => ({
  toast: {
    success: (...args: unknown[]) => toastSuccessMock(...args),
    error: (...args: unknown[]) => toastErrorMock(...args),
    warning: (...args: unknown[]) => toastWarningMock(...args),
  },
}));

const clickedDownloads: Array<{ download: string; href: string }> = [];
let downloadedBlob: Blob | null = null;

const claudeLiveSettingsPath = (homeDir: string): string =>
  path.join(homeDir, ".claude", "settings.json");

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

const getClaudeProviders = async (): Promise<Record<string, Provider>> =>
  await providersApi.getAll("claude");

const getCurrentClaudeProvider = async (): Promise<string> =>
  await providersApi.getCurrent("claude");

const exportSqlSnapshot = async (baseUrl: string): Promise<string> => {
  const response = await fetch(
    new URL("/api/config/export-config-download", baseUrl),
  );
  if (!response.ok) {
    throw new Error(`failed to export SQL snapshot: ${response.status}`);
  }
  return await response.text();
};

function ImportExportSectionHarness() {
  const state = useImportExport();

  return (
    <ImportExportSection
      status={state.status}
      selectedFile={state.selectedFile}
      errorMessage={state.errorMessage}
      backupId={state.backupId}
      isImporting={state.isImporting}
      onSelectFile={state.selectImportFile}
      onImport={state.importConfig}
      onExport={state.exportConfig}
      onClear={state.clearSelection}
    />
  );
}

const renderSection = () => render(<ImportExportSectionHarness />);

describe.sequential("ImportExportSection against real web server", () => {
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
    toastWarningMock.mockReset();
    vi.mocked(pickWebFile).mockReset();
    clickedDownloads.length = 0;
    downloadedBlob = null;

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
    Object.defineProperty(window.URL, "createObjectURL", {
      configurable: true,
      value: vi.fn((blob: Blob) => {
        downloadedBlob = blob;
        return "blob:cc-switch-export";
      }),
    });
    Object.defineProperty(window.URL, "revokeObjectURL", {
      configurable: true,
      value: vi.fn(),
    });
    HTMLAnchorElement.prototype.click = vi.fn(function (this: HTMLAnchorElement) {
      clickedDownloads.push({
        download: this.download,
        href: this.href,
      });
    });
  });

  it(
    "exports SQL to browser download through the rendered section UI",
    async () => {
      renderSection();

      fireEvent.click(
        screen.getByRole("button", { name: "settings.exportConfig" }),
      );

      await waitFor(() => expect(clickedDownloads).toHaveLength(1));
      expect(downloadedBlob).not.toBeNull();
      expect(downloadedBlob?.size).toBeGreaterThan(0);
      expect(clickedDownloads[0]?.href).toBe("blob:cc-switch-export");
      expect(clickedDownloads[0]?.download).toMatch(
        /^cc-switch-export-\d{8}_\d{6}\.sql$/,
      );

      const exportedSql = await downloadedBlob!.text();
      expect(exportedSql).toContain("BEGIN TRANSACTION");
      await waitFor(() =>
        expect(toastSuccessMock).toHaveBeenCalledWith(
          expect.stringContaining(clickedDownloads[0]!.download),
          expect.objectContaining({ closeButton: true }),
        ),
      );
      expect(toastWarningMock).not.toHaveBeenCalled();
      expect(toastErrorMock).not.toHaveBeenCalled();
    },
    20_000,
  );

  it(
    "imports SQL through the rendered section UI and restores the exported provider snapshot",
    async () => {
      const providerA = buildClaudeProvider(
        "import-smoke-a",
        "Import Smoke A",
        "claude-import-a-key",
        "https://claude-import-a.example.com",
        10,
      );
      const providerB = buildClaudeProvider(
        "import-smoke-b",
        "Import Smoke B",
        "claude-import-b-key",
        "https://claude-import-b.example.com",
        20,
      );
      const providerC = buildClaudeProvider(
        "import-smoke-c",
        "Import Smoke C",
        "claude-import-c-key",
        "https://claude-import-c.example.com",
        30,
      );

      await providersApi.add(providerA, "claude", false);
      await providersApi.add(providerB, "claude", false);
      await providersApi.switch(providerA.id, "claude");

      const snapshotA = await getClaudeProviders();
      const currentA = await getCurrentClaudeProvider();
      expect(currentA).toBe(providerA.id);
      expect(snapshotA[providerA.id]?.settingsConfig?.env?.ANTHROPIC_AUTH_TOKEN).toBe(
        "claude-import-a-key",
      );

      const exportedSql = await exportSqlSnapshot(webServer.baseUrl);
      const importFile = new File([exportedSql], "config-snapshot-a.sql", {
        type: "application/sql",
      });

      await providersApi.add(providerC, "claude", false);
      await providersApi.switch(providerC.id, "claude");
      await providersApi.delete(providerA.id, "claude");

      const mutatedProviders = await getClaudeProviders();
      expect(mutatedProviders[providerA.id]).toBeUndefined();
      expect(mutatedProviders[providerC.id]?.name).toBe("Import Smoke C");
      expect(await getCurrentClaudeProvider()).toBe(providerC.id);

      vi.mocked(pickWebFile).mockResolvedValue(importFile);

      renderSection();

      fireEvent.click(
        screen.getByRole("button", { name: /settings\.selectConfigFile/ }),
      );

      await waitFor(() =>
        expect(screen.getByText(/config-snapshot-a\.sql/)).toBeInTheDocument(),
      );

      fireEvent.click(screen.getByRole("button", { name: /settings\.import/ }));

      await waitFor(async () => {
        expect(await getClaudeProviders()).toEqual(snapshotA);
      });
      await waitFor(async () => {
        expect(await getCurrentClaudeProvider()).toBe(currentA);
      });
      await waitFor(async () => {
        const liveSettings = JSON.parse(
          await fs.readFile(claudeLiveSettingsPath(webServer.homeDir), "utf8"),
        ) as {
          env?: { ANTHROPIC_AUTH_TOKEN?: string; ANTHROPIC_BASE_URL?: string };
        };
        expect(liveSettings.env?.ANTHROPIC_AUTH_TOKEN).toBe(
          "claude-import-a-key",
        );
        expect(liveSettings.env?.ANTHROPIC_BASE_URL).toBe(
          "https://claude-import-a.example.com",
        );
      });

      expect(await screen.findByText("settings.importSuccess")).toBeInTheDocument();
      expect(screen.getByText(/config-snapshot-a\.sql/)).toBeInTheDocument();
      expect(screen.getByText(/backup/i)).toBeInTheDocument();
      await waitFor(() => expect(toastSuccessMock).toHaveBeenCalled());
      expect(toastWarningMock).not.toHaveBeenCalled();
      expect(toastErrorMock).not.toHaveBeenCalled();
    },
    30_000,
  );
});
