import {
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { http, HttpResponse } from "msw";
import { createRef, type ReactElement } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import {
  PiPromptTemplates,
  PiSystemPromptFiles,
  type PiPromptTemplatesHandle,
} from "@/components/prompts/PiNativePromptResources";
import { server } from "../msw/server";
// Registers the web command map so `invoke` resolves the pi routes.
import "@/lib/api";

const PI_API = "/api/pi";

const toastMocks = vi.hoisted(() => ({
  success: vi.fn(),
  error: vi.fn(),
  warning: vi.fn(),
}));

vi.mock("sonner", () => ({ toast: toastMocks }));

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

vi.mock("@/components/common/FullScreenPanel", () => ({
  FullScreenPanel: ({
    title,
    children,
    footer,
    onClose,
  }: {
    title: string;
    children: React.ReactNode;
    footer?: React.ReactNode;
    onClose: () => void;
  }) => (
    <div data-testid="full-screen-panel">
      <span>{title}</span>
      <button type="button" onClick={onClose}>
        panel-close
      </button>
      {children}
      {footer}
    </div>
  ),
}));

function renderWithQueryClient(ui: ReactElement) {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(
    <QueryClientProvider client={queryClient}>{ui}</QueryClientProvider>,
  );
}

beforeEach(() => {
  toastMocks.success.mockReset();
  toastMocks.error.mockReset();
  toastMocks.warning.mockReset();
});

describe("PiSystemPromptFiles", () => {
  it("reads both instruction files and reports their configured state", async () => {
    server.use(
      http.get(`${PI_API}/get-pi-prompt-file`, ({ request }) => {
        const kind = new URL(request.url).searchParams.get("kind");
        if (kind === "system_append") {
          return HttpResponse.json({
            exists: true,
            revision: "rev-append",
            content: "# append",
          });
        }
        return HttpResponse.json({
          exists: false,
          revision: "missing",
          content: "",
        });
      }),
    );

    renderWithQueryClient(<PiSystemPromptFiles />);

    expect(await screen.findByText("APPEND_SYSTEM.md")).toBeInTheDocument();
    expect(screen.getByText("SYSTEM.md")).toBeInTheDocument();
    await waitFor(() => {
      expect(screen.getByText("pi.prompts.configured")).toBeInTheDocument();
      expect(screen.getByText("pi.prompts.notConfigured")).toBeInTheDocument();
    });
  });

  it("saves APPEND_SYSTEM.md with the revision it read", async () => {
    const replaceCalls: unknown[] = [];
    server.use(
      http.get(`${PI_API}/get-pi-prompt-file`, () =>
        HttpResponse.json({
          exists: true,
          revision: "rev-1",
          content: "# original",
        }),
      ),
      http.post(`${PI_API}/replace-pi-prompt-file`, async ({ request }) => {
        replaceCalls.push(await request.json());
        return HttpResponse.json({
          exists: true,
          revision: "rev-2",
          content: "# edited",
        });
      }),
    );

    renderWithQueryClient(<PiSystemPromptFiles />);
    // The card stays disabled until its snapshot resolves.
    await screen.findByText("pi.prompts.configured");
    fireEvent.click(screen.getByText("APPEND_SYSTEM.md"));

    const editor = await screen.findByLabelText("markdown-editor");
    fireEvent.change(editor, { target: { value: "# edited" } });
    fireEvent.click(screen.getByRole("button", { name: "common.save" }));

    await waitFor(() => expect(replaceCalls).toHaveLength(1));
    expect(replaceCalls[0]).toMatchObject({
      kind: "system_append",
      expectedRevision: "rev-1",
      content: "# edited",
    });
    expect(toastMocks.success).toHaveBeenCalledWith(
      "pi.prompts.fileSaved",
      expect.objectContaining({ description: "pi.prompts.reloadNotice" }),
    );
  });

  it("requires confirmation before creating SYSTEM.md", async () => {
    const replaceCalls: unknown[] = [];
    server.use(
      http.get(`${PI_API}/get-pi-prompt-file`, ({ request }) => {
        const kind = new URL(request.url).searchParams.get("kind") ?? "";
        return HttpResponse.json({
          exists: false,
          revision: "missing",
          content: kind,
        });
      }),
      http.post(`${PI_API}/replace-pi-prompt-file`, async ({ request }) => {
        replaceCalls.push(await request.json());
        return HttpResponse.json({
          exists: true,
          revision: "rev-new",
          content: "# base",
        });
      }),
    );

    renderWithQueryClient(<PiSystemPromptFiles />);
    await waitFor(() =>
      expect(screen.getAllByText("pi.prompts.notConfigured")).toHaveLength(2),
    );
    fireEvent.click(screen.getByText("SYSTEM.md"));

    const editor = await screen.findByLabelText("markdown-editor");
    fireEvent.change(editor, { target: { value: "# base" } });
    fireEvent.click(
      screen.getByRole("button", { name: "pi.prompts.saveAndConfigure" }),
    );

    // Confirmation gate: nothing is written until the dialog is accepted.
    expect(replaceCalls).toHaveLength(0);
    const dialog = await screen.findByRole("dialog");
    expect(dialog).toHaveTextContent("pi.prompts.activateOverrideMessage");
    fireEvent.click(
      within(dialog).getByRole("button", {
        name: "pi.prompts.saveAndConfigure",
      }),
    );

    await waitFor(() => expect(replaceCalls).toHaveLength(1));
    expect(replaceCalls[0]).toMatchObject({
      kind: "system_override",
      expectedRevision: "missing",
    });
  });
});

describe("PiPromptTemplates", () => {
  it("lists templates with their frontmatter summary", async () => {
    server.use(
      http.get(`${PI_API}/list-pi-prompt-templates`, () =>
        HttpResponse.json([
          {
            slug: "review",
            revision: "rev-review",
            content:
              '---\ndescription: Review the current changes\nargument-hint: "<target>"\n---\nReview $1.',
          },
          {
            slug: "ship",
            revision: "rev-ship",
            content: "Ship it.",
          },
        ]),
      ),
    );

    renderWithQueryClient(<PiPromptTemplates />);

    expect(await screen.findByText("/review")).toBeInTheDocument();
    expect(screen.getByText("Review the current changes")).toBeInTheDocument();
    // Quoted frontmatter values are unwrapped before display.
    expect(screen.getByText("<target>")).toBeInTheDocument();
    expect(screen.getByText("/ship")).toBeInTheDocument();
  });

  it("rejects a duplicate slug before writing", async () => {
    const upsertCalls: unknown[] = [];
    server.use(
      http.get(`${PI_API}/list-pi-prompt-templates`, () =>
        HttpResponse.json([
          { slug: "review", revision: "rev-review", content: "Review $1." },
        ]),
      ),
      http.post(`${PI_API}/upsert-pi-prompt-template`, async ({ request }) => {
        upsertCalls.push(await request.json());
        return HttpResponse.json({
          slug: "audit",
          revision: "rev-audit",
          content: "Audit.",
        });
      }),
    );

    const ref = createRef<PiPromptTemplatesHandle>();
    renderWithQueryClient(<PiPromptTemplates ref={ref} />);
    await screen.findByText("/review");

    ref.current?.openCreate();

    const slugInput = await screen.findByPlaceholderText(
      "pi.prompts.templateSlug",
    );
    fireEvent.change(slugInput, { target: { value: "review" } });
    expect(
      screen.getByText("pi.prompts.templateSlugExists"),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "pi.prompts.createTemplate" }),
    ).toBeDisabled();

    fireEvent.change(slugInput, { target: { value: "bad slug" } });
    expect(
      screen.getByText("pi.prompts.templateSlugInvalid"),
    ).toBeInTheDocument();

    fireEvent.change(slugInput, { target: { value: "audit" } });
    fireEvent.click(
      screen.getByRole("button", { name: "pi.prompts.createTemplate" }),
    );

    await waitFor(() => expect(upsertCalls).toHaveLength(1));
    expect(upsertCalls[0]).toMatchObject({
      slug: "audit",
      originalSlug: null,
      expectedRevision: "missing",
    });
  });

  it("deletes a template with its revision after confirmation", async () => {
    const deleteCalls: URLSearchParams[] = [];
    server.use(
      http.get(`${PI_API}/list-pi-prompt-templates`, () =>
        HttpResponse.json([
          { slug: "review", revision: "rev-review", content: "Review $1." },
        ]),
      ),
      http.delete(`${PI_API}/delete-pi-prompt-template`, ({ request }) => {
        deleteCalls.push(new URL(request.url).searchParams);
        return HttpResponse.json(true);
      }),
    );

    renderWithQueryClient(<PiPromptTemplates />);
    await screen.findByText("/review");

    fireEvent.click(screen.getByTitle("common.delete"));
    const dialog = await screen.findByRole("dialog");
    expect(dialog).toHaveTextContent("pi.prompts.deleteTemplateMessage");
    fireEvent.click(
      within(dialog).getByRole("button", { name: "common.delete" }),
    );

    await waitFor(() => expect(deleteCalls).toHaveLength(1));
    expect(deleteCalls[0].get("slug")).toBe("review");
    expect(deleteCalls[0].get("expectedRevision")).toBe("rev-review");
  });

  it("surfaces a load failure with a retry affordance", async () => {
    let attempts = 0;
    server.use(
      http.get(`${PI_API}/list-pi-prompt-templates`, () => {
        attempts += 1;
        if (attempts === 1) {
          return new HttpResponse("pi agent dir unreadable", { status: 500 });
        }
        return HttpResponse.json([]);
      }),
    );

    renderWithQueryClient(<PiPromptTemplates />);

    expect(
      await screen.findByText("pi.prompts.templateLoadFailed"),
    ).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "common.refresh" }));

    expect(
      await screen.findByText("pi.prompts.noTemplates"),
    ).toBeInTheDocument();
  });
});
