import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { FailoverQueueManager } from "@/components/proxy/FailoverQueueManager";

const addToQueueMutateAsyncMock = vi.fn();
const removeFromQueueMutateAsyncMock = vi.fn();
const resetCircuitBreakerMutateAsyncMock = vi.fn();
const setAutoFailoverEnabledMutateMock = vi.fn();
const toastSuccessMock = vi.fn();
const toastErrorMock = vi.fn();
const i18nState = vi.hoisted(() => ({
  t: (
    key: string,
    options?:
      | string
      | {
          defaultValue?: string;
          count?: number;
          detail?: string;
        },
  ) => {
    if (typeof options === "string") {
      return options;
    }

    if (options?.defaultValue) {
      return options.defaultValue
        .replace("{{count}}", String(options.count ?? ""))
        .replace("{{detail}}", String(options.detail ?? ""));
    }

    return key;
  },
}));
const selectState = vi.hoisted(() => ({
  value: "",
  disabled: false,
  onValueChange: null as ((value: string) => void) | null,
}));

const failoverState = vi.hoisted(() => ({
  isFailoverEnabled: false,
  queue: [] as Array<{
    providerId: string;
    providerName: string;
    providerNotes?: string;
  }>,
  queueLoading: false,
  queueError: null as Error | null,
  availableProviders: [] as Array<{
    id: string;
    name: string;
    settingsConfig: Record<string, unknown>;
    notes?: string;
  }>,
  providersLoading: false,
  providerHealthById: {} as Record<
    string,
    {
      provider_id: string;
      app_type: string;
      is_healthy: boolean;
      consecutive_failures: number;
      last_success_at: string | null;
      last_failure_at: string | null;
      last_error: string | null;
      updated_at: string;
    }
  >,
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: i18nState.t,
  }),
}));

vi.mock("sonner", () => ({
  toast: {
    success: (...args: unknown[]) => toastSuccessMock(...args),
    error: (...args: unknown[]) => toastErrorMock(...args),
  },
}));

vi.mock("@/components/ui/button", () => ({
  Button: ({ children, type = "button", ...props }: any) => (
    <button type={type} {...props}>
      {children}
    </button>
  ),
}));

vi.mock("@/components/ui/switch", () => ({
  Switch: ({ checked, onCheckedChange, ...props }: any) => (
    <button
      type="button"
      aria-label="proxy.failover.autoSwitch"
      aria-pressed={checked}
      onClick={() => onCheckedChange?.(!checked)}
      {...props}
    />
  ),
}));

vi.mock("@/components/ui/alert", () => ({
  Alert: ({ children, ...props }: any) => <div {...props}>{children}</div>,
  AlertDescription: ({ children, ...props }: any) => (
    <div {...props}>{children}</div>
  ),
}));

vi.mock("@/components/ui/select", () => ({
  Select: ({ value, onValueChange, disabled, children }: any) => {
    selectState.value = value;
    selectState.disabled = Boolean(disabled);
    selectState.onValueChange = onValueChange ?? null;

    return <div>{children}</div>;
  },
  SelectTrigger: ({ children }: any) => <div>{children}</div>,
  SelectValue: ({ placeholder }: any) => <span>{placeholder}</span>,
  SelectContent: ({ children }: any) => <div>{children}</div>,
  SelectItem: ({ value, children, disabled }: any) => (
    <button
      type="button"
      disabled={selectState.disabled || disabled}
      onClick={() => selectState.onValueChange?.(value)}
    >
      {children}
    </button>
  ),
}));

vi.mock("@/lib/query/failover", () => ({
  useAutoFailoverEnabled: () => ({
    data: failoverState.isFailoverEnabled,
  }),
  useSetAutoFailoverEnabled: () => ({
    mutate: (...args: unknown[]) => setAutoFailoverEnabledMutateMock(...args),
    isPending: false,
  }),
  useFailoverQueue: () => ({
    data: failoverState.queue,
    isLoading: failoverState.queueLoading,
    error: failoverState.queueError,
  }),
  useAvailableProvidersForFailover: () => ({
    data: failoverState.availableProviders,
    isLoading: failoverState.providersLoading,
  }),
  useAddToFailoverQueue: () => ({
    mutateAsync: (...args: unknown[]) => addToQueueMutateAsyncMock(...args),
    isPending: false,
  }),
  useRemoveFromFailoverQueue: () => ({
    mutateAsync: (...args: unknown[]) => removeFromQueueMutateAsyncMock(...args),
    isPending: false,
  }),
  useProviderHealth: (providerId: string, _appType: string, enabled = true) => ({
    data: enabled ? failoverState.providerHealthById[providerId] ?? null : null,
    isLoading: false,
    error: null,
  }),
  useResetCircuitBreaker: () => ({
    mutateAsync: (...args: unknown[]) =>
      resetCircuitBreakerMutateAsyncMock(...args),
    isPending: false,
  }),
}));

describe("FailoverQueueManager", () => {
  beforeEach(() => {
    addToQueueMutateAsyncMock.mockReset();
    removeFromQueueMutateAsyncMock.mockReset();
    resetCircuitBreakerMutateAsyncMock.mockReset();
    setAutoFailoverEnabledMutateMock.mockReset();
    toastSuccessMock.mockReset();
    toastErrorMock.mockReset();

    addToQueueMutateAsyncMock.mockResolvedValue(undefined);
    removeFromQueueMutateAsyncMock.mockResolvedValue(undefined);
    resetCircuitBreakerMutateAsyncMock.mockResolvedValue(undefined);

    failoverState.isFailoverEnabled = false;
    failoverState.queue = [];
    failoverState.queueLoading = false;
    failoverState.queueError = null;
    failoverState.availableProviders = [];
    failoverState.providersLoading = false;
    failoverState.providerHealthById = {};
  });

  it("toggles auto failover, shows provider health, and persists queue actions", async () => {
    failoverState.queue = [
      {
        providerId: "provider-1",
        providerName: "Primary Provider",
      },
    ];
    failoverState.availableProviders = [
      {
        id: "provider-2",
        name: "Backup Provider",
        settingsConfig: {},
      },
    ];
    failoverState.providerHealthById["provider-1"] = {
      provider_id: "provider-1",
      app_type: "claude",
      is_healthy: false,
      consecutive_failures: 3,
      last_success_at: null,
      last_failure_at: "2026-05-04T00:00:00Z",
      last_error: "timeout",
      updated_at: "2026-05-04T00:00:10Z",
    };

    render(<FailoverQueueManager appType="claude" />);

    fireEvent.click(
      screen.getByRole("button", { name: "proxy.failover.autoSwitch" }),
    );
    expect(setAutoFailoverEnabledMutateMock).toHaveBeenCalledWith({
      appType: "claude",
      enabled: true,
    });

    fireEvent.click(screen.getByRole("button", { name: "Backup Provider" }));
    fireEvent.click(screen.getByRole("button", { name: "添加" }));

    await waitFor(() =>
      expect(addToQueueMutateAsyncMock).toHaveBeenCalledWith({
        appType: "claude",
        providerId: "provider-2",
      }),
    );
    expect(toastSuccessMock).toHaveBeenCalledWith(
      "已添加到故障转移队列",
      { closeButton: true },
    );

    expect(screen.getByText("Tripped")).toBeInTheDocument();
    expect(screen.getByText("3 failures")).toBeInTheDocument();
    expect(screen.getByText("Last error: timeout")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Reset Circuit" }));

    await waitFor(() =>
      expect(resetCircuitBreakerMutateAsyncMock).toHaveBeenCalledWith({
        appType: "claude",
        providerId: "provider-1",
      }),
    );
    expect(toastSuccessMock).toHaveBeenCalledWith(
      "Circuit breaker has been reset",
      { closeButton: true },
    );

    fireEvent.click(screen.getByRole("button", { name: "删除" }));

    await waitFor(() =>
      expect(removeFromQueueMutateAsyncMock).toHaveBeenCalledWith({
        appType: "claude",
        providerId: "provider-1",
      }),
    );
    expect(toastSuccessMock).toHaveBeenCalledWith(
      "已从故障转移队列移除",
      { closeButton: true },
    );
  });

  it("shows empty-state hints and keeps add disabled when no providers are available", () => {
    render(<FailoverQueueManager appType="codex" />);

    expect(
      screen.getByText("故障转移队列为空。添加供应商以启用自动故障转移。"),
    ).toBeInTheDocument();
    expect(
      screen.getByText("没有可添加的供应商"),
    ).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "添加" })).toBeDisabled();
  });

  it("shows extracted detail when adding or removing a provider fails", async () => {
    failoverState.queue = [
      {
        providerId: "provider-1",
        providerName: "Primary Provider",
      },
    ];
    failoverState.availableProviders = [
      {
        id: "provider-2",
        name: "Backup Provider",
        settingsConfig: {},
      },
    ];
    addToQueueMutateAsyncMock.mockRejectedValueOnce({
      detail: "queue add failed",
    });
    removeFromQueueMutateAsyncMock.mockRejectedValueOnce({
      message: "queue remove failed",
    });

    render(<FailoverQueueManager appType="claude" />);

    fireEvent.click(screen.getByRole("button", { name: "Backup Provider" }));
    fireEvent.click(screen.getByRole("button", { name: "添加" }));

    await waitFor(() =>
      expect(toastErrorMock).toHaveBeenCalledWith("添加失败: queue add failed"),
    );

    fireEvent.click(screen.getByRole("button", { name: "删除" }));

    await waitFor(() =>
      expect(toastErrorMock).toHaveBeenCalledWith("移除失败: queue remove failed"),
    );
  });

  it("shows extracted detail when loading the queue fails", () => {
    failoverState.queueError = new Error("queue exploded");

    render(<FailoverQueueManager appType="claude" />);

    expect(
      screen.getByText("加载故障转移队列失败: queue exploded"),
    ).toBeInTheDocument();
  });
});
