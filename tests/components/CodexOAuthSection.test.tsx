import React, { createContext } from "react";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { CodexOAuthSection } from "@/components/providers/forms/CodexOAuthSection";

const copyTextMock = vi.fn();
const useCodexOauthMock = vi.fn();
const selectContext = createContext<((value: string) => void) | null>(null);

const state = {
  accounts: [] as Array<{ id: string; login: string }>,
  defaultAccountId: null as string | null,
  hasAnyAccount: false,
  pollingState: "idle" as "idle" | "polling" | "success" | "error",
  deviceCode: null as null | {
    user_code: string;
    verification_uri: string;
  },
  error: null as string | null,
  isPolling: false,
  isAddingAccount: false,
  isRemovingAccount: false,
  isSettingDefaultAccount: false,
  addAccount: vi.fn(),
  removeAccount: vi.fn(),
  setDefaultAccount: vi.fn(),
  cancelAuth: vi.fn(),
  logout: vi.fn(),
};

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, options?: Record<string, unknown> | string) => {
      if (typeof options === "string") {
        return options;
      }
      if (key === "codexOauth.accountCount") {
        return `${String(options?.count ?? 0)} 个账号`;
      }
      if (options && typeof options.defaultValue === "string") {
        return options.defaultValue;
      }
      return key;
    },
  }),
}));

vi.mock("@/components/ui/badge", () => ({
  Badge: ({ children }: any) => <div>{children}</div>,
}));

vi.mock("@/components/ui/button", () => ({
  Button: ({ children, onClick, ...props }: any) => (
    <button type="button" onClick={onClick} {...props}>
      {children}
    </button>
  ),
}));

vi.mock("@/components/ui/label", () => ({
  Label: ({ children, ...props }: any) => <label {...props}>{children}</label>,
}));

vi.mock("@/components/ui/select", () => ({
  Select: ({ children, onValueChange }: any) => (
    <selectContext.Provider value={onValueChange ?? null}>
      <div>{children}</div>
    </selectContext.Provider>
  ),
  SelectTrigger: ({ children }: any) => <div>{children}</div>,
  SelectValue: ({ placeholder }: any) => <span>{placeholder}</span>,
  SelectContent: ({ children }: any) => <div>{children}</div>,
  SelectItem: ({ children, value }: any) => {
    const onValueChange = React.useContext(selectContext);
    return (
      <button type="button" onClick={() => onValueChange?.(value)}>
        {children}
      </button>
    );
  },
}));

vi.mock("@/components/ui/switch", () => ({
  Switch: ({ checked, onCheckedChange, ...props }: any) => (
    <button
      type="button"
      aria-pressed={checked}
      onClick={() => onCheckedChange?.(!checked)}
      {...props}
    />
  ),
}));

vi.mock("@/components/providers/forms/hooks/useCodexOauth", () => ({
  useCodexOauth: (...args: unknown[]) => useCodexOauthMock(...args),
}));

vi.mock("@/lib/clipboard", () => ({
  copyText: (...args: unknown[]) => copyTextMock(...args),
}));

describe("CodexOAuthSection", () => {
  beforeEach(() => {
    Object.assign(state, {
      accounts: [],
      defaultAccountId: null,
      hasAnyAccount: false,
      pollingState: "idle",
      deviceCode: null,
      error: null,
      isPolling: false,
      isAddingAccount: false,
      isRemovingAccount: false,
      isSettingDefaultAccount: false,
    });
    useCodexOauthMock.mockReset();
    useCodexOauthMock.mockImplementation(() => state);
    copyTextMock.mockReset();
    state.addAccount.mockReset();
    state.removeAccount.mockReset();
    state.setDefaultAccount.mockReset();
    state.cancelAuth.mockReset();
    state.logout.mockReset();
  });

  it("disables initial login while a login request is pending and toggles FAST mode", () => {
    const onFastModeChange = vi.fn();
    state.isAddingAccount = true;

    render(
      <CodexOAuthSection
        fastModeEnabled={false}
        onFastModeChange={onFastModeChange}
      />,
    );

    expect(
      screen.getByRole("button", { name: "使用 ChatGPT 登录" }),
    ).toBeDisabled();

    fireEvent.click(screen.getByLabelText("FAST mode"));
    expect(onFastModeChange).toHaveBeenCalledWith(true);
  });

  it("copies the device code and allows canceling polling", async () => {
    Object.assign(state, {
      pollingState: "polling",
      isPolling: true,
      deviceCode: {
        user_code: "OA-5678",
        verification_uri: "https://openai.com/device",
      },
    });

    render(<CodexOAuthSection />);

    fireEvent.click(screen.getByTitle("复制代码"));
    await waitFor(() => {
      expect(copyTextMock).toHaveBeenCalledWith("OA-5678");
    });

    fireEvent.click(screen.getByRole("button", { name: "取消" }));
    expect(state.cancelAuth).toHaveBeenCalledTimes(1);
    expect(
      screen.getByRole("link", { name: "https://openai.com/device" }),
    ).toHaveAttribute("href", "https://openai.com/device");
  });

  it("retries a failed login flow and exposes account management actions", () => {
    const onAccountSelect = vi.fn();
    Object.assign(state, {
      hasAnyAccount: true,
      accounts: [
        { id: "oa-default", login: "plus-user" },
        { id: "oa-other", login: "pro-user" },
      ],
      defaultAccountId: "oa-default",
      pollingState: "error",
      error: "Auth failed",
    });

    render(
      <CodexOAuthSection
        selectedAccountId="oa-other"
        onAccountSelect={onAccountSelect}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: "设为默认" }));
    expect(state.setDefaultAccount).toHaveBeenCalledWith("oa-other");

    const removeButtons = screen.getAllByTitle("移除账号");
    fireEvent.click(removeButtons[1]);
    expect(state.removeAccount).toHaveBeenCalledWith("oa-other");
    expect(onAccountSelect).toHaveBeenCalledWith(null);

    fireEvent.click(screen.getByRole("button", { name: "注销所有账号" }));
    expect(state.logout).toHaveBeenCalledTimes(1);

    fireEvent.click(screen.getByRole("button", { name: "重试" }));
    expect(state.addAccount).toHaveBeenCalledTimes(1);
  });
});
