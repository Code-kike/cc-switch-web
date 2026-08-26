import { createRef } from "react";
import {
  act,
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import UnifiedMcpPanel, {
  type UnifiedMcpPanelHandle,
} from "@/components/mcp/UnifiedMcpPanel";
import type { McpApps, McpServer, McpServerSpec } from "@/types";

const mocks = vi.hoisted(() => ({
  serversMap: {} as Record<string, McpServer>,
  isLoading: false,
  togglePending: false,
  toggleVariables: undefined as
    | { serverId: string; app: string; enabled: boolean }
    | undefined,
  bulkPending: false,
  bulkVariables: undefined as
    | { serverIds: string[]; app: string; enabled: boolean }
    | undefined,
  toggle: vi.fn(),
  bulkToggle: vi.fn(),
  deleteServer: vi.fn(),
  importServers: vi.fn(),
  toastError: vi.fn(),
  toastSuccess: vi.fn(),
}));

vi.mock("@/hooks/useMcp", () => ({
  useAllMcpServers: () => ({
    data: mocks.serversMap,
    isLoading: mocks.isLoading,
  }),
  useToggleMcpApp: () => ({
    mutateAsync: mocks.toggle,
    isPending: mocks.togglePending,
    variables: mocks.toggleVariables,
  }),
  useBulkToggleMcpApp: () => ({
    mutateAsync: mocks.bulkToggle,
    isPending: mocks.bulkPending,
    variables: mocks.bulkVariables,
  }),
  useDeleteMcpServer: () => ({
    mutateAsync: mocks.deleteServer,
    isPending: false,
  }),
  useImportMcpFromApps: () => ({
    mutateAsync: mocks.importServers,
    isPending: false,
  }),
}));

vi.mock("@/components/mcp/McpFormModal", () => ({
  default: () => <div data-testid="mcp-form-modal" />,
}));

vi.mock("@/components/ConfirmDialog", () => ({
  ConfirmDialog: ({
    isOpen,
    title,
    message,
    confirmText = "confirm",
    cancelText = "cancel",
    pending,
    onConfirm,
    onCancel,
  }: {
    isOpen: boolean;
    title: string;
    message: string;
    confirmText?: string;
    cancelText?: string;
    pending?: boolean;
    onConfirm: (checked: boolean) => void;
    onCancel: () => void;
  }) =>
    isOpen ? (
      <div data-testid="confirm-dialog">
        <div>{title}</div>
        <div>{message}</div>
        <button disabled={pending} onClick={() => onConfirm(false)}>
          {confirmText}
        </button>
        <button disabled={pending} onClick={onCancel}>
          {cancelText}
        </button>
      </div>
    ) : null,
}));

vi.mock("sonner", () => ({
  toast: {
    error: mocks.toastError,
    success: mocks.toastSuccess,
  },
}));

vi.mock("@/lib/api", () => ({
  settingsApi: {
    openExternal: vi.fn(),
  },
}));

type ServerOverrides = Partial<Omit<McpServer, "apps" | "server">> & {
  apps?: Partial<McpApps>;
  server?: Partial<McpServerSpec>;
};

function makeServer(id: string, overrides: ServerOverrides = {}): McpServer {
  const { apps, server, ...metadata } = overrides;
  return {
    id,
    name: id,
    ...metadata,
    server: {
      type: "stdio",
      command: "default-command",
      ...server,
    },
    apps: {
      claude: false,
      codex: false,
      gemini: false,
      grokbuild: false,
      opencode: false,
      openclaw: false,
      hermes: false,
      ...apps,
    },
  } as McpServer;
}

function renderPanel(onInteractionBlockedChange?: (blocked: boolean) => void) {
  return render(
    <UnifiedMcpPanel
      onOpenChange={vi.fn()}
      onInteractionBlockedChange={onInteractionBlockedChange}
    />,
  );
}

describe("UnifiedMcpPanel", () => {
  beforeEach(() => {
    mocks.serversMap = {};
    mocks.isLoading = false;
    mocks.togglePending = false;
    mocks.toggleVariables = undefined;
    mocks.bulkPending = false;
    mocks.bulkVariables = undefined;
    mocks.toggle.mockReset();
    mocks.bulkToggle.mockReset();
    mocks.deleteServer.mockReset();
    mocks.importServers.mockReset();
    mocks.toastError.mockReset();
    mocks.toastSuccess.mockReset();
    mocks.toggle.mockResolvedValue(undefined);
    mocks.bulkToggle.mockResolvedValue({ succeeded: [], failed: [] });
    mocks.deleteServer.mockResolvedValue(true);
    mocks.importServers.mockResolvedValue(0);
  });

  it("imports MCP servers from apps through the panel handle", async () => {
    const ref = createRef<UnifiedMcpPanelHandle>();
    mocks.importServers.mockResolvedValue(2);

    render(<UnifiedMcpPanel ref={ref} onOpenChange={vi.fn()} />);

    await act(async () => {
      await ref.current?.openImport();
    });

    expect(mocks.importServers).toHaveBeenCalledTimes(1);
    expect(mocks.toastSuccess).toHaveBeenCalledWith(
      "mcp.unifiedPanel.importSuccess",
      { closeButton: true },
    );
  });

  it("opens the MCP form through the add handle", () => {
    const ref = createRef<UnifiedMcpPanelHandle>();

    render(<UnifiedMcpPanel ref={ref} onOpenChange={vi.fn()} />);
    act(() => ref.current?.openAdd());

    expect(screen.getByTestId("mcp-form-modal")).toBeInTheDocument();
  });

  it("shows the no-import toast when no MCP servers are discovered", async () => {
    const ref = createRef<UnifiedMcpPanelHandle>();

    render(<UnifiedMcpPanel ref={ref} onOpenChange={vi.fn()} />);

    await act(async () => {
      await ref.current?.openImport();
    });

    expect(mocks.toastSuccess).toHaveBeenCalledWith(
      "mcp.unifiedPanel.noImportFound",
      { closeButton: true },
    );
  });

  it("shows a translated import error when importing MCP servers fails", async () => {
    const ref = createRef<UnifiedMcpPanelHandle>();
    mocks.importServers.mockRejectedValueOnce(
      new Error("解析 config.toml 失败"),
    );

    render(<UnifiedMcpPanel ref={ref} onOpenChange={vi.fn()} />);

    await act(async () => {
      await ref.current?.openImport();
    });

    await waitFor(() =>
      expect(mocks.toastError).toHaveBeenCalledWith("mcp.error.tomlInvalid", {
        duration: 6000,
      }),
    );
  });

  it("toggles an app and confirms deletion from the visible list item", async () => {
    mocks.serversMap = {
      "demo-server": makeServer("demo-server", {
        name: "Demo Server",
        description: "Demo description",
        apps: { claude: true },
        server: { command: "demo" },
      }),
    };

    renderPanel();

    fireEvent.click(screen.getByRole("button", { name: "Codex" }));
    await waitFor(() =>
      expect(mocks.toggle).toHaveBeenCalledWith({
        serverId: "demo-server",
        app: "codex",
        enabled: true,
      }),
    );

    fireEvent.click(screen.getByTitle("common.delete"));
    const dialog = await screen.findByTestId("confirm-dialog");
    fireEvent.click(within(dialog).getByRole("button", { name: "confirm" }));

    await waitFor(() =>
      expect(mocks.deleteServer).toHaveBeenCalledWith("demo-server"),
    );
    expect(mocks.toastSuccess).toHaveBeenCalledWith("common.success", {
      closeButton: true,
    });
  });

  it("falls back to an action-specific toggle error when no detail is available", async () => {
    mocks.serversMap = { server: makeServer("server") };
    mocks.toggle.mockRejectedValueOnce({});

    renderPanel();
    fireEvent.click(screen.getByRole("button", { name: "Codex" }));

    await waitFor(() =>
      expect(mocks.toastError).toHaveBeenCalledWith(
        "mcp.unifiedPanel.toggleFailed",
        { duration: 4000 },
      ),
    );
  });

  it("keeps the delete confirmation open and shows detail when delete fails", async () => {
    mocks.serversMap = { server: makeServer("server") };
    mocks.deleteServer.mockRejectedValueOnce(new Error("delete exploded"));

    renderPanel();
    fireEvent.click(screen.getByTitle("common.delete"));
    const dialog = await screen.findByTestId("confirm-dialog");
    fireEvent.click(within(dialog).getByRole("button", { name: "confirm" }));

    await waitFor(() =>
      expect(mocks.toastError).toHaveBeenCalledWith("delete exploded", {
        duration: 6000,
      }),
    );
    expect(screen.getByTestId("confirm-dialog")).toBeInTheDocument();
  });

  it("closes the delete confirmation without mutating when canceled", async () => {
    mocks.serversMap = { server: makeServer("server") };

    renderPanel();
    fireEvent.click(screen.getByTitle("common.delete"));
    const dialog = await screen.findByTestId("confirm-dialog");
    fireEvent.click(within(dialog).getByRole("button", { name: "cancel" }));

    await waitFor(() =>
      expect(screen.queryByTestId("confirm-dialog")).not.toBeInTheDocument(),
    );
    expect(mocks.deleteServer).not.toHaveBeenCalled();
  });

  it("searches the explicit non-sensitive MCP fields and renders a visible ScrollArea", () => {
    mocks.serversMap = {
      "map-key-hit": makeServer("internal-id-hit", {
        name: "Display Name Hit",
        description: "description-hit",
        tags: ["tag-hit"],
        homepage: "https://homepage-hit.example",
        docs: "https://docs-hit.example",
        source: "source-hit",
        server: {
          type: "sse",
          command: "command-hit",
          args: ["--arg-hit"],
          cwd: "/cwd-hit",
          url: "https://url-hit.example",
        },
      }),
      control: makeServer("control", { name: "Control Server" }),
    };

    const { container } = renderPanel();
    const input = screen.getByLabelText("mcp.unifiedPanel.searchAriaLabel");

    expect(
      container.querySelector("[data-radix-scroll-area-viewport]"),
    ).toBeInTheDocument();

    for (const query of [
      "map-key-hit",
      "internal-id-hit",
      "  DISPLAY NAME HIT  ",
      "description-hit",
      "tag-hit",
      "sse",
      "command-hit",
      "arg-hit",
      "cwd-hit",
      "url-hit.example",
      "homepage-hit.example",
      "docs-hit.example",
      "source-hit",
    ]) {
      fireEvent.change(input, { target: { value: query } });
      expect(screen.getByText("Display Name Hit")).toBeInTheDocument();
      expect(screen.queryByText("Control Server")).not.toBeInTheDocument();
    }
  });

  it("does not index MCP env or headers keys and values", () => {
    mocks.serversMap = {
      secret: makeServer("secret", {
        name: "Secret Holder",
        server: {
          env: { ONLY_ENV_SECRET: "env-value-needle" },
          headers: { Authorization: "header-value-needle" },
        },
      }),
    };

    renderPanel();
    const input = screen.getByLabelText("mcp.unifiedPanel.searchAriaLabel");

    for (const query of [
      "only_env_secret",
      "env-value-needle",
      "authorization",
      "header-value-needle",
    ]) {
      fireEvent.change(input, { target: { value: query } });
      expect(screen.queryByText("Secret Holder")).not.toBeInTheDocument();
      expect(
        screen.getByText("mcp.unifiedPanel.noSearchResults"),
      ).toBeInTheDocument();
    }
  });

  it("keeps the original empty state distinct from an empty search result", () => {
    renderPanel();

    expect(screen.getByText("mcp.unifiedPanel.noServers")).toBeInTheDocument();
    expect(
      screen.queryByText("mcp.unifiedPanel.noSearchResults"),
    ).not.toBeInTheDocument();

    fireEvent.change(
      screen.getByLabelText("mcp.unifiedPanel.searchAriaLabel"),
      { target: { value: "anything" } },
    );

    expect(screen.getByText("mcp.unifiedPanel.noServers")).toBeInTheDocument();
    expect(
      screen.queryByText("mcp.unifiedPanel.noSearchResults"),
    ).not.toBeInTheDocument();
  });

  it("bulk toggles the full collection and submits only servers whose state differs", async () => {
    mocks.serversMap = {
      visible: makeServer("visible", {
        name: "Visible Needle",
        apps: { claude: false },
      }),
      "hidden-disabled": makeServer("hidden-disabled", {
        name: "Hidden Disabled",
        apps: { claude: false },
      }),
      "hidden-enabled": makeServer("hidden-enabled", {
        name: "Hidden Enabled",
        apps: { claude: true },
      }),
    };
    mocks.bulkToggle.mockResolvedValue({
      succeeded: ["visible", "hidden-disabled"],
      failed: [],
    });

    renderPanel();
    fireEvent.change(
      screen.getByLabelText("mcp.unifiedPanel.searchAriaLabel"),
      { target: { value: "visible needle" } },
    );

    expect(screen.getByText("Visible Needle")).toBeInTheDocument();
    expect(screen.queryByText("Hidden Disabled")).not.toBeInTheDocument();
    expect(screen.queryByText("Hidden Enabled")).not.toBeInTheDocument();

    fireEvent.click(screen.getAllByRole("checkbox")[0]);

    await waitFor(() => {
      expect(mocks.bulkToggle).toHaveBeenCalledWith({
        serverIds: ["visible", "hidden-disabled"],
        app: "claude",
        enabled: true,
      });
    });
  });

  it("blocks edit and delete while a toggle write is pending", async () => {
    mocks.serversMap = {
      server: makeServer("server", { name: "Managed Server" }),
    };
    mocks.bulkPending = true;
    mocks.bulkVariables = {
      serverIds: ["server"],
      app: "claude",
      enabled: true,
    };
    const onInteractionBlockedChange = vi.fn();

    renderPanel(onInteractionBlockedChange);

    expect(screen.getByTitle("common.edit")).toBeDisabled();
    expect(screen.getByTitle("common.delete")).toBeDisabled();
    for (const bulkControl of screen.getAllByRole("checkbox")) {
      expect(bulkControl).toBeDisabled();
    }
    await waitFor(() =>
      expect(onInteractionBlockedChange).toHaveBeenCalledWith(true),
    );
  });
});
