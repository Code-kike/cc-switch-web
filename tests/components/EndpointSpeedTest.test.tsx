import type { ComponentProps, ReactNode } from "react";
import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import EndpointSpeedTest from "@/components/providers/forms/EndpointSpeedTest";

const testApiEndpointsMock = vi.fn();
const getCustomEndpointsMock = vi.fn();
const addCustomEndpointMock = vi.fn();
const removeCustomEndpointMock = vi.fn();
const i18nState = vi.hoisted(() => ({
  t: (
    key: string,
    options?: {
      error?: string;
    },
  ) => (options?.error ? `${key}:${options.error}` : key),
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: i18nState.t,
  }),
}));

vi.mock("@/components/common/FullScreenPanel", () => ({
  FullScreenPanel: ({
    isOpen,
    title,
    children,
    footer,
  }: {
    isOpen: boolean;
    title: string;
    children: ReactNode;
    footer?: ReactNode;
  }) =>
    isOpen ? (
      <div data-testid="endpoint-speed-test-panel">
        <h2>{title}</h2>
        <div>{children}</div>
        <div>{footer}</div>
      </div>
    ) : null,
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

vi.mock("@/lib/api/vscode", () => ({
  vscodeApi: {
    testApiEndpoints: (...args: unknown[]) => testApiEndpointsMock(...args),
    getCustomEndpoints: (...args: unknown[]) => getCustomEndpointsMock(...args),
    addCustomEndpoint: (...args: unknown[]) => addCustomEndpointMock(...args),
    removeCustomEndpoint: (...args: unknown[]) =>
      removeCustomEndpointMock(...args),
  },
}));

function renderPanel(
  overrides: Partial<ComponentProps<typeof EndpointSpeedTest>> = {},
) {
  const props: ComponentProps<typeof EndpointSpeedTest> = {
    appId: "claude",
    value: "",
    onChange: vi.fn(),
    initialEndpoints: [],
    visible: true,
    onClose: vi.fn(),
    autoSelect: false,
    onAutoSelectChange: vi.fn(),
    ...overrides,
  };

  render(<EndpointSpeedTest {...props} />);

  return props;
}

function getAddButton() {
  const input = screen.getByPlaceholderText("endpointTest.addEndpointPlaceholder");
  return within(input.parentElement as HTMLElement).getByRole("button");
}

function getRemoveButtonForUrl(url: string) {
  const row = screen.getByText(url).closest(".group");
  expect(row).not.toBeNull();
  return within(row as HTMLElement).getByRole("button");
}

describe("EndpointSpeedTest", () => {
  beforeEach(() => {
    testApiEndpointsMock.mockReset();
    getCustomEndpointsMock.mockReset();
    addCustomEndpointMock.mockReset();
    removeCustomEndpointMock.mockReset();

    testApiEndpointsMock.mockResolvedValue([]);
    getCustomEndpointsMock.mockResolvedValue([]);
    addCustomEndpointMock.mockResolvedValue(undefined);
    removeCustomEndpointMock.mockResolvedValue(undefined);
  });

  it("runs endpoint tests with app-specific timeout and auto-selects the fastest result", async () => {
    testApiEndpointsMock.mockResolvedValue([
      { url: "https://slow.example.com", latency: 110, status: 200 },
      { url: "https://fast.example.com", latency: 45, status: 200 },
    ]);

    const props = renderPanel({
      appId: "codex",
      value: "https://slow.example.com",
      initialEndpoints: [
        { url: "https://slow.example.com" },
        { url: "https://fast.example.com" },
      ],
      autoSelect: true,
    });

    fireEvent.click(
      screen.getByRole("button", { name: "endpointTest.testSpeed" }),
    );

    await waitFor(() =>
      expect(testApiEndpointsMock).toHaveBeenCalledWith(
        ["https://slow.example.com", "https://fast.example.com"],
        { timeoutSecs: 12 },
      ),
    );
    await waitFor(() =>
      expect(props.onChange).toHaveBeenCalledWith("https://fast.example.com"),
    );

    expect(screen.getByText("45ms")).toBeInTheDocument();
    expect(screen.getByText("110ms")).toBeInTheDocument();
  });

  it("shows the returned endpoint error while still auto-selecting the fastest successful endpoint", async () => {
    testApiEndpointsMock.mockResolvedValue([
      {
        url: "https://broken.example.com",
        latency: null,
        status: undefined,
        error: "connection failed",
      },
      { url: "https://fast.example.com", latency: 35, status: 200 },
    ]);

    const props = renderPanel({
      value: "https://broken.example.com",
      initialEndpoints: [
        { url: "https://broken.example.com" },
        { url: "https://fast.example.com" },
      ],
      autoSelect: true,
    });

    fireEvent.click(
      screen.getByRole("button", { name: "endpointTest.testSpeed" }),
    );

    await waitFor(() =>
      expect(props.onChange).toHaveBeenCalledWith("https://fast.example.com"),
    );

    expect(screen.getByText("connection failed")).toBeInTheDocument();
    expect(screen.getByText("35ms")).toBeInTheDocument();
  });

  it("shows structured details when endpoint speed testing fails", async () => {
    testApiEndpointsMock.mockRejectedValueOnce({ message: "network down" });

    renderPanel({
      initialEndpoints: [{ url: "https://broken.example.com" }],
    });

    fireEvent.click(
      screen.getByRole("button", { name: "endpointTest.testSpeed" }),
    );

    await waitFor(() =>
      expect(
        screen.getByText("endpointTest.testFailed:network down"),
      ).toBeInTheDocument(),
    );
    expect(screen.queryByText(/\[object Object\]/)).not.toBeInTheDocument();
  });

  it("loads saved custom endpoints in edit mode and persists only add/remove diffs on save", async () => {
    getCustomEndpointsMock.mockResolvedValue([
      { url: "https://saved.example.com", addedAt: 1 },
    ]);

    const props = renderPanel({
      appId: "claude",
      providerId: "provider-1",
      value: "https://preset.example.com",
      initialEndpoints: [{ url: "https://preset.example.com" }],
    });

    await waitFor(() =>
      expect(getCustomEndpointsMock).toHaveBeenCalledWith(
        "claude",
        "provider-1",
      ),
    );
    expect(screen.getByText("https://saved.example.com")).toBeInTheDocument();

    fireEvent.change(
      screen.getByPlaceholderText("endpointTest.addEndpointPlaceholder"),
      {
        target: { value: "https://new.example.com/" },
      },
    );
    fireEvent.click(getAddButton());

    fireEvent.click(getRemoveButtonForUrl("https://saved.example.com"));
    fireEvent.click(screen.getByRole("button", { name: "common.save" }));

    await waitFor(() =>
      expect(addCustomEndpointMock).toHaveBeenCalledWith(
        "claude",
        "provider-1",
        "https://new.example.com",
      ),
    );
    await waitFor(() =>
      expect(removeCustomEndpointMock).toHaveBeenCalledWith(
        "claude",
        "provider-1",
        "https://saved.example.com",
      ),
    );

    expect(addCustomEndpointMock).toHaveBeenCalledTimes(1);
    expect(removeCustomEndpointMock).toHaveBeenCalledTimes(1);
    expect(props.onClose).toHaveBeenCalledTimes(1);
  });

  it("shows structured details when saving endpoint diffs fails", async () => {
    getCustomEndpointsMock.mockResolvedValue([]);
    addCustomEndpointMock.mockRejectedValueOnce({
      payload: { detail: "save exploded" },
    });

    const props = renderPanel({
      appId: "claude",
      providerId: "provider-1",
      initialEndpoints: [{ url: "https://preset.example.com" }],
    });

    await waitFor(() =>
      expect(getCustomEndpointsMock).toHaveBeenCalledWith(
        "claude",
        "provider-1",
      ),
    );

    fireEvent.change(
      screen.getByPlaceholderText("endpointTest.addEndpointPlaceholder"),
      {
        target: { value: "https://new.example.com/" },
      },
    );
    fireEvent.click(getAddButton());
    fireEvent.click(screen.getByRole("button", { name: "common.save" }));

    await waitFor(() =>
      expect(addCustomEndpointMock).toHaveBeenCalledWith(
        "claude",
        "provider-1",
        "https://new.example.com",
      ),
    );
    expect(screen.getByText("save exploded")).toBeInTheDocument();
    expect(screen.queryByText(/\[object Object\]/)).not.toBeInTheDocument();
    expect(props.onClose).not.toHaveBeenCalled();
  });

  it("rejects invalid or duplicate URLs and forwards new custom endpoints in create mode", async () => {
    const onCustomEndpointsChange = vi.fn();
    const props = renderPanel({
      initialEndpoints: [{ url: "https://existing.example.com" }],
      onCustomEndpointsChange,
    });

    const input = screen.getByPlaceholderText(
      "endpointTest.addEndpointPlaceholder",
    );

    fireEvent.change(input, { target: { value: "not-a-url" } });
    fireEvent.click(getAddButton());
    expect(screen.getByText("endpointTest.invalidUrlFormat")).toBeInTheDocument();

    fireEvent.change(input, {
      target: { value: "https://existing.example.com/" },
    });
    fireEvent.click(getAddButton());
    expect(screen.getByText("endpointTest.urlExists")).toBeInTheDocument();

    fireEvent.change(input, { target: { value: "https://custom.example.com/" } });
    fireEvent.click(getAddButton());

    await waitFor(() =>
      expect(props.onChange).toHaveBeenCalledWith("https://custom.example.com"),
    );
    await waitFor(() =>
      expect(onCustomEndpointsChange).toHaveBeenLastCalledWith([
        "https://custom.example.com",
      ]),
    );
  });
});
