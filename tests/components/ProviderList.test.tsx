import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { describe, it, expect, vi, beforeEach } from "vitest";
import type { ReactElement } from "react";
import type { Provider } from "@/types";
import { ProviderList } from "@/components/providers/ProviderList";
import { providersApi } from "@/lib/api/providers";
import * as importCurrentConfigModule from "@/lib/providers/import-current-config";

const toastErrorMock = vi.fn();

vi.mock("sonner", () => ({
  toast: {
    error: (...args: unknown[]) => toastErrorMock(...args),
    success: vi.fn(),
    info: vi.fn(),
  },
}));

const useDragSortMock = vi.fn();
const useSortableMock = vi.fn();
const providerCardRenderSpy = vi.fn();
const useHermesLiveProviderIdsMock = vi.fn();
const useHermesModelConfigMock = vi.fn();
const usePiCurrentStateMock = vi.fn();

vi.mock("@/hooks/useDragSort", () => ({
  useDragSort: (...args: unknown[]) => useDragSortMock(...args),
}));

vi.mock("@/hooks/useHermes", () => ({
  useHermesLiveProviderIds: (...args: unknown[]) =>
    useHermesLiveProviderIdsMock(...args),
  useHermesModelConfig: (...args: unknown[]) =>
    useHermesModelConfigMock(...args),
}));

vi.mock("@/components/providers/ProviderCard", () => ({
  ProviderCard: (props: any) => {
    providerCardRenderSpy(props);
    const {
      provider,
      onSwitch,
      onEdit,
      onDelete,
      onDuplicate,
      onConfigureUsage,
    } = props;

    return (
      <div data-testid={`provider-card-${provider.id}`}>
        <button
          data-testid={`switch-${provider.id}`}
          onClick={() => onSwitch(provider)}
        >
          switch
        </button>
        <button
          data-testid={`edit-${provider.id}`}
          onClick={() => onEdit(provider)}
        >
          edit
        </button>
        <button
          data-testid={`duplicate-${provider.id}`}
          onClick={() => onDuplicate(provider)}
        >
          duplicate
        </button>
        <button
          data-testid={`usage-${provider.id}`}
          onClick={() => onConfigureUsage(provider)}
        >
          usage
        </button>
        <button
          data-testid={`delete-${provider.id}`}
          onClick={() => onDelete(provider)}
        >
          delete
        </button>
        <span data-testid={`is-current-${provider.id}`}>
          {props.isCurrent ? "current" : "inactive"}
        </span>
        <span data-testid={`drag-attr-${provider.id}`}>
          {props.dragHandleProps?.attributes?.["data-dnd-id"] ?? "none"}
        </span>
      </div>
    );
  },
}));

vi.mock("@/components/UsageFooter", () => ({
  default: () => <div data-testid="usage-footer" />,
}));

vi.mock("@dnd-kit/sortable", async () => {
  const actual = await vi.importActual<any>("@dnd-kit/sortable");

  return {
    ...actual,
    useSortable: (...args: unknown[]) => useSortableMock(...args),
  };
});

// Mock hooks that use QueryClient
vi.mock("@/hooks/useStreamCheck", () => ({
  useStreamCheck: () => ({
    checkProvider: vi.fn(),
    isChecking: () => false,
  }),
}));

vi.mock("@/lib/query/failover", () => ({
  useAutoFailoverEnabled: () => ({ data: false }),
  useFailoverQueue: () => ({ data: [] }),
  useAddToFailoverQueue: () => ({ mutate: vi.fn() }),
  useRemoveFromFailoverQueue: () => ({ mutate: vi.fn() }),
  useReorderFailoverQueue: () => ({ mutate: vi.fn() }),
}));

vi.mock("@/lib/query/omo", () => ({
  useCurrentOmoProviderId: () => ({ data: null }),
  useCurrentOmoSlimProviderId: () => ({ data: null }),
}));

vi.mock("@/lib/query/pi", () => ({
  usePiCurrentState: (...args: unknown[]) => usePiCurrentStateMock(...args),
}));

function createProvider(overrides: Partial<Provider> = {}): Provider {
  return {
    id: overrides.id ?? "provider-1",
    name: overrides.name ?? "Test Provider",
    settingsConfig: overrides.settingsConfig ?? {},
    category: overrides.category,
    createdAt: overrides.createdAt,
    sortIndex: overrides.sortIndex,
    meta: overrides.meta,
    websiteUrl: overrides.websiteUrl,
  };
}

function renderWithQueryClient(ui: ReactElement) {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });

  const result = render(
    <QueryClientProvider client={queryClient}>{ui}</QueryClientProvider>,
  );
  return { ...result, queryClient };
}

beforeEach(() => {
  useDragSortMock.mockReset();
  useSortableMock.mockReset();
  providerCardRenderSpy.mockClear();
  toastErrorMock.mockReset();
  useHermesLiveProviderIdsMock.mockReset();
  useHermesModelConfigMock.mockReset();
  usePiCurrentStateMock.mockReset();

  useSortableMock.mockImplementation(({ id }: { id: string }) => ({
    setNodeRef: vi.fn(),
    attributes: { "data-dnd-id": id },
    listeners: { onPointerDown: vi.fn() },
    transform: null,
    transition: null,
    isDragging: false,
  }));

  useDragSortMock.mockReturnValue({
    sortedProviders: [],
    sensors: [],
    handleDragEnd: vi.fn(),
  });
  useHermesLiveProviderIdsMock.mockReturnValue({ data: undefined });
  useHermesModelConfigMock.mockReturnValue({ data: null });
  usePiCurrentStateMock.mockReturnValue({
    data: undefined,
    isSuccess: false,
    isError: false,
    error: null,
  });
});

describe("ProviderList Component", () => {
  beforeEach(() => {
    Object.defineProperty(window, "__TAURI_INTERNALS__", {
      configurable: true,
      value: {},
    });
  });

  it("should render skeleton placeholders when loading", () => {
    const { container } = renderWithQueryClient(
      <ProviderList
        providers={{}}
        currentProviderId=""
        appId="claude"
        onSwitch={vi.fn()}
        onEdit={vi.fn()}
        onDelete={vi.fn()}
        onDuplicate={vi.fn()}
        onOpenWebsite={vi.fn()}
        isLoading
      />,
    );

    const placeholders = container.querySelectorAll(
      ".border-dashed.border-muted-foreground\\/40",
    );
    expect(placeholders).toHaveLength(3);
  });

  it("should show empty state and trigger create callback when no providers exist", () => {
    const handleCreate = vi.fn();
    useDragSortMock.mockReturnValueOnce({
      sortedProviders: [],
      sensors: [],
      handleDragEnd: vi.fn(),
    });

    renderWithQueryClient(
      <ProviderList
        providers={{}}
        currentProviderId=""
        appId="claude"
        onSwitch={vi.fn()}
        onEdit={vi.fn()}
        onDelete={vi.fn()}
        onDuplicate={vi.fn()}
        onOpenWebsite={vi.fn()}
        onCreate={handleCreate}
      />,
    );

    const addButton = screen.getByRole("button", {
      name: "provider.addProvider",
    });
    fireEvent.click(addButton);

    expect(handleCreate).toHaveBeenCalledTimes(1);
  });

  it("imports default config for non-additive apps from the empty state", async () => {
    const importDefaultSpy = vi
      .spyOn(providersApi, "importDefault")
      .mockResolvedValue(true);

    useDragSortMock.mockReturnValueOnce({
      sortedProviders: [],
      sensors: [],
      handleDragEnd: vi.fn(),
    });

    renderWithQueryClient(
      <ProviderList
        providers={{}}
        currentProviderId=""
        appId="claude"
        onSwitch={vi.fn()}
        onEdit={vi.fn()}
        onDelete={vi.fn()}
        onDuplicate={vi.fn()}
        onOpenWebsite={vi.fn()}
      />,
    );

    fireEvent.click(
      screen.getByRole("button", { name: "provider.importCurrent" }),
    );

    await waitFor(() =>
      expect(importDefaultSpy).toHaveBeenCalledWith("claude"),
    );
    importDefaultSpy.mockRestore();
  });

  it("imports OpenClaw live config from the empty state", async () => {
    const importOpenClawSpy = vi
      .spyOn(providersApi, "importOpenClawFromLive")
      .mockResolvedValue(1);

    useDragSortMock.mockReturnValueOnce({
      sortedProviders: [],
      sensors: [],
      handleDragEnd: vi.fn(),
    });

    renderWithQueryClient(
      <ProviderList
        providers={{}}
        currentProviderId=""
        appId="openclaw"
        onSwitch={vi.fn()}
        onEdit={vi.fn()}
        onDelete={vi.fn()}
        onDuplicate={vi.fn()}
        onOpenWebsite={vi.fn()}
      />,
    );

    fireEvent.click(
      screen.getByRole("button", { name: "provider.importCurrent" }),
    );

    await waitFor(() => expect(importOpenClawSpy).toHaveBeenCalledTimes(1));
    importOpenClawSpy.mockRestore();
  });

  it("imports OpenCode live config from the empty state", async () => {
    const importOpenCodeSpy = vi
      .spyOn(providersApi, "importOpenCodeFromLive")
      .mockResolvedValue(1);

    useDragSortMock.mockReturnValueOnce({
      sortedProviders: [],
      sensors: [],
      handleDragEnd: vi.fn(),
    });

    renderWithQueryClient(
      <ProviderList
        providers={{}}
        currentProviderId=""
        appId="opencode"
        onSwitch={vi.fn()}
        onEdit={vi.fn()}
        onDelete={vi.fn()}
        onDuplicate={vi.fn()}
        onOpenWebsite={vi.fn()}
      />,
    );

    fireEvent.click(
      screen.getByRole("button", { name: "provider.importCurrent" }),
    );

    await waitFor(() => expect(importOpenCodeSpy).toHaveBeenCalledTimes(1));
    importOpenCodeSpy.mockRestore();
  });

  it("imports Hermes live config from the empty state", async () => {
    const importHermesSpy = vi
      .spyOn(providersApi, "importHermesFromLive")
      .mockResolvedValue(1);

    useDragSortMock.mockReturnValueOnce({
      sortedProviders: [],
      sensors: [],
      handleDragEnd: vi.fn(),
    });

    renderWithQueryClient(
      <ProviderList
        providers={{}}
        currentProviderId=""
        appId="hermes"
        onSwitch={vi.fn()}
        onEdit={vi.fn()}
        onDelete={vi.fn()}
        onDuplicate={vi.fn()}
        onOpenWebsite={vi.fn()}
      />,
    );

    fireEvent.click(
      screen.getByRole("button", { name: "provider.importCurrent" }),
    );

    await waitFor(() => expect(importHermesSpy).toHaveBeenCalledTimes(1));
    importHermesSpy.mockRestore();
  });

  it("shows serialized import errors and refreshes providers after failure", async () => {
    vi.spyOn(
      importCurrentConfigModule,
      "importCurrentProviderConfig",
    ).mockRejectedValueOnce("current config import failed");

    useDragSortMock.mockReturnValueOnce({
      sortedProviders: [],
      sensors: [],
      handleDragEnd: vi.fn(),
    });

    const { queryClient } = renderWithQueryClient(
      <ProviderList
        providers={{}}
        currentProviderId=""
        appId="claude"
        onSwitch={vi.fn()}
        onEdit={vi.fn()}
        onDelete={vi.fn()}
        onDuplicate={vi.fn()}
        onOpenWebsite={vi.fn()}
      />,
    );
    const invalidateSpy = vi.spyOn(queryClient, "invalidateQueries");

    fireEvent.click(
      screen.getByRole("button", { name: "provider.importCurrent" }),
    );

    await waitFor(() => {
      expect(toastErrorMock).toHaveBeenCalledWith(
        "current config import failed",
      );
      expect(invalidateSpy).toHaveBeenCalledWith({
        queryKey: ["providers", "claude"],
      });
    });
  });

  it("should render in order returned by useDragSort and pass through action callbacks", () => {
    const providerA = createProvider({ id: "a", name: "A" });
    const providerB = createProvider({ id: "b", name: "B" });

    const handleSwitch = vi.fn();
    const handleEdit = vi.fn();
    const handleDelete = vi.fn();
    const handleDuplicate = vi.fn();
    const handleUsage = vi.fn();
    const handleOpenWebsite = vi.fn();

    useDragSortMock.mockReturnValue({
      sortedProviders: [providerB, providerA],
      sensors: [],
      handleDragEnd: vi.fn(),
    });

    renderWithQueryClient(
      <ProviderList
        providers={{ a: providerA, b: providerB }}
        currentProviderId="b"
        appId="claude"
        onSwitch={handleSwitch}
        onEdit={handleEdit}
        onDelete={handleDelete}
        onDuplicate={handleDuplicate}
        onConfigureUsage={handleUsage}
        onOpenWebsite={handleOpenWebsite}
      />,
    );

    // Verify sort order
    expect(providerCardRenderSpy).toHaveBeenCalledTimes(2);
    expect(providerCardRenderSpy.mock.calls[0][0].provider.id).toBe("b");
    expect(providerCardRenderSpy.mock.calls[1][0].provider.id).toBe("a");

    // Verify current provider marker
    expect(providerCardRenderSpy.mock.calls[0][0].isCurrent).toBe(true);

    // Drag attributes from useSortable
    expect(
      providerCardRenderSpy.mock.calls[0][0].dragHandleProps?.attributes[
        "data-dnd-id"
      ],
    ).toBe("b");
    expect(
      providerCardRenderSpy.mock.calls[1][0].dragHandleProps?.attributes[
        "data-dnd-id"
      ],
    ).toBe("a");

    // Trigger action buttons
    fireEvent.click(screen.getByTestId("switch-b"));
    fireEvent.click(screen.getByTestId("edit-b"));
    fireEvent.click(screen.getByTestId("duplicate-b"));
    fireEvent.click(screen.getByTestId("usage-b"));
    fireEvent.click(screen.getByTestId("delete-a"));

    expect(handleSwitch).toHaveBeenCalledWith(providerB);
    expect(handleEdit).toHaveBeenCalledWith(providerB);
    expect(handleDuplicate).toHaveBeenCalledWith(providerB);
    expect(handleUsage).toHaveBeenCalledWith(providerB);
    expect(handleDelete).toHaveBeenCalledWith(providerA);

    // Verify useDragSort call parameters
    expect(useDragSortMock).toHaveBeenCalledWith(
      { a: providerA, b: providerB },
      "claude",
    );
  });

  it("does not mark a Hermes provider as current when model.provider is stale outside live ids", () => {
    const provider = createProvider({ id: "hermes-a", name: "Hermes A" });
    useDragSortMock.mockReturnValue({
      sortedProviders: [provider],
      sensors: [],
      handleDragEnd: vi.fn(),
    });
    useHermesLiveProviderIdsMock.mockReturnValue({ data: [] });
    useHermesModelConfigMock.mockReturnValue({
      data: { provider: "hermes-a", default: "anthropic/claude-sonnet-4" },
    });

    renderWithQueryClient(
      <ProviderList
        providers={{ "hermes-a": provider }}
        currentProviderId="hermes-a"
        appId="hermes"
        onSwitch={vi.fn()}
        onEdit={vi.fn()}
        onDelete={vi.fn()}
        onDuplicate={vi.fn()}
        onOpenWebsite={vi.fn()}
        onSetAsDefault={vi.fn()}
      />,
    );

    const cardProps = providerCardRenderSpy.mock.calls[0][0];
    expect(cardProps.isInConfig).toBe(false);
    expect(cardProps.isCurrent).toBe(false);
    expect(cardProps.isDefaultModel).toBe(false);
  });

  it("marks only the live Hermes model.provider as current and default", () => {
    const providerA = createProvider({ id: "hermes-a", name: "Hermes A" });
    const providerB = createProvider({ id: "hermes-b", name: "Hermes B" });
    useDragSortMock.mockReturnValue({
      sortedProviders: [providerA, providerB],
      sensors: [],
      handleDragEnd: vi.fn(),
    });
    useHermesLiveProviderIdsMock.mockReturnValue({
      data: ["hermes-a", "hermes-b"],
    });
    useHermesModelConfigMock.mockReturnValue({
      data: { provider: "hermes-a", default: "anthropic/claude-sonnet-4" },
    });

    renderWithQueryClient(
      <ProviderList
        providers={{ "hermes-a": providerA, "hermes-b": providerB }}
        currentProviderId="hermes-b"
        appId="hermes"
        onSwitch={vi.fn()}
        onEdit={vi.fn()}
        onDelete={vi.fn()}
        onDuplicate={vi.fn()}
        onOpenWebsite={vi.fn()}
        onSetAsDefault={vi.fn()}
      />,
    );

    const cardAProps = providerCardRenderSpy.mock.calls[0][0];
    const cardBProps = providerCardRenderSpy.mock.calls[1][0];
    expect(cardAProps.isInConfig).toBe(true);
    expect(cardAProps.isCurrent).toBe(true);
    expect(cardAProps.isDefaultModel).toBe(true);
    expect(cardBProps.isInConfig).toBe(true);
    expect(cardBProps.isCurrent).toBe(false);
    expect(cardBProps.isDefaultModel).toBe(false);
  });

  it("derives Pi membership from the native provider state", () => {
    const providerA = createProvider({ id: "pi-a", name: "Pi A" });
    const providerB = createProvider({ id: "pi-b", name: "Pi B" });
    useDragSortMock.mockReturnValue({
      sortedProviders: [providerA, providerB],
      sensors: [],
      handleDragEnd: vi.fn(),
    });
    usePiCurrentStateMock.mockReturnValue({
      data: { enabledProviderIds: ["pi-a"], defaultProviderId: "pi-a" },
      isSuccess: true,
      isError: false,
      error: null,
    });

    renderWithQueryClient(
      <ProviderList
        providers={{ "pi-a": providerA, "pi-b": providerB }}
        currentProviderId="pi-b"
        appId="pi"
        onSwitch={vi.fn()}
        onEdit={vi.fn()}
        onDelete={vi.fn()}
        onDuplicate={vi.fn()}
        onOpenWebsite={vi.fn()}
        isProxyRunning
        isProxyTakeover
        activeProviderId="pi-b"
      />,
    );

    const cardAProps = providerCardRenderSpy.mock.calls[0][0];
    const cardBProps = providerCardRenderSpy.mock.calls[1][0];
    expect(cardAProps.isInConfig).toBe(true);
    expect(cardBProps.isInConfig).toBe(false);
    // Pi has no "current provider" concept and never participates in routing.
    expect(cardAProps.isCurrent).toBe(false);
    expect(cardBProps.isCurrent).toBe(false);
    expect(cardAProps.isStateChangeProtected).toBe(false);
    expect(cardAProps.isProxyRunning).toBe(false);
    expect(cardAProps.isProxyTakeover).toBe(false);
    expect(cardAProps.activeProviderId).toBeUndefined();
  });

  it("freezes Pi state changes and warns when the native state cannot be read", () => {
    const provider = createProvider({ id: "pi-a", name: "Pi A" });
    useDragSortMock.mockReturnValue({
      sortedProviders: [provider],
      sensors: [],
      handleDragEnd: vi.fn(),
    });
    usePiCurrentStateMock.mockReturnValue({
      data: undefined,
      isSuccess: false,
      isError: true,
      error: new Error("models.json unreadable"),
    });

    renderWithQueryClient(
      <ProviderList
        providers={{ "pi-a": provider }}
        currentProviderId=""
        appId="pi"
        onSwitch={vi.fn()}
        onEdit={vi.fn()}
        onDelete={vi.fn()}
        onDuplicate={vi.fn()}
        onOpenWebsite={vi.fn()}
      />,
    );

    const cardProps = providerCardRenderSpy.mock.calls[0][0];
    expect(cardProps.isInConfig).toBe(false);
    expect(cardProps.isStateChangeProtected).toBe(true);

    const notice = screen.getByRole("alert");
    expect(notice).toHaveTextContent("models.json unreadable");
  });

  it("hides create and import affordances on the Pi empty state", () => {
    useDragSortMock.mockReturnValue({
      sortedProviders: [],
      sensors: [],
      handleDragEnd: vi.fn(),
    });

    renderWithQueryClient(
      <ProviderList
        providers={{}}
        currentProviderId=""
        appId="pi"
        onSwitch={vi.fn()}
        onEdit={vi.fn()}
        onDelete={vi.fn()}
        onDuplicate={vi.fn()}
        onOpenWebsite={vi.fn()}
        onCreate={vi.fn()}
      />,
    );

    expect(screen.getByText("pi.empty.title")).toBeInTheDocument();
    expect(screen.queryByRole("button")).not.toBeInTheDocument();
  });

  it("filters providers with the search input", () => {
    const providerAlpha = createProvider({ id: "alpha", name: "Alpha Labs" });
    const providerBeta = createProvider({ id: "beta", name: "Beta Works" });

    useDragSortMock.mockReturnValue({
      sortedProviders: [providerAlpha, providerBeta],
      sensors: [],
      handleDragEnd: vi.fn(),
    });

    renderWithQueryClient(
      <ProviderList
        providers={{ alpha: providerAlpha, beta: providerBeta }}
        currentProviderId=""
        appId="claude"
        onSwitch={vi.fn()}
        onEdit={vi.fn()}
        onDelete={vi.fn()}
        onDuplicate={vi.fn()}
        onOpenWebsite={vi.fn()}
      />,
    );

    fireEvent.keyDown(window, { key: "f", metaKey: true });
    const searchInput = screen.getByPlaceholderText(
      "Search name, notes, or URL...",
    );
    // Initially both providers are rendered
    expect(screen.getByTestId("provider-card-alpha")).toBeInTheDocument();
    expect(screen.getByTestId("provider-card-beta")).toBeInTheDocument();

    fireEvent.change(searchInput, { target: { value: "beta" } });
    expect(screen.queryByTestId("provider-card-alpha")).not.toBeInTheDocument();
    expect(screen.getByTestId("provider-card-beta")).toBeInTheDocument();

    fireEvent.change(searchInput, { target: { value: "gamma" } });
    expect(screen.queryByTestId("provider-card-alpha")).not.toBeInTheDocument();
    expect(screen.queryByTestId("provider-card-beta")).not.toBeInTheDocument();
    expect(
      screen.getByText("No providers match your search."),
    ).toBeInTheDocument();
  });

  it("does not pass onOpenTerminal to provider cards in web mode", () => {
    Object.defineProperty(window, "__TAURI_INTERNALS__", {
      configurable: true,
      value: undefined,
    });

    const provider = createProvider({ id: "alpha", name: "Alpha Labs" });
    useDragSortMock.mockReturnValue({
      sortedProviders: [provider],
      sensors: [],
      handleDragEnd: vi.fn(),
    });

    renderWithQueryClient(
      <ProviderList
        providers={{ alpha: provider }}
        currentProviderId=""
        appId="claude"
        onSwitch={vi.fn()}
        onEdit={vi.fn()}
        onDelete={vi.fn()}
        onDuplicate={vi.fn()}
        onOpenWebsite={vi.fn()}
        onOpenTerminal={vi.fn()}
      />,
    );

    expect(providerCardRenderSpy).toHaveBeenCalled();
    expect(
      providerCardRenderSpy.mock.calls[0][0].onOpenTerminal,
    ).toBeUndefined();
  });
});
