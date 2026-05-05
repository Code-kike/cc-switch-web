import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { AutoFailoverConfigPanel } from "@/components/proxy/AutoFailoverConfigPanel";

const updateAppProxyConfigMutateAsyncMock = vi.fn();
const toastSuccessMock = vi.fn();
const toastErrorMock = vi.fn();

const proxyState = vi.hoisted(() => ({
  config: {
    appType: "claude",
    enabled: true,
    autoFailoverEnabled: true,
    maxRetries: 4,
    streamingFirstByteTimeout: 60,
    streamingIdleTimeout: 120,
    nonStreamingTimeout: 600,
    circuitFailureThreshold: 3,
    circuitSuccessThreshold: 2,
    circuitTimeoutSeconds: 90,
    circuitErrorRateThreshold: 0.5,
    circuitMinRequests: 12,
  },
  isLoading: false,
  error: null as Error | null,
  isPending: false,
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (
      key: string,
      options?:
        | string
        | {
            defaultValue?: string;
            fields?: string;
            detail?: string;
          },
    ) => {
      if (typeof options === "string") {
        return options;
      }
      return (
        options?.defaultValue
          ?.replace("{{fields}}", options.fields ?? "")
          .replace("{{detail}}", options.detail ?? "") ?? key
      );
    },
  }),
}));

vi.mock("sonner", () => ({
  toast: {
    success: (...args: unknown[]) => toastSuccessMock(...args),
    error: (...args: unknown[]) => toastErrorMock(...args),
  },
}));

vi.mock("@/lib/query/proxy", () => ({
  useAppProxyConfig: () => ({
    data: proxyState.config,
    isLoading: proxyState.isLoading,
    error: proxyState.error,
  }),
  useUpdateAppProxyConfig: () => ({
    mutateAsync: (...args: unknown[]) =>
      updateAppProxyConfigMutateAsyncMock(...args),
    isPending: proxyState.isPending,
  }),
}));

vi.mock("@/components/ui/button", () => ({
  Button: ({ children, type = "button", ...props }: any) => (
    <button type={type} {...props}>
      {children}
    </button>
  ),
}));

vi.mock("@/components/ui/input", () => ({
  Input: (props: any) => <input {...props} />,
}));

vi.mock("@/components/ui/label", () => ({
  Label: ({ children, ...props }: any) => <label {...props}>{children}</label>,
}));

vi.mock("@/components/ui/alert", () => ({
  Alert: ({ children, ...props }: any) => <div {...props}>{children}</div>,
  AlertDescription: ({ children, ...props }: any) => (
    <div {...props}>{children}</div>
  ),
}));

describe("AutoFailoverConfigPanel", () => {
  beforeEach(() => {
    updateAppProxyConfigMutateAsyncMock.mockReset();
    updateAppProxyConfigMutateAsyncMock.mockResolvedValue(undefined);
    toastSuccessMock.mockReset();
    toastErrorMock.mockReset();

    proxyState.config = {
      appType: "claude",
      enabled: true,
      autoFailoverEnabled: true,
      maxRetries: 4,
      streamingFirstByteTimeout: 60,
      streamingIdleTimeout: 120,
      nonStreamingTimeout: 600,
      circuitFailureThreshold: 3,
      circuitSuccessThreshold: 2,
      circuitTimeoutSeconds: 90,
      circuitErrorRateThreshold: 0.5,
      circuitMinRequests: 12,
    };
    proxyState.isLoading = false;
    proxyState.error = null;
    proxyState.isPending = false;
  });

  it("loads config, resets edited values, and saves parsed auto-failover settings", async () => {
    render(<AutoFailoverConfigPanel appType="claude" />);

    const failureThresholdInput = screen.getByLabelText("失败阈值");
    const errorRateInput = screen.getByLabelText("错误率阈值 (%)");

    expect(screen.getByLabelText("最大重试次数")).toHaveValue(4);
    expect(failureThresholdInput).toHaveValue(3);
    expect(errorRateInput).toHaveValue(50);

    fireEvent.change(failureThresholdInput, { target: { value: "7" } });
    fireEvent.change(errorRateInput, { target: { value: "45" } });

    fireEvent.click(screen.getByRole("button", { name: "重置" }));

    expect(failureThresholdInput).toHaveValue(3);
    expect(errorRateInput).toHaveValue(50);

    fireEvent.change(failureThresholdInput, { target: { value: "7" } });
    fireEvent.change(errorRateInput, { target: { value: "45" } });
    fireEvent.click(screen.getByRole("button", { name: "保存" }));

    await waitFor(() =>
      expect(updateAppProxyConfigMutateAsyncMock).toHaveBeenCalledWith({
        appType: "claude",
        enabled: true,
        autoFailoverEnabled: true,
        maxRetries: 4,
        streamingFirstByteTimeout: 60,
        streamingIdleTimeout: 120,
        nonStreamingTimeout: 600,
        circuitFailureThreshold: 7,
        circuitSuccessThreshold: 2,
        circuitTimeoutSeconds: 90,
        circuitErrorRateThreshold: 0.45,
        circuitMinRequests: 12,
      }),
    );
    expect(toastSuccessMock).toHaveBeenCalledWith("自动故障转移配置已保存", {
      closeButton: true,
    });
  });

  it("shows a validation error instead of saving out-of-range values", async () => {
    render(<AutoFailoverConfigPanel appType="claude" />);

    fireEvent.change(screen.getByLabelText("最大重试次数"), {
      target: { value: "99" },
    });
    fireEvent.click(screen.getByRole("button", { name: "保存" }));

    await waitFor(() =>
      expect(toastErrorMock).toHaveBeenCalledWith(
        expect.stringContaining("以下字段超出有效范围"),
      ),
    );
    expect(updateAppProxyConfigMutateAsyncMock).not.toHaveBeenCalled();
  });

  it("disables inputs and actions when the panel is disabled", () => {
    render(<AutoFailoverConfigPanel appType="claude" disabled />);

    expect(screen.getByLabelText("最大重试次数")).toBeDisabled();
    expect(screen.getByRole("button", { name: "重置" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "保存" })).toBeDisabled();
  });

  it("shows extracted detail when saving fails", async () => {
    updateAppProxyConfigMutateAsyncMock.mockRejectedValueOnce({
      detail: "save exploded",
    });

    render(<AutoFailoverConfigPanel appType="claude" />);

    fireEvent.click(screen.getByRole("button", { name: "保存" }));

    await waitFor(() =>
      expect(toastErrorMock).toHaveBeenCalledWith("保存失败: save exploded"),
    );
  });

  it("shows extracted detail when loading the config fails", () => {
    proxyState.error = new Error("load exploded");

    render(<AutoFailoverConfigPanel appType="claude" />);

    expect(
      screen.getByText("加载自动故障转移配置失败: load exploded"),
    ).toBeInTheDocument();
  });
});
