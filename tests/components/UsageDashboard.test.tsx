import type { ReactNode } from "react";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { UsageDashboard } from "@/components/usage/UsageDashboard";
import { usageKeys } from "@/lib/query/usage";
import type { UsageRangeSelection } from "@/types/usage";

const summaryPropsSpy = vi.fn();
const trendPropsSpy = vi.fn();
const logsPropsSpy = vi.fn();
const providerStatsPropsSpy = vi.fn();
const modelStatsPropsSpy = vi.fn();
const dataSourceBarPropsSpy = vi.fn();
const tabsValueSpy = vi.fn();

const customRange: UsageRangeSelection = {
  preset: "custom",
  customStartDate: 1_710_000_000,
  customEndDate: 1_710_086_400,
};

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (
      key: string,
      options?: {
        defaultValue?: string;
      },
    ) => options?.defaultValue ?? key,
    i18n: {
      resolvedLanguage: "en",
      language: "en",
    },
  }),
}));

vi.mock("framer-motion", () => ({
  motion: {
    div: ({ children, ...props }: React.HTMLAttributes<HTMLDivElement>) => (
      <div {...props}>{children}</div>
    ),
  },
}));

vi.mock("@/components/ui/button", () => ({
  Button: ({ children, type = "button", ...props }: any) => (
    <button type={type} {...props}>
      {children}
    </button>
  ),
}));

vi.mock("@/components/ui/accordion", () => ({
  Accordion: ({ children }: { children: ReactNode }) => <div>{children}</div>,
  AccordionItem: ({ children }: { children: ReactNode }) => <div>{children}</div>,
  AccordionTrigger: ({ children }: { children: ReactNode }) => (
    <div>{children}</div>
  ),
  AccordionContent: ({ children }: { children: ReactNode }) => (
    <div>{children}</div>
  ),
}));

vi.mock("@/components/ui/tabs", async () => {
  const React = await import("react");
  const TabsContext = React.createContext<((value: string) => void) | null>(null);

  return {
    Tabs: ({
      children,
      value,
      onValueChange,
    }: {
      children: ReactNode;
      value?: string;
      onValueChange?: (value: string) => void;
    }) => {
      tabsValueSpy(value);
      return (
        <TabsContext.Provider value={onValueChange ?? null}>
          <div>{children}</div>
        </TabsContext.Provider>
      );
    },
    TabsList: ({ children }: { children: ReactNode }) => <div>{children}</div>,
    TabsTrigger: ({
      children,
      value,
    }: {
      children: ReactNode;
      value: string;
    }) => {
      const onValueChange = React.useContext(TabsContext);
      return <button onClick={() => onValueChange?.(value)}>{children}</button>;
    },
    TabsContent: ({ children }: { children: ReactNode }) => <div>{children}</div>,
  };
});

vi.mock("@/components/usage/UsageSummaryCards", () => ({
  UsageSummaryCards: (props: any) => {
    summaryPropsSpy(props);
    return <div data-testid="usage-summary-cards">{props.appType}</div>;
  },
}));

vi.mock("@/components/usage/UsageTrendChart", () => ({
  UsageTrendChart: (props: any) => {
    trendPropsSpy(props);
    return <div data-testid="usage-trend-chart">{props.rangeLabel}</div>;
  },
}));

vi.mock("@/components/usage/RequestLogTable", () => ({
  RequestLogTable: (props: any) => {
    logsPropsSpy(props);
    return <div data-testid="request-log-table">{props.appType}</div>;
  },
}));

vi.mock("@/components/usage/ProviderStatsTable", () => ({
  ProviderStatsTable: (props: any) => {
    providerStatsPropsSpy(props);
    return <div data-testid="provider-stats-table">{props.appType}</div>;
  },
}));

vi.mock("@/components/usage/ModelStatsTable", () => ({
  ModelStatsTable: (props: any) => {
    modelStatsPropsSpy(props);
    return <div data-testid="model-stats-table">{props.appType}</div>;
  },
}));

vi.mock("@/components/usage/DataSourceBar", () => ({
  DataSourceBar: (props: any) => {
    dataSourceBarPropsSpy(props);
    return <div data-testid="data-source-bar">{props.refreshIntervalMs}</div>;
  },
}));

vi.mock("@/components/usage/PricingConfigPanel", () => ({
  PricingConfigPanel: () => <div data-testid="pricing-config-panel" />,
}));

vi.mock("@/components/usage/UsageDateRangePicker", () => ({
  UsageDateRangePicker: ({
    triggerLabel,
    onApply,
  }: {
    triggerLabel: string;
    onApply: (selection: UsageRangeSelection) => void;
  }) => (
    <button type="button" onClick={() => onApply(customRange)}>
      {triggerLabel}
    </button>
  ),
}));

function renderDashboard() {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  const invalidateQueriesSpy = vi.spyOn(queryClient, "invalidateQueries");

  render(
    <QueryClientProvider client={queryClient}>
      <UsageDashboard />
    </QueryClientProvider>,
  );

  return {
    invalidateQueriesSpy,
  };
}

describe("UsageDashboard", () => {
  beforeEach(() => {
    summaryPropsSpy.mockReset();
    trendPropsSpy.mockReset();
    logsPropsSpy.mockReset();
    providerStatsPropsSpy.mockReset();
    modelStatsPropsSpy.mockReset();
    dataSourceBarPropsSpy.mockReset();
    tabsValueSpy.mockReset();
  });

  it("propagates app filter, refresh interval, and custom range updates to child panels", async () => {
    const { invalidateQueriesSpy } = renderDashboard();

    expect(screen.getByText("usage.title")).toBeInTheDocument();
    expect(screen.getByTestId("pricing-config-panel")).toBeInTheDocument();

    expect(summaryPropsSpy).toHaveBeenLastCalledWith(
      expect.objectContaining({
        appType: "all",
        range: { preset: "today" },
        refreshIntervalMs: 30000,
      }),
    );
    expect(logsPropsSpy).toHaveBeenLastCalledWith(
      expect.objectContaining({
        appType: "all",
        range: { preset: "today" },
        refreshIntervalMs: 30000,
      }),
    );
    expect(dataSourceBarPropsSpy).toHaveBeenLastCalledWith(
      expect.objectContaining({
        refreshIntervalMs: 30000,
      }),
    );

    fireEvent.click(screen.getByRole("button", { name: "usage.appFilter.claude" }));

    await waitFor(() =>
      expect(summaryPropsSpy).toHaveBeenLastCalledWith(
        expect.objectContaining({
          appType: "claude",
        }),
      ),
    );
    expect(providerStatsPropsSpy).toHaveBeenLastCalledWith(
      expect.objectContaining({
        appType: "claude",
      }),
    );
    expect(modelStatsPropsSpy).toHaveBeenLastCalledWith(
      expect.objectContaining({
        appType: "claude",
      }),
    );

    fireEvent.click(screen.getByRole("button", { name: "30s" }));

    await waitFor(() =>
      expect(summaryPropsSpy).toHaveBeenLastCalledWith(
        expect.objectContaining({
          refreshIntervalMs: 60000,
        }),
      ),
    );
    expect(logsPropsSpy).toHaveBeenLastCalledWith(
      expect.objectContaining({
        refreshIntervalMs: 60000,
      }),
    );
    expect(dataSourceBarPropsSpy).toHaveBeenLastCalledWith(
      expect.objectContaining({
        refreshIntervalMs: 60000,
      }),
    );
    expect(invalidateQueriesSpy).toHaveBeenCalledWith({
      queryKey: usageKeys.all,
    });

    fireEvent.click(screen.getByRole("button", { name: "当天" }));

    await waitFor(() =>
      expect(summaryPropsSpy).toHaveBeenLastCalledWith(
        expect.objectContaining({
          range: customRange,
        }),
      ),
    );
    expect(trendPropsSpy).toHaveBeenLastCalledWith(
      expect.objectContaining({
        range: customRange,
      }),
    );
    expect(logsPropsSpy).toHaveBeenLastCalledWith(
      expect.objectContaining({
        range: customRange,
      }),
    );
  });

  it("preserves the selected tab when dashboard-level filters change", async () => {
    renderDashboard();

    fireEvent.click(screen.getByRole("button", { name: "usage.modelStats" }));

    await waitFor(() => expect(tabsValueSpy).toHaveBeenLastCalledWith("models"));

    fireEvent.click(screen.getByRole("button", { name: "usage.appFilter.claude" }));
    await waitFor(() => expect(tabsValueSpy).toHaveBeenLastCalledWith("models"));

    fireEvent.click(screen.getByRole("button", { name: "30s" }));
    await waitFor(() => expect(tabsValueSpy).toHaveBeenLastCalledWith("models"));

    fireEvent.click(screen.getByRole("button", { name: "当天" }));
    await waitFor(() => expect(tabsValueSpy).toHaveBeenLastCalledWith("models"));
  });
});
