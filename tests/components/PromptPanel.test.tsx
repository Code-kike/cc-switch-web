import { createRef } from "react";
import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import PromptPanel, { type PromptPanelHandle } from "@/components/prompts/PromptPanel";

const importFromFileMock = vi.fn();
const reloadMock = vi.fn();
const deletePromptMock = vi.fn();
const toggleEnabledMock = vi.fn();
const savePromptMock = vi.fn();

vi.mock("@/hooks/usePromptActions", () => ({
  usePromptActions: () => ({
    prompts: {},
    loading: false,
    currentFileContent: "# Current prompt content",
    reload: reloadMock,
    savePrompt: savePromptMock,
    deletePrompt: deletePromptMock,
    toggleEnabled: toggleEnabledMock,
    importFromFile: importFromFileMock,
  }),
}));

vi.mock("@/components/ConfirmDialog", () => ({
  ConfirmDialog: () => null,
}));

vi.mock("@/components/prompts/PromptFormPanel", () => ({
  default: () => <div data-testid="prompt-form-panel">prompt-form</div>,
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, params?: Record<string, unknown>) => {
      if (key === "prompts.currentFile") {
        return `prompts.currentFile:${String(params?.filename ?? "")}`;
      }
      return key;
    },
  }),
}));

describe("PromptPanel", () => {
  beforeEach(() => {
    importFromFileMock.mockReset();
    reloadMock.mockReset();
    deletePromptMock.mockReset();
    toggleEnabledMock.mockReset();
    savePromptMock.mockReset();
  });

  it("exposes openImport and renders current file content", async () => {
    const ref = createRef<PromptPanelHandle>();

    render(<PromptPanel ref={ref} open onOpenChange={vi.fn()} appId="codex" />);

    await waitFor(() => expect(reloadMock).toHaveBeenCalled());
    expect(
      screen.getByText("prompts.currentFile", { exact: false }),
    ).toBeInTheDocument();
    expect(screen.getByText("# Current prompt content")).toBeInTheDocument();

    importFromFileMock.mockResolvedValueOnce("imported-prompt");
    await act(async () => {
      await ref.current?.openImport();
    });

    expect(importFromFileMock).toHaveBeenCalledTimes(1);
  });

  it("renders the correct Gemini prompt filename in the current file card", async () => {
    render(<PromptPanel open onOpenChange={vi.fn()} appId="gemini" />);

    await waitFor(() => expect(reloadMock).toHaveBeenCalled());
    expect(
      screen.getByText("prompts.currentFile:GEMINI.md"),
    ).toBeInTheDocument();
  });

  it("shows import action in the empty state", async () => {
    render(<PromptPanel open onOpenChange={vi.fn()} appId="claude" />);

    const importButton = screen.getByRole("button", {
      name: "prompts.import",
    });
    fireEvent.click(importButton);

    await waitFor(() => expect(importFromFileMock).toHaveBeenCalledTimes(1));
  });

  it("exposes openAdd through the panel handle", async () => {
    const ref = createRef<PromptPanelHandle>();

    render(<PromptPanel ref={ref} open onOpenChange={vi.fn()} appId="claude" />);

    await act(async () => {
      ref.current?.openAdd();
    });

    expect(screen.getByTestId("prompt-form-panel")).toBeInTheDocument();
  });
});
