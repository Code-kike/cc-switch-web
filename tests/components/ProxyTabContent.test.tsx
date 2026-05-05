import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { ProxyTabContent } from "@/components/settings/ProxyTabContent";
import type { SettingsFormState } from "@/hooks/useSettings";

const startProxyServerMock = vi.fn();
const stopWithRestoreMock = vi.fn();
const proxyPanelToggleSpy = vi.fn();
const proxyPanelPropsSpy = vi.fn();
const failoverQueuePropsSpy = vi.fn();
const autoFailoverConfigPropsSpy = vi.fn();

let proxyStatusFixture = {
  isRunning: false,
  startProxyServer: startProxyServerMock,
  stopWithRestore: stopWithRestoreMock,
  isPending: false,
};

const createSettings = (
  overrides: Partial<SettingsFormState> = {},
): SettingsFormState => ({
  showInTray: true,
  minimizeToTrayOnClose: true,
  useAppWindowControls: false,
  enableClaudePluginIntegration: false,
  skipClaudeOnboarding: false,
  silentStartup: false,
  enableLocalProxy: false,
  proxyConfirmed: false,
  enableFailoverToggle: false,
  failoverConfirmed: false,
  language: "zh",
  ...overrides,
});

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) => key,
  }),
}));

vi.mock("@/lib/api/adapter", () => ({
  isWebMode: () => true,
}));

vi.mock("framer-motion", () => ({
  motion: {
    div: ({ children, ...props }: React.HTMLAttributes<HTMLDivElement>) => (
      <div {...props}>{children}</div>
    ),
  },
}));

vi.mock("@/hooks/useProxyStatus", () => ({
  useProxyStatus: () => proxyStatusFixture,
}));

vi.mock("@/components/ui/accordion", () => ({
  Accordion: ({ children }: { children: React.ReactNode }) => <div>{children}</div>,
  AccordionItem: ({ children }: { children: React.ReactNode }) => <div>{children}</div>,
  AccordionTrigger: ({ children }: { children: React.ReactNode }) => <div>{children}</div>,
  AccordionContent: ({ children }: { children: React.ReactNode }) => <div>{children}</div>,
}));

vi.mock("@/components/ui/tabs", () => ({
  Tabs: ({ children }: { children: React.ReactNode }) => <div>{children}</div>,
  TabsList: ({ children }: { children: React.ReactNode }) => <div>{children}</div>,
  TabsTrigger: ({ children }: { children: React.ReactNode }) => <button>{children}</button>,
  TabsContent: ({ children }: { children: React.ReactNode }) => <div>{children}</div>,
}));

vi.mock("@/components/ui/toggle-row", () => ({
  ToggleRow: ({
    title,
    checked,
    onCheckedChange,
  }: {
    title: string;
    checked: boolean;
    onCheckedChange: (checked: boolean) => void;
  }) => (
    <button type="button" aria-label={title} onClick={() => onCheckedChange(!checked)}>
      {title}
    </button>
  ),
}));

vi.mock("@/components/proxy", () => ({
  ProxyPanel: ({
    onToggleProxy,
    disableRuntimeControls,
  }: {
    onToggleProxy: (checked: boolean) => Promise<void>;
    disableRuntimeControls?: boolean;
  }) => (
    proxyPanelPropsSpy({ disableRuntimeControls }),
    <button
      type="button"
      onClick={() => {
        proxyPanelToggleSpy(true);
        void onToggleProxy(true);
      }}
    >
      proxy-panel-toggle
    </button>
  ),
}));

vi.mock("@/components/proxy/FailoverQueueManager", () => ({
  FailoverQueueManager: (props: { appType: string; disabled?: boolean }) => {
    failoverQueuePropsSpy(props);
    return (
      <div data-testid={`failover-queue-${props.appType}`}>
        {`${props.appType}:${String(props.disabled)}`}
      </div>
    );
  },
}));

vi.mock("@/components/proxy/AutoFailoverConfigPanel", () => ({
  AutoFailoverConfigPanel: (props: { appType: string; disabled?: boolean }) => {
    autoFailoverConfigPropsSpy(props);
    return (
      <div data-testid={`auto-failover-${props.appType}`}>
        {`${props.appType}:${String(props.disabled)}`}
      </div>
    );
  },
}));

vi.mock("@/components/settings/RectifierConfigPanel", () => ({
  RectifierConfigPanel: () => <div>rectifier-panel</div>,
}));

vi.mock("@/components/settings/GlobalProxySettings", () => ({
  GlobalProxySettings: () => <div>global-proxy-settings</div>,
}));

vi.mock("@/components/ConfirmDialog", () => ({
  ConfirmDialog: ({
    isOpen,
    title,
    message,
    confirmText = "confirm",
    cancelText = "cancel",
    onConfirm,
    onCancel,
  }: {
    isOpen: boolean;
    title: string;
    message: string;
    confirmText?: string;
    cancelText?: string;
    onConfirm: () => void;
    onCancel: () => void;
  }) =>
    isOpen ? (
      <div data-testid="confirm-dialog">
        <div>{title}</div>
        <div>{message}</div>
        <button onClick={onConfirm}>{confirmText}</button>
        <button onClick={onCancel}>{cancelText}</button>
      </div>
    ) : null,
}));

describe("ProxyTabContent", () => {
  beforeEach(() => {
    startProxyServerMock.mockReset();
    stopWithRestoreMock.mockReset();
    proxyPanelToggleSpy.mockReset();
    proxyPanelPropsSpy.mockReset();
    failoverQueuePropsSpy.mockReset();
    autoFailoverConfigPropsSpy.mockReset();
    proxyStatusFixture = {
      isRunning: false,
      startProxyServer: startProxyServerMock,
      stopWithRestore: stopWithRestoreMock,
      isPending: false,
    };
  });

  it("asks for confirmation before starting the proxy for the first time", async () => {
    const onAutoSave = vi.fn().mockResolvedValue(undefined);
    startProxyServerMock.mockResolvedValue(undefined);

    render(
      <ProxyTabContent
        settings={createSettings({
          proxyConfirmed: false,
          failoverConfirmed: false,
        })}
        onAutoSave={onAutoSave}
      />,
    );

    fireEvent.click(screen.getByText("proxy-panel-toggle"));

    expect(startProxyServerMock).not.toHaveBeenCalled();
    expect(screen.getByText("confirm.proxy.title")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "confirm.proxy.confirm" }));

    await waitFor(() =>
      expect(onAutoSave).toHaveBeenCalledWith({ proxyConfirmed: true }),
    );
    expect(startProxyServerMock).toHaveBeenCalledTimes(1);
  });

  it("asks for confirmation before enabling failover for the first time", async () => {
    const onAutoSave = vi.fn().mockResolvedValue(undefined);

    render(
      <ProxyTabContent
        settings={createSettings({
          proxyConfirmed: true,
          failoverConfirmed: false,
        })}
        onAutoSave={onAutoSave}
      />,
    );

    fireEvent.click(
      screen.getByRole("button", {
        name: "settings.advanced.proxy.enableFailoverToggle",
      }),
    );

    expect(onAutoSave).not.toHaveBeenCalledWith({
      enableFailoverToggle: true,
    });
    expect(screen.getByText("confirm.failover.title")).toBeInTheDocument();

    fireEvent.click(
      screen.getByRole("button", { name: "confirm.failover.confirm" }),
    );

    await waitFor(() =>
      expect(onAutoSave).toHaveBeenCalledWith({
        failoverConfirmed: true,
        enableFailoverToggle: true,
      }),
    );
  });

  it("keeps failover editors available in Web mode and shows explicit runtime-only notices", () => {
    render(
      <ProxyTabContent
        settings={createSettings({
          proxyConfirmed: true,
          failoverConfirmed: true,
        })}
        onAutoSave={vi.fn()}
      />,
    );

    expect(
      screen.getByText("proxy.failover.webConfigOnlyTitle"),
    ).toBeInTheDocument();
    expect(
      screen.getByText("proxy.failover.webConfigOnlyDescription"),
    ).toBeInTheDocument();
    expect(screen.getByTestId("failover-queue-claude")).toHaveTextContent(
      "claude:false",
    );
    expect(screen.getByTestId("auto-failover-claude")).toHaveTextContent(
      "claude:false",
    );
    expect(
      screen.getByText("proxy.failover.runtimeStatsUnavailableTitle"),
    ).toBeInTheDocument();
    expect(
      screen.getByText("proxy.failover.runtimeStatsUnavailableDescription"),
    ).toBeInTheDocument();
    expect(proxyPanelPropsSpy).toHaveBeenCalledWith(
      expect.objectContaining({ disableRuntimeControls: true }),
    );
    expect(failoverQueuePropsSpy).toHaveBeenCalledWith(
      expect.objectContaining({ appType: "claude", disabled: false }),
    );
    expect(autoFailoverConfigPropsSpy).toHaveBeenCalledWith(
      expect.objectContaining({ appType: "claude", disabled: false }),
    );
  });
});
