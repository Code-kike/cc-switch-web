import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { ProfileSwitcher } from "@/components/profiles/ProfileSwitcher";

const mocks = vi.hoisted(() => ({
  apply: vi.fn(),
  clear: vi.fn(),
  create: vi.fn(),
  update: vi.fn(),
  remove: vi.fn(),
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) => key,
  }),
}));

vi.mock("@/lib/query/profiles", () => ({
  useProfilesQuery: () => ({
    data: {
      profiles: [
        {
          id: "p1",
          name: "Project A",
          payload: {
            providers: { claude: "claude-a", codex: null },
            mcp: { claude: [], codex: null },
            skills: { claude: [], codex: null },
            prompts: { claude: null, codex: null },
          },
        },
        {
          id: "p2",
          name: "Project B",
          payload: {
            providers: { claude: null, codex: "codex-b" },
            mcp: { claude: null, codex: [] },
            skills: { claude: null, codex: [] },
            prompts: { claude: null, codex: null },
          },
        },
      ],
      currentIds: { claude: "p1", codex: "p2" },
    },
  }),
  useApplyProfileMutation: () => ({ mutate: mocks.apply, isPending: false }),
  useClearProfileMutation: () => ({ mutate: mocks.clear, isPending: false }),
  useCreateProfileMutation: () => ({ mutate: mocks.create, isPending: false }),
  useUpdateProfileMutation: () => ({ mutate: mocks.update, isPending: false }),
  useDeleteProfileMutation: () => ({ mutate: mocks.remove, isPending: false }),
}));

describe("ProfileSwitcher", () => {
  beforeEach(() => {
    Object.values(mocks).forEach((mock) => mock.mockReset());
    Object.defineProperty(HTMLElement.prototype, "scrollIntoView", {
      configurable: true,
      value: vi.fn(),
    });
  });

  it("uses independent camelCase current ids and applies within the active scope", () => {
    const { rerender } = render(<ProfileSwitcher activeApp="claude" />);

    expect(screen.getByRole("combobox")).toHaveTextContent("Project A");

    rerender(<ProfileSwitcher activeApp="codex" />);
    expect(screen.getByRole("combobox")).toHaveTextContent("Project B");

    fireEvent.click(screen.getByRole("combobox"));
    const projectAItems = screen.getAllByText("Project A");
    fireEvent.click(projectAItems[projectAItems.length - 1]);

    expect(mocks.apply).toHaveBeenCalledWith({ id: "p1", scope: "codex" });
  });

  it("does not render on unsupported fork-only app tabs", () => {
    const { container } = render(<ProfileSwitcher activeApp="gemini" />);

    expect(container).toBeEmptyDOMElement();
    expect(screen.queryByRole("combobox")).not.toBeInTheDocument();
  });
});
