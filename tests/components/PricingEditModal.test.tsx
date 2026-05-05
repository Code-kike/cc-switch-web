import type { ReactNode } from "react";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { PricingEditModal } from "@/components/usage/PricingEditModal";

const toastErrorMock = vi.fn();
const mutateAsyncMock = vi.fn();

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (
      key: string,
      options?: {
        defaultValue?: string;
        error?: string;
      },
    ) =>
      options?.defaultValue ??
      (options?.error ? `${key}:${options.error}` : key),
  }),
}));

vi.mock("sonner", () => ({
  toast: {
    error: (...args: unknown[]) => toastErrorMock(...args),
    success: vi.fn(),
  },
}));

vi.mock("@/components/common/FullScreenPanel", () => ({
  FullScreenPanel: ({
    isOpen,
    title,
    children,
    footer,
  }: {
    isOpen: boolean;
    title: string;
    children: ReactNode;
    footer?: ReactNode;
  }) =>
    isOpen ? (
      <div data-testid="pricing-edit-modal">
        <h2>{title}</h2>
        <div>{children}</div>
        <div>{footer}</div>
      </div>
    ) : null,
}));

vi.mock("@/components/ui/button", () => ({
  Button: ({ children, type = "button", ...props }: any) => (
    <button type={type} {...props}>
      {children}
    </button>
  ),
}));

vi.mock("@/components/ui/input", () => ({
  Input: (props: any) => <input {...props} />,
}));

vi.mock("@/components/ui/label", () => ({
  Label: ({ children, ...props }: any) => <label {...props}>{children}</label>,
}));

vi.mock("@/lib/query/usage", () => ({
  useUpdateModelPricing: () => ({
    mutateAsync: mutateAsyncMock,
    isPending: false,
  }),
}));

describe("PricingEditModal", () => {
  it("shows structured details when save fails", async () => {
    const onClose = vi.fn();
    mutateAsyncMock.mockRejectedValueOnce({ message: "pricing save failed" });

    render(
      <PricingEditModal
        open={true}
        isNew={true}
        onClose={onClose}
        model={{
          modelId: "",
          displayName: "",
          inputCostPerMillion: "1",
          outputCostPerMillion: "2",
          cacheReadCostPerMillion: "0.1",
          cacheCreationCostPerMillion: "0.2",
        }}
      />,
    );

    fireEvent.change(screen.getByLabelText("usage.modelId"), {
      target: { value: "claude-3-5-sonnet" },
    });
    fireEvent.change(screen.getByLabelText("usage.displayName"), {
      target: { value: "Claude 3.5 Sonnet" },
    });
    fireEvent.click(screen.getByRole("button", { name: "common.add" }));

    await waitFor(() =>
      expect(mutateAsyncMock).toHaveBeenCalledWith(
        expect.objectContaining({
          modelId: "claude-3-5-sonnet",
          displayName: "Claude 3.5 Sonnet",
        }),
      ),
    );
    expect(toastErrorMock).toHaveBeenCalledWith(
      "usage.pricingSaveFailed:pricing save failed",
    );
    expect(onClose).not.toHaveBeenCalled();
  });
});
