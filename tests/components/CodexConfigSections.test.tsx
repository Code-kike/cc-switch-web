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
  it("shows the 1M context window toggle and compact-limit input", () => {
    render(<CodexConfigSection {...baseProps} />);

    // TOML editor remains visible
    expect(screen.getByTestId("json-editor")).toHaveValue(baseProps.value);

    // 1M context window toggle is restored (v3.20.0 6e424fd3)
    const toggle = screen.getByRole("checkbox", {
      name: "codexConfig.contextWindow1M",
    });
    expect(toggle).toBeChecked();

    // Compact-limit input mirrors the persisted 900000 and is enabled while
    // the 1M toggle is checked.
    const compactInput = screen.getByRole("textbox", {
      name: /codexConfig\.autoCompactLimit/,
    });
    expect(compactInput).toHaveValue("900000");
    expect(compactInput).not.toBeDisabled();
  });

  it("hides neither toggle when context window is absent (unchecked state)", () => {
    render(
      <CodexConfigSection
        {...baseProps}
        value={['model = "gpt-5.4"', 'model_provider = "any"'].join("\n")}
      />,
    );

    const toggle = screen.getByRole("checkbox", {
      name: "codexConfig.contextWindow1M",
    });
    expect(toggle).not.toBeChecked();

    const compactInput = screen.getByRole("textbox", {
      name: /codexConfig\.autoCompactLimit/,
    });
    // Fallback default when the field is absent
    expect(compactInput).toHaveValue("900000");
    expect(compactInput).toBeDisabled();
  });
});
