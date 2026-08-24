import {
  act,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { createRef, type ReactElement } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import PiPromptPanel, {
  type PiPromptPanelHandle,
} from "@/components/prompts/PiPromptPanel";
import type { AppId, Prompt } from "@/lib/api";

const mocks = vi.hoisted(() => ({
  state: {
    prompts: {} as Record<string, Prompt>,
    loading: false,
    currentFileContent: null as string | null,
    togglingId: null as string | null,
  },
  appIds: [] as AppId[],
  reload: vi.fn(),
  savePrompt: vi.fn(),
  deletePrompt: vi.fn(),
  toggleEnabled: vi.fn(),
  importFromFile: vi.fn(),
  openCreate: vi.fn(),
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, options?: Record<string, unknown>) => {
      if (key === "prompts.count") return `${key}:${options?.count}`;
      if (key === "prompts.enabledName") return `${key}:${options?.name}`;
      if (key === "prompts.confirm.deleteMessage") {
        return `${key}:${options?.name}`;
      }
      return key;
    },
  }),
}));

vi.mock("@/hooks/usePromptActions", () => ({
  usePromptActions: (appId: AppId) => {
    mocks.appIds.push(appId);
    return {
      prompts: mocks.state.prompts,
      loading: mocks.state.loading,
      currentFileContent: mocks.state.currentFileContent,
      togglingId: mocks.state.togglingId,
      reload: mocks.reload,
      savePrompt: mocks.savePrompt,
      deletePrompt: mocks.deletePrompt,
      toggleEnabled: mocks.toggleEnabled,
      importFromFile: mocks.importFromFile,
    };
  },
}));

vi.mock("@/components/prompts/PiNativePromptResources", async () => {
  const { forwardRef, useImperativeHandle } = await import("react");
  return {
    PiSystemPromptFiles: () => (
      <div data-testid="pi-system-prompt-files">system-files</div>
    ),
    PiPromptTemplates: forwardRef(function PiPromptTemplates(_props, ref) {
      useImperativeHandle(ref, () => ({ openCreate: mocks.openCreate }));
      return <div data-testid="pi-prompt-templates">templates</div>;
    }),
  };
});

vi.mock("@/components/prompts/PromptFormPanel", () => ({
  default: ({
    appId,
    editingId,
    onClose,
  }: {
    appId: AppId;
    editingId?: string;
    onClose: () => void;
  }) => (
    <div data-testid="prompt-form">
      {appId}:{editingId ?? "new"}
      <button type="button" onClick={onClose}>
        form-close
      </button>
    </div>
  ),
}));

vi.mock("@/components/ConfirmDialog", () => ({
  ConfirmDialog: ({
    isOpen,
    message,
    onConfirm,
    onCancel,
  }: {
    isOpen: boolean;
    message: string;
    onConfirm: (checked: boolean) => void;
    onCancel: () => void;
  }) =>
    isOpen ? (
      <div role="dialog">
        <span>{message}</span>
        <button type="button" onClick={() => onConfirm(false)}>
          confirm-dialog
        </button>
        <button type="button" onClick={onCancel}>
          cancel-dialog
        </button>
      </div>
    ) : null,
}));

const createPrompts = (): Record<string, Prompt> => ({
  "pi-active": {
    id: "pi-active",
    name: "Aurora Prompt",
    description: "Written to AGENTS.md",
    content: "Follow the quasar instruction exactly.",
    enabled: true,
  },
  "pi-idle": {
    id: "pi-idle",
    name: "Harbor Prompt",
    description: "Deployment checklist",
    content: "Prepare the release notes.",
    enabled: false,
  },
});

function renderPanel(ui: ReactElement) {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(
    <QueryClientProvider client={queryClient}>{ui}</QueryClientProvider>,
  );
}

async function switchToTab(name: string) {
  // Radix TabsTrigger activates on mousedown, not click.
  fireEvent.mouseDown(screen.getByRole("tab", { name }));
  await waitFor(() => {
    expect(screen.getByRole("tab", { name })).toHaveAttribute(
      "aria-selected",
      "true",
    );
  });
}

describe("PiPromptPanel", () => {
  beforeEach(() => {
    mocks.state.prompts = createPrompts();
    mocks.state.loading = false;
    mocks.state.currentFileContent = null;
    mocks.state.togglingId = null;
    mocks.appIds = [];
    mocks.reload.mockReset();
    mocks.reload.mockResolvedValue(true);
    mocks.savePrompt.mockReset();
    mocks.savePrompt.mockResolvedValue(true);
    mocks.deletePrompt.mockReset();
    mocks.deletePrompt.mockResolvedValue(true);
    mocks.toggleEnabled.mockReset();
    mocks.toggleEnabled.mockResolvedValue(true);
    mocks.importFromFile.mockReset();
    mocks.importFromFile.mockResolvedValue("imported-prompt");
    mocks.openCreate.mockReset();
  });

  it("reads pi prompts and renders the three native tabs", async () => {
    const { container } = renderPanel(<PiPromptPanel open />);

    await waitFor(() => expect(mocks.reload).toHaveBeenCalledTimes(1));
    expect(mocks.appIds).toContain("pi");
    expect(
      screen.getByRole("tab", { name: "pi.prompts.globalTab" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("tab", { name: "pi.prompts.systemTab" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("tab", { name: "pi.prompts.templatesTab" }),
    ).toBeInTheDocument();

    // Global tab renders the shared prompt library (header + search + list).
    const summary = container.querySelector(".glass .text-sm");
    expect(summary).toHaveTextContent("prompts.count:2");
    expect(summary).toHaveTextContent("prompts.enabledName:Aurora Prompt");
    expect(
      screen.getByRole("textbox", { name: "prompts.searchAriaLabel" }),
    ).toBeInTheDocument();
    expect(screen.getByText("Aurora Prompt")).toBeInTheDocument();
    expect(screen.getByText("Harbor Prompt")).toBeInTheDocument();
  });

  it("labels an unmanaged AGENTS.md as an external file", async () => {
    mocks.state.prompts = {
      "pi-idle": createPrompts()["pi-idle"],
    };
    mocks.state.currentFileContent = "# hand-written AGENTS.md";
    const { container } = renderPanel(<PiPromptPanel open />);

    await waitFor(() =>
      expect(container.querySelector(".glass .text-sm")).toHaveTextContent(
        "pi.prompts.externalAgents",
      ),
    );
  });

  it("blocks deleting the prompt that owns AGENTS.md", async () => {
    renderPanel(<PiPromptPanel open />);
    await waitFor(() => expect(mocks.reload).toHaveBeenCalled());

    const blockedDelete = screen.getByTitle("pi.prompts.stopBeforeDelete");
    expect(blockedDelete).toBeDisabled();

    fireEvent.click(screen.getByTitle("common.delete"));
    expect(
      screen.getByText("prompts.confirm.deleteMessage:Harbor Prompt"),
    ).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "confirm-dialog" }));

    await waitFor(() =>
      expect(mocks.deletePrompt).toHaveBeenCalledWith("pi-idle"),
    );
  });

  it("reports the primary action per tab and routes openAdd accordingly", async () => {
    const onPrimaryActionChange = vi.fn();
    const ref = createRef<PiPromptPanelHandle>();
    renderPanel(
      <PiPromptPanel
        ref={ref}
        open
        onPrimaryActionChange={onPrimaryActionChange}
      />,
    );

    await waitFor(() =>
      expect(onPrimaryActionChange).toHaveBeenLastCalledWith("prompt"),
    );
    act(() => ref.current?.openAdd());
    expect(screen.getByTestId("prompt-form")).toHaveTextContent("pi:new");
    fireEvent.click(screen.getByRole("button", { name: "form-close" }));

    await switchToTab("pi.prompts.systemTab");
    expect(onPrimaryActionChange).toHaveBeenLastCalledWith(null);
    expect(screen.getByTestId("pi-system-prompt-files")).toBeInTheDocument();
    act(() => ref.current?.openAdd());
    expect(screen.queryByTestId("prompt-form")).not.toBeInTheDocument();
    expect(mocks.openCreate).not.toHaveBeenCalled();

    await switchToTab("pi.prompts.templatesTab");
    expect(onPrimaryActionChange).toHaveBeenLastCalledWith("template");
    act(() => ref.current?.openAdd());
    expect(mocks.openCreate).toHaveBeenCalledTimes(1);
    expect(screen.queryByTestId("prompt-form")).not.toBeInTheDocument();
  });

  it("imports AGENTS.md only from the global tab", async () => {
    const ref = createRef<PiPromptPanelHandle>();
    renderPanel(<PiPromptPanel ref={ref} open />);
    await waitFor(() => expect(mocks.reload).toHaveBeenCalled());

    await act(async () => {
      await ref.current?.openImport();
    });
    expect(mocks.importFromFile).toHaveBeenCalledTimes(1);

    await switchToTab("pi.prompts.templatesTab");
    await act(async () => {
      await ref.current?.openImport();
    });
    expect(mocks.importFromFile).toHaveBeenCalledTimes(1);
  });

  it("freezes the list while a pi toggle write is in flight", async () => {
    mocks.state.togglingId = "pi-idle";
    const onInteractionBlockedChange = vi.fn();
    const onNavigationBlockedChange = vi.fn();
    renderPanel(
      <PiPromptPanel
        open
        onInteractionBlockedChange={onInteractionBlockedChange}
        onNavigationBlockedChange={onNavigationBlockedChange}
      />,
    );

    await waitFor(() => {
      expect(onInteractionBlockedChange).toHaveBeenLastCalledWith(true);
      expect(onNavigationBlockedChange).toHaveBeenLastCalledWith(true);
    });
    expect(screen.getAllByRole("switch")[0]).toBeDisabled();
    expect(screen.getAllByTitle("common.edit")[0]).toBeDisabled();
  });

  it("exposes reload for the profile-applied refresh path", async () => {
    const ref = createRef<PiPromptPanelHandle>();
    renderPanel(<PiPromptPanel ref={ref} open />);
    await waitFor(() => expect(mocks.reload).toHaveBeenCalledTimes(1));

    await act(async () => {
      await ref.current?.reload();
    });
    expect(mocks.reload).toHaveBeenCalledTimes(2);
  });

  it("reloads when a pi prompt is imported through a deep link", async () => {
    renderPanel(<PiPromptPanel open />);
    await waitFor(() => expect(mocks.reload).toHaveBeenCalledTimes(1));

    act(() => {
      window.dispatchEvent(
        new CustomEvent("prompt-imported", { detail: { app: "claude" } }),
      );
    });
    expect(mocks.reload).toHaveBeenCalledTimes(1);

    act(() => {
      window.dispatchEvent(
        new CustomEvent("prompt-imported", { detail: { app: "pi" } }),
      );
    });
    await waitFor(() => expect(mocks.reload).toHaveBeenCalledTimes(2));
  });
});
