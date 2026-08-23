import { render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { Provider } from "@/types";
import { ProviderCard } from "@/components/providers/ProviderCard";

const useProviderHealthMock = vi.fn();
const useUsageQueryMock = vi.fn();
const useProviderLimitsMock = vi.fn();
const useProviderStatsMock = vi.fn();
const providerActionsPropsSpy = vi.fn();

vi.mock("@/components/providers/ProviderActions", () => ({
  ProviderActions: (props: any) => {
    providerActionsPropsSpy(props);
    return <div data-testid="provider-actions" />;
  },
}));

vi.mock("@/components/ProviderIcon", () => ({
  ProviderIcon: () => <div data-testid="provider-icon" />,
}));

vi.mock("@/components/UsageFooter", () => ({
  default: () => <div data-testid="usage-footer" />,
}));

vi.mock("@/components/SubscriptionQuotaFooter", () => ({
  default: () => <div data-testid="subscription-footer" />,
}));

vi.mock("@/components/CopilotQuotaFooter", () => ({
  default: () => <div data-testid="copilot-footer" />,
}));

vi.mock("@/components/CodexOauthQuotaFooter", () => ({
  default: () => <div data-testid="codex-oauth-footer" />,
}));

vi.mock("@/components/XaiOauthQuotaFooter", () => ({
  default: () => <div data-testid="xai-oauth-footer" />,
}));

vi.mock("@/lib/query/failover", () => ({
  useProviderHealth: (...args: unknown[]) => useProviderHealthMock(...args),
}));

vi.mock("@/lib/query/queries", () => ({
  useUsageQuery: (...args: unknown[]) => useUsageQueryMock(...args),
}));

vi.mock("@/lib/query/usage", () => ({
  useProviderLimits: (...args: unknown[]) => useProviderLimitsMock(...args),
  useProviderStats: (...args: unknown[]) => useProviderStatsMock(...args),
}));

vi.mock("@/components/providers/forms/hooks/useManagedAuth", () => ({
  useManagedAuth: () => ({
    accounts: [],
    authStatus: undefined,
    isLoadingStatus: false,
  }),
}));

function createProvider(overrides: Partial<Provider> = {}): Provider {
  return {
    id: overrides.id ?? "provider-1",
    name: overrides.name ?? "Test Provider",
    category: overrides.category ?? "custom",
    settingsConfig: overrides.settingsConfig ?? {
      env: {
        ANTHROPIC_BASE_URL: "https://provider.example.com",
      },
    },
    meta: overrides.meta,
    websiteUrl: overrides.websiteUrl,
    notes: overrides.notes,
    icon: overrides.icon,
    iconColor: overrides.iconColor,
    sortIndex: overrides.sortIndex,
    createdAt: overrides.createdAt,
  };
}

const baseProps = {
  isCurrent: false,
  appId: "claude" as const,
  onSwitch: vi.fn(),
  onEdit: vi.fn(),
  onDelete: vi.fn(),
  onConfigureUsage: vi.fn(),
  onOpenWebsite: vi.fn(),
  onDuplicate: vi.fn(),
  isProxyRunning: false,
};

describe("ProviderCard", () => {
  beforeEach(() => {
    useProviderHealthMock.mockReset();
    useUsageQueryMock.mockReset();
    useProviderLimitsMock.mockReset();
    useProviderStatsMock.mockReset();
    providerActionsPropsSpy.mockReset();

    useProviderHealthMock.mockReturnValue({ data: undefined });
    useUsageQueryMock.mockReturnValue({ data: undefined });
    useProviderLimitsMock.mockReturnValue({ data: undefined });
    useProviderStatsMock.mockReturnValue({ data: [] });
  });

  it("renders health and provider limit diagnostics when configured", () => {
    const provider = createProvider({
      name: "Claude Diagnostics",
      meta: {
        limitDailyUsd: "1.25",
        limitMonthlyUsd: "9.5",
      },
    });

    useProviderHealthMock.mockReturnValue({
      data: {
        provider_id: provider.id,
        app_type: "claude",
        is_healthy: true,
        consecutive_failures: 2,
        last_success_at: null,
        last_failure_at: null,
        last_error: null,
        updated_at: new Date().toISOString(),
      },
    });
    useProviderLimitsMock.mockReturnValue({
      data: {
        providerId: provider.id,
        dailyUsage: "0.125",
        dailyLimit: "1.25",
        dailyExceeded: false,
        monthlyUsage: "0.5",
        monthlyLimit: "9.50",
        monthlyExceeded: false,
      },
    });
    useProviderStatsMock.mockReturnValue({
      data: [
        {
          providerId: provider.id,
          providerName: provider.name,
          requestCount: 2,
          totalTokens: 300,
          totalCost: "0.03",
          successRate: 100,
          avgLatencyMs: 180,
        },
      ],
    });

    render(
      <ProviderCard
        {...baseProps}
        provider={provider}
        isProxyRunning={true}
        isInFailoverQueue={true}
      />,
    );

    expect(screen.getByText("降级")).toBeInTheDocument();
    expect(screen.getByText(/30天 2 次 \/ \$0\.0300/)).toBeInTheDocument();
    expect(screen.getByText(/日限额 \$0\.1250 \/ \$1\.25/)).toBeInTheDocument();
    expect(screen.getByText(/月限额 \$0\.5000 \/ \$9\.50/)).toBeInTheDocument();
    expect(useProviderHealthMock).toHaveBeenCalledWith(
      provider.id,
      "claude",
      true,
    );
    expect(useProviderLimitsMock).toHaveBeenCalledWith(
      provider.id,
      "claude",
      true,
    );
    expect(useProviderStatsMock).toHaveBeenCalledWith(
      { preset: "30d" },
      "claude",
      { refetchInterval: false },
    );
  });

  it("disables the model test action for third-party Claude providers", () => {
    const provider = createProvider({
      category: "third_party",
      name: "Third-party Claude",
    });

    render(
      <ProviderCard {...baseProps} provider={provider} onTest={vi.fn()} />,
    );

    expect(providerActionsPropsSpy).toHaveBeenCalled();
    expect(providerActionsPropsSpy.mock.calls[0]?.[0].onTest).toBeUndefined();
  });

  it("keeps the model test action enabled for Claude aggregators", () => {
    const onTest = vi.fn();
    const provider = createProvider({
      category: "aggregator",
      name: "ClaudeAPI",
    });

    render(<ProviderCard {...baseProps} provider={provider} onTest={onTest} />);

    expect(providerActionsPropsSpy).toHaveBeenCalled();
    expect(providerActionsPropsSpy.mock.calls[0]?.[0].onTest).toEqual(
      expect.any(Function),
    );
  });

  it("shows routing capability badges for Claude Code and Codex providers", () => {
    const { rerender } = render(
      <ProviderCard
        {...baseProps}
        provider={createProvider({
          name: "OpenAI Compatible Claude",
          meta: { apiFormat: "openai_chat" },
        })}
      />,
    );

    expect(screen.getByText("需要路由")).toBeInTheDocument();

    rerender(
      <ProviderCard
        {...baseProps}
        provider={createProvider({
          name: "Official Claude",
          category: "official",
        })}
      />,
    );

    expect(screen.getByText("不支持路由")).toBeInTheDocument();

    rerender(
      <ProviderCard
        {...baseProps}
        appId="codex"
        provider={createProvider({
          name: "Managed xAI Codex",
          meta: {
            providerType: "xai_oauth",
            apiFormat: "openai_responses",
          },
        })}
      />,
    );

    expect(screen.getByText("需要路由")).toBeInTheDocument();

    rerender(
      <ProviderCard
        {...baseProps}
        appId="codex"
        provider={createProvider({
          name: "Official Codex",
          category: "official",
        })}
      />,
    );

    expect(screen.getByText("不支持路由")).toBeInTheDocument();
  });

  it("disables live diagnostics queries when no health or usage limits are relevant", () => {
    const provider = createProvider({ name: "Plain Provider" });

    render(<ProviderCard {...baseProps} provider={provider} />);

    expect(useProviderHealthMock).toHaveBeenCalledWith(
      provider.id,
      "claude",
      false,
    );
    expect(useProviderLimitsMock).toHaveBeenCalledWith(
      provider.id,
      "claude",
      false,
    );
    expect(screen.queryByText(/日限额/)).not.toBeInTheDocument();
    expect(screen.queryByText(/月限额/)).not.toBeInTheDocument();
  });

  it("shows explicit live config status for additive-mode apps", () => {
    const provider = createProvider({ name: "OpenCode Provider" });

    render(
      <ProviderCard
        {...baseProps}
        provider={provider}
        appId="opencode"
        isInConfig={false}
      />,
    );

    expect(screen.getByText("DB Only")).toBeInTheDocument();
  });

  it("splits provider diagnostics error states into separate inline badges", () => {
    const provider = createProvider({
      name: "Error Provider",
      meta: {
        limitDailyUsd: "1.25",
      },
    });

    useProviderHealthMock.mockReturnValue({
      data: undefined,
      error: new Error("health failed"),
    });
    useProviderLimitsMock.mockReturnValue({
      data: undefined,
      error: new Error("limits failed"),
    });
    useProviderStatsMock.mockReturnValue({
      data: [],
      error: new Error("stats failed"),
    });

    render(
      <ProviderCard
        {...baseProps}
        provider={provider}
        isProxyRunning={true}
        isInFailoverQueue={true}
      />,
    );

    expect(screen.getByText("健康状态不可用")).toBeInTheDocument();
    expect(screen.getByText("限额状态不可用")).toBeInTheDocument();
    expect(screen.getByText("用量摘要不可用")).toBeInTheDocument();
  });
});
