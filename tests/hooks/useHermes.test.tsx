import { act, renderHook } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useOpenHermesWebUI } from "@/hooks/useHermes";

const toastErrorMock = vi.fn();
const openWebUIMock = vi.fn();
const isWebModeMock = vi.fn();

vi.mock("sonner", () => ({
  toast: {
    error: (...args: unknown[]) => toastErrorMock(...args),
    success: vi.fn(),
  },
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, params?: Record<string, unknown>) => {
      if (key === "hermes.webui.remoteHintDescription") {
        return `remote:${String(params?.url ?? "")}`;
      }
      return key;
    },
  }),
}));

vi.mock("@/lib/api/adapter", () => ({
  isWebMode: () => isWebModeMock(),
}));

vi.mock("@/lib/api/hermes", () => ({
  hermesApi: {
    openWebUI: (...args: unknown[]) => openWebUIMock(...args),
  },
}));

describe("useOpenHermesWebUI", () => {
  beforeEach(() => {
    toastErrorMock.mockReset();
    openWebUIMock.mockReset();
    isWebModeMock.mockReset();
  });

  it("shows a remote-host hint in web mode instead of trying to open Hermes locally", async () => {
    isWebModeMock.mockReturnValue(true);

    const { result } = renderHook(() => useOpenHermesWebUI());

    await act(async () => {
      await result.current("/config");
    });

    expect(openWebUIMock).not.toHaveBeenCalled();
    expect(toastErrorMock).toHaveBeenCalledWith("hermes.webui.remoteHint", {
      description: "remote:http://127.0.0.1:9119/config",
    });
  });

  it("keeps the desktop flow when not in web mode", async () => {
    isWebModeMock.mockReturnValue(false);
    openWebUIMock.mockResolvedValue(undefined);

    const { result } = renderHook(() => useOpenHermesWebUI());

    await act(async () => {
      await result.current("/config");
    });

    expect(openWebUIMock).toHaveBeenCalledWith("/config");
    expect(toastErrorMock).not.toHaveBeenCalled();
  });
});
