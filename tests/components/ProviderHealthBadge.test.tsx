import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { ProviderHealthBadge } from "@/components/providers/ProviderHealthBadge";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, options?: Record<string, unknown>) => {
      if (key === "health.consecutiveFailures") {
        return `failures:${String(options?.count ?? "")}`;
      }
      return key;
    },
  }),
}));

describe("ProviderHealthBadge", () => {
  it("renders the healthy state for zero failures", () => {
    render(<ProviderHealthBadge consecutiveFailures={0} />);

    const badge = screen.getByText("health.operational").closest("div");
    expect(screen.getByText("health.operational")).toBeInTheDocument();
    expect(badge).toHaveAttribute("title", "failures:0");
    expect(badge?.className).toContain("bg-green-500/10");
  });

  it("renders the circuit-open state for five or more failures", () => {
    render(<ProviderHealthBadge consecutiveFailures={5} />);

    const badge = screen.getByText("health.circuitOpen").closest("div");
    expect(screen.getByText("health.circuitOpen")).toBeInTheDocument();
    expect(badge).toHaveAttribute("title", "failures:5");
    expect(badge?.className).toContain("bg-red-500/10");
  });
});
