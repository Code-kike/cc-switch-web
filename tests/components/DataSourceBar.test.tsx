import type { ReactNode } from "react";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { DataSourceBar } from "@/components/usage/DataSourceBar";
import { usageKeys } from "@/lib/query/usage";

const getDataSourceBreakdownMock = vi.fn();
const syncSessionUsageMock = vi.fn();
const toastSuccessMock = vi.fn();
const toastInfoMock = vi.fn();
const toastErrorMock = vi.fn();
const i18nState = vi.hoisted(() => ({
  t: (key: string, options?: Record<string, unknown>) =>
    String(options?.defaultValue ?? key),
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: i18nState.t,
  }),
}));

vi.mock("sonner", () => ({
  toast: {
    success: (...args: unknown[]) => toastSuccessMock(...args),
    info: (...args: unknown[]) => toastInfoMock(...args),
    error: (...args: unknown[]) => toastErrorMock(...args),
  },
}));

vi.mock("@/components/ui/button", () => ({
  Button: ({
    children,
    type = "button",
    ...props
  }: {
    children: ReactNode;
    type?: "button" | "submit" | "reset";
    [key: string]: unknown;
  }) => (
    <button type={type} {...props}>
      {children}
    </button>
  ),
}));

vi.mock("@/lib/api/usage", () => ({
  usageApi: {
    getDataSourceBreakdown: (...args: unknown[]) =>
      getDataSourceBreakdownMock(...args),
    syncSessionUsage: (...args: unknown[]) => syncSessionUsageMock(...args),
  },
}));

function renderBar(refreshIntervalMs = 0) {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  const invalidateQueriesSpy = vi.spyOn(queryClient, "invalidateQueries");

  const result = render(
    <QueryClientProvider client={queryClient}>
      <DataSourceBar refreshIntervalMs={refreshIntervalMs} />
    </QueryClientProvider>,
  );

  return {
    ...result,
    invalidateQueriesSpy,
  };
}

describe("DataSourceBar", () => {
  beforeEach(() => {
    getDataSourceBreakdownMock.mockReset();
    syncSessionUsageMock.mockReset();
    toastSuccessMock.mockReset();
    toastInfoMock.mockReset();
    toastErrorMock.mockReset();
  });

  it("renders data-source chips and refreshes usage queries after importing session logs", async () => {
    getDataSourceBreakdownMock.mockResolvedValue([
      { dataSource: "proxy", requestCount: 12, totalCostUsd: "1.230000" },
      {
        dataSource: "session_log",
        requestCount: 3,
        totalCostUsd: "0.450000",
      },
    ]);
    syncSessionUsageMock.mockResolvedValue({
      imported: 3,
      skipped: 0,
      filesScanned: 1,
      errors: [],
    });

    const { invalidateQueriesSpy } = renderBar(30000);

    expect(await screen.findByText("proxy")).toBeInTheDocument();
    expect(screen.getByText("session_log")).toBeInTheDocument();
    expect(screen.getByText("12")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Sync/ })).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: /Sync/ }));

    await waitFor(() => expect(syncSessionUsageMock).toHaveBeenCalledTimes(1));
    expect(toastSuccessMock).toHaveBeenCalledWith(
      "Imported {{count}} records from session logs",
    );
    expect(invalidateQueriesSpy).toHaveBeenCalledWith({
      queryKey: usageKeys.all,
    });
  });

  it("uses the import label and shows an up-to-date toast when only proxy data exists", async () => {
    getDataSourceBreakdownMock.mockResolvedValue([
      { dataSource: "proxy", requestCount: 8, totalCostUsd: "0.120000" },
    ]);
    syncSessionUsageMock.mockResolvedValue({
      imported: 0,
      skipped: 2,
      filesScanned: 2,
      errors: [],
    });

    renderBar();

    expect(await screen.findByText("proxy")).toBeInTheDocument();
    const importButton = screen.getByRole("button", {
      name: /Import Sessions/,
    });

    fireEvent.click(importButton);

    await waitFor(() => expect(syncSessionUsageMock).toHaveBeenCalledTimes(1));
    expect(toastInfoMock).toHaveBeenCalledWith("Session logs are up to date");
  });

  it("shows an empty-state message and import button when there are no data sources", async () => {
    getDataSourceBreakdownMock.mockResolvedValue([]);

    renderBar();

    await waitFor(() =>
      expect(getDataSourceBreakdownMock).toHaveBeenCalledTimes(1),
    );
    expect(
      await screen.findByText(
        "No usage data yet. Import session logs to populate this dashboard.",
      ),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", {
        name: /Import Sessions/,
      }),
    ).toBeInTheDocument();
  });

  it("shows an error toast with backend detail when session sync fails", async () => {
    getDataSourceBreakdownMock.mockResolvedValue([
      { dataSource: "proxy", requestCount: 1, totalCostUsd: "0.010000" },
    ]);
    syncSessionUsageMock.mockRejectedValue({
      message: "session log directory denied",
    });

    renderBar();

    expect(await screen.findByText("proxy")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: /Import Sessions/ }));

    await waitFor(() => expect(syncSessionUsageMock).toHaveBeenCalledTimes(1));
    expect(toastErrorMock).toHaveBeenCalledWith("Session sync failed", {
      description: "session log directory denied",
    });
    expect(toastErrorMock).not.toHaveBeenCalledWith(
      expect.anything(),
      expect.objectContaining({ description: "[object Object]" }),
    );
  });
});
