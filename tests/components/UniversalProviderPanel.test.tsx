import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { UniversalProviderPanel } from "@/components/universal/UniversalProviderPanel";

const toastSuccessMock = vi.fn();
const toastErrorMock = vi.fn();

const getAllMock = vi.fn();
const upsertMock = vi.fn();
const syncMock = vi.fn();
const deleteMock = vi.fn();

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) => key,
  }),
}));

vi.mock("sonner", () => ({
  toast: {
    success: (...args: unknown[]) => toastSuccessMock(...args),
    error: (...args: unknown[]) => toastErrorMock(...args),
  },
}));

vi.mock("@/lib/api", () => ({
  universalProvidersApi: {
    getAll: (...args: unknown[]) => getAllMock(...args),
    upsert: (...args: unknown[]) => upsertMock(...args),
    sync: (...args: unknown[]) => syncMock(...args),
    delete: (...args: unknown[]) => deleteMock(...args),
  },
}));

vi.mock("@/components/universal/UniversalProviderCard", () => ({
  UniversalProviderCard: ({
    provider,
    onEdit,
    onDelete,
    onSync,
  }: {
    provider: { id: string; name: string };
    onEdit: (provider: { id: string; name: string }) => void;
    onDelete: (id: string) => void;
    onSync: (id: string) => void;
  }) => (
    <div data-testid={`universal-card-${provider.id}`}>
      <span>{provider.name}</span>
      <button type="button" onClick={() => onEdit(provider)}>
        edit-{provider.id}
      </button>
      <button type="button" onClick={() => onDelete(provider.id)}>
        delete-{provider.id}
      </button>
      <button type="button" onClick={() => onSync(provider.id)}>
        sync-{provider.id}
      </button>
    </div>
  ),
}));

vi.mock("@/components/universal/UniversalProviderFormModal", () => ({
  UniversalProviderFormModal: ({
    isOpen,
    editingProvider,
    onSave,
    onClose,
  }: {
    isOpen: boolean;
    editingProvider?: { id: string; name: string } | null;
    onSave: (provider: any) => void;
    onClose: () => void;
  }) =>
    isOpen ? (
      <div data-testid="universal-provider-form">
        <div>
          {editingProvider ? `editing:${editingProvider.name}` : "creating"}
        </div>
        <button
          type="button"
          onClick={() =>
            onSave(
              editingProvider
                ? {
                    id: editingProvider.id,
                    name: `${editingProvider.name}-edited`,
                    providerType: "newapi",
                    baseUrl: "https://edited.example.com",
                    apiKey: "edited-key",
                    apps: { claude: true, codex: true, gemini: false },
                    models: {},
                    createdAt: 1,
                  }
                : {
                    id: "universal-new",
                    name: "Universal New",
                    providerType: "newapi",
                    baseUrl: "https://api.example.com",
                    apiKey: "secret",
                    apps: { claude: true, codex: true, gemini: true },
                    models: {},
                    createdAt: 1,
                  },
            )
          }
        >
          submit-universal-provider
        </button>
        <button type="button" onClick={onClose}>
          close-universal-provider
        </button>
      </div>
    ) : null,
}));

vi.mock("@/components/ConfirmDialog", () => ({
  ConfirmDialog: ({ isOpen, onConfirm, onCancel, title }: any) =>
    isOpen ? (
      <div data-testid={`confirm-${title}`}>
        <button type="button" onClick={onConfirm}>
          confirm
        </button>
        <button type="button" onClick={onCancel}>
          cancel
        </button>
      </div>
    ) : null,
}));

describe("UniversalProviderPanel", () => {
  beforeEach(() => {
    toastSuccessMock.mockReset();
    toastErrorMock.mockReset();
    getAllMock.mockReset();
    upsertMock.mockReset();
    syncMock.mockReset();
    deleteMock.mockReset();
  });

  it("opens the create form from the empty state and auto-syncs new providers", async () => {
    getAllMock.mockResolvedValue({
      "universal-new": {
        id: "universal-new",
        name: "Universal New",
        providerType: "newapi",
        baseUrl: "https://api.example.com",
        apiKey: "secret",
        apps: { claude: true, codex: true, gemini: true },
        models: {},
        createdAt: 1,
      },
    });
    getAllMock.mockResolvedValueOnce({});
    upsertMock.mockResolvedValue(true);
    syncMock.mockResolvedValue(true);

    render(<UniversalProviderPanel />);

    await waitFor(() => {
      expect(screen.getByText("universalProvider.empty")).toBeInTheDocument();
    });

    fireEvent.click(
      screen.getAllByRole("button", { name: "universalProvider.add" })[0],
    );

    expect(screen.getByTestId("universal-provider-form")).toBeInTheDocument();
    expect(screen.getByText("creating")).toBeInTheDocument();

    fireEvent.click(screen.getByText("submit-universal-provider"));

    await waitFor(() => {
      expect(upsertMock).toHaveBeenCalledWith(
        expect.objectContaining({
          id: "universal-new",
          name: "Universal New",
        }),
      );
    });
    await waitFor(() => {
      expect(syncMock).toHaveBeenCalledWith("universal-new");
    });
    await waitFor(() => {
      expect(screen.getByText("Universal New")).toBeInTheDocument();
    });

    expect(toastSuccessMock).toHaveBeenCalledWith(
      "universalProvider.addedAndSynced",
    );
  });

  it("opens the edit form and updates without re-syncing automatically", async () => {
    getAllMock.mockResolvedValue({
      existing: {
        id: "existing",
        name: "Existing Provider-edited",
        providerType: "newapi",
        baseUrl: "https://edited.example.com",
        apiKey: "edited-key",
        apps: { claude: true, codex: true, gemini: false },
        models: {},
        createdAt: 1,
      },
    });
    getAllMock.mockResolvedValueOnce({
      existing: {
        id: "existing",
        name: "Existing Provider",
        providerType: "newapi",
        baseUrl: "https://existing.example.com",
        apiKey: "secret",
        apps: { claude: true, codex: false, gemini: false },
        models: {},
        createdAt: 1,
      },
    });
    upsertMock.mockResolvedValue(true);
    syncMock.mockResolvedValue(true);

    render(<UniversalProviderPanel />);

    await waitFor(() => {
      expect(screen.getByText("Existing Provider")).toBeInTheDocument();
    });

    fireEvent.click(screen.getByText("edit-existing"));
    expect(screen.getByTestId("universal-provider-form")).toBeInTheDocument();

    fireEvent.click(screen.getByText("submit-universal-provider"));

    await waitFor(() => {
      expect(upsertMock).toHaveBeenCalledWith(
        expect.objectContaining({
          id: "existing",
        }),
      );
    });
    await waitFor(() => {
      expect(screen.getByText("Existing Provider-edited")).toBeInTheDocument();
    });

    expect(syncMock).not.toHaveBeenCalled();
    expect(toastSuccessMock).toHaveBeenCalledWith("universalProvider.updated");
  });
});
