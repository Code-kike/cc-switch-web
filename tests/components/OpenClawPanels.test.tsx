import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import EnvPanel from "@/components/openclaw/EnvPanel";
import ToolsPanel from "@/components/openclaw/ToolsPanel";
import AgentsDefaultsPanel from "@/components/openclaw/AgentsDefaultsPanel";

const saveEnvMutateAsyncMock = vi.fn();
const saveToolsMutateAsyncMock = vi.fn();
const saveAgentsMutateAsyncMock = vi.fn();
const toastSuccessMock = vi.fn();
const toastErrorMock = vi.fn();

const openClawState = vi.hoisted(() => ({
  envData: {} as Record<string, unknown>,
  envLoading: false,
  toolsData: {} as Record<string, unknown>,
  toolsLoading: false,
  agentsData: null as Record<string, unknown> | null,
  agentsLoading: false,
  modelOptions: [] as Array<{ value: string; label: string }>,
  modelOptionsLoading: false,
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) => key,
  }),
}));

vi.mock("sonner", () => ({
  toast: {
    success: (...args: unknown[]) => toastSuccessMock(...args),
    error: (...args: unknown[]) => toastErrorMock(...args),
  },
}));

vi.mock("@/components/JsonEditor", () => ({
  default: ({
    value,
    onChange,
  }: {
    value: string;
    onChange: (value: string) => void;
  }) => (
    <textarea
      aria-label="json-editor"
      value={value}
      onChange={(event) => onChange(event.target.value)}
    />
  ),
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
  AlertTitle: ({ children }: any) => <div>{children}</div>,
  AlertDescription: ({ children }: any) => <div>{children}</div>,
}));

vi.mock("@/components/ui/select", () => ({
  Select: ({ value, onValueChange, children }: any) => (
    <select
      value={value}
      onChange={(event) => onValueChange?.(event.target.value)}
    >
      {children}
    </select>
  ),
  SelectTrigger: ({ children }: any) => <>{children}</>,
  SelectValue: () => null,
  SelectContent: ({ children }: any) => <>{children}</>,
  SelectItem: ({ value, children, disabled }: any) => (
    <option value={value} disabled={disabled}>
      {children}
    </option>
  ),
}));

vi.mock("@/hooks/useOpenClaw", () => ({
  useOpenClawEnv: () => ({
    data: openClawState.envData,
    isLoading: openClawState.envLoading,
  }),
  useSaveOpenClawEnv: () => ({
    mutateAsync: (...args: unknown[]) => saveEnvMutateAsyncMock(...args),
    isPending: false,
  }),
  useOpenClawTools: () => ({
    data: openClawState.toolsData,
    isLoading: openClawState.toolsLoading,
  }),
  useSaveOpenClawTools: () => ({
    mutateAsync: (...args: unknown[]) => saveToolsMutateAsyncMock(...args),
    isPending: false,
  }),
  useOpenClawAgentsDefaults: () => ({
    data: openClawState.agentsData,
    isLoading: openClawState.agentsLoading,
  }),
  useSaveOpenClawAgentsDefaults: () => ({
    mutateAsync: (...args: unknown[]) => saveAgentsMutateAsyncMock(...args),
    isPending: false,
  }),
}));

vi.mock("@/components/openclaw/hooks/useOpenClawModelOptions", () => ({
  useOpenClawModelOptions: () => ({
    options: openClawState.modelOptions,
    isLoading: openClawState.modelOptionsLoading,
  }),
}));

describe("OpenClaw panel parity", () => {
  beforeEach(() => {
    saveEnvMutateAsyncMock.mockReset();
    saveToolsMutateAsyncMock.mockReset();
    saveAgentsMutateAsyncMock.mockReset();
    toastSuccessMock.mockReset();
    toastErrorMock.mockReset();

    saveEnvMutateAsyncMock.mockResolvedValue(undefined);
    saveToolsMutateAsyncMock.mockResolvedValue(undefined);
    saveAgentsMutateAsyncMock.mockResolvedValue(undefined);

    openClawState.envData = {};
    openClawState.envLoading = false;
    openClawState.toolsData = {};
    openClawState.toolsLoading = false;
    openClawState.agentsData = null;
    openClawState.agentsLoading = false;
    openClawState.modelOptions = [];
    openClawState.modelOptionsLoading = false;
  });

  it("saves env JSON and surfaces parse validation errors", async () => {
    openClawState.envData = { vars: { TOKEN: "abc" } };

    const { rerender } = render(<EnvPanel />);

    fireEvent.change(screen.getByLabelText("json-editor"), {
      target: { value: '{\n  "vars": {\n    "TOKEN": "next"\n  }\n}' },
    });
    fireEvent.click(screen.getByText("common.save"));

    await waitFor(() =>
      expect(saveEnvMutateAsyncMock).toHaveBeenCalledWith({
        vars: { TOKEN: "next" },
      }),
    );

    fireEvent.change(screen.getByLabelText("json-editor"), {
      target: { value: "{ invalid json" },
    });
    fireEvent.click(screen.getByText("common.save"));

    await waitFor(() =>
      expect(toastErrorMock).toHaveBeenCalledWith("openclaw.env.saveFailed", {
        description: "openclaw.env.invalidJson",
      }),
    );

    rerender(<EnvPanel />);
  });

  it("allows replacing an unsupported tools profile and saves trimmed lists", async () => {
    openClawState.toolsData = {
      profile: "legacy-profile",
      allow: ["allow:read"],
      deny: ["deny:write"],
      passthrough: true,
    };

    render(<ToolsPanel />);

    expect(
      screen.getByText("openclaw.tools.unsupportedProfileTitle"),
    ).toBeInTheDocument();

    const select = screen.getByRole("combobox");
    fireEvent.change(select, { target: { value: "full" } });

    fireEvent.click(screen.getByText("openclaw.tools.addAllow"));
    fireEvent.click(screen.getByText("openclaw.tools.addDeny"));

    const patternInputs = screen.getAllByPlaceholderText(
      "openclaw.tools.patternPlaceholder",
    );
    fireEvent.change(patternInputs[1], { target: { value: "allow:new" } });
    fireEvent.change(patternInputs[3], { target: { value: "deny:new" } });

    fireEvent.click(screen.getByText("common.save"));

    await waitFor(() =>
      expect(saveToolsMutateAsyncMock).toHaveBeenCalledWith({
        profile: "full",
        allow: ["allow:read", "allow:new"],
        deny: ["deny:write", "deny:new"],
        passthrough: true,
      }),
    );
  });

  it("clears stale model config and migrates legacy timeout seconds on save", async () => {
    openClawState.modelOptions = [
      { value: "provider-a/model-1", label: "Provider A / Model 1" },
      { value: "provider-a/model-2", label: "Provider A / Model 2" },
    ];
    openClawState.agentsData = {
      model: {
        primary: "provider-a/model-1",
        fallbacks: ["provider-a/model-2"],
      },
      workspace: "~/projects",
      timeout: 300,
      contextTokens: 200000,
      maxConcurrent: 4,
      unknownFlag: true,
    };

    render(<AgentsDefaultsPanel />);

    expect(
      screen.getByText("openclaw.agents.legacyTimeoutTitle"),
    ).toBeInTheDocument();

    const selects = screen.getAllByRole("combobox");
    fireEvent.change(selects[0], { target: { value: "__unset__" } });
    fireEvent.change(selects[1], { target: { value: "__unset__" } });

    fireEvent.click(screen.getByText("common.save"));

    await waitFor(() =>
      expect(saveAgentsMutateAsyncMock).toHaveBeenCalledWith({
        workspace: "~/projects",
        timeoutSeconds: 300,
        contextTokens: 200000,
        maxConcurrent: 4,
        unknownFlag: true,
      }),
    );
  });
});
