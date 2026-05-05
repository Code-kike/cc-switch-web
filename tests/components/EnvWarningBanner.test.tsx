import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { EnvWarningBanner } from "@/components/env/EnvWarningBanner";

const deleteEnvVarsMock = vi.fn();
const toastErrorMock = vi.fn();
const toastSuccessMock = vi.fn();
const toastWarningMock = vi.fn();

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) => key,
  }),
}));

vi.mock("sonner", () => ({
  toast: {
    error: (...args: unknown[]) => toastErrorMock(...args),
    success: (...args: unknown[]) => toastSuccessMock(...args),
    warning: (...args: unknown[]) => toastWarningMock(...args),
  },
}));

vi.mock("@/lib/api/env", () => ({
  deleteEnvVars: (...args: unknown[]) => deleteEnvVarsMock(...args),
}));

vi.mock("@/components/ui/button", () => ({
  Button: ({ children, ...props }: any) => <button {...props}>{children}</button>,
}));

vi.mock("@/components/ui/checkbox", () => ({
  Checkbox: ({ checked, onCheckedChange, ...props }: any) => (
    <input
      type="checkbox"
      checked={checked}
      onChange={() => onCheckedChange?.(!checked)}
      {...props}
    />
  ),
}));

vi.mock("@/components/ui/dialog", () => ({
  Dialog: ({ open, children }: any) => (open ? <div>{children}</div> : null),
  DialogContent: ({ children }: any) => <div>{children}</div>,
  DialogDescription: ({ children }: any) => <div>{children}</div>,
  DialogFooter: ({ children }: any) => <div>{children}</div>,
  DialogHeader: ({ children }: any) => <div>{children}</div>,
  DialogTitle: ({ children }: any) => <h2>{children}</h2>,
}));

describe("EnvWarningBanner", () => {
  beforeEach(() => {
    deleteEnvVarsMock.mockReset();
    toastErrorMock.mockReset();
    toastSuccessMock.mockReset();
    toastWarningMock.mockReset();
  });

  it("shows extracted detail when deleting env conflicts fails", async () => {
    deleteEnvVarsMock.mockRejectedValueOnce({ detail: "env delete exploded" });

    render(
      <EnvWarningBanner
        conflicts={[
          {
            varName: "OPENAI_API_KEY",
            varValue: "secret",
            sourcePath: "/tmp/.bashrc",
            sourceType: "file",
          },
        ]}
        onDismiss={vi.fn()}
        onDeleted={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: "env.actions.expand" }));
    fireEvent.click(screen.getByLabelText("OPENAI_API_KEY"));
    fireEvent.click(
      screen.getByRole("button", { name: "env.actions.deleteSelected" }),
    );
    fireEvent.click(screen.getByRole("button", { name: "env.confirm.confirm" }));

    await waitFor(() => {
      expect(toastErrorMock).toHaveBeenCalledWith("env.delete.error", {
        description: "env delete exploded",
      });
    });
  });
});
