import fs from "node:fs/promises";
import http from "node:http";
import path from "node:path";
import { AddressInfo } from "node:net";
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

import "@/lib/api/web-commands";
import {
  SkillsPage,
  type SkillsPageHandle,
} from "@/components/skills/SkillsPage";
import UnifiedSkillsPanel, {
  type UnifiedSkillsPanelHandle,
} from "@/components/skills/UnifiedSkillsPanel";
import { setCsrfToken } from "@/lib/api/adapter";
import type { InstalledSkill, SkillRepo } from "@/lib/api/skills";
import { server } from "../msw/server";
import { startTestWebServer, type TestWebServer } from "../helpers/web-server";

const toastSuccessMock = vi.fn();
const toastErrorMock = vi.fn();
const toastInfoMock = vi.fn();

const repoSmokeSkill = {
  owner: "demo",
  repo: "skills",
  branch: "main",
  name: "Repo Smoke Skill",
  installName: "repo-smoke-skill",
  descriptionV1: "Repository smoke skill v1",
  descriptionV2: "Repository smoke skill v2 updated",
} as const;

const skillsShFixtures = Array.from({ length: 21 }, (_, index) => {
  if (index === 0) {
    return {
      id: "repo-smoke-skill",
      skillId: repoSmokeSkill.installName,
      name: repoSmokeSkill.name,
      installs: 42,
      source: `${repoSmokeSkill.owner}/${repoSmokeSkill.repo}`,
    };
  }

  if (index === 20) {
    return {
      id: "tail-smoke-skill",
      skillId: "tail-smoke-skill",
      name: "Tail Smoke Skill",
      installs: 7,
      source: `${repoSmokeSkill.owner}/${repoSmokeSkill.repo}`,
    };
  }

  return {
    id: `search-smoke-skill-${index}`,
    skillId: `search-smoke-skill-${index}`,
    name: `Search Smoke Skill ${index}`,
    installs: 10 + index,
    source: `${repoSmokeSkill.owner}/${repoSmokeSkill.repo}`,
  };
});

const repoSmokeArchives = {
  v1: "UEsDBBQAAAAIAHu4pFyVMdROQgAAAFoAAAAqAAAAZGVtby1za2lsbHMtbWFpbi9yZXBvLXNtb2tlLXNraWxsL1NLSUxMLm1k09XV5cpLzE21UghKLchXCM7Nz05VCM7OzMnhSkktTi7KLCjJzM+DyBZnluQXVSoUg9UUg9QolBly6QJN4FLG1A4AUEsBAhQDFAAAAAgAe7ikXJUx1E5CAAAAWgAAACoAAAAAAAAAAAAAAIABAAAAAGRlbW8tc2tpbGxzLW1haW4vcmVwby1zbW9rZS1za2lsbC9TS0lMTC5tZFBLBQYAAAAAAQABAFgAAACKAAAAAAA=",
  v2: "UEsDBBQAAAAIAHu4pFxjqSZgSgAAAGIAAAAqAAAAZGVtby1za2lsbHMtbWFpbi9yZXBvLXNtb2tlLXNraWxsL1NLSUxMLm1k09XV5cpLzE21UghKLchXCM7Nz05VCM7OzMnhSkktTi7KLCjJzM+DyBZnluQXVSoUg9UUg9QolBkplBakJJakpnDpAk3iUsY0BgBQSwECFAMUAAAACAB7uKRcY6kmYEoAAABiAAAAKgAAAAAAAAAAAAAAgAEAAAAAZGVtby1za2lsbHMtbWFpbi9yZXBvLXNtb2tlLXNraWxsL1NLSUxMLm1kUEsFBgAAAAABAAEAWAAAAJIAAAAAAA==",
} as const;

vi.mock("sonner", () => ({
  toast: {
    success: (...args: unknown[]) => toastSuccessMock(...args),
    error: (...args: unknown[]) => toastErrorMock(...args),
    info: (...args: unknown[]) => toastInfoMock(...args),
  },
}));

type RepoArchiveServer = {
  baseUrl: string;
  setVersion: (version: keyof typeof repoSmokeArchives) => void;
  setSearchFailure: (message: string | null) => void;
  getSearchRequestCount: () => number;
  resetSearchRequestCount: () => void;
  stop: () => Promise<void>;
};

async function startRepoArchiveServer(): Promise<RepoArchiveServer> {
  let currentArchive: string = repoSmokeArchives.v1;
  let searchFailureMessage: string | null = null;
  let searchRequestCount = 0;

  const server = http.createServer((req, res) => {
    const requestUrl = new URL(req.url ?? "/", "http://127.0.0.1");

    if (req.method === "GET" && requestUrl.pathname === "/api/search") {
      searchRequestCount += 1;
      if (searchFailureMessage) {
        res.statusCode = 503;
        res.setHeader("Content-Type", "application/json; charset=utf-8");
        res.end(JSON.stringify({ error: searchFailureMessage }));
        return;
      }

      const query = requestUrl.searchParams.get("q") ?? "";
      const limit = Number.parseInt(
        requestUrl.searchParams.get("limit") ?? "20",
        10,
      );
      const offset = Number.parseInt(
        requestUrl.searchParams.get("offset") ?? "0",
        10,
      );
      const noMatch = query.trim().toLowerCase() === "no-match";
      const skills =
        query.trim().length >= 2 && !noMatch
          ? skillsShFixtures.slice(offset, offset + limit)
          : [];

      res.statusCode = 200;
      res.setHeader("Content-Type", "application/json; charset=utf-8");
      res.end(
        JSON.stringify({
          query,
          searchType: "fulltext",
          skills,
          count: noMatch ? 0 : skillsShFixtures.length,
          duration_ms: 1,
        }),
      );
      return;
    }

    if (
      req.method === "GET" &&
      requestUrl.pathname ===
        `/${repoSmokeSkill.owner}/${repoSmokeSkill.repo}/archive/refs/heads/${repoSmokeSkill.branch}.zip`
    ) {
      res.statusCode = 200;
      res.setHeader("Content-Type", "application/zip");
      res.end(Buffer.from(currentArchive, "base64"));
      return;
    }

    res.statusCode = 404;
    res.setHeader("Content-Type", "text/plain; charset=utf-8");
    res.end("not found");
  });

  await new Promise<void>((resolve, reject) => {
    server.once("error", reject);
    server.listen(0, "127.0.0.1", () => resolve());
  });

  const address = server.address();
  if (!address || typeof address === "string") {
    throw new Error("failed to start repo archive server");
  }

  return {
    baseUrl: `http://127.0.0.1:${(address as AddressInfo).port}`,
    setVersion: (version) => {
      currentArchive = repoSmokeArchives[version];
    },
    setSearchFailure: (message) => {
      searchFailureMessage = message;
    },
    getSearchRequestCount: () => searchRequestCount,
    resetSearchRequestCount: () => {
      searchRequestCount = 0;
    },
    stop: async () => {
      await new Promise<void>((resolve, reject) => {
        server.close((error) => {
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

function renderSkillsViews() {
  const client = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  });
  const skillsPageRef = createRef<SkillsPageHandle>();
  const unifiedPanelRef = createRef<UnifiedSkillsPanelHandle>();

  render(
    <QueryClientProvider client={client}>
      <div>
        <SkillsPage ref={skillsPageRef} initialApp="claude" />
        <UnifiedSkillsPanel
          ref={unifiedPanelRef}
          currentApp="claude"
          onOpenDiscovery={vi.fn()}
        />
      </div>
    </QueryClientProvider>,
  );

  return { skillsPageRef, unifiedPanelRef };
}

async function getInstalledSkills(baseUrl: string): Promise<InstalledSkill[]> {
  const response = await fetch(
    new URL("/api/skills/get-installed-skills", baseUrl),
  );
  if (!response.ok) {
    throw new Error(`failed to load installed skills: ${response.status}`);
  }
  return (await response.json()) as InstalledSkill[];
}

async function getSkillRepos(baseUrl: string): Promise<SkillRepo[]> {
  const response = await fetch(new URL("/api/skills/get-skill-repos", baseUrl));
  if (!response.ok) {
    throw new Error(`failed to load skill repos: ${response.status}`);
  }
  return (await response.json()) as SkillRepo[];
}

function getInstalledRow(name: string): HTMLElement {
  for (const label of screen.getAllByText(name)) {
    let current: HTMLElement | null = label.parentElement;
    while (current) {
      if (
        current.classList.contains("group") &&
        current.querySelector('button[title="skills.uninstall"]')
      ) {
        return current;
      }
      current = current.parentElement;
    }
  }

  throw new Error(`could not locate installed skill row for ${name}`);
}

describe.sequential(
  "Skills discovery and updates against real web server",
  () => {
    let repoArchiveServer: RepoArchiveServer;
    let webServer: TestWebServer;

    beforeAll(async () => {
      server.close();
      repoArchiveServer = await startRepoArchiveServer();
      webServer = await startTestWebServer({
        env: {
          CC_SWITCH_SKILL_ARCHIVE_BASE_URL: repoArchiveServer.baseUrl,
          CC_SWITCH_SKILL_DOC_BASE_URL: repoArchiveServer.baseUrl,
          CC_SWITCH_SKILLS_SH_API_BASE_URL: repoArchiveServer.baseUrl,
        },
      });
    }, 360_000);

    afterAll(async () => {
      await webServer.stop();
      await repoArchiveServer.stop();
      server.listen({ onUnhandledRequest: "warn" });
    }, 20_000);

    beforeEach(() => {
      repoArchiveServer.setVersion("v1");
      repoArchiveServer.setSearchFailure(null);
      repoArchiveServer.resetSearchRequestCount();
      toastSuccessMock.mockReset();
      toastErrorMock.mockReset();
      toastInfoMock.mockReset();
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

    it("renders automatic skills.sh fallback and the repo empty state when no repos are configured", async () => {
      await expect(getSkillRepos(webServer.baseUrl)).resolves.toEqual([]);

      renderSkillsViews();

      await waitFor(() => {
        expect(
          screen.getByPlaceholderText("skills.skillssh.searchPlaceholder"),
        ).toBeInTheDocument();
      });
      expect(
        screen.getByText("skills.skillssh.searchPlaceholder"),
      ).toBeInTheDocument();
      expect(repoArchiveServer.getSearchRequestCount()).toBe(0);

      fireEvent.click(
        screen.getByRole("button", { name: "skills.searchSource.repos" }),
      );

      await waitFor(() => {
        expect(screen.getByText("skills.empty")).toBeInTheDocument();
      });
      expect(screen.getByText("skills.emptyDescription")).toBeInTheDocument();

      fireEvent.click(screen.getByRole("button", { name: "skills.addRepo" }));

      await waitFor(() => {
        expect(screen.getByLabelText("skills.repo.url")).toBeInTheDocument();
      });
      expect(toastErrorMock).not.toHaveBeenCalled();
    }, 30_000);

    it("installs a repo skill through SkillsPage, links installed state to UnifiedSkillsPanel, and updates it through the rendered UI", async () => {
      const { skillsPageRef } = renderSkillsViews();

      await act(async () => {
        skillsPageRef.current?.openRepoManager();
      });

      fireEvent.change(await screen.findByLabelText("skills.repo.url"), {
        target: { value: `${repoSmokeSkill.owner}/${repoSmokeSkill.repo}` },
      });
      fireEvent.change(screen.getByLabelText("skills.repo.branch"), {
        target: { value: repoSmokeSkill.branch },
      });
      fireEvent.click(screen.getByRole("button", { name: "skills.repo.add" }));

      await waitFor(() => {
        expect(toastSuccessMock).toHaveBeenCalled();
      });

      fireEvent.keyDown(window, { key: "Escape" });

      await waitFor(() => {
        expect(
          screen.queryByLabelText("skills.repo.url"),
        ).not.toBeInTheDocument();
      });
      await waitFor(() => {
        expect(screen.getByText(repoSmokeSkill.name)).toBeInTheDocument();
      });

      fireEvent.click(screen.getByRole("button", { name: "skills.install" }));

      await waitFor(async () => {
        const installed = await getInstalledSkills(webServer.baseUrl);
        const skill = installed.find(
          (entry) => entry.name === repoSmokeSkill.name,
        );
        expect(skill?.directory).toBe(repoSmokeSkill.installName);
        expect(skill?.apps.claude).toBe(true);
        expect(skill?.description).toBe(repoSmokeSkill.descriptionV1);
      });
      await waitFor(async () => {
        const ssotSkill = await fs.readFile(
          path.join(
            webServer.dataDir,
            "skills",
            repoSmokeSkill.installName,
            "SKILL.md",
          ),
          "utf8",
        );
        expect(ssotSkill).toContain(repoSmokeSkill.descriptionV1);
      });
      await waitFor(async () => {
        const liveSkill = await fs.readFile(
          path.join(
            webServer.homeDir,
            ".claude",
            "skills",
            repoSmokeSkill.installName,
            "SKILL.md",
          ),
          "utf8",
        );
        expect(liveSkill).toContain(repoSmokeSkill.descriptionV1);
      });
      await waitFor(() => {
        expect(
          screen.getAllByRole("button", { name: "skills.uninstall" }),
        ).toHaveLength(2);
      });
      await waitFor(() => {
        expect(
          within(getInstalledRow(repoSmokeSkill.name)).getByText(
            repoSmokeSkill.descriptionV1,
          ),
        ).toBeInTheDocument();
      });

      repoArchiveServer.setVersion("v2");

      fireEvent.click(
        screen.getByRole("button", { name: "skills.checkUpdates" }),
      );

      await waitFor(() => {
        expect(
          within(getInstalledRow(repoSmokeSkill.name)).getByText(
            "skills.updateAvailable",
          ),
        ).toBeInTheDocument();
      });

      fireEvent.click(
        within(getInstalledRow(repoSmokeSkill.name)).getByTitle(
          "skills.update",
        ),
      );

      await waitFor(async () => {
        const installed = await getInstalledSkills(webServer.baseUrl);
        const skill = installed.find(
          (entry) => entry.name === repoSmokeSkill.name,
        );
        expect(skill?.description).toBe(repoSmokeSkill.descriptionV2);
        expect(skill?.updatedAt).toBeGreaterThan(0);
      });
      await waitFor(async () => {
        const ssotSkill = await fs.readFile(
          path.join(
            webServer.dataDir,
            "skills",
            repoSmokeSkill.installName,
            "SKILL.md",
          ),
          "utf8",
        );
        expect(ssotSkill).toContain(repoSmokeSkill.descriptionV2);
      });
      await waitFor(async () => {
        const liveSkill = await fs.readFile(
          path.join(
            webServer.homeDir,
            ".claude",
            "skills",
            repoSmokeSkill.installName,
            "SKILL.md",
          ),
          "utf8",
        );
        expect(liveSkill).toContain(repoSmokeSkill.descriptionV2);
      });
      await waitFor(() => {
        const row = getInstalledRow(repoSmokeSkill.name);
        expect(
          within(row).getByText(repoSmokeSkill.descriptionV2),
        ).toBeInTheDocument();
        expect(
          within(row).queryByText("skills.updateAvailable"),
        ).not.toBeInTheDocument();
      });

      fireEvent.click(
        within(getInstalledRow(repoSmokeSkill.name)).getByTitle(
          "skills.uninstall",
        ),
      );
      fireEvent.click(
        await screen.findByRole("button", { name: "common.confirm" }),
      );

      await waitFor(async () => {
        const installed = await getInstalledSkills(webServer.baseUrl);
        expect(
          installed.some((entry) => entry.name === repoSmokeSkill.name),
        ).toBe(false);
      });
      await waitFor(() => {
        expect(
          screen.getByRole("button", { name: "skills.install" }),
        ).toBeInTheDocument();
      });

      expect(toastErrorMock).not.toHaveBeenCalled();
    }, 30_000);

    it("searches skills.sh with pagination and installs a result through the rendered UI", async () => {
      renderSkillsViews();

      fireEvent.click(screen.getByRole("button", { name: "skills.sh" }));
      fireEvent.change(
        screen.getByPlaceholderText("skills.skillssh.searchPlaceholder"),
        {
          target: { value: "repo" },
        },
      );
      fireEvent.click(screen.getByRole("button", { name: "skills.search" }));

      await waitFor(() => {
        expect(screen.getByText(repoSmokeSkill.name)).toBeInTheDocument();
      });
      expect(screen.getByText("42")).toBeInTheDocument();
      expect(
        screen.getByRole("button", { name: "skills.skillssh.loadMore" }),
      ).toBeInTheDocument();

      fireEvent.click(
        screen.getByRole("button", { name: "skills.skillssh.loadMore" }),
      );

      await waitFor(() => {
        expect(screen.getByText("Tail Smoke Skill")).toBeInTheDocument();
      });

      fireEvent.click(
        screen.getAllByRole("button", { name: "skills.install" })[0],
      );

      await waitFor(async () => {
        const installed = await getInstalledSkills(webServer.baseUrl);
        const skill = installed.find(
          (entry) => entry.name === repoSmokeSkill.name,
        );
        expect(skill?.directory).toBe(repoSmokeSkill.installName);
        expect(skill?.apps.claude).toBe(true);
      });
      await waitFor(() => {
        expect(getInstalledRow(repoSmokeSkill.name)).toBeInstanceOf(
          HTMLElement,
        );
      });

      expect(toastErrorMock).not.toHaveBeenCalled();
    }, 30_000);

    it("renders an empty skills.sh result state through the real web server", async () => {
      renderSkillsViews();

      fireEvent.click(screen.getByRole("button", { name: "skills.sh" }));
      fireEvent.change(
        screen.getByPlaceholderText("skills.skillssh.searchPlaceholder"),
        {
          target: { value: "no-match" },
        },
      );
      fireEvent.click(screen.getByRole("button", { name: "skills.search" }));

      await waitFor(() => {
        expect(
          screen.getByText("skills.skillssh.noResults"),
        ).toBeInTheDocument();
      });
      expect(
        screen.queryByRole("button", { name: "skills.skillssh.loadMore" }),
      ).not.toBeInTheDocument();
      expect(screen.queryByText("Search Smoke Skill 1")).not.toBeInTheDocument();
      expect(repoArchiveServer.getSearchRequestCount()).toBeGreaterThan(0);
      expect(toastErrorMock).not.toHaveBeenCalled();
    }, 30_000);

    it("trims skills.sh queries before fetching through the rendered UI", async () => {
      renderSkillsViews();

      fireEvent.click(screen.getByRole("button", { name: "skills.sh" }));
      fireEvent.change(
        screen.getByPlaceholderText("skills.skillssh.searchPlaceholder"),
        {
          target: { value: "  trim-smoke  " },
        },
      );
      fireEvent.click(screen.getByRole("button", { name: "skills.search" }));

      await waitFor(() => {
        expect(repoArchiveServer.getSearchRequestCount()).toBeGreaterThan(0);
      });
      await waitFor(() => {
        expect(screen.getAllByText(repoSmokeSkill.name).length).toBeGreaterThan(
          0,
        );
      });
      expect(screen.getAllByText(repoSmokeSkill.name).length).toBeGreaterThan(0);
      expect(screen.queryByText("skills.skillssh.noResults")).not.toBeInTheDocument();
      expect(toastErrorMock).not.toHaveBeenCalled();
    }, 30_000);

    it("shows a skills.sh search error from the rendered web page and retries the same query", async () => {
      repoArchiveServer.setSearchFailure("skills.sh temporary outage");
      renderSkillsViews();

      fireEvent.click(screen.getByRole("button", { name: "skills.sh" }));
      fireEvent.change(
        screen.getByPlaceholderText("skills.skillssh.searchPlaceholder"),
        {
          target: { value: "repo" },
        },
      );
      fireEvent.click(screen.getByRole("button", { name: "skills.search" }));

      await waitFor(() => {
        expect(
          screen.getAllByText("skills.skillssh.error").length,
        ).toBeGreaterThan(0);
      });
      expect(screen.getByText(/503|Service Unavailable/i)).toBeInTheDocument();
      const failedRequestCount = repoArchiveServer.getSearchRequestCount();
      expect(failedRequestCount).toBeGreaterThan(0);

      repoArchiveServer.setSearchFailure(null);
      const errorBanner = screen
        .getByText(/503|Service Unavailable/i)
        .closest(".rounded-xl");
      expect(errorBanner).toBeInstanceOf(HTMLElement);
      fireEvent.click(
        within(errorBanner as HTMLElement).getByRole("button", {
          name: "common.refresh",
        }),
      );

      await waitFor(() => {
        expect(
          repoArchiveServer.getSearchRequestCount(),
        ).toBeGreaterThan(failedRequestCount);
      });
      expect(screen.getByText("Search Smoke Skill 1")).toBeInTheDocument();
      expect(
        screen.queryByText(/503|Service Unavailable/i),
      ).not.toBeInTheDocument();
      expect(toastErrorMock).not.toHaveBeenCalled();
    }, 30_000);
  },
);
