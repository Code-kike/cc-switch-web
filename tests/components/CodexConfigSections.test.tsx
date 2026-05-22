import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { CodexConfigSection } from "@/components/providers/forms/CodexConfigSections";

vi.mock("@/components/JsonEditor", () => ({
  default: ({ value }: { value: string }) => (
    <textarea data-testid="json-editor" readOnly value={value} />
  ),
}));

const baseProps = {
  value: [
    'model_provider = "any"',
    'model = "gpt-5.4"',
    "model_context_window = 1000000",
    "model_auto_compact_token_limit = 900000",
  ].join("\n"),
  onChange: vi.fn(),
  useCommonConfig: false,
  onCommonConfigToggle: vi.fn(),
  onEditCommonConfig: vi.fn(),
};

describe("CodexConfigSection", () => {
  it("keeps the TOML editor but hides the unsupported 1M context controls", () => {
    render(<CodexConfigSection {...baseProps} />);

    expect(screen.getByTestId("json-editor")).toHaveValue(baseProps.value);
    expect(
      screen.queryByText("codexConfig.contextWindow1M"),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByText(/codexConfig\.autoCompactLimit/),
    ).not.toBeInTheDocument();
  });
});
