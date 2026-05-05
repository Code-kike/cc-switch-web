import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi, beforeEach } from "vitest";
import { TerminalSettings } from "@/components/settings/TerminalSettings";

const isMacMock = vi.fn();
const isWindowsMock = vi.fn();
const isLinuxMock = vi.fn();

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) => key,
  }),
}));

vi.mock("@/lib/platform", () => ({
  isMac: () => isMacMock(),
  isWindows: () => isWindowsMock(),
  isLinux: () => isLinuxMock(),
}));

vi.mock("@/components/ui/select", () => ({
  Select: ({ value, onValueChange, children }: any) => (
    <select
      aria-label="terminal-select"
      value={value}
      onChange={(event) => onValueChange(event.target.value)}
    >
      {children}
    </select>
  ),
  SelectTrigger: ({ children }: any) => <>{children}</>,
  SelectValue: () => null,
  SelectContent: ({ children }: any) => <>{children}</>,
  SelectItem: ({ value, children }: any) => (
    <option value={value}>{children}</option>
  ),
}));

describe("TerminalSettings", () => {
  beforeEach(() => {
    isMacMock.mockReset();
    isWindowsMock.mockReset();
    isLinuxMock.mockReset();
  });

  it("uses Linux defaults and options when on Linux", () => {
    const onChange = vi.fn();
    isMacMock.mockReturnValue(false);
    isWindowsMock.mockReturnValue(false);
    isLinuxMock.mockReturnValue(true);

    render(<TerminalSettings onChange={onChange} />);

    const select = screen.getByLabelText("terminal-select");
    expect(select).toHaveValue("gnome-terminal");
    expect(
      screen.getByRole("option", {
        name: "settings.terminal.options.linux.gnomeTerminal",
      }),
    ).toBeInTheDocument();

    fireEvent.change(select, { target: { value: "kitty" } });
    expect(onChange).toHaveBeenCalledWith("kitty");
  });

  it("falls back to macOS defaults when no platform matches", () => {
    const onChange = vi.fn();
    isMacMock.mockReturnValue(false);
    isWindowsMock.mockReturnValue(false);
    isLinuxMock.mockReturnValue(false);

    render(<TerminalSettings value="terminal" onChange={onChange} />);

    const select = screen.getByLabelText("terminal-select");
    expect(select).toHaveValue("terminal");
    expect(
      screen.getByRole("option", {
        name: "settings.terminal.options.macos.terminal",
      }),
    ).toBeInTheDocument();

    fireEvent.change(select, { target: { value: "iterm2" } });
    expect(onChange).toHaveBeenCalledWith("iterm2");
  });
});
