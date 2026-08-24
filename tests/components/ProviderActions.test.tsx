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

  it("labels the Pi membership action as enable when not in models.json", () => {
    const props = renderActions({ appId: "pi", isInConfig: false });

    const enableButton = screen.getByRole("button", {
      name: /^(provider\.enable|启用)$/,
    });
    expect(enableButton).not.toBeDisabled();

    fireEvent.click(enableButton);

    expect(props.onSwitch).toHaveBeenCalledTimes(1);
  });

  it("freezes Pi membership and deletion while the native state is unreadable", () => {
    const props = renderActions({
      appId: "pi",
      isInConfig: true,
      isStateChangeProtected: true,
    });

    const removeButton = screen.getByRole("button", {
      name: /^(provider\.removeFromConfig|移除)$/,
    });
    expect(removeButton).toBeDisabled();

    fireEvent.click(removeButton);
    expect(props.onRemoveFromConfig).not.toHaveBeenCalled();

    const deleteButton = screen
      .getAllByTitle("pi.current.stateUnavailableHint")
      .find((element) => element.tagName === "BUTTON");
    expect(deleteButton).toBeDefined();
    fireEvent.click(deleteButton!);
    expect(props.onDelete).not.toHaveBeenCalled();
  });
});
