import { createRef } from "react";
import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import {
  SkillsPage,
  type SkillsPageHandle,
} from "@/components/skills/SkillsPage";

const toastSuccessMock = vi.fn();
const toastErrorMock = vi.fn();
const toastInfoMock = vi.fn();

const installSkillMock = vi.fn();
const addRepoMock = vi.fn();
const removeRepoMock = vi.fn();
const refetchDiscoverableMock = vi.fn();
const refetchReposMock = vi.fn();
const searchSkillsShHookMock = vi.fn();
const refetchSkillsShMock = vi.fn();
const formatSkillErrorMock = vi.fn(
  (_error: string, _t: any, _key: string) => ({
    title: "skills.installFailed",
    description: "install failed",
  }),
);

const emptySkillsShState = {
  data: undefined,
  isLoading: false,
  isFetching: false,
  error: null,
  refetch: refetchSkillsShMock,
};

let discoverableSkillsFixture: any[] = [];
let installedSkillsFixture: any[] = [];
let reposFixture: any[] = [];

vi.mock("sonner", () => ({
  toast: {
    success: (...args: unknown[]) => toastSuccessMock(...args),
    error: (...args: unknown[]) => toastErrorMock(...args),
    info: (...args: unknown[]) => toastInfoMock(...args),
  },
}));

vi.mock("@/hooks/useSkills", () => ({
  useDiscoverableSkills: () => ({
    data: discoverableSkillsFixture,
    isLoading: false,
    isFetching: false,
    refetch: refetchDiscoverableMock,
  }),
  useInstalledSkills: () => ({
    data: installedSkillsFixture,
  }),
  useInstallSkill: () => ({
    mutateAsync: installSkillMock,
  }),
  useSkillRepos: () => ({
    data: reposFixture,
    refetch: refetchReposMock,
  }),
  useAddSkillRepo: () => ({
    mutateAsync: addRepoMock,
  }),
  useRemoveSkillRepo: () => ({
    mutateAsync: removeRepoMock,
  }),
  useSearchSkillsSh: (...args: unknown[]) => searchSkillsShHookMock(...args),
}));

vi.mock("@/lib/errors/skillErrorParser", () => ({
  formatSkillError: (error: string, t: any, key: string) =>
    formatSkillErrorMock(error, t, key),
}));

vi.mock("@/components/skills/SkillCard", () => ({
  SkillCard: ({ skill, onInstall, onUninstall, installs }: any) => (
    <div data-testid={`skill-card-${skill.directory}`}>
      <span>{skill.name}</span>
      <span>{skill.installed ? "installed" : "uninstalled"}</span>
      {typeof installs === "number" ? <span>{installs}</span> : null}
      <button onClick={() => onInstall(skill.directory)}>
        install-{skill.directory}
      </button>
      <button onClick={() => onUninstall(skill.directory)}>
        uninstall-{skill.directory}
      </button>
    </div>
  ),
}));

vi.mock("@/components/skills/RepoManagerPanel", () => ({
  RepoManagerPanel: ({ onAdd, onRemove, onClose }: any) => (
    <div data-testid="repo-manager">
      <button
        onClick={() => {
          void onAdd({
            owner: "foo",
            name: "bar",
            branch: "main",
            enabled: true,
          }).catch(() => undefined);
        }}
      >
        add-repo
      </button>
      <button
        onClick={() => {
          void onRemove("foo", "bar").catch(() => undefined);
        }}
      >
        remove-repo
      </button>
      <button onClick={onClose}>close-repo-manager</button>
    </div>
  ),
}));

describe("SkillsPage", () => {
  beforeEach(() => {
    discoverableSkillsFixture = [];
    installedSkillsFixture = [];
    reposFixture = [];
    toastSuccessMock.mockReset();
    toastErrorMock.mockReset();
    toastInfoMock.mockReset();
    installSkillMock.mockReset();
    addRepoMock.mockReset();
    removeRepoMock.mockReset();
    refetchDiscoverableMock.mockReset();
    refetchReposMock.mockReset();
    searchSkillsShHookMock.mockReset();
    refetchSkillsShMock.mockReset();
    formatSkillErrorMock.mockClear();
    searchSkillsShHookMock.mockReturnValue(emptySkillsShState);
  });

  it("exposes refresh and repo manager actions through the imperative handle", async () => {
    const ref = createRef<SkillsPageHandle>();
    render(<SkillsPage ref={ref} initialApp="claude" />);

    await act(async () => {
      ref.current?.refresh();
    });

    expect(refetchDiscoverableMock).toHaveBeenCalledTimes(1);
    expect(refetchReposMock).toHaveBeenCalledTimes(1);

    await act(async () => {
      ref.current?.openRepoManager();
    });

    expect(screen.getByTestId("repo-manager")).toBeInTheDocument();
  });

  it("installs discovered repo skills with the current app and shows a success toast", async () => {
    discoverableSkillsFixture = [
      {
        key: "repo-skill:foo:bar",
        name: "Repo Skill",
        description: "A repo-backed skill",
        directory: "repo-skill",
        repoOwner: "foo",
        repoName: "bar",
        repoBranch: "main",
        readmeUrl: "https://example.com/readme",
      },
    ];
    reposFixture = [{ owner: "foo", name: "bar", branch: "main", enabled: true }];
    installSkillMock.mockResolvedValue({
      id: "installed-repo-skill",
      directory: "repo-skill",
    });

    render(<SkillsPage initialApp="codex" />);

    fireEvent.click(screen.getByText("install-repo-skill"));

    await waitFor(() =>
      expect(installSkillMock).toHaveBeenCalledWith({
        skill: discoverableSkillsFixture[0],
        currentApp: "codex",
      }),
    );

    expect(toastSuccessMock).toHaveBeenCalledWith("skills.installSuccess", {
      closeButton: true,
    });
  });

  it("passes extracted structured detail into the install error formatter", async () => {
    discoverableSkillsFixture = [
      {
        key: "repo-skill:foo:bar",
        name: "Repo Skill",
        description: "A repo-backed skill",
        directory: "repo-skill",
        repoOwner: "foo",
        repoName: "bar",
        repoBranch: "main",
        readmeUrl: "https://example.com/readme",
      },
    ];
    reposFixture = [{ owner: "foo", name: "bar", branch: "main", enabled: true }];
    installSkillMock.mockRejectedValueOnce({ detail: "skill install exploded" });

    render(<SkillsPage initialApp="claude" />);

    fireEvent.click(screen.getByText("install-repo-skill"));

    await waitFor(() => {
      expect(formatSkillErrorMock).toHaveBeenCalledWith(
        "skill install exploded",
        expect.any(Function),
        "skills.installFailed",
      );
    });
    expect(toastErrorMock).toHaveBeenCalledWith("skills.installFailed", {
      description: "install failed",
      duration: 10000,
    });
  });

  it("supports skills.sh search and accumulates paginated results", async () => {
    const firstPageState = {
      data: {
        query: "agent",
        skills: [
          {
            key: "agent-one",
            name: "Agent One",
            directory: "agent-one",
            repoOwner: "demo",
            repoName: "skills",
            repoBranch: "main",
            readmeUrl: "https://example.com/agent-one",
            installs: 12,
          },
        ],
        totalCount: 2,
      },
      isLoading: false,
      isFetching: false,
      error: null,
      refetch: refetchSkillsShMock,
    };
    const secondPageState = {
      data: {
        query: "agent",
        skills: [
          {
            key: "agent-two",
            name: "Agent Two",
            directory: "agent-two",
            repoOwner: "demo",
            repoName: "skills",
            repoBranch: "main",
            readmeUrl: "https://example.com/agent-two",
            installs: 6,
          },
        ],
        totalCount: 2,
      },
      isLoading: false,
      isFetching: false,
      error: null,
      refetch: refetchSkillsShMock,
    };

    searchSkillsShHookMock.mockImplementation(
      (query: string, _limit: number, offset: number) => {
        if (query !== "agent") {
          return emptySkillsShState;
        }

        if (offset === 0) {
          return firstPageState;
        }

        return secondPageState;
      },
    );

    render(<SkillsPage initialApp="claude" />);

    const searchInput = screen.getByPlaceholderText(
      "skills.skillssh.searchPlaceholder",
    );
    fireEvent.change(searchInput, { target: { value: "agent" } });
    fireEvent.click(screen.getByRole("button", { name: "skills.search" }));

    await waitFor(() =>
      expect(screen.getByTestId("skill-card-agent-one")).toBeInTheDocument(),
    );

    fireEvent.click(
      screen.getByRole("button", { name: "skills.skillssh.loadMore" }),
    );

    await waitFor(() =>
      expect(screen.getByTestId("skill-card-agent-two")).toBeInTheDocument(),
    );
  });

  it("keeps stale placeholder results hidden while a new skills.sh query is still loading", async () => {
    let resolveNewQuery = false;
    const repoResultsState = {
      data: {
        query: "repo",
        skills: [
          {
            key: "repo-old",
            name: "Repo Old Skill",
            directory: "repo-old",
            repoOwner: "demo",
            repoName: "skills",
            repoBranch: "main",
            readmeUrl: "https://example.com/repo-old",
            installs: 12,
          },
        ],
        totalCount: 1,
      },
      isLoading: false,
      isFetching: false,
      error: null,
      refetch: refetchSkillsShMock,
    };
    const staleOtherResultsState = {
      data: repoResultsState.data,
      isLoading: false,
      isFetching: true,
      error: null,
      refetch: refetchSkillsShMock,
    };
    const freshOtherResultsState = {
      data: {
        query: "other",
        skills: [
          {
            key: "other-new",
            name: "Other New Skill",
            directory: "other-new",
            repoOwner: "demo",
            repoName: "skills",
            repoBranch: "main",
            readmeUrl: "https://example.com/other-new",
            installs: 7,
          },
        ],
        totalCount: 1,
      },
      isLoading: false,
      isFetching: false,
      error: null,
      refetch: refetchSkillsShMock,
    };

    searchSkillsShHookMock.mockImplementation((query: string) => {
      if (query === "repo") {
        return repoResultsState;
      }

      if (query === "other") {
        if (!resolveNewQuery) {
          return staleOtherResultsState;
        }

        return freshOtherResultsState;
      }

      return emptySkillsShState;
    });

    const { rerender } = render(<SkillsPage initialApp="claude" />);

    const searchInput = screen.getByPlaceholderText(
      "skills.skillssh.searchPlaceholder",
    );
    fireEvent.change(searchInput, { target: { value: "repo" } });
    fireEvent.click(screen.getByRole("button", { name: "skills.search" }));

    await waitFor(() =>
      expect(screen.getByTestId("skill-card-repo-old")).toBeInTheDocument(),
    );

    fireEvent.change(searchInput, { target: { value: "other" } });
    fireEvent.click(screen.getByRole("button", { name: "skills.search" }));

    await waitFor(() =>
      expect(screen.getByText("skills.skillssh.loading")).toBeInTheDocument(),
    );
    expect(screen.queryByTestId("skill-card-repo-old")).not.toBeInTheDocument();
    expect(screen.queryByText("skills.skillssh.noResults")).not.toBeInTheDocument();

    resolveNewQuery = true;
    rerender(<SkillsPage initialApp="claude" />);

    await waitFor(() =>
      expect(screen.getByTestId("skill-card-other-new")).toBeInTheDocument(),
    );
    expect(screen.queryByTestId("skill-card-repo-old")).not.toBeInTheDocument();
  });

  it("clears prior skills.sh results when the input is shortened below the search threshold", async () => {
    const agentResultsState = {
      data: {
        query: "agent",
        skills: [
          {
            key: "agent-one",
            name: "Agent One",
            directory: "agent-one",
            repoOwner: "demo",
            repoName: "skills",
            repoBranch: "main",
            readmeUrl: "https://example.com/agent-one",
            installs: 12,
          },
        ],
        totalCount: 1,
      },
      isLoading: false,
      isFetching: false,
      error: null,
      refetch: refetchSkillsShMock,
    };

    searchSkillsShHookMock.mockImplementation((query: string) =>
      query === "agent" ? agentResultsState : emptySkillsShState,
    );

    render(<SkillsPage initialApp="claude" />);

    const searchInput = await screen.findByPlaceholderText(
      "skills.skillssh.searchPlaceholder",
    );
    fireEvent.change(searchInput, { target: { value: "agent" } });
    fireEvent.click(screen.getByRole("button", { name: "skills.search" }));

    await waitFor(() =>
      expect(screen.getByTestId("skill-card-agent-one")).toBeInTheDocument(),
    );

    fireEvent.change(searchInput, { target: { value: "a" } });

    await waitFor(() =>
      expect(
        screen.getByText("skills.skillssh.searchPlaceholder"),
      ).toBeInTheDocument(),
    );
    expect(screen.queryByTestId("skill-card-agent-one")).not.toBeInTheDocument();
    expect(screen.queryByText("skills.skillssh.error")).not.toBeInTheDocument();
  });

  it("allows returning to the repo empty state when no repos are configured", async () => {
    const ref = createRef<SkillsPageHandle>();
    render(<SkillsPage ref={ref} initialApp="claude" />);

    await waitFor(() =>
      expect(
        screen.getByPlaceholderText("skills.skillssh.searchPlaceholder"),
      ).toBeInTheDocument(),
    );

    fireEvent.click(screen.getByRole("button", { name: "skills.searchSource.repos" }));

    await waitFor(() =>
      expect(screen.getByText("skills.empty")).toBeInTheDocument(),
    );
    expect(screen.getByText("skills.addRepo")).toBeInTheDocument();

    fireEvent.click(screen.getByText("skills.addRepo"));
    expect(screen.getByTestId("repo-manager")).toBeInTheDocument();
  });

  it("returns to repo results when repos become available after automatic skills.sh fallback", async () => {
    const { rerender } = render(<SkillsPage initialApp="claude" />);

    await waitFor(() =>
      expect(
        screen.getByPlaceholderText("skills.skillssh.searchPlaceholder"),
      ).toBeInTheDocument(),
    );

    discoverableSkillsFixture = [
      {
        key: "repo-skill:foo:bar",
        name: "Repo Skill",
        description: "A repo-backed skill",
        directory: "repo-skill",
        repoOwner: "foo",
        repoName: "bar",
        repoBranch: "main",
        readmeUrl: "https://example.com/readme",
      },
    ];
    reposFixture = [{ owner: "foo", name: "bar", branch: "main", enabled: true }];

    rerender(<SkillsPage initialApp="claude" />);

    await waitFor(() =>
      expect(screen.getByTestId("skill-card-repo-skill")).toBeInTheDocument(),
    );
    expect(
      screen.getByPlaceholderText("skills.searchPlaceholder"),
    ).toBeInTheDocument();
  });

  it("preserves an explicit skills.sh selection when repos later become available", async () => {
    const { rerender } = render(<SkillsPage initialApp="claude" />);

    const skillsShInput = await screen.findByPlaceholderText(
      "skills.skillssh.searchPlaceholder",
    );
    expect(skillsShInput).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "skills.sh" }));

    discoverableSkillsFixture = [
      {
        key: "repo-skill:foo:bar",
        name: "Repo Skill",
        description: "A repo-backed skill",
        directory: "repo-skill",
        repoOwner: "foo",
        repoName: "bar",
        repoBranch: "main",
        readmeUrl: "https://example.com/readme",
      },
    ];
    reposFixture = [{ owner: "foo", name: "bar", branch: "main", enabled: true }];

    rerender(<SkillsPage initialApp="claude" />);

    await waitFor(() =>
      expect(
        screen.getByPlaceholderText("skills.skillssh.searchPlaceholder"),
      ).toBeInTheDocument(),
    );
    expect(screen.queryByTestId("skill-card-repo-skill")).not.toBeInTheDocument();
  });

  it("shows a skills.sh error state and retries the same query", async () => {
    searchSkillsShHookMock.mockImplementation((query: string) => {
      if (query === "agent") {
        return {
          data: undefined,
          isLoading: false,
          isFetching: false,
          error: new Error("skills.sh request failed"),
          refetch: refetchSkillsShMock,
        };
      }

      return emptySkillsShState;
    });

    render(<SkillsPage initialApp="claude" />);

    const searchInput = await screen.findByPlaceholderText(
      "skills.skillssh.searchPlaceholder",
    );
    fireEvent.change(searchInput, { target: { value: "agent" } });
    fireEvent.click(screen.getByRole("button", { name: "skills.search" }));

    await waitFor(() => {
      expect(screen.getAllByText("skills.skillssh.error").length).toBeGreaterThan(0);
    });
    expect(screen.getByText("skills.sh request failed")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "common.refresh" }));

    await waitFor(() => {
      expect(refetchSkillsShMock).toHaveBeenCalledTimes(1);
    });

    fireEvent.click(screen.getByRole("button", { name: "skills.search" }));

    await waitFor(() => {
      expect(refetchSkillsShMock).toHaveBeenCalledTimes(2);
    });
  });

  it("clears a skills.sh error state when the input is cleared", async () => {
    searchSkillsShHookMock.mockImplementation((query: string) => {
      if (query === "agent") {
        return {
          data: undefined,
          isLoading: false,
          isFetching: false,
          error: new Error("skills.sh request failed"),
          refetch: refetchSkillsShMock,
        };
      }

      return emptySkillsShState;
    });

    render(<SkillsPage initialApp="claude" />);

    const searchInput = await screen.findByPlaceholderText(
      "skills.skillssh.searchPlaceholder",
    );
    fireEvent.change(searchInput, { target: { value: "agent" } });
    fireEvent.click(screen.getByRole("button", { name: "skills.search" }));

    await waitFor(() => {
      expect(screen.getAllByText("skills.skillssh.error").length).toBeGreaterThan(0);
    });
    expect(screen.getByText("skills.sh request failed")).toBeInTheDocument();

    fireEvent.change(searchInput, { target: { value: "" } });

    await waitFor(() =>
      expect(
        screen.getByText("skills.skillssh.searchPlaceholder"),
      ).toBeInTheDocument(),
    );
    expect(screen.queryByText("skills.sh request failed")).not.toBeInTheDocument();
    expect(screen.queryByText("skills.skillssh.error")).not.toBeInTheDocument();
  });

  it("adds and removes repos through the repo manager with success toasts", async () => {
    refetchDiscoverableMock.mockResolvedValue({
      data: [
        {
          key: "foo-bar-skill",
          name: "Foo Bar Skill",
          description: "",
          directory: "foo-bar-skill",
          repoOwner: "foo",
          repoName: "bar",
          repoBranch: "main",
        },
      ],
    });
    addRepoMock.mockResolvedValue(true);
    removeRepoMock.mockResolvedValue(true);

    const ref = createRef<SkillsPageHandle>();
    render(<SkillsPage ref={ref} initialApp="claude" />);

    await act(async () => {
      ref.current?.openRepoManager();
    });

    fireEvent.click(screen.getByText("add-repo"));

    await waitFor(() =>
      expect(addRepoMock).toHaveBeenCalledWith({
        owner: "foo",
        name: "bar",
        branch: "main",
        enabled: true,
      }),
    );

    expect(toastSuccessMock).toHaveBeenCalledWith("skills.repo.addSuccess", {
      closeButton: true,
    });

    fireEvent.click(screen.getByText("remove-repo"));

    await waitFor(() =>
      expect(removeRepoMock).toHaveBeenCalledWith({
        owner: "foo",
        name: "bar",
      }),
    );

    expect(toastSuccessMock).toHaveBeenCalledWith("skills.repo.removeSuccess", {
      closeButton: true,
    });
  });

  it("shows an error toast when adding a repo fails", async () => {
    addRepoMock.mockRejectedValue(new Error("repo add failed"));

    const ref = createRef<SkillsPageHandle>();
    render(<SkillsPage ref={ref} initialApp="claude" />);

    await act(async () => {
      ref.current?.openRepoManager();
    });

    fireEvent.click(screen.getByText("add-repo"));

    await waitFor(() => {
      expect(toastErrorMock).toHaveBeenCalledWith("common.error", {
        description: "repo add failed",
      });
    });
  });

  it("shows an error toast when removing a repo fails", async () => {
    removeRepoMock.mockRejectedValue(new Error("repo remove failed"));

    const ref = createRef<SkillsPageHandle>();
    render(<SkillsPage ref={ref} initialApp="claude" />);

    await act(async () => {
      ref.current?.openRepoManager();
    });

    fireEvent.click(screen.getByText("remove-repo"));

    await waitFor(() => {
      expect(toastErrorMock).toHaveBeenCalledWith("common.error", {
        description: "repo remove failed",
      });
    });
  });
});
