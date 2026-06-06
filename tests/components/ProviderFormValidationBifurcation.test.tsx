import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { describe, it, expect, vi, beforeEach } from "vitest";
import type { ReactElement } from "react";
import { toast } from "sonner";
import { ProviderForm } from "@/components/providers/forms/ProviderForm";

vi.mock("sonner", () => ({
  toast: {
    error: vi.fn(),
    success: vi.fn(),
    info: vi.fn(),
  },
}));

vi.mock("@/lib/query", () => ({
  useSettingsQuery: () => ({
    data: { commonConfigConfirmed: true },
  }),
}));

vi.mock("@/hooks/useOpenClaw", () => ({
  useOpenClawLiveProviderIds: () => ({ data: [], isLoading: false }),
}));

vi.mock("@/hooks/useHermes", () => ({
  useHermesLiveProviderIds: () => ({ data: [], isLoading: false }),
}));

vi.mock("@/components/providers/forms/hooks", async (importOriginal) => {
  const actual =
    await importOriginal<typeof import("@/components/providers/forms/hooks")>();

  return {
    ...actual,
    useCopilotAuth: () => ({ isAuthenticated: false }),
    useCodexOauth: () => ({ isAuthenticated: false }),
    useCommonConfigSnippet: () => ({
      useCommonConfig: false,
      commonConfigSnippet: "",
      commonConfigError: "",
      handleCommonConfigToggle: vi.fn(),
      handleCommonConfigSnippetChange: vi.fn(),
      isExtracting: false,
      handleExtract: vi.fn(),
    }),
    useCodexCommonConfig: () => ({
      useCommonConfig: false,
      commonConfigSnippet: "",
      commonConfigError: "",
      handleCommonConfigToggle: vi.fn(),
      handleCommonConfigSnippetChange: vi.fn(),
      isExtracting: false,
      handleExtract: vi.fn(),
      clearCommonConfigError: vi.fn(),
    }),
    useGeminiCommonConfig: () => ({
      useCommonConfig: false,
      commonConfigSnippet: "",
      commonConfigError: "",
      handleCommonConfigToggle: vi.fn(),
      handleCommonConfigSnippetChange: vi.fn(),
      isExtracting: false,
      handleExtract: vi.fn(),
      clearCommonConfigError: vi.fn(),
    }),
    // M40: inject an INVALID gemini config from hook state. The form field
    // (textarea) defaults to a valid JSON object, but the actually-submitted
    // gemini config is rebuilt from this hook state — exactly the bifurcation
    // the assembler must catch instead of silently saving.
    useGeminiConfigState: () => ({
      geminiEnv: "",
      geminiConfig: "{not valid json",
      geminiApiKey: "",
      geminiBaseUrl: "",
      geminiModel: "",
      envError: "",
      configError: "Invalid JSON format",
      handleGeminiApiKeyChange: vi.fn(),
      handleGeminiBaseUrlChange: vi.fn(),
      handleGeminiModelChange: vi.fn(),
      handleGeminiEnvChange: vi.fn(),
      handleGeminiConfigChange: vi.fn(),
      resetGeminiConfig: vi.fn(),
      envStringToObj: () => ({}),
      envObjToString: () => "",
    }),
  };
});

vi.mock("@/components/providers/forms/ProviderPresetSelector", () => ({
  ProviderPresetSelector: () => null,
}));

vi.mock("@/components/providers/forms/BasicFormFields", () => ({
  BasicFormFields: () => null,
}));

vi.mock("@/components/providers/forms/ClaudeFormFields", () => ({
  ClaudeFormFields: () => null,
}));

vi.mock("@/components/providers/forms/CodexFormFields", () => ({
  CodexFormFields: () => null,
}));

vi.mock("@/components/providers/forms/GeminiFormFields", () => ({
  GeminiFormFields: () => null,
}));

vi.mock("@/components/providers/forms/OpenCodeFormFields", () => ({
  OpenCodeFormFields: () => null,
}));

vi.mock("@/components/providers/forms/OpenClawFormFields", () => ({
  OpenClawFormFields: () => null,
}));

vi.mock("@/components/providers/forms/HermesFormFields", () => ({
  HermesFormFields: () => null,
}));

vi.mock("@/components/providers/forms/OmoFormFields", () => ({
  OmoFormFields: () => null,
}));

vi.mock("@/components/providers/forms/ProviderAdvancedConfig", () => ({
  ProviderAdvancedConfig: () => null,
}));

vi.mock("@/components/providers/forms/CommonConfigEditor", () => ({
  CommonConfigEditor: () => null,
}));

vi.mock("@/components/providers/forms/CodexConfigEditor", () => ({
  default: () => null,
}));

vi.mock("@/components/providers/forms/GeminiConfigEditor", () => ({
  default: () => null,
}));

vi.mock("@/components/JsonEditor", () => ({
  default: () => null,
}));

function renderWithQueryClient(ui: ReactElement) {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });

  return render(
    <QueryClientProvider client={queryClient}>{ui}</QueryClientProvider>,
  );
}

describe("ProviderForm validation bifurcation (M40)", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("submits the assembled config for a valid provider (validated === submitted)", async () => {
    const onSubmit = vi.fn();

    renderWithQueryClient(
      <ProviderForm
        appId="claude"
        submitLabel="Save"
        onSubmit={onSubmit}
        onCancel={vi.fn()}
        initialData={{
          name: "My Claude",
          category: "official",
          settingsConfig: { env: { ANTHROPIC_AUTH_TOKEN: "tok" } },
        }}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() => expect(onSubmit).toHaveBeenCalledTimes(1));
    const payload = onSubmit.mock.calls[0][0];
    const saved = JSON.parse(payload.settingsConfig);
    expect(saved).toEqual({
      env: { ANTHROPIC_AUTH_TOKEN: "tok" },
      ui: { displayName: "My Claude" },
    });
    expect(toast.error).not.toHaveBeenCalled();
  });

  it("hard-rejects a gemini provider whose rebuilt config is invalid, before submitting", async () => {
    const onSubmit = vi.fn();

    renderWithQueryClient(
      <ProviderForm
        appId="gemini"
        submitLabel="Save"
        onSubmit={onSubmit}
        onCancel={vi.fn()}
        initialData={{
          name: "My Gemini",
          category: "official",
          // form-field/textarea value is a VALID object; the invalid part is
          // the rebuilt config coming from the (mocked) gemini hook state.
          settingsConfig: { env: {}, config: {} },
        }}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() => expect(toast.error).toHaveBeenCalled());
    expect(onSubmit).not.toHaveBeenCalled();
  });

  it("keeps the empty-name soft-confirm flow despite providerSchema requiring a name", async () => {
    const onSubmit = vi.fn();

    renderWithQueryClient(
      <ProviderForm
        appId="claude"
        submitLabel="Save"
        onSubmit={onSubmit}
        onCancel={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: "Save" }));

    // The react-hook-form resolver (providerFormSchema) must NOT block the
    // empty name; instead the soft-validation dialog is shown.
    expect(await screen.findByText("配置存在以下问题")).toBeInTheDocument();
    expect(screen.getByText(/请填写供应商名称/)).toBeInTheDocument();
    expect(onSubmit).not.toHaveBeenCalled();
  });
});
