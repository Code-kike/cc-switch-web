import type { ReactNode } from "react";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { Provider } from "@/types";
import UsageScriptModal from "@/components/UsageScriptModal";

const toastErrorMock = vi.fn();
const toastSuccessMock = vi.fn();
const usageApiTestScriptMock = vi.fn();
const prettierFormatMock = vi.fn();

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) => key,
  }),
}));

vi.mock("sonner", () => ({
  toast: {
    error: (...args: unknown[]) => toastErrorMock(...args),
    success: (...args: unknown[]) => toastSuccessMock(...args),
    info: vi.fn(),
    warning: vi.fn(),
  },
}));

vi.mock("@/lib/query", () => ({
  useSettingsQuery: () => ({
    data: { usageConfirmed: true },
  }),
}));

vi.mock("@/lib/api", () => ({
  usageApi: {
    testScript: (...args: unknown[]) => usageApiTestScriptMock(...args),
  },
  settingsApi: {
    save: vi.fn(),
  },
}));

vi.mock("prettier/standalone", () => ({
  format: (...args: unknown[]) => prettierFormatMock(...args),
}));

vi.mock("@/lib/api/copilot", () => ({
  copilotGetUsage: vi.fn(),
  copilotGetUsageForAccount: vi.fn(),
}));

vi.mock("@/components/common/FullScreenPanel", () => ({
  FullScreenPanel: ({
    isOpen,
    children,
    footer,
  }: {
    isOpen: boolean;
    children: ReactNode;
    footer?: ReactNode;
  }) =>
    isOpen ? (
      <div>
        {children}
        {footer}
      </div>
    ) : null,
}));

vi.mock("@/components/ConfirmDialog", () => ({
  ConfirmDialog: () => null,
}));

vi.mock("@/components/ui/button", () => ({
  Button: ({ children, ...props }: any) => <button {...props}>{children}</button>,
}));

vi.mock("@/components/ui/input", () => ({
  Input: (props: any) => <input {...props} />,
}));

vi.mock("@/components/ui/label", () => ({
  Label: ({ children, ...props }: any) => <label {...props}>{children}</label>,
}));

vi.mock("@/components/ui/switch", () => ({
  Switch: ({ checked, onCheckedChange, ...props }: any) => (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      onClick={() => onCheckedChange?.(!checked)}
      {...props}
    />
  ),
}));

vi.mock("@/components/JsonEditor", () => ({
  default: ({ value, onChange }: any) => (
    <textarea value={value} onChange={(event) => onChange(event.target.value)} />
  ),
}));

function renderModal(providerOverrides: Partial<Provider> = {}) {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  });

  const provider: Provider = {
    id: "provider-1",
    name: "Test Provider",
    settingsConfig: {},
    meta: {
      usage_script: {
        enabled: true,
        language: "javascript",
        code: "return { remaining: 1, unit: 'USD' }",
        apiKey: "key",
        baseUrl: "https://api.example.com",
        timeout: 10,
      },
    },
    ...providerOverrides,
  };

  return render(
    <QueryClientProvider client={queryClient}>
      <UsageScriptModal
        provider={provider}
        appId="claude"
        isOpen={true}
        onClose={vi.fn()}
        onSave={vi.fn()}
      />
    </QueryClientProvider>,
  );
}

describe("UsageScriptModal", () => {
  beforeEach(() => {
    toastErrorMock.mockReset();
    toastSuccessMock.mockReset();
    usageApiTestScriptMock.mockReset();
    prettierFormatMock.mockReset();
  });

  it("shows structured detail when usage-script testing throws", async () => {
    usageApiTestScriptMock.mockRejectedValueOnce({ detail: "usage test exploded" });
    renderModal();

    fireEvent.click(screen.getByRole("button", { name: "usageScript.testScript" }));

    await waitFor(() => {
      expect(toastErrorMock).toHaveBeenCalledWith(
        "usageScript.testFailed: usage test exploded",
        { duration: 5000 },
      );
    });
  });

  it("shows structured detail when formatting throws", async () => {
    prettierFormatMock.mockRejectedValueOnce({ detail: "format exploded" });
    renderModal();

    fireEvent.click(screen.getByTitle("usageScript.format"));

    await waitFor(() => {
      expect(toastErrorMock).toHaveBeenCalledWith(
        "usageScript.formatFailed: format exploded",
        { duration: 3000 },
      );
    });
  });
});
