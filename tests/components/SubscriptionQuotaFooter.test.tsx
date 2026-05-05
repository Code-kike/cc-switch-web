import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";

import SubscriptionQuotaFooter, {
  SubscriptionQuotaView,
} from "@/components/SubscriptionQuotaFooter";
import type { SubscriptionQuota } from "@/types/subscription";

const refetchMock = vi.fn();
const useSubscriptionQuotaMock = vi.fn();
const i18nState = vi.hoisted(() => ({
  t: (key: string, options?: Record<string, unknown>) => {
    if (key === "subscription.utilization") {
      return `${String(options?.value ?? "")}%`;
    }
    if (key === "subscription.resetsIn") {
      return `in ${String(options?.time ?? "")}`;
    }
    if (key === "subscription.expiredHint") {
      return `expired:${String(options?.tool ?? "")}`;
    }
    if (key === "usage.justNow") return "just now";
    if (key === "usage.minutesAgo") return `${String(options?.count ?? 0)}m ago`;
    if (key === "usage.hoursAgo") return `${String(options?.count ?? 0)}h ago`;
    if (key === "usage.daysAgo") return `${String(options?.count ?? 0)}d ago`;
    return String(options?.defaultValue ?? key);
  },
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: i18nState.t,
  }),
}));

vi.mock("@/lib/query/subscription", () => ({
  useSubscriptionQuota: (...args: unknown[]) => useSubscriptionQuotaMock(...args),
}));

function createQuota(
  overrides: Partial<SubscriptionQuota> = {},
): SubscriptionQuota {
  return {
    tool: "claude",
    credentialStatus: "valid",
    credentialMessage: null,
    success: true,
    tiers: [],
    extraUsage: null,
    error: null,
    queriedAt: null,
    ...overrides,
  };
}

describe("SubscriptionQuota surfaces", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2026-05-04T00:00:00Z"));
    refetchMock.mockReset();
    useSubscriptionQuotaMock.mockReset();
    useSubscriptionQuotaMock.mockReturnValue({
      data: createQuota(),
      isFetching: false,
      refetch: refetchMock,
    });
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("renders nothing for missing credentials and disables wrapper output when not current", () => {
    const { container: missingContainer } = render(
      <SubscriptionQuotaView
        quota={createQuota({ credentialStatus: "not_found", success: false })}
        loading={false}
        refetch={refetchMock}
        appIdForExpiredHint="claude"
      />,
    );
    expect(missingContainer).toBeEmptyDOMElement();

    const { container: parseErrorContainer } = render(
      <SubscriptionQuotaView
        quota={createQuota({ credentialStatus: "parse_error", success: false })}
        loading={false}
        refetch={refetchMock}
        appIdForExpiredHint="claude"
      />,
    );
    expect(parseErrorContainer).toBeEmptyDOMElement();

    const { container: wrapperContainer } = render(
      <SubscriptionQuotaFooter appId="claude" isCurrent={false} />,
    );
    expect(useSubscriptionQuotaMock).toHaveBeenCalledWith("claude", false, false);
    expect(wrapperContainer).toBeEmptyDOMElement();
  });

  it("shows expired state with a refresh action", () => {
    render(
      <SubscriptionQuotaView
        quota={createQuota({
          credentialStatus: "expired",
          success: false,
        })}
        loading={false}
        refetch={refetchMock}
        appIdForExpiredHint="codex"
      />,
    );

    expect(screen.getByText("subscription.expired")).toBeInTheDocument();
    expect(screen.getByText("expired:codex")).toBeInTheDocument();

    fireEvent.click(screen.getByTitle("subscription.refresh"));
    expect(refetchMock).toHaveBeenCalledTimes(1);
  });

  it("renders successful quota tiers, extra usage, and inline filtering", () => {
    const successQuota = createQuota({
      queriedAt: null,
      tiers: [
        {
          name: "five_hour",
          utilization: 87,
          resetsAt: "2026-05-04T05:30:00Z",
        },
        {
          name: "seven_day_sonnet",
          utilization: 42,
          resetsAt: null,
        },
      ],
      extraUsage: {
        isEnabled: true,
        monthlyLimit: 10,
        usedCredits: 2.5,
        utilization: 25,
        currency: "USD",
      },
    });

    const { rerender } = render(
      <SubscriptionQuotaView
        quota={successQuota}
        loading={false}
        refetch={refetchMock}
        appIdForExpiredHint="claude"
      />,
    );

    expect(screen.getByText("Subscription Quota")).toBeInTheDocument();
    expect(screen.getByText(/subscription\.fiveHour/)).toBeInTheDocument();
    expect(screen.getByText("87%")).toBeInTheDocument();
    expect(screen.getByText("$2.50 / $10.00")).toBeInTheDocument();

    rerender(
      <SubscriptionQuotaView
        quota={successQuota}
        loading={false}
        refetch={refetchMock}
        appIdForExpiredHint="claude"
        inline
      />,
    );

    expect(screen.getByText(/subscription\.fiveHour/)).toBeInTheDocument();
    expect(screen.queryByText("subscription.sevenDaySonnet")).toBeNull();

    fireEvent.click(screen.getByTitle("subscription.refresh"));
    expect(refetchMock).toHaveBeenCalledTimes(1);
  });
});
