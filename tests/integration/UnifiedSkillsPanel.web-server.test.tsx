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
import {
  afterAll,
  beforeAll,
  beforeEach,
  describe,
  expect,
  it,
  vi,
} from "vitest";

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
          : "skills.zip";
      const contentType =
        source instanceof Blob && source.type.length > 0
          ? source.type
          : "application/zip";
      const boundary = `----vitest-skills-zip-${Date.now()}`;
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
import UnifiedSkillsPanel, {
  type UnifiedSkillsPanelHandle,
} from "@/components/skills/UnifiedSkillsPanel";
import { SKILLS_APP_IDS } from "@/config/appConfig";
import { pickWebFile, setCsrfToken } from "@/lib/api/adapter";
import type { InstalledSkill, SkillBackupEntry } from "@/lib/api/skills";
import { server } from "../msw/server";
import {
  startTestWebServer,
  type TestWebServer,
} from "../helpers/web-server";

const toastSuccessMock = vi.fn();
const toastErrorMock = vi.fn();
const toastInfoMock = vi.fn();

const smokeSkill = {
  directory: "page-smoke-shared-skill",
  name: "Page Smoke Shared Skill",
  description: "Imported from live app skill directories",
  id: "local:page-smoke-shared-skill",
} as const;

const zipSmokeSkill = {
  directory: "page-smoke-zip-skill",
  name: "Page Smoke ZIP Skill",
  description: "Installed from ZIP web-server smoke test",
  id: "local:page-smoke-zip-skill",
} as const;

const zipSmokeArchiveBase64 =
  "UEsDBAoAAAAAAOK1pFwAAAAAAAAAAAAAAAAVABwAcGFnZS1zbW9rZS16aXAtc2tpbGwvVVQJAANnsfhpZ7H4aXV4CwABBOgDAAAE6AMAAFBLAwQUAAAACADitaRc3q6WpFcAAABxAAAAHQAcAHBhZ2Utc21va2UtemlwLXNraWxsL1NLSUxMLm1kVVQJAANnsfhpZ7H4aXV4CwABBOgDAAAE6AMAAG3MOw6AIBBF0Z5VTGI9G2AHdCR0dihPQ/gZhuj2NdT29x5mVtUXaLL+BLnSEmg1llyKOasA2Xu8RmxVk6kyfM4IdPRWZvVgY0G/0UnmOiBD8Yeq5V98AVBLAQIeAwoAAAAAAOK1pFwAAAAAAAAAAAAAAAAVABgAAAAAAAAAEAD9QQAAAABwYWdlLXNtb2tlLXppcC1za2lsbC9VVAUAA2ex+Gl1eAsAAQToAwAABOgDAABQSwECHgMUAAAACADitaRc3q6WpFcAAABxAAAAHQAYAAAAAAABAAAAtIFPAAAAcGFnZS1zbW9rZS16aXAtc2tpbGwvU0tJTEwubWRVVAUAA2ex+Gl1eAsAAQToAwAABOgDAABQSwUGAAAAAAIAAgC+AAAA/QAAAAAA";

vi.mock("sonner", () => ({
  toast: {
    success: (...args: unknown[]) => toastSuccessMock(...args),
    error: (...args: unknown[]) => toastErrorMock(...args),
    info: (...args: unknown[]) => toastInfoMock(...args),
  },
}));

async function writeFixtureFile(
  filePath: string,
  content: string,
): Promise<void> {
  await fs.mkdir(path.dirname(filePath), { recursive: true });
  await fs.writeFile(filePath, content, "utf8");
}

async function seedUnmanagedSkillFixtures(homeDir: string): Promise<void> {
  const content = [
    "---",
    `name: ${smokeSkill.name}`,
    `description: ${smokeSkill.description}`,
    "---",
    "",
  ].join("\n");

  await writeFixtureFile(
    path.join(
      homeDir,
      ".claude",
      "skills",
      smokeSkill.directory,
      "SKILL.md",
    ),
    content,
  );
  await writeFixtureFile(
    path.join(
      homeDir,
      ".codex",
      "skills",
      smokeSkill.directory,
      "SKILL.md",
    ),
    content,
  );
}

const renderPanel = () => {
  const client = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  });
  const ref = createRef<UnifiedSkillsPanelHandle>();

  render(
    <QueryClientProvider client={client}>
      <UnifiedSkillsPanel
        ref={ref}
        onOpenDiscovery={vi.fn()}
        currentApp="claude"
      />
    </QueryClientProvider>,
  );

  return { ref };
};

const getInstalledSkills = async (baseUrl: string): Promise<InstalledSkill[]> => {
  const response = await fetch(new URL("/api/skills/get-installed-skills", baseUrl));
  if (!response.ok) {
    throw new Error(`failed to load skills: ${response.status}`);
  }
  return (await response.json()) as InstalledSkill[];
};

const getSkillBackups = async (
  baseUrl: string,
): Promise<SkillBackupEntry[]> => {
  const response = await fetch(new URL("/api/backups/get-skill-backups", baseUrl));
  if (!response.ok) {
    throw new Error(`failed to load skill backups: ${response.status}`);
  }
  return (await response.json()) as SkillBackupEntry[];
};

const getInstalledRow = (name: string): HTMLElement => {
  const label = screen.getByText(name);
  let current: HTMLElement | null = label;

  while (current && !current.classList.contains("group")) {
    current = current.parentElement;
  }

  if (!(current instanceof HTMLElement)) {
    throw new Error(`could not locate skill row for ${name}`);
  }

  return current;
};

const getAppToggleButton = (
  row: HTMLElement,
  appId: (typeof SKILLS_APP_IDS)[number],
): HTMLButtonElement => {
  const buttons = Array.from(row.querySelectorAll("button")).filter(
    (button) => !button.getAttribute("title"),
  );
  const index = SKILLS_APP_IDS.indexOf(appId);
  const button = buttons[index];

  if (!(button instanceof HTMLButtonElement)) {
    throw new Error(`could not locate skill toggle button for ${appId}`);
  }

  return button;
};

const createZipUploadFile = (): File =>
  new File(
    [Buffer.from(zipSmokeArchiveBase64, "base64")],
    "page-smoke-zip-skill.zip",
    { type: "application/zip" },
  );

describe.sequential("UnifiedSkillsPanel against real web server", () => {
  let webServer: TestWebServer;

  beforeAll(async () => {
    server.close();
    webServer = await startTestWebServer();
    await seedUnmanagedSkillFixtures(webServer.homeDir);
  }, 360_000);

  afterAll(async () => {
    await webServer.stop();
    server.listen({ onUnhandledRequest: "warn" });
  }, 20_000);

  beforeEach(() => {
    toastSuccessMock.mockReset();
    toastErrorMock.mockReset();
    toastInfoMock.mockReset();
    vi.mocked(pickWebFile).mockReset();
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
    "installs a skill ZIP through the rendered panel handle and writes SSOT/live files",
    async () => {
      const { ref } = renderPanel();
      const zipFile = createZipUploadFile();
      vi.mocked(pickWebFile).mockResolvedValue(zipFile);

      const initialInstalled = await getInstalledSkills(webServer.baseUrl);

      await act(async () => {
        await ref.current?.openInstallFromZip();
      });

      await waitFor(async () => {
        const installed = await getInstalledSkills(webServer.baseUrl);
        expect(installed).toHaveLength(initialInstalled.length + 1);
        const zipSkill = installed.find((skill) => skill.id === zipSmokeSkill.id);
        expect(zipSkill?.name).toBe(zipSmokeSkill.name);
        expect(zipSkill?.directory).toBe(zipSmokeSkill.directory);
        expect(zipSkill?.apps.claude).toBe(true);
        expect(zipSkill?.apps.codex).toBe(false);
      });
      await waitFor(async () => {
        const ssotSkill = await fs.readFile(
          path.join(
            webServer.dataDir,
            "skills",
            zipSmokeSkill.directory,
            "SKILL.md",
          ),
          "utf8",
        );
        expect(ssotSkill).toContain(zipSmokeSkill.name);
      });
      await waitFor(async () => {
        const liveSkill = await fs.readFile(
          path.join(
            webServer.homeDir,
            ".claude",
            "skills",
            zipSmokeSkill.directory,
            "SKILL.md",
          ),
          "utf8",
        );
        expect(liveSkill).toContain(zipSmokeSkill.name);
      });

      expect(vi.mocked(pickWebFile)).toHaveBeenCalledWith(
        ".zip,.skill,application/zip",
      );
      await waitFor(() => expect(toastSuccessMock).toHaveBeenCalled());
      expect(toastInfoMock).not.toHaveBeenCalled();
      expect(toastErrorMock).not.toHaveBeenCalled();
    },
    20_000,
  );

  it(
    "imports unmanaged skills, toggles app sync, uninstalls, and restores through the rendered panel UI",
    async () => {
      const { ref } = renderPanel();
      const initialInstalled = await getInstalledSkills(webServer.baseUrl);

      if (initialInstalled.length === 0) {
        await waitFor(async () => {
          expect(await getInstalledSkills(webServer.baseUrl)).toEqual([]);
        });
        expect(screen.getByText("skills.noInstalled")).toBeInTheDocument();
      }

      await act(async () => {
        await ref.current?.openImport();
      });

      await waitFor(() =>
        expect(screen.getByText("skills.import")).toBeInTheDocument(),
      );
      expect(screen.getByText(smokeSkill.name)).toBeInTheDocument();

      fireEvent.click(
        screen.getByRole("button", { name: "skills.importSelected" }),
      );

      let installedSkill: InstalledSkill | undefined;
      await waitFor(async () => {
        const installed = await getInstalledSkills(webServer.baseUrl);
        expect(installed).toHaveLength(initialInstalled.length + 1);
        installedSkill = installed.find((skill) => skill.id === smokeSkill.id);
        expect(installedSkill?.id).toBe(smokeSkill.id);
        expect(installedSkill?.apps.claude).toBe(true);
        expect(installedSkill?.apps.codex).toBe(true);
      });
      await waitFor(async () => {
        const ssotSkill = await fs.readFile(
          path.join(
            webServer.dataDir,
            "skills",
            smokeSkill.directory,
            "SKILL.md",
          ),
          "utf8",
        );
        expect(ssotSkill).toContain(smokeSkill.name);
      });

      const installedRow = await screen.findByText(smokeSkill.name);
      expect(installedRow).toBeInTheDocument();

      const row = getInstalledRow(smokeSkill.name);
      fireEvent.click(getAppToggleButton(row, "codex"));

      await waitFor(async () => {
        const installed = await getInstalledSkills(webServer.baseUrl);
        expect(installed[0]?.apps.codex).toBe(false);
      });
      await waitFor(async () => {
        await expect(
          fs.access(
            path.join(
              webServer.homeDir,
              ".codex",
              "skills",
              smokeSkill.directory,
              "SKILL.md",
            ),
          ),
        ).rejects.toThrow();
      });

      fireEvent.click(within(row).getByTitle("skills.uninstall"));
      await screen.findByText("skills.uninstall");
      fireEvent.click(screen.getByRole("button", { name: "common.confirm" }));

      let backup: SkillBackupEntry | undefined;
      await waitFor(async () => {
        const installed = await getInstalledSkills(webServer.baseUrl);
        expect(installed).toHaveLength(initialInstalled.length);
        expect(installed.some((skill) => skill.id === smokeSkill.id)).toBe(false);
        const backups = await getSkillBackups(webServer.baseUrl);
        backup = backups.find((entry) => entry.skill.directory === smokeSkill.directory);
        expect(backup).toBeDefined();
        expect(backup?.skill.directory).toBe(smokeSkill.directory);
      });
      await waitFor(async () => {
        await expect(
          fs.access(
            path.join(webServer.dataDir, "skills", smokeSkill.directory),
          ),
        ).rejects.toThrow();
      });
      await waitFor(async () => {
        await expect(
          fs.access(
            path.join(webServer.homeDir, ".claude", "skills", smokeSkill.directory),
          ),
        ).rejects.toThrow();
      });

      await act(async () => {
        await ref.current?.openRestoreFromBackup();
      });

      await waitFor(() =>
        expect(
          screen.getByText("skills.restoreFromBackup.title"),
        ).toBeInTheDocument(),
      );

      expect(screen.getByText(smokeSkill.directory)).toBeInTheDocument();
      fireEvent.click(
        screen.getByRole("button", {
          name: "skills.restoreFromBackup.restore",
        }),
      );

      await waitFor(async () => {
        const installed = await getInstalledSkills(webServer.baseUrl);
        expect(installed).toHaveLength(initialInstalled.length + 1);
        const restored = installed.find((skill) => skill.id === smokeSkill.id);
        expect(restored?.id).toBe(smokeSkill.id);
        expect(restored?.apps.claude).toBe(true);
        expect(restored?.apps.codex).toBe(false);
      });
      await waitFor(async () => {
        const restoredSkill = await fs.readFile(
          path.join(
            webServer.homeDir,
            ".claude",
            "skills",
            smokeSkill.directory,
            "SKILL.md",
          ),
          "utf8",
        );
        expect(restoredSkill).toContain(smokeSkill.name);
      });
      await waitFor(async () => {
        await expect(
          fs.access(
            path.join(
              webServer.homeDir,
              ".codex",
              "skills",
              smokeSkill.directory,
            ),
          ),
        ).rejects.toThrow();
      });

      expect(toastErrorMock).not.toHaveBeenCalled();
    },
    20_000,
  );
});
