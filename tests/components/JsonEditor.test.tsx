import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import JsonEditor from "@/components/JsonEditor";

const toastErrorMock = vi.fn();
const toastSuccessMock = vi.fn();
const formatJSONMock = vi.fn();

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, options?: Record<string, unknown>) => {
      const defaultValue = String(options?.defaultValue ?? key);
      if (typeof options?.error === "string") {
        return defaultValue.replace("{{error}}", String(options.error));
      }
      return defaultValue;
    },
  }),
}));

vi.mock("sonner", () => ({
  toast: {
    error: (...args: unknown[]) => toastErrorMock(...args),
    success: (...args: unknown[]) => toastSuccessMock(...args),
  },
}));

vi.mock("@/utils/formatters", () => ({
  formatJSON: (...args: unknown[]) => formatJSONMock(...args),
}));

describe("JsonEditor", () => {
  beforeEach(() => {
    toastErrorMock.mockReset();
    toastSuccessMock.mockReset();
    formatJSONMock.mockReset();
    Object.defineProperty(HTMLElement.prototype, "getClientRects", {
      configurable: true,
      value: () => [],
    });
    Object.defineProperty(HTMLElement.prototype, "getBoundingClientRect", {
      configurable: true,
      value: () => ({
        x: 0,
        y: 0,
        top: 0,
        left: 0,
        right: 0,
        bottom: 0,
        width: 0,
        height: 0,
        toJSON: () => ({}),
      }),
    });
  });

  it("shows a generic invalid-json message when format fails", async () => {
    formatJSONMock.mockImplementation(() => {
      throw new Error("unexpected formatter crash");
    });

    render(
      <JsonEditor
        value='{"foo":1}'
        onChange={vi.fn()}
        language="json"
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: "格式化" }));

    await waitFor(() => {
      expect(toastErrorMock).toHaveBeenCalledWith(
        "格式化失败：jsonEditor.invalidJson",
      );
    });
  });
});
