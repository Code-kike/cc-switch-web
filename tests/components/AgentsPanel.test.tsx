import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { AgentsPanel } from "@/components/agents/AgentsPanel";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) => key,
  }),
}));

describe("AgentsPanel", () => {
  it("renders a stable localized placeholder without exposing actions", () => {
    const { container } = render(<AgentsPanel onOpenChange={vi.fn()} />);

    expect(screen.getByText("agents.comingSoonTitle")).toBeInTheDocument();
    expect(
      screen.getByText("agents.comingSoonDescription"),
    ).toBeInTheDocument();
    expect(container.querySelectorAll("button")).toHaveLength(0);
  });
});
