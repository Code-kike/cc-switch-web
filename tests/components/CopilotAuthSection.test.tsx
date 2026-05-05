import React, { createContext } from "react";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { CopilotAuthSection } from "@/components/providers/forms/CopilotAuthSection";

const copyTextMock = vi.fn();
const useCopilotAuthMock = vi.fn();
const selectContext = createContext<((value: string) => void) | null>(null);

const state = {
  accounts: [] as Array<{
    id: string;
    login: string;
    avatar_url: string | null;
    github_domain?: string;
  }>,
  defaultAccountId: null as string | null,
  migrationError: null as string | null,
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
      if (key === "copilot.accountCount") {
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

vi.mock("@/components/ui/input", () => ({
  Input: (props: any) => <input {...props} />,
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

vi.mock("@/components/providers/forms/hooks/useCopilotAuth", () => ({
  useCopilotAuth: (...args: unknown[]) => useCopilotAuthMock(...args),
}));

vi.mock("@/lib/clipboard", () => ({
  copyText: (...args: unknown[]) => copyTextMock(...args),
}));

describe("CopilotAuthSection", () => {
  beforeEach(() => {
    Object.assign(state, {
      accounts: [],
      defaultAccountId: null,
      migrationError: null,
      hasAnyAccount: false,
      pollingState: "idle",
      deviceCode: null,
      error: null,
      isPolling: false,
      isAddingAccount: false,
      isRemovingAccount: false,
      isSettingDefaultAccount: false,
    });
    useCopilotAuthMock.mockReset();
    useCopilotAuthMock.mockImplementation(() => state);
    copyTextMock.mockReset();
    state.addAccount.mockReset();
    state.removeAccount.mockReset();
    state.setDefaultAccount.mockReset();
    state.cancelAuth.mockReset();
    state.logout.mockReset();
  });

  it("requires an enterprise domain before allowing enterprise login", () => {
    render(<CopilotAuthSection />);

    fireEvent.click(
      screen.getByRole("button", { name: "GitHub Enterprise Server" }),
    );

    expect(
      screen.getByRole("button", { name: "使用 GitHub 登录" }),
    ).toBeDisabled();

    fireEvent.change(
      screen.getByPlaceholderText("例如：company.ghe.com"),
      { target: { value: " https://company.ghe.com/ " } },
    );

    expect(useCopilotAuthMock).toHaveBeenLastCalledWith("company.ghe.com");
    expect(
      screen.getByRole("button", { name: "使用 GitHub 登录" }),
    ).not.toBeDisabled();

    fireEvent.click(screen.getByRole("button", { name: "使用 GitHub 登录" }));
    expect(state.addAccount).toHaveBeenCalledTimes(1);
  });

  it("disables the initial login button while a login request is pending", () => {
    state.isAddingAccount = true;

    render(<CopilotAuthSection />);

    expect(
      screen.getByRole("button", { name: "使用 GitHub 登录" }),
    ).toBeDisabled();
  });

  it("copies the device code and allows canceling polling", async () => {
    Object.assign(state, {
      pollingState: "polling",
      isPolling: true,
      deviceCode: {
        user_code: "GH-1234",
        verification_uri: "https://github.com/login/device",
      },
    });

    render(<CopilotAuthSection />);

    fireEvent.click(screen.getByTitle("复制代码"));
    await waitFor(() => {
      expect(copyTextMock).toHaveBeenCalledWith("GH-1234");
    });

    fireEvent.click(screen.getByRole("button", { name: "取消" }));
    expect(state.cancelAuth).toHaveBeenCalledTimes(1);
    expect(
      screen.getByRole("link", { name: "https://github.com/login/device" }),
    ).toHaveAttribute("href", "https://github.com/login/device");
  });

  it("removes the selected account and clears the external selection", () => {
    const onAccountSelect = vi.fn();
    Object.assign(state, {
      hasAnyAccount: true,
      accounts: [
        { id: "gh-default", login: "octocat", avatar_url: null },
        { id: "gh-other", login: "hubot", avatar_url: null },
      ],
      defaultAccountId: "gh-default",
    });

    render(
      <CopilotAuthSection
        selectedAccountId="gh-other"
        onAccountSelect={onAccountSelect}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: "设为默认" }));
    expect(state.setDefaultAccount).toHaveBeenCalledWith("gh-other");

    const removeButtons = screen.getAllByTitle("移除账号");
    fireEvent.click(removeButtons[1]);
    expect(state.removeAccount).toHaveBeenCalledWith("gh-other");
    expect(onAccountSelect).toHaveBeenCalledWith(null);

    fireEvent.click(screen.getByRole("button", { name: "注销所有账号" }));
    expect(state.logout).toHaveBeenCalledTimes(1);
  });
});
