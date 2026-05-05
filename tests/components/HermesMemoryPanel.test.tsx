import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import HermesMemoryPanel from "@/components/hermes/HermesMemoryPanel";

const openWebUIMock = vi.fn();
const saveMemoryMutateAsyncMock = vi.fn();
const toggleMemoryMutateMock = vi.fn();
const toastSuccessMock = vi.fn();
const isWebModeMock = vi.fn();

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) => key,
  }),
}));

vi.mock("sonner", () => ({
  toast: {
    success: (...args: unknown[]) => toastSuccessMock(...args),
  },
}));

vi.mock("@/hooks/useDarkMode", () => ({
  useDarkMode: () => false,
}));

vi.mock("@/lib/api/adapter", () => ({
  isWebMode: () => isWebModeMock(),
}));

vi.mock("@/components/MarkdownEditor", () => ({
  default: ({
    value,
    onChange,
  }: {
    value: string;
    onChange: (value: string) => void;
  }) => (
    <textarea
      aria-label="markdown-editor"
      value={value}
      onChange={(event) => onChange(event.target.value)}
    />
  ),
}));

vi.mock("@/components/ui/switch", () => ({
  Switch: ({
    checked,
    disabled,
    onCheckedChange,
  }: {
    checked: boolean;
    disabled?: boolean;
    onCheckedChange: (value: boolean) => void;
  }) => (
    <input
      type="checkbox"
      role="switch"
      checked={checked}
      disabled={disabled}
      onChange={(event) => onCheckedChange(event.target.checked)}
    />
  ),
}));

vi.mock("@/hooks/useHermes", () => ({
  HERMES_WEB_LOCAL_BASE_URL: "http://127.0.0.1:9119",
  useHermesMemory: (kind: "memory" | "user") => ({
    data: kind === "memory" ? "Server memory" : "User memory",
    isLoading: false,
  }),
  useHermesMemoryLimits: () => ({
    data: {
      memory: 2200,
      user: 1375,
      memoryEnabled: true,
      userEnabled: false,
    },
  }),
  useOpenHermesWebUI: () => openWebUIMock,
  useSaveHermesMemory: () => ({
    mutateAsync: (...args: unknown[]) => saveMemoryMutateAsyncMock(...args),
    isPending: false,
  }),
  useToggleHermesMemoryEnabled: () => ({
    mutate: (...args: unknown[]) => toggleMemoryMutateMock(...args),
    isPending: false,
  }),
}));

describe("HermesMemoryPanel", () => {
  beforeEach(() => {
    openWebUIMock.mockReset();
    saveMemoryMutateAsyncMock.mockReset();
    toggleMemoryMutateMock.mockReset();
    toastSuccessMock.mockReset();
    saveMemoryMutateAsyncMock.mockResolvedValue(undefined);
    isWebModeMock.mockReset();
    isWebModeMock.mockReturnValue(false);
  });

  it("opens the Hermes config entrypoint from the panel toolbar", () => {
    render(<HermesMemoryPanel />);

    fireEvent.click(screen.getByText("hermes.memory.openConfig"));

    expect(openWebUIMock).toHaveBeenCalledWith("/config");
  });

  it("saves edited memory content and toggles the enabled switch", async () => {
    render(<HermesMemoryPanel />);

    const editor = await screen.findByLabelText("markdown-editor");
    fireEvent.change(editor, { target: { value: "Updated server memory" } });

    const switchControl = screen.getByRole("switch");
    fireEvent.click(switchControl);
    expect(toggleMemoryMutateMock).toHaveBeenCalledWith({
      kind: "memory",
      enabled: false,
    });

    const saveButton = screen.getByText("common.save");
    await waitFor(() => expect(saveButton).not.toBeDisabled());
    fireEvent.click(saveButton);

    await waitFor(() =>
      expect(saveMemoryMutateAsyncMock).toHaveBeenCalledWith({
        kind: "memory",
        content: "Updated server memory",
      }),
    );
  });

  it("saves and toggles the user memory tab independently", async () => {
    render(<HermesMemoryPanel />);

    const userTab = screen.getByRole("tab", {
      name: "hermes.memory.userTab",
    });
    fireEvent.pointerDown(userTab, { button: 0, ctrlKey: false });
    fireEvent.mouseDown(userTab, { button: 0, ctrlKey: false });
    fireEvent.click(userTab);

    const editor = await screen.findByDisplayValue("User memory");
    const switchControl = screen.getByRole("switch");
    expect(switchControl).not.toBeChecked();

    fireEvent.click(switchControl);
    expect(toggleMemoryMutateMock).toHaveBeenCalledWith({
      kind: "user",
      enabled: true,
    });

    fireEvent.change(editor, { target: { value: "Updated user memory" } });
    fireEvent.click(screen.getByText("common.save"));

    await waitFor(() =>
      expect(saveMemoryMutateAsyncMock).toHaveBeenCalledWith({
        kind: "user",
        content: "Updated user memory",
      }),
    );
  });

  it("shows an inline server-host hint and disables the Hermes Web UI button in web mode", () => {
    isWebModeMock.mockReturnValue(true);

    render(<HermesMemoryPanel />);

    expect(screen.getByText("hermes.webui.remoteHint")).toBeInTheDocument();
    expect(
      screen.getByText(
        "hermes.webui.remoteHintDescription",
      ),
    ).toBeInTheDocument();

    const openButton = screen.getByText("hermes.memory.openConfig");
    expect(openButton).toBeDisabled();

    fireEvent.click(openButton);
    expect(openWebUIMock).not.toHaveBeenCalled();
  });
});
