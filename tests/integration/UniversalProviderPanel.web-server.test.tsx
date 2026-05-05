import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterAll, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";

import "@/lib/api/web-commands";
import { UniversalProviderPanel } from "@/components/universal/UniversalProviderPanel";
import { setCsrfToken } from "@/lib/api/adapter";
import { providersApi, universalProvidersApi } from "@/lib/api/providers";
import { server } from "../msw/server";
import {
  startTestWebServer,
  type TestWebServer,
} from "../helpers/web-server";

const toastSuccessMock = vi.fn();
const toastErrorMock = vi.fn();

vi.mock("sonner", () => ({
  toast: {
    success: (...args: unknown[]) => toastSuccessMock(...args),
    error: (...args: unknown[]) => toastErrorMock(...args),
  },
}));

const addRegex = /^(common\.add|添加|Add)$/;
const saveRegex = /^(common\.save|保存|Save)$/;
const syncConfirmRegex =
  /^(universalProvider\.syncConfirm|同步|Sync)$/;
const universalAddRegex =
  /^(universalProvider\.add|添加统一供应商)$/;
const universalEmptyRegex =
  /^(universalProvider\.empty|还没有统一供应商|No universal providers yet)$/;
const nameRegex = /^(universalProvider\.name|名称|Name)$/;
const baseUrlRegex =
  /^(universalProvider\.baseUrl|API 地址|API URL|Base URL)$/;
const apiKeyRegex = /^(universalProvider\.apiKey|API Key)$/;
const geminiAppRegex = /^Gemini CLI$/;
const syncTitleRegex =
  /^(universalProvider\.sync|同步到应用|Sync to apps)$/;
const editTitleRegex = /^(common\.edit|编辑|Edit)$/;
const deleteTitleRegex = /^(common\.delete|删除|Delete)$/;

function getUniversalCard(providerName: string): HTMLElement {
  const label = screen.getByText(providerName);
  let current: HTMLElement | null = label instanceof HTMLElement ? label : null;

  while (
    current &&
    !current.className.includes("group relative rounded-xl border")
  ) {
    current = current.parentElement;
  }

  if (!(current instanceof HTMLElement)) {
    throw new Error(`could not locate universal provider card for ${providerName}`);
  }

  return current;
}

describe.sequential("UniversalProviderPanel against real web server", () => {
  let webServer: TestWebServer;

  beforeAll(async () => {
    server.close();
    webServer = await startTestWebServer();
  }, 360_000);

  afterAll(async () => {
    await webServer.stop();
    server.listen({ onUnhandledRequest: "warn" });
  }, 20_000);

  beforeEach(() => {
    toastSuccessMock.mockReset();
    toastErrorMock.mockReset();
    setCsrfToken(null);

    Object.defineProperty(window, "__TAURI_INTERNALS__", {
      configurable: true,
      value: undefined,
    });
    Object.defineProperty(window, "__TAURI__", {
      configurable: true,
      value: undefined,
    });
    Object.defineProperty(window, "__CC_SWITCH_API_BASE__", {
      configurable: true,
      value: webServer.baseUrl,
    });
    Object.defineProperty(window, "scrollTo", {
      configurable: true,
      value: vi.fn(),
    });

    const emptyRectList = {
      length: 0,
      item: () => null,
      [Symbol.iterator]: function* () {
        yield* [];
      },
    };
    Object.defineProperty(Range.prototype, "getClientRects", {
      configurable: true,
      value: () => emptyRectList,
    });
    Object.defineProperty(Range.prototype, "getBoundingClientRect", {
      configurable: true,
      value: () => ({
        x: 0,
        y: 0,
        top: 0,
        right: 0,
        bottom: 0,
        left: 0,
        width: 0,
        height: 0,
        toJSON: () => ({}),
      }),
    });
  });

  it(
    "creates, edits, syncs, and deletes universal providers through the rendered panel UI",
    async () => {
      render(<UniversalProviderPanel />);

      await waitFor(async () => {
        expect(await universalProvidersApi.getAll()).toEqual({});
      });
      expect(await screen.findByText(universalEmptyRegex)).toBeInTheDocument();

      fireEvent.click(
        screen.getAllByRole("button", { name: universalAddRegex })[0],
      );

      fireEvent.change(await screen.findByLabelText(nameRegex), {
        target: { value: "Universal Smoke" },
      });
      fireEvent.change(screen.getByLabelText(baseUrlRegex), {
        target: { value: "https://universal-smoke.example.com" },
      });
      fireEvent.change(screen.getByLabelText(apiKeyRegex), {
        target: { value: "universal-secret" },
      });
      fireEvent.click(screen.getByRole("button", { name: addRegex }));

      let universalId = "";
      await waitFor(async () => {
        const providers = await universalProvidersApi.getAll();
        const entry = Object.values(providers).find(
          (provider) => provider.name === "Universal Smoke",
        );
        expect(entry).toBeDefined();
        universalId = entry!.id;
      });
      expect(await screen.findByText("Universal Smoke")).toBeInTheDocument();

      await waitFor(async () => {
        const claudeProviders = await providersApi.getAll("claude");
        const claudeProvider = claudeProviders[`universal-claude-${universalId}`];
        expect(
          claudeProvider?.settingsConfig?.env?.ANTHROPIC_BASE_URL,
        ).toBe("https://universal-smoke.example.com");
        expect(
          claudeProvider?.settingsConfig?.env?.ANTHROPIC_AUTH_TOKEN,
        ).toBe("universal-secret");

        const codexProviders = await providersApi.getAll("codex");
        const codexProvider = codexProviders[`universal-codex-${universalId}`];
        expect(codexProvider).toBeDefined();
        expect(
          codexProvider?.settingsConfig?.config,
        ).toContain('base_url = "https://universal-smoke.example.com/v1"');
        expect(
          codexProvider?.settingsConfig?.auth?.OPENAI_API_KEY,
        ).toBe("universal-secret");

        const geminiProviders = await providersApi.getAll("gemini");
        const geminiProvider = geminiProviders[`universal-gemini-${universalId}`];
        expect(
          geminiProvider?.settingsConfig?.env?.GOOGLE_GEMINI_BASE_URL,
        ).toBe("https://universal-smoke.example.com");
      });

      const initialCard = getUniversalCard("Universal Smoke");
      fireEvent.click(within(initialCard).getByTitle(editTitleRegex));

      fireEvent.change(await screen.findByLabelText(nameRegex), {
        target: { value: "Universal Smoke Edited" },
      });
      fireEvent.change(screen.getByLabelText(baseUrlRegex), {
        target: { value: "https://universal-edited.example.com" },
      });

      const geminiSection = screen.getByText(geminiAppRegex).closest("div");
      if (!(geminiSection instanceof HTMLElement)) {
        throw new Error("could not locate Gemini app toggle section");
      }
      fireEvent.click(within(geminiSection.parentElement ?? geminiSection).getByRole("switch"));

      fireEvent.click(screen.getByRole("button", { name: saveRegex }));

      expect(
        await screen.findByText("Universal Smoke Edited"),
      ).toBeInTheDocument();
      await waitFor(async () => {
        const universalProviders = await universalProvidersApi.getAll();
        expect(universalProviders[universalId]?.name).toBe("Universal Smoke Edited");
        expect(universalProviders[universalId]?.baseUrl).toBe(
          "https://universal-edited.example.com",
        );
        expect(universalProviders[universalId]?.apps.gemini).toBe(false);
      });

      await waitFor(async () => {
        const claudeProviders = await providersApi.getAll("claude");
        expect(
          claudeProviders[`universal-claude-${universalId}`]?.settingsConfig?.env
            ?.ANTHROPIC_BASE_URL,
        ).toBe("https://universal-smoke.example.com");

        const geminiProviders = await providersApi.getAll("gemini");
        expect(geminiProviders[`universal-gemini-${universalId}`]).toBeDefined();
      });

      const editedCard = getUniversalCard("Universal Smoke Edited");
      fireEvent.click(within(editedCard).getByTitle(syncTitleRegex));
      fireEvent.click(await screen.findByRole("button", { name: syncConfirmRegex }));

      await waitFor(async () => {
        const claudeProviders = await providersApi.getAll("claude");
        expect(
          claudeProviders[`universal-claude-${universalId}`]?.settingsConfig?.env
            ?.ANTHROPIC_BASE_URL,
        ).toBe("https://universal-edited.example.com");

        const codexProviders = await providersApi.getAll("codex");
        expect(
          codexProviders[`universal-codex-${universalId}`]?.settingsConfig?.config,
        ).toContain('base_url = "https://universal-edited.example.com/v1"');

        const geminiProviders = await providersApi.getAll("gemini");
        expect(geminiProviders[`universal-gemini-${universalId}`]).toBeUndefined();
      });

      fireEvent.click(within(editedCard).getByTitle(deleteTitleRegex));
      fireEvent.click(await screen.findByRole("button", { name: deleteTitleRegex }));

      await waitFor(async () => {
        expect(await universalProvidersApi.getAll()).toEqual({});
      });
      await waitFor(() => {
        expect(screen.queryByText("Universal Smoke Edited")).not.toBeInTheDocument();
      });
      await waitFor(async () => {
        const claudeProviders = await providersApi.getAll("claude");
        expect(claudeProviders[`universal-claude-${universalId}`]).toBeUndefined();

        const codexProviders = await providersApi.getAll("codex");
        expect(codexProviders[`universal-codex-${universalId}`]).toBeUndefined();
      });
    },
    30_000,
  );
});
