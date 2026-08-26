import { fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { AppSwitcher } from "@/components/AppSwitcher";

describe("AppSwitcher", () => {
  afterEach(() => {
    vi.restoreAllMocks();
    localStorage.clear();
  });

  it("keeps the active app visible and switches apps from the overflow menu", async () => {
    vi.spyOn(HTMLElement.prototype, "offsetWidth", "get").mockReturnValue(44);
    vi.spyOn(HTMLElement.prototype, "clientWidth", "get").mockReturnValue(140);
    const onSwitch = vi.fn();

    render(
      <div>
        <AppSwitcher activeApp="hermes" onSwitch={onSwitch} />
      </div>,
    );

    expect(screen.getByRole("button", { name: "Hermes" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /Codex/ })).toBeNull();

    fireEvent.click(screen.getByRole("button", { name: "appSwitcher.more" }));
    fireEvent.click(await screen.findByRole("button", { name: /Codex/ }));

    expect(onSwitch).toHaveBeenCalledWith("codex");
    expect(localStorage.getItem("cc-switch-last-app")).toBe("codex");
  });
});
