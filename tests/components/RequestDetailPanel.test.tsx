import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { RequestDetailPanel } from "@/components/usage/RequestDetailPanel";

const useRequestDetailMock = vi.hoisted(() => vi.fn());

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (
      key: string,
      options?:
        | string
        | {
            defaultValue?: string;
          },
    ) => {
      if (typeof options === "string") {
        return options;
      }
      return options?.defaultValue ?? key;
    },
    i18n: {
      language: "en",
    },
  }),
}));

vi.mock("@/lib/query/usage", () => ({
  useRequestDetail: (requestId: string) => useRequestDetailMock(requestId),
}));

vi.mock("@/components/ui/dialog", () => ({
  Dialog: ({ children }: any) => <div>{children}</div>,
  DialogContent: ({ children }: any) => <div>{children}</div>,
  DialogHeader: ({ children }: any) => <div>{children}</div>,
  DialogDescription: ({ children }: any) => <p>{children}</p>,
  DialogTitle: ({ children }: any) => <h2>{children}</h2>,
  DialogFooter: ({ children }: any) => <div>{children}</div>,
}));

vi.mock("@/components/ui/button", () => ({
  Button: ({ children, ...props }: any) => (
    <button {...props}>{children}</button>
  ),
}));

describe("RequestDetailPanel", () => {
  beforeEach(() => {
    useRequestDetailMock.mockReset();
  });

  it("renders request detail content and closes via the footer button", () => {
    const onClose = vi.fn();
    useRequestDetailMock.mockReturnValue({
      data: {
        requestId: "req-123",
        providerId: "provider-1",
        providerName: "Provider One",
        appType: "claude",
        model: "claude-3-7-sonnet",
        requestModel: "claude-3-7-sonnet",
        costMultiplier: "1.25",
        inputTokens: 10,
        outputTokens: 5,
        cacheReadTokens: 2,
        cacheCreationTokens: 1,
        inputCostUsd: "0.010000",
        outputCostUsd: "0.020000",
        cacheReadCostUsd: "0.001000",
        cacheCreationCostUsd: "0.002000",
        totalCostUsd: "0.041250",
        isStreaming: true,
        latencyMs: 1200,
        firstTokenMs: 400,
        durationMs: 1500,
        statusCode: 200,
        createdAt: 1_710_000_000,
        dataSource: "proxy",
      },
      isLoading: false,
    });

    render(<RequestDetailPanel requestId="req-123" onClose={onClose} />);

    expect(screen.getByText("请求详情")).toBeInTheDocument();
    expect(screen.getByText("req-123")).toBeInTheDocument();
    expect(screen.getByText("Provider One")).toBeInTheDocument();
    expect(screen.getByText("$0.041250")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Close" }));

    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("renders a not-found message when the request detail is missing", () => {
    useRequestDetailMock.mockReturnValue({
      data: null,
      isLoading: false,
    });

    render(<RequestDetailPanel requestId="missing-request" onClose={vi.fn()} />);

    expect(screen.getByText("请求详情")).toBeInTheDocument();
    expect(screen.getByText("请求未找到")).toBeInTheDocument();
  });
});
