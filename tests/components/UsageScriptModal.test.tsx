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

vi.mock("@/lib/api/subscription", () => ({
  subscriptionApi: {
    getBalance: vi.fn(),
    getCodingPlanQuota: vi.fn(),
  },
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
  Button: ({ children, ...props }: any) => (
    <button {...props}>{children}</button>
  ),
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
    <textarea
      data-testid="json-editor"
      value={value}
      onChange={(event) => onChange(event.target.value)}
    />
  ),
}));

function renderModal(
  providerOverrides: Partial<Provider> = {},
  appId: "claude" | "hermes" = "claude",
) {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  });

  const provider: Provider = {
    id: "provider-1",
    name: "Test Provider",
    // A third-party base URL keeps the official-subscription auto-detection
    // (isOfficialSubscriptionProvider) from hijacking custom-script fixtures:
    // credential-less Claude providers now default to the official template.
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: "https://api.example.com",
        ANTHROPIC_AUTH_TOKEN: "key",
      },
    },
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
        appId={appId}
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
    usageApiTestScriptMock.mockRejectedValueOnce({
      detail: "usage test exploded",
    });
    renderModal();

    fireEvent.click(
      screen.getByRole("button", { name: "usageScript.testScript" }),
    );

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

  it("tests the balance template through the unified usage API", async () => {
    usageApiTestScriptMock.mockResolvedValueOnce({
      success: true,
      data: [{ planName: "Hermes", remaining: 12.34, unit: "USD" }],
    });

    renderModal(
      {
        settingsConfig: {
          api_key: "hermes-key",
          base_url: "https://api.deepseek.com",
        },
        meta: {
          usage_script: {
            enabled: true,
            language: "javascript",
            code: "",
            timeout: 10,
            templateType: "balance",
          },
        },
      },
      "hermes",
    );

    fireEvent.click(
      screen.getByRole("button", { name: "usageScript.testScript" }),
    );

    await waitFor(() => {
      expect(usageApiTestScriptMock).toHaveBeenCalledWith(
        "provider-1",
        "hermes",
        "",
        10,
        undefined,
        undefined,
        undefined,
        undefined,
        "balance",
      );
    });
  });

  it("does not show the script editor for the balance template", () => {
    renderModal(
      {
        meta: {
          usage_script: {
            enabled: true,
            language: "javascript",
            code: "ignored code",
            timeout: 10,
            templateType: "balance",
          },
        },
      },
      "hermes",
    );

    expect(screen.queryByTestId("json-editor")).not.toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "usageScript.format" }),
    ).not.toBeInTheDocument();
  });

  it("tests the token-plan template through the unified usage API", async () => {
    usageApiTestScriptMock.mockResolvedValueOnce({
      success: true,
      data: [
        {
          planName: "weekly",
          remaining: 25,
          total: 100,
          used: 75,
          unit: "%",
        },
      ],
    });

    renderModal({
      settingsConfig: {
        env: {
          ANTHROPIC_AUTH_TOKEN: "coding-key",
          ANTHROPIC_BASE_URL: "https://api.kimi.com/coding",
        },
      },
      meta: {
        usage_script: {
          enabled: true,
          language: "javascript",
          code: "",
          timeout: 10,
          templateType: "token_plan",
        },
      },
    });

    fireEvent.click(
      screen.getByRole("button", { name: "usageScript.testScript" }),
    );

    await waitFor(() => {
      expect(usageApiTestScriptMock).toHaveBeenCalledWith(
        "provider-1",
        "claude",
        "",
        10,
        undefined,
        undefined,
        undefined,
        undefined,
        "token_plan",
      );
    });
    expect(toastSuccessMock).toHaveBeenCalledWith(
      "usageScript.testSuccessweekly: 75%",
      {
        duration: 3000,
        closeButton: true,
      },
    );
  });

  it("tests the GitHub Copilot template through the unified usage API", async () => {
    usageApiTestScriptMock.mockResolvedValueOnce({
      success: true,
      data: [
        {
          planName: "Copilot Pro",
          remaining: 42,
          total: 50,
          used: 8,
          unit: "requests",
          extra: "Reset: 2026-05-31",
        },
      ],
    });

    renderModal({
      meta: {
        providerType: "github_copilot",
        authBinding: {
          source: "managed_account",
          authProvider: "github_copilot",
          accountId: "github-account-1",
        },
        usage_script: {
          enabled: true,
          language: "javascript",
          code: "",
          timeout: 10,
          templateType: "github_copilot",
        },
      },
    });

    fireEvent.click(
      screen.getByRole("button", { name: "usageScript.testScript" }),
    );

    await waitFor(() => {
      expect(usageApiTestScriptMock).toHaveBeenCalledWith(
        "provider-1",
        "claude",
        "",
        10,
        undefined,
        undefined,
        undefined,
        undefined,
        "github_copilot",
      );
    });
    expect(toastSuccessMock).toHaveBeenCalledWith(
      "usageScript.testSuccess[Copilot Pro] usage.remaining 42/50 (Reset: 2026-05-31)",
      {
        duration: 3000,
        closeButton: true,
      },
    );
  });
});
