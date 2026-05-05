import {
  Suspense,
  type ComponentType,
  forwardRef,
  useImperativeHandle,
} from "react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor, fireEvent } from "@testing-library/react";
import { describe, it, expect, beforeEach, vi, type MockInstance } from "vitest";
import { providersApi } from "@/lib/api/providers";
import { closeAllSubscriptions } from "@/lib/api/event-adapter";
import {
  resetProviderState,
  setCurrentProviderId,
  setLiveProviderIds,
  setProviders,
} from "../msw/state";

const appEventListeners = vi.hoisted(
  () =>
    new Map<
      string,
      Set<(event: { event: string; payload: unknown }) => void>
    >(),
);

const toastSuccessMock = vi.fn();
const toastErrorMock = vi.fn();
const toastInfoMock = vi.fn();
const promptOpenImportMock = vi.fn();
const promptOpenAddMock = vi.fn();
const mcpOpenImportMock = vi.fn();
const mcpOpenAddMock = vi.fn();
const skillsOpenRestoreFromBackupMock = vi.fn();
const skillsOpenInstallFromZipMock = vi.fn();
const skillsOpenImportMock = vi.fn();
const deepLinkOpenManualImportMock = vi.fn();

vi.mock("@/lib/api/event-adapter", () => ({
  listen: async (
    event: string,
    handler: (event: { event: string; payload: unknown }) => void,
  ) => {
    const set = appEventListeners.get(event) ?? new Set();
    set.add(handler);
    appEventListeners.set(event, set);
    return () => {
      set.delete(handler);
      if (set.size === 0) {
        appEventListeners.delete(event);
      }
    };
  },
  closeAllSubscriptions: () => {
    appEventListeners.clear();
  },
  emitMockEvent: (event: string, payload: unknown) => {
    appEventListeners
      .get(event)
      ?.forEach((handler) => handler({ event, payload }));
  },
  getMockListenerCount: (event: string) => appEventListeners.get(event)?.size ?? 0,
}));

const emitMockEvent = (event: string, payload: unknown) => {
  appEventListeners.get(event)?.forEach((handler) => handler({ event, payload }));
};

const getMockListenerCount = (event: string) =>
  appEventListeners.get(event)?.size ?? 0;

vi.mock("sonner", () => ({
  toast: {
    success: (...args: unknown[]) => toastSuccessMock(...args),
    error: (...args: unknown[]) => toastErrorMock(...args),
    info: (...args: unknown[]) => toastInfoMock(...args),
  },
}));

vi.mock("@/components/providers/ProviderList", () => ({
  ProviderList: ({
    providers,
    currentProviderId,
    onSwitch,
    onEdit,
    onDuplicate,
    onConfigureUsage,
    onOpenWebsite,
    onCreate,
  }: any) => (
    <div>
      <div data-testid="provider-list">{JSON.stringify(providers)}</div>
      <div data-testid="current-provider">{currentProviderId}</div>
      <button onClick={() => onSwitch(providers[currentProviderId])}>
        switch
      </button>
      <button onClick={() => onEdit(providers[currentProviderId])}>edit</button>
      <button onClick={() => onDuplicate(providers[currentProviderId])}>
        duplicate
      </button>
      <button onClick={() => onConfigureUsage(providers[currentProviderId])}>
        usage
      </button>
      <button onClick={() => onOpenWebsite("https://example.com")}>
        open-website
      </button>
      <button onClick={() => onCreate?.()}>create</button>
    </div>
  ),
}));

vi.mock("@/components/providers/AddProviderDialog", () => ({
  AddProviderDialog: ({ open, onOpenChange, onSubmit, appId }: any) =>
    open ? (
      <div data-testid="add-provider-dialog">
        <button
          onClick={() =>
            onSubmit({
              name: `New ${appId} Provider`,
              settingsConfig: {},
              category: "custom",
              sortIndex: 99,
            })
          }
        >
          confirm-add
        </button>
        <button onClick={() => onOpenChange(false)}>close-add</button>
      </div>
    ) : null,
}));

vi.mock("@/components/providers/EditProviderDialog", () => ({
  EditProviderDialog: ({ open, provider, onSubmit, onOpenChange }: any) =>
    open ? (
      <div data-testid="edit-provider-dialog">
        <button
          onClick={() =>
            onSubmit({
              provider: {
                ...provider,
                name: `${provider.name}-edited`,
              },
              originalId: provider.id,
            })
          }
        >
          confirm-edit
        </button>
        <button onClick={() => onOpenChange(false)}>close-edit</button>
      </div>
    ) : null,
}));

vi.mock("@/components/UsageScriptModal", () => ({
  default: ({ isOpen, provider, onSave, onClose }: any) =>
    isOpen ? (
      <div data-testid="usage-modal">
        <span data-testid="usage-provider">{provider?.id}</span>
        <button onClick={() => onSave("script-code")}>save-script</button>
        <button onClick={() => onClose()}>close-usage</button>
      </div>
    ) : null,
}));

vi.mock("@/components/ConfirmDialog", () => ({
  ConfirmDialog: ({ isOpen, onConfirm, onCancel }: any) =>
    isOpen ? (
      <div data-testid="confirm-dialog">
        <button onClick={() => onConfirm()}>confirm-delete</button>
        <button onClick={() => onCancel()}>cancel-delete</button>
      </div>
    ) : null,
}));

vi.mock("@/components/AppSwitcher", () => ({
  AppSwitcher: ({ activeApp, onSwitch }: any) => (
    <div data-testid="app-switcher">
      <span>{activeApp}</span>
      <button onClick={() => onSwitch("claude")}>switch-claude</button>
      <button onClick={() => onSwitch("codex")}>switch-codex</button>
      <button onClick={() => onSwitch("gemini")}>switch-gemini</button>
      <button onClick={() => onSwitch("opencode")}>switch-opencode</button>
      <button onClick={() => onSwitch("openclaw")}>switch-openclaw</button>
      <button onClick={() => onSwitch("hermes")}>switch-hermes</button>
    </div>
  ),
}));

vi.mock("@/components/UpdateBadge", () => ({
  UpdateBadge: ({ onClick }: any) => (
    <button onClick={onClick}>update-badge</button>
  ),
}));

vi.mock("@/components/prompts/PromptPanel", () => ({
  default: forwardRef((_props: any, ref) => {
    useImperativeHandle(ref, () => ({
      openImport: () => promptOpenImportMock(),
      openAdd: () => promptOpenAddMock(),
    }));
    return <div data-testid="prompt-panel">prompt-panel</div>;
  }),
}));

vi.mock("@/components/mcp/UnifiedMcpPanel", () => ({
  default: forwardRef((_props: any, ref) => {
    useImperativeHandle(ref, () => ({
      openImport: () => mcpOpenImportMock(),
      openAdd: () => mcpOpenAddMock(),
    }));
    return <div data-testid="mcp-panel">mcp-panel</div>;
  }),
}));

vi.mock("@/components/skills/UnifiedSkillsPanel", () => ({
  default: forwardRef((_props: any, ref) => {
    useImperativeHandle(ref, () => ({
      openRestoreFromBackup: () => skillsOpenRestoreFromBackupMock(),
      openInstallFromZip: () => skillsOpenInstallFromZipMock(),
      openImport: () => skillsOpenImportMock(),
    }));
    return <div data-testid="unified-skills-panel">unified-skills-panel</div>;
  }),
}));

vi.mock("@/components/DeepLinkImportDialog", () => ({
  DeepLinkImportDialog: forwardRef((_props: any, ref) => {
    useImperativeHandle(ref, () => ({
      openManualImport: (...args: unknown[]) =>
        deepLinkOpenManualImportMock(...args),
    }));
    return <div data-testid="deeplink-import-dialog">deeplink-import-dialog</div>;
  }),
}));

vi.mock("@/components/universal", () => ({
  UniversalProviderPanel: () => (
    <div data-testid="universal-provider-panel">universal-provider-panel</div>
  ),
}));

const renderApp = (AppComponent: ComponentType) => {
  const client = new QueryClient();
  return render(
    <QueryClientProvider client={client}>
      <Suspense fallback={<div data-testid="loading">loading</div>}>
        <AppComponent />
      </Suspense>
    </QueryClientProvider>,
  );
};

const waitForEventSubscription = async (event: string) => {
  await waitFor(() => {
    expect(getMockListenerCount(event)).toBeGreaterThan(0);
  });
};

describe("App integration with MSW", () => {
  beforeEach(() => {
    resetProviderState();
    closeAllSubscriptions();
    toastSuccessMock.mockReset();
    toastErrorMock.mockReset();
    toastInfoMock.mockReset();
    promptOpenImportMock.mockReset();
    promptOpenAddMock.mockReset();
    mcpOpenImportMock.mockReset();
    mcpOpenAddMock.mockReset();
    skillsOpenRestoreFromBackupMock.mockReset();
    skillsOpenInstallFromZipMock.mockReset();
    skillsOpenImportMock.mockReset();
    deepLinkOpenManualImportMock.mockReset();
    localStorage.clear();
    sessionStorage.clear();
    Object.defineProperty(window, "__TAURI_INTERNALS__", {
      configurable: true,
      value: undefined,
    });
    Object.defineProperty(window, "__TAURI__", {
      configurable: true,
      value: undefined,
    });
    vi.spyOn(window, "open").mockReturnValue(null);
  });

  it("covers basic provider flows via real hooks", async () => {
    const { default: App } = await import("@/App");
    renderApp(App);

    await waitFor(() =>
      expect(screen.getByTestId("provider-list").textContent).toContain(
        "claude-1",
      ),
    );

    fireEvent.click(screen.getByText("switch-codex"));
    await waitFor(() =>
      expect(screen.getByTestId("provider-list").textContent).toContain(
        "codex-1",
      ),
    );

    fireEvent.click(screen.getByText("usage"));
    expect(screen.getByTestId("usage-modal")).toBeInTheDocument();
    fireEvent.click(screen.getByText("save-script"));
    fireEvent.click(screen.getByText("close-usage"));

    fireEvent.click(screen.getByText("create"));
    expect(screen.getByTestId("add-provider-dialog")).toBeInTheDocument();
    fireEvent.click(screen.getByText("confirm-add"));
    await waitFor(() =>
      expect(screen.getByTestId("provider-list").textContent).toMatch(
        /New codex Provider/,
      ),
    );

    fireEvent.click(screen.getByText("edit"));
    expect(screen.getByTestId("edit-provider-dialog")).toBeInTheDocument();
    fireEvent.click(screen.getByText("confirm-edit"));
    await waitFor(() =>
      expect(screen.getByTestId("provider-list").textContent).toMatch(
        /-edited/,
      ),
    );

    fireEvent.click(screen.getByText("switch"));
    fireEvent.click(screen.getByText("duplicate"));
    await waitFor(() =>
      expect(screen.getByTestId("provider-list").textContent).toMatch(/copy/),
    );

    fireEvent.click(screen.getByText("open-website"));

    await waitForEventSubscription("provider-switched");
    emitMockEvent("provider-switched", {
      appType: "codex",
      providerId: "codex-2",
    });

    expect(toastErrorMock).not.toHaveBeenCalled();
    expect(toastSuccessMock).toHaveBeenCalled();
  }, 10000);

  it("shows toast when auto sync fails in background", async () => {
    const { default: App } = await import("@/App");
    renderApp(App);

    await waitFor(() =>
      expect(screen.getByTestId("provider-list").textContent).toContain(
        "claude-1",
      ),
    );

    await waitForEventSubscription("webdav-sync-status-updated");
    emitMockEvent("webdav-sync-status-updated", {
      source: "auto",
      status: "error",
      error: "network timeout",
    });

    await waitFor(() => {
      expect(toastErrorMock).toHaveBeenCalled();
    });
  });

  it("duplicates openclaw providers with a generated key that avoids live-only ids", async () => {
    setProviders("openclaw", {
      deepseek: {
        id: "deepseek",
        name: "DeepSeek",
        settingsConfig: {
          baseUrl: "https://api.deepseek.com",
          apiKey: "test-key",
          api: "openai-completions",
          models: [],
        },
        category: "custom",
        sortIndex: 0,
        createdAt: Date.now(),
      },
    });
    setCurrentProviderId("openclaw", "deepseek");
    setLiveProviderIds("openclaw", ["deepseek-copy"]);

    const { default: App } = await import("@/App");
    renderApp(App);

    fireEvent.click(screen.getByText("switch-openclaw"));

    await waitFor(() =>
      expect(screen.getByTestId("provider-list").textContent).toContain(
        "deepseek",
      ),
    );

    fireEvent.click(screen.getByText("duplicate"));

    await waitFor(() => {
      const providerList = screen.getByTestId("provider-list").textContent;
      expect(providerList).toContain("deepseek-copy-2");
      expect(providerList).toContain("DeepSeek copy");
    });

    expect(toastErrorMock).not.toHaveBeenCalledWith(
      expect.stringContaining("Provider key is required for openclaw"),
    );
  });

  it.each([
    {
      appId: "claude",
      switchLabel: null,
      primeSpy: () =>
        vi.spyOn(providersApi, "importDefault").mockResolvedValueOnce(true),
      expectSpy: (spy: MockInstance) =>
        expect(spy).toHaveBeenCalledWith("claude"),
    },
    {
      appId: "codex",
      switchLabel: "switch-codex",
      primeSpy: () =>
        vi.spyOn(providersApi, "importDefault").mockResolvedValueOnce(true),
      expectSpy: (spy: MockInstance) =>
        expect(spy).toHaveBeenCalledWith("codex"),
    },
    {
      appId: "gemini",
      switchLabel: "switch-gemini",
      primeSpy: () =>
        vi.spyOn(providersApi, "importDefault").mockResolvedValueOnce(true),
      expectSpy: (spy: MockInstance) =>
        expect(spy).toHaveBeenCalledWith("gemini"),
    },
    {
      appId: "opencode",
      switchLabel: "switch-opencode",
      primeSpy: () =>
        vi
          .spyOn(providersApi, "importOpenCodeFromLive")
          .mockResolvedValueOnce(1),
      expectSpy: (spy: MockInstance) =>
        expect(spy).toHaveBeenCalledTimes(1),
    },
    {
      appId: "openclaw",
      switchLabel: "switch-openclaw",
      primeSpy: () =>
        vi
          .spyOn(providersApi, "importOpenClawFromLive")
          .mockResolvedValueOnce(1),
      expectSpy: (spy: MockInstance) =>
        expect(spy).toHaveBeenCalledTimes(1),
    },
    {
      appId: "hermes",
      switchLabel: "switch-hermes",
      primeSpy: () =>
        vi
          .spyOn(providersApi, "importHermesFromLive")
          .mockResolvedValueOnce(1),
      expectSpy: (spy: MockInstance) =>
        expect(spy).toHaveBeenCalledTimes(1),
    },
  ])(
    "imports current config from the providers toolbar for $appId",
    async ({ switchLabel, primeSpy, expectSpy }) => {
      const importSpy = primeSpy();

      const { default: App } = await import("@/App");
      renderApp(App);

      await waitFor(() =>
        expect(screen.getByTestId("provider-list")).toBeInTheDocument(),
      );

      if (switchLabel) {
        fireEvent.click(screen.getByText(switchLabel));
      }

      fireEvent.click(
        screen.getByRole("button", { name: "provider.importCurrent" }),
      );

      await waitFor(() => expectSpy(importSpy));

      importSpy.mockRestore();
    },
  );

  it("shows an informational toast when import current config finds no new provider", async () => {
    const importDefaultSpy = vi
      .spyOn(providersApi, "importDefault")
      .mockResolvedValueOnce(false);

    const { default: App } = await import("@/App");
    renderApp(App);

    await waitFor(() =>
      expect(screen.getByTestId("provider-list").textContent).toContain(
        "claude-1",
      ),
    );

    fireEvent.click(
      screen.getByRole("button", { name: "provider.importCurrent" }),
    );

    await waitFor(() =>
      expect(importDefaultSpy).toHaveBeenCalledWith("claude"),
    );
    expect(toastInfoMock).toHaveBeenCalledWith(
      "provider.importCurrentNoChanges",
    );

    importDefaultSpy.mockRestore();
  });

  it("routes the prompts toolbar actions through the PromptPanel handle", async () => {
    const { default: App } = await import("@/App");
    renderApp(App);

    await waitFor(() =>
      expect(screen.getByTestId("provider-list")).toBeInTheDocument(),
    );

    fireEvent.click(screen.getByTitle("prompts.manage"));

    await waitFor(() =>
      expect(screen.getByTestId("prompt-panel")).toBeInTheDocument(),
    );

    fireEvent.click(screen.getByRole("button", { name: "prompts.import" }));
    fireEvent.click(screen.getByRole("button", { name: "prompts.add" }));

    expect(promptOpenImportMock).toHaveBeenCalledTimes(1);
    expect(promptOpenAddMock).toHaveBeenCalledTimes(1);
  });

  it("routes the MCP toolbar actions through the UnifiedMcpPanel handle", async () => {
    const { default: App } = await import("@/App");
    renderApp(App);

    await waitFor(() =>
      expect(screen.getByTestId("provider-list")).toBeInTheDocument(),
    );

    fireEvent.click(screen.getByTitle("mcp.title"));

    await waitFor(() =>
      expect(screen.getByTestId("mcp-panel")).toBeInTheDocument(),
    );

    fireEvent.click(
      screen.getByRole("button", { name: "mcp.importExisting" }),
    );
    fireEvent.click(screen.getByRole("button", { name: "mcp.addMcp" }));

    expect(mcpOpenImportMock).toHaveBeenCalledTimes(1);
    expect(mcpOpenAddMock).toHaveBeenCalledTimes(1);
  });

  it("routes the skills toolbar actions through the UnifiedSkillsPanel handle", async () => {
    const { default: App } = await import("@/App");
    renderApp(App);

    await waitFor(() =>
      expect(screen.getByTestId("provider-list")).toBeInTheDocument(),
    );

    fireEvent.click(screen.getByTitle("skills.manage"));

    await waitFor(() =>
      expect(screen.getByTestId("unified-skills-panel")).toBeInTheDocument(),
    );

    fireEvent.click(
      screen.getByRole("button", { name: "skills.restoreFromBackup.button" }),
    );
    fireEvent.click(
      screen.getByRole("button", { name: "skills.installFromZip.button" }),
    );
    fireEvent.click(screen.getByRole("button", { name: "skills.import" }));

    expect(skillsOpenRestoreFromBackupMock).toHaveBeenCalledTimes(1);
    expect(skillsOpenInstallFromZipMock).toHaveBeenCalledTimes(1);
    expect(skillsOpenImportMock).toHaveBeenCalledTimes(1);
  });

  it("routes the web deeplink import toolbar action through the dialog handle", async () => {
    const { default: App } = await import("@/App");
    renderApp(App);

    await waitFor(() =>
      expect(screen.getByTestId("provider-list")).toBeInTheDocument(),
    );

    fireEvent.click(screen.getByRole("button", { name: "deeplink.pasteImport" }));

    expect(deepLinkOpenManualImportMock).toHaveBeenCalledTimes(1);
  });

  it("opens the universal provider view from supported apps", async () => {
    const { default: App } = await import("@/App");
    renderApp(App);

    await waitFor(() =>
      expect(screen.getByTestId("provider-list")).toBeInTheDocument(),
    );

    fireEvent.click(screen.getByTestId("open-universal-provider-view"));

    await waitFor(() =>
      expect(
        screen.getByTestId("universal-provider-panel"),
      ).toBeInTheDocument(),
    );
  });

  it("hides the universal provider entry for unsupported apps", async () => {
    const { default: App } = await import("@/App");
    renderApp(App);

    await waitFor(() =>
      expect(screen.getByTestId("provider-list")).toBeInTheDocument(),
    );

    expect(
      screen.getByTestId("open-universal-provider-view"),
    ).toBeInTheDocument();

    fireEvent.click(screen.getByText("switch-opencode"));

    await waitFor(() =>
      expect(screen.getByTestId("app-switcher").textContent).toContain(
        "opencode",
      ),
    );

    expect(
      screen.queryByTestId("open-universal-provider-view"),
    ).not.toBeInTheDocument();
  });

  it("shows toast when duplicate cannot load live provider ids", async () => {
    setProviders("openclaw", {
      deepseek: {
        id: "deepseek",
        name: "DeepSeek",
        settingsConfig: {
          baseUrl: "https://api.deepseek.com",
          apiKey: "test-key",
          api: "openai-completions",
          models: [],
        },
        category: "custom",
        sortIndex: 0,
        createdAt: Date.now(),
      },
    });
    setCurrentProviderId("openclaw", "deepseek");

    const liveIdsSpy = vi
      .spyOn(providersApi, "getOpenClawLiveProviderIds")
      .mockRejectedValueOnce(new Error("broken config"));

    const { default: App } = await import("@/App");
    renderApp(App);

    fireEvent.click(screen.getByText("switch-openclaw"));

    await waitFor(() =>
      expect(screen.getByTestId("provider-list").textContent).toContain(
        "deepseek",
      ),
    );

    fireEvent.click(screen.getByText("duplicate"));

    await waitFor(() => {
      expect(toastErrorMock).toHaveBeenCalledWith(
        expect.stringContaining("读取配置中的供应商标识失败"),
      );
    });

    expect(screen.getByTestId("provider-list").textContent).not.toContain(
      "deepseek-copy",
    );

    liveIdsSpy.mockRestore();
  });
});
