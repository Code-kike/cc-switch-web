import type { ReactNode } from "react";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { UniversalProviderFormModal } from "@/components/universal/UniversalProviderFormModal";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) => key,
  }),
}));

vi.mock("@/components/common/FullScreenPanel", () => ({
  FullScreenPanel: ({
    isOpen,
    title,
    children,
    footer,
  }: {
    isOpen: boolean;
    title: string;
    children: ReactNode;
    footer?: ReactNode;
  }) =>
    isOpen ? (
      <div data-testid="universal-provider-modal">
        <h2>{title}</h2>
        <div>{children}</div>
        <div>{footer}</div>
      </div>
    ) : null,
}));

vi.mock("@/components/ConfirmDialog", () => ({
  ConfirmDialog: ({
    isOpen,
    onConfirm,
    onCancel,
  }: {
    isOpen: boolean;
    onConfirm: () => void;
    onCancel: () => void;
  }) =>
    isOpen ? (
      <div data-testid="universal-provider-sync-confirm">
        <button type="button" onClick={onConfirm}>
          confirm-universal-sync
        </button>
        <button type="button" onClick={onCancel}>
          cancel-universal-sync
        </button>
      </div>
    ) : null,
}));

vi.mock("@/components/ProviderIcon", () => ({
  ProviderIcon: ({ name }: { name: string }) => <span>{name}</span>,
}));

vi.mock("@/components/JsonEditor", () => ({
  default: ({ value }: { value: string }) => <div>{value}</div>,
}));

describe("UniversalProviderFormModal", () => {
  it("keeps the create modal open when save returns false", async () => {
    const onClose = vi.fn();
    const onSave = vi.fn().mockResolvedValue(false);

    render(
      <UniversalProviderFormModal
        isOpen={true}
        onClose={onClose}
        onSave={onSave}
      />,
    );

    fireEvent.change(screen.getByLabelText("universalProvider.baseUrl"), {
      target: { value: "https://api.example.com" },
    });
    fireEvent.change(screen.getByLabelText("universalProvider.apiKey"), {
      target: { value: "secret-key" },
    });

    fireEvent.click(screen.getByRole("button", { name: "common.add" }));

    await waitFor(() =>
      expect(onSave).toHaveBeenCalledWith(
        expect.objectContaining({
          name: "NewAPI",
          baseUrl: "https://api.example.com",
        }),
      ),
    );

    expect(onClose).not.toHaveBeenCalled();
    expect(
      screen.getByTestId("universal-provider-modal"),
    ).toBeInTheDocument();
  });

  it("saves edit mode without opening the sync confirmation", async () => {
    const onClose = vi.fn();
    const onSave = vi.fn().mockResolvedValue(true);
    const onSaveAndSync = vi.fn();

    render(
      <UniversalProviderFormModal
        isOpen={true}
        onClose={onClose}
        onSave={onSave}
        onSaveAndSync={onSaveAndSync}
        editingProvider={{
          id: "universal-1",
          name: "Existing Universal",
          providerType: "newapi",
          baseUrl: "https://existing.example.com",
          apiKey: "secret",
          apps: { claude: true, codex: true, gemini: false },
          models: {},
          createdAt: 1,
        }}
      />,
    );

    fireEvent.change(screen.getByLabelText("universalProvider.name"), {
      target: { value: "Existing Universal Edited" },
    });

    fireEvent.click(screen.getByRole("button", { name: "common.save" }));

    await waitFor(() =>
      expect(onSave).toHaveBeenCalledWith(
        expect.objectContaining({
          id: "universal-1",
          name: "Existing Universal Edited",
        }),
      ),
    );

    expect(onSaveAndSync).not.toHaveBeenCalled();
    expect(
      screen.queryByTestId("universal-provider-sync-confirm"),
    ).not.toBeInTheDocument();
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("keeps the edit sync confirmation open when save-and-sync returns false", async () => {
    const onClose = vi.fn();
    const onSave = vi.fn();
    const onSaveAndSync = vi.fn().mockResolvedValue(false);

    render(
      <UniversalProviderFormModal
        isOpen={true}
        onClose={onClose}
        onSave={onSave}
        onSaveAndSync={onSaveAndSync}
        editingProvider={{
          id: "universal-1",
          name: "Existing Universal",
          providerType: "newapi",
          baseUrl: "https://existing.example.com",
          apiKey: "secret",
          apps: { claude: true, codex: true, gemini: false },
          models: {},
          createdAt: 1,
        }}
      />,
    );

    fireEvent.click(
      screen.getAllByRole("button", { name: "universalProvider.saveAndSync" })[0],
    );

    expect(
      screen.getByTestId("universal-provider-sync-confirm"),
    ).toBeInTheDocument();

    fireEvent.click(screen.getByText("confirm-universal-sync"));

    await waitFor(() =>
      expect(onSaveAndSync).toHaveBeenCalledWith(
        expect.objectContaining({
          id: "universal-1",
          name: "Existing Universal",
        }),
      ),
    );

    expect(onClose).not.toHaveBeenCalled();
    expect(
      screen.getByTestId("universal-provider-sync-confirm"),
    ).toBeInTheDocument();
  });
});
