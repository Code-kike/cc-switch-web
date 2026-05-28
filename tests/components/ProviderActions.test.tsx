import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { ProviderActions } from "@/components/providers/ProviderActions";

function renderActions(
  overrides: Partial<Parameters<typeof ProviderActions>[0]> = {},
) {
  const props: Parameters<typeof ProviderActions>[0] = {
    appId: "hermes",
    isCurrent: false,
    isInConfig: true,
    onSwitch: vi.fn(),
    onEdit: vi.fn(),
    onDuplicate: vi.fn(),
    onDelete: vi.fn(),
    onRemoveFromConfig: vi.fn(),
    ...overrides,
  };

  render(<ProviderActions {...props} />);
  return props;
}

describe("ProviderActions", () => {
  it("allows removing the current Hermes provider from live config", () => {
    const props = renderActions({
      appId: "hermes",
      isCurrent: true,
      isDefaultModel: true,
    });

    const removeButton = screen.getByRole("button", {
      name: /^(provider\.removeFromConfig|移除)$/,
    });
    expect(removeButton).not.toBeDisabled();

    fireEvent.click(removeButton);

    expect(props.onRemoveFromConfig).toHaveBeenCalledTimes(1);
    expect(props.onDelete).not.toHaveBeenCalled();
  });

  it("keeps the current OpenClaw default model removal guard", () => {
    const props = renderActions({
      appId: "openclaw",
      isCurrent: true,
      isDefaultModel: true,
    });

    const removeButton = screen.getByRole("button", {
      name: /^(provider\.removeFromConfig|移除)$/,
    });
    expect(removeButton).toBeDisabled();

    fireEvent.click(removeButton);

    expect(props.onRemoveFromConfig).not.toHaveBeenCalled();
    expect(props.onDelete).not.toHaveBeenCalled();
  });
});
