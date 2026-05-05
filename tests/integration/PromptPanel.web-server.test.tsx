import fs from "node:fs/promises";
import path from "node:path";
import { createRef } from "react";
import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterAll, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("@/lib/api/adapter", async () => {
  const actual = await vi.importActual<typeof import("@/lib/api/adapter")>(
    "@/lib/api/adapter",
  );

  const readUploadBytes = async (value: unknown): Promise<Uint8Array> => {
    if (value instanceof Blob) {
      if (typeof (value as Blob & { arrayBuffer?: unknown }).arrayBuffer === "function") {
        return new Uint8Array(await (value as Blob).arrayBuffer());
      }
      return await new Promise<Uint8Array>((resolve, reject) => {
        const reader = new FileReader();
        reader.onerror = () => reject(reader.error ?? new Error("failed to read blob"));
        reader.onload = () => {
          const result = reader.result;
          if (result instanceof ArrayBuffer) {
            resolve(new Uint8Array(result));
            return;
          }
          resolve(new TextEncoder().encode(String(result ?? "")));
        };
        reader.readAsArrayBuffer(value);
      });
    }
    return new Uint8Array(
      await new Response(String(value), {
        headers: { "Content-Type": "text/plain; charset=utf-8" },
      }).arrayBuffer(),
    );
  };

  return {
    ...actual,
    pickWebFile: vi.fn(),
    webUpload: vi.fn(async (uploadPath: string, formData: FormData) => {
      const source = formData.get("file");
      if (source === null) {
        throw new Error("missing upload field");
      }

      let token = actual.getCsrfToken();
      if (!token) {
        const csrfResponse = await fetch(
          `${actual.apiBase()}/api/system/csrf-token`,
          {
            credentials: "include",
            headers: { Accept: "application/json" },
          },
        );
        const payload = (await csrfResponse.json()) as { token: string };
        token = payload.token;
        actual.setCsrfToken(token);
      }

      const fileName =
        source instanceof Blob && "name" in source && typeof source.name === "string"
          ? source.name
          : "upload.md";
      const contentType =
        source instanceof Blob && source.type.length > 0
          ? source.type
          : "application/octet-stream";
      const boundary = `----vitest-prompt-${Date.now()}`;
      const body = Buffer.concat([
        Buffer.from(
          `--${boundary}\r\nContent-Disposition: form-data; name="file"; filename="${fileName}"\r\nContent-Type: ${contentType}\r\n\r\n`,
          "utf8",
        ),
        Buffer.from(await readUploadBytes(source)),
        Buffer.from(`\r\n--${boundary}--\r\n`, "utf8"),
      ]);

      const response = await fetch(`${actual.apiBase()}${uploadPath}`, {
        method: "POST",
        body,
        credentials: "include",
        headers: {
          Accept: "application/json",
          "Content-Type": `multipart/form-data; boundary=${boundary}`,
          ...(token ? { "X-CSRF-Token": token } : {}),
        },
      });
      if (!response.ok) {
        throw new Error(`upload failed: ${response.status}`);
      }
      return await response.json();
    }),
  };
});

import "@/lib/api/web-commands";
import PromptPanel, {
  type PromptPanelHandle,
} from "@/components/prompts/PromptPanel";
import { getCsrfToken, pickWebFile, setCsrfToken } from "@/lib/api/adapter";
import type { Prompt } from "@/lib/api/prompts";
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

vi.mock("@/components/MarkdownEditor", () => ({
  default: ({
    value,
    onChange,
  }: {
    value: string;
    onChange?: (value: string) => void;
  }) => (
    <textarea
      aria-label="markdown-editor"
      value={value}
      onChange={(event) => onChange?.(event.target.value)}
    />
  ),
}));

const promptFile = new File(
  ["# Smoke Prompt\n\nImported from PromptPanel web-server test.\n"],
  "smoke-prompt.md",
  { type: "text/markdown" },
);

const claudePromptPath = (homeDir: string): string =>
  path.join(homeDir, ".claude", "CLAUDE.md");

const geminiPromptPath = (homeDir: string): string =>
  path.join(homeDir, ".gemini", "GEMINI.md");

const getPrompts = async (
  baseUrl: string,
  app: "claude" | "gemini" = "claude",
): Promise<Record<string, Prompt>> => {
  const response = await fetch(
    new URL(`/api/prompts/get-prompts?app=${app}`, baseUrl),
  );
  if (!response.ok) {
    throw new Error(`failed to load prompts: ${response.status}`);
  }
  return (await response.json()) as Record<string, Prompt>;
};

const getPromptRow = (name: string): HTMLElement => {
  const label = screen.getByText(name);
  let current: HTMLElement | null = label;

  while (current && !current.classList.contains("group")) {
    current = current.parentElement;
  }

  if (!(current instanceof HTMLElement)) {
    throw new Error(`could not locate prompt row for ${name}`);
  }

  return current;
};

const editRegex = /^(common\.edit|编辑|Edit)$/;
const nameRegex = /^(prompts\.name|名称|Name)$/;
const descriptionRegex = /^(prompts\.description|描述|Description)$/;
const saveRegex = /^(common\.save|保存|Save)$/;

describe.sequential("PromptPanel against real web server", () => {
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
    vi.mocked(pickWebFile).mockReset();
    vi.mocked(pickWebFile).mockResolvedValue(promptFile);

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
    expect(getCsrfToken()).toBeNull();
  });

  it(
    "imports, enables, disables, and deletes prompts through the rendered panel UI",
    async () => {
      render(<PromptPanel open onOpenChange={vi.fn()} appId="claude" />);

      await waitFor(async () => {
        expect(await getPrompts(webServer.baseUrl)).toEqual({});
      });
      expect(await screen.findByText("prompts.empty")).toBeInTheDocument();

      fireEvent.click(screen.getByRole("button", { name: "prompts.import" }));

      let importedPromptId = "";
      let importedPromptName = "";
      await waitFor(async () => {
        const prompts = await getPrompts(webServer.baseUrl);
        const entries = Object.entries(prompts);
        expect(entries).toHaveLength(1);
        const [id, prompt] = entries[0]!;
        importedPromptId = id;
        importedPromptName = prompt.name;
        expect(importedPromptId).toMatch(/^imported-/);
        expect(prompt.name).not.toBe("");
        expect(prompt.enabled).toBe(false);
        expect(prompt.content).toContain("Imported from PromptPanel");
      });
      await waitFor(() =>
        expect(screen.getByText(importedPromptName)).toBeInTheDocument(),
      );
      await expect(
        fs.access(claudePromptPath(webServer.homeDir)),
      ).rejects.toThrow();
      expect(
        screen.queryByText("prompts.currentFile", { exact: false }),
      ).not.toBeInTheDocument();

      fireEvent.click(await screen.findByRole("switch"));

      await waitFor(async () => {
        const prompts = await getPrompts(webServer.baseUrl);
        expect(prompts[importedPromptId]?.enabled).toBe(true);
      });
      await waitFor(async () => {
        expect(
          await fs.readFile(claudePromptPath(webServer.homeDir), "utf8"),
        ).toBe("# Smoke Prompt\n\nImported from PromptPanel web-server test.\n");
      });
      expect(
        await screen.findByText("prompts.currentFile"),
      ).toBeInTheDocument();
      expect(
        screen.getByText("Imported from PromptPanel web-server test.", {
          exact: false,
        }),
      ).toBeInTheDocument();

      fireEvent.click(screen.getByRole("switch", { checked: true }));

      await waitFor(async () => {
        const prompts = await getPrompts(webServer.baseUrl);
        expect(prompts[importedPromptId]?.enabled).toBe(false);
      });
      await waitFor(async () => {
        expect(
          await fs.readFile(claudePromptPath(webServer.homeDir), "utf8"),
        ).toBe("");
      });
      await waitFor(() =>
        expect(
          screen.queryByText("prompts.currentFile", { exact: false }),
        ).not.toBeInTheDocument(),
      );

      const row = getPromptRow(importedPromptName);
      fireEvent.click(within(row).getByTitle("common.delete"));
      fireEvent.click(screen.getByRole("button", { name: "common.confirm" }));

      await waitFor(async () => {
        expect(await getPrompts(webServer.baseUrl)).toEqual({});
      });
      await waitFor(() =>
        expect(screen.queryByRole("dialog")).not.toBeInTheDocument(),
      );
      expect(await screen.findByText("prompts.empty")).toBeInTheDocument();

      expect(toastSuccessMock).toHaveBeenCalledWith("prompts.importSuccess", {
        closeButton: true,
      });
      expect(toastSuccessMock).toHaveBeenCalledWith("prompts.enableSuccess", {
        closeButton: true,
      });
      expect(toastSuccessMock).toHaveBeenCalledWith("prompts.disableSuccess", {
        closeButton: true,
      });
      expect(toastSuccessMock).toHaveBeenCalledWith("prompts.deleteSuccess", {
        closeButton: true,
      });
      expect(toastErrorMock).not.toHaveBeenCalled();
    },
    20_000,
  );

  it(
    "creates, edits, and enables a Gemini prompt through the rendered panel UI",
    async () => {
      const ref = createRef<PromptPanelHandle>();

      render(
        <PromptPanel ref={ref} open onOpenChange={vi.fn()} appId="gemini" />,
      );

      await waitFor(async () => {
        expect(await getPrompts(webServer.baseUrl, "gemini")).toEqual({});
      });

      await act(async () => {
        ref.current?.openAdd();
      });
      expect(await screen.findByLabelText(nameRegex)).toBeInTheDocument();

      fireEvent.change(screen.getByLabelText(nameRegex), {
        target: { value: "Gemini Smoke Prompt" },
      });
      fireEvent.change(screen.getByLabelText(descriptionRegex), {
        target: { value: "Gemini prompt description" },
      });
      fireEvent.change(screen.getByLabelText("markdown-editor"), {
        target: { value: "# GEMINI.md\n\nGemini prompt content v1\n" },
      });
      fireEvent.click(screen.getByRole("button", { name: saveRegex }));

      let createdPromptId = "";
      await waitFor(async () => {
        const prompts = await getPrompts(webServer.baseUrl, "gemini");
        const entries = Object.entries(prompts);
        expect(entries).toHaveLength(1);
        const [id, prompt] = entries[0]!;
        createdPromptId = id;
        expect(prompt.name).toBe("Gemini Smoke Prompt");
        expect(prompt.description).toBe("Gemini prompt description");
        expect(prompt.content).toContain("Gemini prompt content v1");
        expect(prompt.enabled).toBe(false);
      });

      const createdRow = await waitFor(() =>
        getPromptRow("Gemini Smoke Prompt"),
      );
      fireEvent.click(within(createdRow).getByTitle(editRegex));

      const nameInput = await screen.findByLabelText(nameRegex);
      const descriptionInput = screen.getByLabelText(descriptionRegex);
      const editor = screen.getByLabelText("markdown-editor");

      expect(nameInput).toHaveValue("Gemini Smoke Prompt");
      expect(descriptionInput).toHaveValue("Gemini prompt description");
      expect((editor as HTMLTextAreaElement).value).toContain("GEMINI.md");
      expect((editor as HTMLTextAreaElement).value).toContain(
        "Gemini prompt content v1",
      );

      fireEvent.change(nameInput, {
        target: { value: "Gemini Smoke Prompt Edited" },
      });
      fireEvent.change(descriptionInput, {
        target: { value: "Gemini prompt description edited" },
      });
      fireEvent.change(editor, {
        target: { value: "# GEMINI.md\n\nGemini prompt content v2\n" },
      });
      fireEvent.click(screen.getByRole("button", { name: saveRegex }));

      await waitFor(async () => {
        const prompts = await getPrompts(webServer.baseUrl, "gemini");
        expect(prompts[createdPromptId]?.name).toBe(
          "Gemini Smoke Prompt Edited",
        );
        expect(prompts[createdPromptId]?.description).toBe(
          "Gemini prompt description edited",
        );
        expect(prompts[createdPromptId]?.content).toBe(
          "# GEMINI.md\n\nGemini prompt content v2",
        );
      });

      fireEvent.click(screen.getByRole("switch"));

      await waitFor(async () => {
        const prompts = await getPrompts(webServer.baseUrl, "gemini");
        expect(prompts[createdPromptId]?.enabled).toBe(true);
      });
      await waitFor(async () => {
        expect(
          await fs.readFile(geminiPromptPath(webServer.homeDir), "utf8"),
        ).toBe("# GEMINI.md\n\nGemini prompt content v2");
      });
      expect(
        await screen.findByText("prompts.currentFile"),
      ).toBeInTheDocument();
      expect(
        screen.getByText("Gemini prompt content v2", { exact: false }),
      ).toBeInTheDocument();
    },
    20_000,
  );

  it(
    "surfaces live prompt file read failures from the rendered web page",
    async () => {
      const livePromptPath = claudePromptPath(webServer.homeDir);
      await fs.rm(livePromptPath, { force: true, recursive: true });
      await fs.mkdir(livePromptPath, { recursive: true });

      try {
        render(<PromptPanel open onOpenChange={vi.fn()} appId="claude" />);

        await waitFor(() =>
          expect(toastErrorMock).toHaveBeenCalledWith(
            "prompts.currentFileLoadFailed",
            {
              description: expect.stringMatching(/\S/),
            },
          ),
        );
        const currentFileError = toastErrorMock.mock.calls.find(
          ([title]) => title === "prompts.currentFileLoadFailed",
        );
        expect(currentFileError?.[1]?.description).not.toBe("[object Object]");
        expect(
          screen.queryByText("prompts.currentFile", { exact: false }),
        ).not.toBeInTheDocument();
      } finally {
        await fs.rm(livePromptPath, { force: true, recursive: true });
      }
    },
    20_000,
  );
});
