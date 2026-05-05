import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { AuthCenterPanel } from "@/components/settings/AuthCenterPanel";

const copilotState = {
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

const codexState = {
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
    t: (key: string, options?: Record<string, unknown>) => {
      if (key === "copilot.accountCount" || key === "codexOauth.accountCount") {
        return `${String(options?.count ?? 0)} 个账号`;
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
  Label: ({ children }: any) => <label>{children}</label>,
}));

vi.mock("@/components/ui/input", () => ({
  Input: (props: any) => <input {...props} />,
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

vi.mock("@/components/ui/select", () => ({
  Select: ({ children }: any) => <div>{children}</div>,
  SelectTrigger: ({ children }: any) => <div>{children}</div>,
  SelectValue: ({ placeholder }: any) => <span>{placeholder}</span>,
  SelectContent: ({ children }: any) => <div>{children}</div>,
  SelectItem: ({ children, value }: any) => <div data-value={value}>{children}</div>,
}));

vi.mock("@/components/BrandIcons", () => ({
  CodexIcon: () => <span>codex-icon</span>,
}));

vi.mock("@/components/providers/forms/hooks/useCopilotAuth", () => ({
  useCopilotAuth: () => copilotState,
}));

vi.mock("@/components/providers/forms/hooks/useCodexOauth", () => ({
  useCodexOauth: () => codexState,
}));

vi.mock("@/lib/clipboard", () => ({
  copyText: vi.fn(),
}));

describe("AuthCenterPanel", () => {
  beforeEach(() => {
    Object.assign(copilotState, {
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
    Object.assign(codexState, {
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
    copilotState.addAccount.mockReset();
    codexState.addAccount.mockReset();
    copilotState.setDefaultAccount.mockReset();
    codexState.setDefaultAccount.mockReset();
    copilotState.removeAccount.mockReset();
    codexState.removeAccount.mockReset();
    copilotState.logout.mockReset();
    codexState.logout.mockReset();
  });

  it("renders login actions for both auth providers when unauthenticated", () => {
    render(<AuthCenterPanel />);

    expect(screen.getByText("settings.authCenter.title")).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "copilot.loginWithGitHub" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "codexOauth.loginWithChatGPT" }),
    ).toBeInTheDocument();
  });

  it("shows device-code polling UI with verification links", () => {
    Object.assign(copilotState, {
      hasAnyAccount: false,
      pollingState: "polling",
      isPolling: true,
      deviceCode: {
        user_code: "GH-1234",
        verification_uri: "https://github.com/login/device",
      },
    });
    Object.assign(codexState, {
      hasAnyAccount: false,
      pollingState: "polling",
      isPolling: true,
      deviceCode: {
        user_code: "OA-5678",
        verification_uri: "https://openai.com/device",
      },
    });

    render(<AuthCenterPanel />);

    expect(screen.getByText("GH-1234")).toBeInTheDocument();
    expect(screen.getByText("OA-5678")).toBeInTheDocument();
    expect(
      screen.getByRole("link", { name: "https://github.com/login/device" }),
    ).toHaveAttribute("href", "https://github.com/login/device");
    expect(
      screen.getByRole("link", { name: "https://openai.com/device" }),
    ).toHaveAttribute("href", "https://openai.com/device");
  });

  it("shows account management actions when multiple accounts exist", () => {
    Object.assign(copilotState, {
      hasAnyAccount: true,
      accounts: [
        { id: "gh-default", login: "octocat", avatar_url: null },
        { id: "gh-other", login: "hubot", avatar_url: null },
      ],
      defaultAccountId: "gh-default",
    });
    Object.assign(codexState, {
      hasAnyAccount: true,
      accounts: [
        { id: "oa-default", login: "plus-user" },
        { id: "oa-other", login: "pro-user" },
      ],
      defaultAccountId: "oa-default",
    });

    render(<AuthCenterPanel />);

    const defaultButtons = screen.getAllByRole("button", {
      name: /setAsDefault/i,
    });
    expect(defaultButtons).toHaveLength(2);

    const removeButtons = screen.getAllByTitle(/removeAccount/i);
    expect(removeButtons).toHaveLength(4);

    const logoutButtons = screen.getAllByRole("button", {
      name: /logoutAll/i,
    });
    expect(logoutButtons).toHaveLength(2);

    fireEvent.click(defaultButtons[0]);
    expect(copilotState.setDefaultAccount).toHaveBeenCalledWith("gh-other");

    fireEvent.click(defaultButtons[1]);
    expect(codexState.setDefaultAccount).toHaveBeenCalledWith("oa-other");

    fireEvent.click(removeButtons[1]);
    expect(copilotState.removeAccount).toHaveBeenCalledWith("gh-other");

    fireEvent.click(removeButtons[3]);
    expect(codexState.removeAccount).toHaveBeenCalledWith("oa-other");

    fireEvent.click(logoutButtons[0]);
    expect(copilotState.logout).toHaveBeenCalledTimes(1);

    fireEvent.click(logoutButtons[1]);
    expect(codexState.logout).toHaveBeenCalledTimes(1);
  });
});
