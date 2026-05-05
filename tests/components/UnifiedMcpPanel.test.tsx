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

const toastSuccessMock = vi.fn();
const toastErrorMock = vi.fn();
const toggleMcpAppMock = vi.fn();
const deleteMcpServerMock = vi.fn();
const importMcpFromAppsMock = vi.fn();
let mcpServersFixture: Record<string, any> = {};

vi.mock("sonner", () => ({
  toast: {
    success: (...args: unknown[]) => toastSuccessMock(...args),
    error: (...args: unknown[]) => toastErrorMock(...args),
  },
}));

vi.mock("@/hooks/useMcp", () => ({
  useAllMcpServers: () => ({
    data: mcpServersFixture,
    isLoading: false,
  }),
  useToggleMcpApp: () => ({
    mutateAsync: toggleMcpAppMock,
  }),
  useDeleteMcpServer: () => ({
    mutateAsync: deleteMcpServerMock,
  }),
  useImportMcpFromApps: () => ({
    mutateAsync: importMcpFromAppsMock,
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
    onConfirm,
    onCancel,
  }: {
    isOpen: boolean;
    title: string;
    message: string;
    confirmText?: string;
    cancelText?: string;
    onConfirm: () => void;
    onCancel: () => void;
  }) =>
    isOpen ? (
      <div data-testid="confirm-dialog">
        <div>{title}</div>
        <div>{message}</div>
        <button onClick={onConfirm}>{confirmText}</button>
        <button onClick={onCancel}>{cancelText}</button>
      </div>
    ) : null,
}));

vi.mock("@/components/common/AppToggleGroup", () => ({
  AppToggleGroup: ({
    apps,
    onToggle,
    appIds,
  }: {
    apps: Record<string, boolean>;
    onToggle: (app: string, enabled: boolean) => void;
    appIds: string[];
  }) => (
    <div>
      {appIds.map((app) => (
        <button
          key={app}
          type="button"
          aria-label={app}
          onClick={() => onToggle(app, !apps[app])}
        >
          {app}
        </button>
      ))}
    </div>
  ),
}));

vi.mock("@/components/common/AppCountBar", () => ({
  AppCountBar: ({ totalLabel }: { totalLabel: string }) => (
    <div>{totalLabel}</div>
  ),
}));

vi.mock("@/lib/api", () => ({
  settingsApi: {
    openExternal: vi.fn(),
  },
}));

describe("UnifiedMcpPanel", () => {
  beforeEach(() => {
    toastSuccessMock.mockReset();
    toastErrorMock.mockReset();
    toggleMcpAppMock.mockReset();
    deleteMcpServerMock.mockReset();
    importMcpFromAppsMock.mockReset();
    mcpServersFixture = {
      "demo-server": {
        id: "demo-server",
        name: "Demo Server",
        description: "Demo description",
        apps: {
          claude: true,
          codex: false,
          gemini: false,
          opencode: false,
          hermes: false,
        },
        server: {
          type: "stdio",
          command: "demo",
        },
      },
    };
  });

  it("imports MCP servers from apps through the panel handle", async () => {
    const ref = createRef<UnifiedMcpPanelHandle>();
    importMcpFromAppsMock.mockResolvedValue(2);

    render(<UnifiedMcpPanel ref={ref} onOpenChange={vi.fn()} />);

    await act(async () => {
      await ref.current?.openImport();
    });

    expect(importMcpFromAppsMock).toHaveBeenCalledTimes(1);
    expect(toastSuccessMock).toHaveBeenCalledWith(
      "mcp.unifiedPanel.importSuccess",
      { closeButton: true },
    );
  });

  it("opens the MCP form through the add handle", async () => {
    const ref = createRef<UnifiedMcpPanelHandle>();

    render(<UnifiedMcpPanel ref={ref} onOpenChange={vi.fn()} />);

    await act(async () => {
      ref.current?.openAdd();
    });

    expect(screen.getByTestId("mcp-form-modal")).toBeInTheDocument();
  });

  it("shows the no-import toast when no MCP servers are discovered", async () => {
    const ref = createRef<UnifiedMcpPanelHandle>();
    importMcpFromAppsMock.mockResolvedValue(0);

    render(<UnifiedMcpPanel ref={ref} onOpenChange={vi.fn()} />);

    await act(async () => {
      await ref.current?.openImport();
    });

    expect(toastSuccessMock).toHaveBeenCalledWith(
      "mcp.unifiedPanel.noImportFound",
      { closeButton: true },
    );
  });

  it("shows a translated import error when importing MCP servers fails", async () => {
    const ref = createRef<UnifiedMcpPanelHandle>();
    importMcpFromAppsMock.mockRejectedValueOnce(
      new Error("解析 config.toml 失败"),
    );

    render(<UnifiedMcpPanel ref={ref} onOpenChange={vi.fn()} />);

    await act(async () => {
      await ref.current?.openImport();
    });

    await waitFor(() =>
      expect(toastErrorMock).toHaveBeenCalledWith("mcp.error.tomlInvalid", {
        duration: 6000,
      }),
    );
  });

  it("toggles an app and confirms deletion from the visible list item", async () => {
    toggleMcpAppMock.mockResolvedValue(undefined);
    deleteMcpServerMock.mockResolvedValue(true);

    render(<UnifiedMcpPanel onOpenChange={vi.fn()} />);

    fireEvent.click(screen.getByRole("button", { name: "codex" }));

    await waitFor(() =>
      expect(toggleMcpAppMock).toHaveBeenCalledWith({
        serverId: "demo-server",
        app: "codex",
        enabled: true,
      }),
    );

    fireEvent.click(screen.getByTitle("common.delete"));

    const dialog = await screen.findByTestId("confirm-dialog");
    expect(dialog).toBeInTheDocument();
    expect(screen.getByText("mcp.unifiedPanel.deleteServer")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "confirm" }));

    await waitFor(() =>
      expect(deleteMcpServerMock).toHaveBeenCalledWith("demo-server"),
    );
    expect(toastSuccessMock).toHaveBeenCalledWith("common.success", {
      closeButton: true,
    });
  });

  it("falls back to an action-specific toggle error when no detail is available", async () => {
    toggleMcpAppMock.mockRejectedValueOnce({});

    render(<UnifiedMcpPanel onOpenChange={vi.fn()} />);

    fireEvent.click(screen.getByRole("button", { name: "codex" }));

    await waitFor(() =>
      expect(toastErrorMock).toHaveBeenCalledWith(
        "mcp.unifiedPanel.toggleFailed",
        {
          duration: 4000,
        },
      ),
    );
  });

  it("keeps the delete confirmation open and shows detail when delete fails", async () => {
    deleteMcpServerMock.mockRejectedValueOnce(new Error("delete exploded"));

    render(<UnifiedMcpPanel onOpenChange={vi.fn()} />);

    fireEvent.click(screen.getByTitle("common.delete"));

    const dialog = await screen.findByTestId("confirm-dialog");
    fireEvent.click(within(dialog).getByRole("button", { name: "confirm" }));

    await waitFor(() =>
      expect(deleteMcpServerMock).toHaveBeenCalledWith("demo-server"),
    );
    await waitFor(() =>
      expect(toastErrorMock).toHaveBeenCalledWith("delete exploded", {
        duration: 6000,
      }),
    );
    expect(screen.getByTestId("confirm-dialog")).toBeInTheDocument();
  });

  it("closes the delete confirmation without mutating when canceled", async () => {
    render(<UnifiedMcpPanel onOpenChange={vi.fn()} />);

    fireEvent.click(screen.getByTitle("common.delete"));

    const dialog = await screen.findByTestId("confirm-dialog");
    fireEvent.click(within(dialog).getByRole("button", { name: "cancel" }));

    await waitFor(() =>
      expect(screen.queryByTestId("confirm-dialog")).not.toBeInTheDocument(),
    );
    expect(deleteMcpServerMock).not.toHaveBeenCalled();
    expect(toastSuccessMock).not.toHaveBeenCalled();
    expect(toastErrorMock).not.toHaveBeenCalled();
  });
});
