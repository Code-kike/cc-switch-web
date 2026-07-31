import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import type { ComponentProps } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { toast } from "sonner";
import { CodexFormFields } from "@/components/providers/forms/CodexFormFields";

const modelFetchApiMock = vi.hoisted(() => ({
  fetchModelsForConfig: vi.fn(),
  fetchXaiOauthModels: vi.fn(),
  showFetchModelsError: vi.fn(),
}));

vi.mock("sonner", () => ({
  toast: {
    error: vi.fn(),
    info: vi.fn(),
    success: vi.fn(),
  },
}));

vi.mock("@/lib/api/model-fetch", () => ({
  fetchModelsForConfig: modelFetchApiMock.fetchModelsForConfig,
  fetchXaiOauthModels: modelFetchApiMock.fetchXaiOauthModels,
  showFetchModelsError: modelFetchApiMock.showFetchModelsError,
}));

vi.mock("@/components/providers/forms/XaiOAuthSection", () => ({
  XaiOAuthSection: () => <div data-testid="xai-oauth-section" />,
}));

vi.mock("@/components/providers/forms/EndpointSpeedTest", () => ({
  default: () => <div data-testid="endpoint-speed-test" />,
}));

vi.mock("@/components/providers/forms/shared", () => ({
  ApiKeySection: () => <div data-testid="api-key-section" />,
  EndpointField: () => <div data-testid="endpoint-field" />,
  ModelInputWithFetch: () => <div data-testid="model-input" />,
}));

type CodexFormFieldsProps = ComponentProps<typeof CodexFormFields>;

const renderCodexForm = (overrides: Partial<CodexFormFieldsProps> = {}) => {
  const props: CodexFormFieldsProps = {
    providerId: "provider-1",
    codexApiKey: "",
    onApiKeyChange: vi.fn(),
    category: "third_party",
    shouldShowApiKeyLink: false,
    websiteUrl: "",
    shouldShowSpeedTest: true,
    codexBaseUrl: "https://api.example.com/v1",
    onBaseUrlChange: vi.fn(),
    isFullUrl: false,
    onFullUrlChange: vi.fn(),
    isEndpointModalOpen: false,
    onEndpointModalToggle: vi.fn(),
    onCustomEndpointsChange: vi.fn(),
    autoSelect: false,
    onAutoSelectChange: vi.fn(),
    shouldShowModelField: true,
    modelName: "grok-4.5",
    onModelNameChange: vi.fn(),
    speedTestEndpoints: [],
    ...overrides,
  };

  return render(<CodexFormFields {...props} />);
};

describe("CodexFormFields", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    modelFetchApiMock.fetchModelsForConfig.mockResolvedValue([]);
    modelFetchApiMock.fetchXaiOauthModels.mockResolvedValue([]);
  });

  it("uses the managed xAI account flow without editable credentials", async () => {
    renderCodexForm({
      isXaiOauthPreset: true,
      isXaiOauthAuthenticated: true,
      selectedXaiAccountId: "xai-1",
    });

    expect(screen.getByTestId("xai-oauth-section")).toBeInTheDocument();
    expect(screen.queryByTestId("api-key-section")).not.toBeInTheDocument();
    expect(screen.queryByTestId("endpoint-field")).not.toBeInTheDocument();

    fireEvent.click(
      screen.getByRole("button", { name: "providerForm.fetchModels" }),
    );

    await waitFor(() => {
      expect(modelFetchApiMock.fetchXaiOauthModels).toHaveBeenCalledWith(
        "xai-1",
      );
    });
    expect(modelFetchApiMock.fetchModelsForConfig).not.toHaveBeenCalled();
  });

  it("requires an authenticated xAI account before fetching models", () => {
    renderCodexForm({
      isXaiOauthPreset: true,
      isXaiOauthAuthenticated: false,
    });

    fireEvent.click(
      screen.getByRole("button", { name: "providerForm.fetchModels" }),
    );

    expect(toast.error).toHaveBeenCalledWith("请先登录 xAI 账号");
    expect(modelFetchApiMock.fetchXaiOauthModels).not.toHaveBeenCalled();
  });
});
