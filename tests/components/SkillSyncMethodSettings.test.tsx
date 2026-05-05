import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { SkillSyncMethodSettings } from "@/components/settings/SkillSyncMethodSettings";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) => key,
  }),
}));

describe("SkillSyncMethodSettings", () => {
  it("treats auto as symlink in the UI", () => {
    const onChange = vi.fn();

    render(<SkillSyncMethodSettings value="auto" onChange={onChange} />);

    expect(screen.getByRole("button", { name: "settings.skillSync.symlink" })).toHaveClass(
      "shadow-sm",
    );
    expect(screen.getByText("settings.skillSync.symlinkHint")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "settings.skillSync.copy" }));
    expect(onChange).toHaveBeenCalledWith("copy");
  });

  it("selects symlink and copy explicitly", () => {
    const onChange = vi.fn();

    render(<SkillSyncMethodSettings value="copy" onChange={onChange} />);

    expect(screen.getByRole("button", { name: "settings.skillSync.copy" })).toHaveClass(
      "shadow-sm",
    );
    expect(
      screen.queryByText("settings.skillSync.symlinkHint"),
    ).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "settings.skillSync.symlink" }));
    expect(onChange).toHaveBeenCalledWith("symlink");
  });
});
