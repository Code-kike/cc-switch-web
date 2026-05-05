import { createRef } from "react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import {
  act,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { afterAll, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";

import "@/lib/api/web-commands";
import { setCsrfToken } from "@/lib/api/adapter";
import {
  DeepLinkImportDialog,
  type DeepLinkImportDialogHandle,
} from "@/components/DeepLinkImportDialog";
import { server } from "../msw/server";
import {
  startTestWebServer,
  type TestWebServer,
} from "../helpers/web-server";

const toastSuccessMock = vi.fn();
const toastErrorMock = vi.fn();
const toastWarningMock = vi.fn();

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

describe.sequential("DeepLinkImportDialog against real web server", () => {
  let webServer: TestWebServer;

  beforeAll(async () => {
    server.close();
    webServer = await startTestWebServer();
  }, 360_000);

  afterAll(async () => {
    await webServer.stop();
    server.listen({ onUnhandledRequest: "warn" });
  }, 20_000);

  beforeEach(() => {
    toastSuccessMock.mockReset();
    toastErrorMock.mockReset();
    toastWarningMock.mockReset();
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

  it(
    "parses and imports a provider deep link against the real web server",
    async () => {
      const { ref } = renderDialog();
      const deeplinkUrl =
        "ccswitch://v1/import?resource=provider&app=openclaw&name=Real%20Web%20DeepLink&endpoint=https%3A%2F%2Freal-web-deeplink.example.com%2Fv1&apiKey=sk-real-web";

      act(() => {
        ref.current?.openManualImport();
      });

      fireEvent.change(
        screen.getByPlaceholderText("deeplink.pasteImportPlaceholder"),
        {
          target: {
            value: deeplinkUrl,
          },
        },
      );
      fireEvent.click(screen.getByText("deeplink.parseAction"));

      await waitFor(() =>
        expect(screen.getByText("Real Web DeepLink")).toBeInTheDocument(),
      );

      fireEvent.click(screen.getByText("deeplink.import"));

      await waitFor(() =>
        expect(toastSuccessMock).toHaveBeenCalledWith(
          "deeplink.importSuccess",
          expect.objectContaining({ closeButton: true }),
        ),
      );

      const response = await fetch(
        new URL("/api/providers/get-providers?app=openclaw", webServer.baseUrl),
      );
      const providers = (await response.json()) as Record<
        string,
        { name?: string; settingsConfig?: { baseUrl?: string } }
      >;

      expect(Object.values(providers)).toEqual(
        expect.arrayContaining([
          expect.objectContaining({
            name: "Real Web DeepLink",
            settingsConfig: expect.objectContaining({
              baseUrl: "https://real-web-deeplink.example.com/v1",
            }),
          }),
        ]),
      );
      expect(
        screen.queryByPlaceholderText("deeplink.pasteImportPlaceholder"),
      ).not.toBeInTheDocument();
      expect(toastErrorMock).not.toHaveBeenCalled();
    },
    20_000,
  );
});
