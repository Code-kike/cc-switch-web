import type { ReactNode } from "react";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { PricingConfigPanel } from "@/components/usage/PricingConfigPanel";

const toastSuccessMock = vi.fn();
const toastErrorMock = vi.fn();
const getDefaultCostMultiplierMock = vi.fn();
const getPricingModelSourceMock = vi.fn();
const setDefaultCostMultiplierMock = vi.fn();
const setPricingModelSourceMock = vi.fn();
const deleteMutateMock = vi.fn();
const pricingEditModalPropsSpy = vi.fn();
const i18nState = vi.hoisted(() => ({
  t: (
    key: string,
    options?: {
      defaultValue?: string;
      error?: string;
    },
  ) =>
    options?.defaultValue ??
    (options?.error ? `${key}:${options.error}` : key),
}));

const pricingState = vi.hoisted(() => ({
  pricing: [
    {
      modelId: "claude-3-5-sonnet",
      displayName: "Claude 3.5 Sonnet",
      inputCostPerMillion: "3",
      outputCostPerMillion: "15",
      cacheReadCostPerMillion: "0.3",
      cacheCreationCostPerMillion: "3.75",
    },
  ],
  isLoading: false,
  error: null as Error | null,
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: i18nState.t,
  }),
}));

vi.mock("sonner", () => ({
  toast: {
    success: (...args: unknown[]) => toastSuccessMock(...args),
    error: (...args: unknown[]) => toastErrorMock(...args),
  },
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

vi.mock("@/components/ui/alert", () => ({
  Alert: ({ children, ...props }: any) => <div {...props}>{children}</div>,
  AlertDescription: ({ children }: any) => <div>{children}</div>,
}));

vi.mock("@/components/ui/dialog", () => ({
  Dialog: ({
    open,
    children,
  }: {
    open: boolean;
    children: ReactNode;
  }) => (open ? <div>{children}</div> : null),
  DialogContent: ({ children }: { children: ReactNode }) => <div>{children}</div>,
  DialogDescription: ({ children }: { children: ReactNode }) => <div>{children}</div>,
  DialogFooter: ({ children }: { children: ReactNode }) => <div>{children}</div>,
  DialogHeader: ({ children }: { children: ReactNode }) => <div>{children}</div>,
  DialogTitle: ({ children }: { children: ReactNode }) => <div>{children}</div>,
}));

vi.mock("@/components/ui/table", () => ({
  Table: ({ children }: any) => <table>{children}</table>,
  TableBody: ({ children }: any) => <tbody>{children}</tbody>,
  TableCell: ({ children, ...props }: any) => <td {...props}>{children}</td>,
  TableHead: ({ children, ...props }: any) => <th {...props}>{children}</th>,
  TableHeader: ({ children }: any) => <thead>{children}</thead>,
  TableRow: ({ children }: any) => <tr>{children}</tr>,
}));

vi.mock("@/components/ui/select", () => ({
  Select: ({
    value,
    onValueChange,
    disabled,
    children,
  }: {
    value: string;
    onValueChange?: (value: string) => void;
    disabled?: boolean;
    children: ReactNode;
  }) => (
    <select
      value={value}
      disabled={disabled}
      onChange={(event) => onValueChange?.(event.target.value)}
    >
      {children}
    </select>
  ),
  SelectTrigger: () => null,
  SelectValue: () => null,
  SelectContent: ({ children }: { children: ReactNode }) => <>{children}</>,
  SelectItem: ({
    value,
    children,
    disabled,
  }: {
    value: string;
    children: ReactNode;
    disabled?: boolean;
  }) => (
    <option value={value} disabled={disabled}>
      {children}
    </option>
  ),
}));

vi.mock("@/components/usage/PricingEditModal", () => ({
  PricingEditModal: (props: any) => {
    pricingEditModalPropsSpy(props);
    return (
      <div data-testid="pricing-edit-modal">
        {props.isNew ? "new" : "edit"}:{props.model.modelId || "empty"}
      </div>
    );
  },
}));

vi.mock("@/lib/query/usage", () => ({
  useModelPricing: () => ({
    data: pricingState.pricing,
    isLoading: pricingState.isLoading,
    error: pricingState.error,
  }),
  useDeleteModelPricing: () => ({
    mutate: (...args: unknown[]) => deleteMutateMock(...args),
    isPending: false,
  }),
}));

vi.mock("@/lib/api/proxy", () => ({
  proxyApi: {
    getDefaultCostMultiplier: (...args: unknown[]) =>
      getDefaultCostMultiplierMock(...args),
    getPricingModelSource: (...args: unknown[]) =>
      getPricingModelSourceMock(...args),
    setDefaultCostMultiplier: (...args: unknown[]) =>
      setDefaultCostMultiplierMock(...args),
    setPricingModelSource: (...args: unknown[]) =>
      setPricingModelSourceMock(...args),
  },
}));

describe("PricingConfigPanel", () => {
  beforeEach(() => {
    toastSuccessMock.mockReset();
    toastErrorMock.mockReset();
    getDefaultCostMultiplierMock.mockReset();
    getPricingModelSourceMock.mockReset();
    setDefaultCostMultiplierMock.mockReset();
    setPricingModelSourceMock.mockReset();
    deleteMutateMock.mockReset();
    pricingEditModalPropsSpy.mockReset();

    pricingState.pricing = [
      {
        modelId: "claude-3-5-sonnet",
        displayName: "Claude 3.5 Sonnet",
        inputCostPerMillion: "3",
        outputCostPerMillion: "15",
        cacheReadCostPerMillion: "0.3",
        cacheCreationCostPerMillion: "3.75",
      },
    ];
    pricingState.isLoading = false;
    pricingState.error = null;

    getDefaultCostMultiplierMock.mockImplementation((app: string) =>
      Promise.resolve(
        app === "claude" ? "1.25" : app === "codex" ? "0.9" : "1",
      ),
    );
    getPricingModelSourceMock.mockImplementation((app: string) =>
      Promise.resolve(app === "codex" ? "request" : "response"),
    );
    setDefaultCostMultiplierMock.mockResolvedValue(undefined);
    setPricingModelSourceMock.mockResolvedValue(undefined);
    deleteMutateMock.mockImplementation(
      (_modelId: string, options?: { onSuccess?: () => void }) => {
        options?.onSuccess?.();
      },
    );
  });

  it("loads per-app defaults and saves edited multipliers and pricing sources", async () => {
    render(<PricingConfigPanel />);

    await waitFor(() =>
      expect(getDefaultCostMultiplierMock).toHaveBeenCalledWith("claude"),
    );
    expect(getDefaultCostMultiplierMock).toHaveBeenCalledWith("codex");
    expect(getDefaultCostMultiplierMock).toHaveBeenCalledWith("gemini");
    expect(getPricingModelSourceMock).toHaveBeenCalledWith("claude");
    expect(getPricingModelSourceMock).toHaveBeenCalledWith("codex");
    expect(getPricingModelSourceMock).toHaveBeenCalledWith("gemini");

    const multiplierInputs = screen.getAllByRole("spinbutton");
    fireEvent.change(multiplierInputs[0], { target: { value: "2.5" } });

    const sourceSelects = screen.getAllByRole("combobox");
    fireEvent.change(sourceSelects[2], { target: { value: "request" } });

    fireEvent.click(screen.getByRole("button", { name: "common.save" }));

    await waitFor(() =>
      expect(setDefaultCostMultiplierMock).toHaveBeenCalledWith("claude", "2.5"),
    );
    expect(setDefaultCostMultiplierMock).toHaveBeenCalledWith("codex", "0.9");
    expect(setDefaultCostMultiplierMock).toHaveBeenCalledWith("gemini", "1");
    expect(setPricingModelSourceMock).toHaveBeenCalledWith("claude", "response");
    expect(setPricingModelSourceMock).toHaveBeenCalledWith("codex", "request");
    expect(setPricingModelSourceMock).toHaveBeenCalledWith("gemini", "request");
    expect(toastSuccessMock).toHaveBeenCalledWith(
      "settings.globalProxy.pricingSaved",
    );
  });

  it("shows structured details in the page-level pricing load error state", () => {
    pricingState.error = { message: "pricing fetch failed" } as Error;
    getDefaultCostMultiplierMock.mockImplementation(
      () => new Promise(() => undefined),
    );
    getPricingModelSourceMock.mockImplementation(
      () => new Promise(() => undefined),
    );
    render(<PricingConfigPanel />);

    expect(
      screen.getByText("usage.loadPricingError: pricing fetch failed"),
    ).toBeInTheDocument();
    expect(
      screen.queryByText(/\\[object Object\\]/),
    ).not.toBeInTheDocument();
  });

  it("shows structured details when saving pricing defaults fails", async () => {
    setDefaultCostMultiplierMock.mockRejectedValueOnce({
      message: "pricing defaults write failed",
    });

    render(<PricingConfigPanel />);

    await waitFor(() =>
      expect(getDefaultCostMultiplierMock).toHaveBeenCalledWith("claude"),
    );

    fireEvent.change(screen.getAllByRole("spinbutton")[0], {
      target: { value: "2" },
    });
    fireEvent.click(screen.getByRole("button", { name: "common.save" }));

    await waitFor(() =>
      expect(toastErrorMock).toHaveBeenCalledWith(
        "settings.globalProxy.pricingSaveFailed:pricing defaults write failed",
      ),
    );
  });

  it("opens add/edit flows and deletes model pricing entries", async () => {
    render(<PricingConfigPanel />);

    await waitFor(() =>
      expect(getDefaultCostMultiplierMock).toHaveBeenCalledTimes(3),
    );

    fireEvent.click(screen.getByRole("button", { name: "common.add" }));
    expect(screen.getByTestId("pricing-edit-modal")).toHaveTextContent("new:empty");

    fireEvent.click(screen.getByTitle("common.edit"));
    expect(screen.getByTestId("pricing-edit-modal")).toHaveTextContent(
      "edit:claude-3-5-sonnet",
    );

    fireEvent.click(screen.getByTitle("common.delete"));
    expect(screen.getByText("usage.deleteConfirmTitle")).toBeInTheDocument();

    const deleteButtons = screen.getAllByRole("button", { name: "common.delete" });
    fireEvent.click(deleteButtons[deleteButtons.length - 1]);

    await waitFor(() =>
      expect(deleteMutateMock).toHaveBeenCalledWith(
        "claude-3-5-sonnet",
        expect.objectContaining({
          onSuccess: expect.any(Function),
        }),
      ),
    );
  });
});
