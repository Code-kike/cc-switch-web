import { createRef } from "react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import {
  act,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { afterAll, beforeEach, describe, expect, it, vi } from "vitest";

import {
  DeepLinkImportDialog,
  type DeepLinkImportDialogHandle,
} from "@/components/DeepLinkImportDialog";

const parseDeeplinkMock = vi.fn();
const mergeDeeplinkConfigMock = vi.fn();
const importFromDeeplinkMock = vi.fn();
const toastSuccessMock = vi.fn();
const toastErrorMock = vi.fn();
const toastWarningMock = vi.fn();
const consoleErrorSpy = vi
  .spyOn(console, "error")
  .mockImplementation(() => undefined);

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) => key,
  }),
}));

vi.mock("sonner", () => ({
  toast: {
    success: (...args: unknown[]) => toastSuccessMock(...args),
    error: (...args: unknown[]) => toastErrorMock(...args),
    warning: (...args: unknown[]) => toastWarningMock(...args),
  },
}));

vi.mock("@/lib/api/event-adapter", () => ({
  listen: vi.fn(async () => () => undefined),
}));

vi.mock("@/lib/api/deeplink", () => ({
  deeplinkApi: {
    parseDeeplink: (...args: unknown[]) => parseDeeplinkMock(...args),
    mergeDeeplinkConfig: (...args: unknown[]) =>
      mergeDeeplinkConfigMock(...args),
    importFromDeeplink: (...args: unknown[]) => importFromDeeplinkMock(...args),
  },
}));

vi.mock("@/components/ui/dialog", () => ({
  Dialog: ({ open, children }: { open: boolean; children: React.ReactNode }) =>
    open ? <div>{children}</div> : null,
  DialogContent: ({ children }: { children: React.ReactNode }) => (
    <div>{children}</div>
  ),
  DialogHeader: ({ children }: { children: React.ReactNode }) => (
    <div>{children}</div>
  ),
  DialogTitle: ({ children }: { children: React.ReactNode }) => (
    <h1>{children}</h1>
  ),
  DialogDescription: ({ children }: { children: React.ReactNode }) => (
    <p>{children}</p>
  ),
  DialogFooter: ({ children }: { children: React.ReactNode }) => (
    <div>{children}</div>
  ),
}));

const renderDialog = () => {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  });
  const ref = createRef<DeepLinkImportDialogHandle>();

  render(
    <QueryClientProvider client={queryClient}>
      <DeepLinkImportDialog ref={ref} />
    </QueryClientProvider>,
  );

  return { ref };
};

describe("DeepLinkImportDialog Web paste flow", () => {
  afterAll(() => {
    consoleErrorSpy.mockRestore();
  });

  beforeEach(() => {
    parseDeeplinkMock.mockReset();
    mergeDeeplinkConfigMock.mockReset();
    importFromDeeplinkMock.mockReset();
    toastSuccessMock.mockReset();
    toastErrorMock.mockReset();
    toastWarningMock.mockReset();
  });

  it("parses a pasted link and imports the merged request", async () => {
    const parsedRequest = {
      version: "v1",
      resource: "provider" as const,
      app: "openclaw" as const,
      name: "Demo Provider",
      configUrl: "https://example.com/provider.json",
    };
    const mergedRequest = {
      ...parsedRequest,
      endpoint: "https://api.example.com",
      apiKey: "sk-demo",
    };

    parseDeeplinkMock.mockResolvedValue(parsedRequest);
    mergeDeeplinkConfigMock.mockResolvedValue(mergedRequest);
    importFromDeeplinkMock.mockResolvedValue({ type: "provider", id: "demo" });

    const { ref } = renderDialog();

    act(() => {
      ref.current?.openManualImport();
    });

    fireEvent.change(
      screen.getByPlaceholderText("deeplink.pasteImportPlaceholder"),
      {
        target: {
          value: "ccswitch://v1/import?resource=provider&app=openclaw",
        },
      },
    );
    fireEvent.click(screen.getByText("deeplink.parseAction"));

    await waitFor(() =>
      expect(parseDeeplinkMock).toHaveBeenCalledWith(
        "ccswitch://v1/import?resource=provider&app=openclaw",
      ),
    );
    await waitFor(() =>
      expect(mergeDeeplinkConfigMock).toHaveBeenCalledWith(parsedRequest),
    );
    await waitFor(() =>
      expect(screen.getByText("Demo Provider")).toBeInTheDocument(),
    );

    fireEvent.click(screen.getByText("deeplink.import"));

    await waitFor(() =>
      expect(importFromDeeplinkMock).toHaveBeenCalledWith(mergedRequest),
    );
  });

  it("keeps the paste dialog open when parsing fails with structured detail", async () => {
    parseDeeplinkMock.mockRejectedValue({ message: "bad link" });

    const { ref } = renderDialog();

    act(() => {
      ref.current?.openManualImport("ccswitch://bad");
    });

    fireEvent.click(screen.getByText("deeplink.parseAction"));

    await waitFor(() =>
      expect(toastErrorMock).toHaveBeenCalledWith(
        "deeplink.parseError",
        expect.objectContaining({
          description: "bad link",
        }),
      ),
    );

    expect(
      screen.getByPlaceholderText("deeplink.pasteImportPlaceholder"),
    ).toBeInTheDocument();
    expect(screen.queryByText(/\\[object Object\\]/)).not.toBeInTheDocument();
    expect(importFromDeeplinkMock).not.toHaveBeenCalled();
  });

  it("falls back to the parsed request when config merge fails and shows structured detail", async () => {
    const parsedRequest = {
      version: "v1",
      resource: "provider" as const,
      app: "openclaw" as const,
      name: "Merge Fallback Provider",
      configUrl: "https://example.com/provider.json",
    };

    parseDeeplinkMock.mockResolvedValue(parsedRequest);
    mergeDeeplinkConfigMock.mockRejectedValue({ message: "merge failed" });

    const { ref } = renderDialog();

    act(() => {
      ref.current?.openManualImport("ccswitch://merge-fail");
    });

    fireEvent.click(screen.getByText("deeplink.parseAction"));

    await waitFor(() =>
      expect(toastErrorMock).toHaveBeenCalledWith(
        "deeplink.configMergeError",
        expect.objectContaining({
          description: "merge failed",
        }),
      ),
    );
    expect(
      await screen.findByText("Merge Fallback Provider"),
    ).toBeInTheDocument();
  });

  it("keeps the confirmation open when import fails with structured detail", async () => {
    const parsedRequest = {
      version: "v1",
      resource: "provider" as const,
      app: "openclaw" as const,
      name: "Broken Import Provider",
    };

    parseDeeplinkMock.mockResolvedValue(parsedRequest);
    importFromDeeplinkMock.mockRejectedValue({ message: "import failed" });

    const { ref } = renderDialog();

    act(() => {
      ref.current?.openManualImport("ccswitch://import-fail");
    });

    fireEvent.click(screen.getByText("deeplink.parseAction"));

    await waitFor(() =>
      expect(screen.getByText("Broken Import Provider")).toBeInTheDocument(),
    );

    fireEvent.click(screen.getByText("deeplink.import"));

    await waitFor(() =>
      expect(toastErrorMock).toHaveBeenCalledWith(
        "deeplink.importError",
        expect.objectContaining({
          description: "import failed",
        }),
      ),
    );
    expect(screen.getByText("Broken Import Provider")).toBeInTheDocument();
  });
});
