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
  // Ported from upstream a2e22f33, which made `onDuplicate` optional plus
  // conditionally rendered. NOTE ON REACHABILITY: no production caller omits the
  // prop — in this fork `ProviderCard` (:66 required, :910 unconditional),
  // `ProviderList` and `App.tsx` all pass it, and upstream v3.20.0 is the same
  // (ProviderCard is its only consumer). So the optional branch is exercised only
  // from tests. Kept upstream-verbatim to avoid drift in the next sync, with the
  // reverse anchor below so the conditional cannot silently become always-false.
  it("omits duplication when the caller disallows it", () => {
    render(
      <ProviderActions
        appId="codex"
        isCurrent={false}
        onSwitch={vi.fn()}
        onEdit={vi.fn()}
        onDelete={vi.fn()}
      />,
    );

    expect(screen.queryByTitle("provider.duplicate")).not.toBeInTheDocument();
  });

  it("renders duplication when the caller supplies a handler", () => {
    // Reverse anchor for the conditional above: without this, `onDuplicate &&`
    // could regress to always-false and only the negative test would pass.
    renderActions({ appId: "codex" });

    expect(screen.getByTitle("provider.duplicate")).toBeInTheDocument();
  });

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
