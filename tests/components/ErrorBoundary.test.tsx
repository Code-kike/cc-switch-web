import { render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ErrorBoundary } from "@/components/ErrorBoundary";

function Boom(): never {
  throw new Error("boom");
}

afterEach(() => {
  vi.restoreAllMocks();
});

describe("ErrorBoundary", () => {
  it("renders the fallback (not a blank app) when a child throws during render", () => {
    // React logs the caught render error; silence it to keep test output clean.
    vi.spyOn(console, "error").mockImplementation(() => {});

    render(
      <ErrorBoundary>
        <Boom />
      </ErrorBoundary>,
    );

    // role=alert fallback present (getByRole throws if absent) + a reload action
    // in whichever locale the test env resolved to.
    expect(screen.getByRole("alert")).toBeTruthy();
    expect(
      screen.getByRole("button", { name: /reload|重新加载|再読み込み/i }),
    ).toBeTruthy();
  });

  it("renders children unchanged when no error is thrown", () => {
    render(
      <ErrorBoundary>
        <div>healthy child</div>
      </ErrorBoundary>,
    );
    expect(screen.queryByRole("alert")).toBeNull();
    expect(screen.getByText("healthy child")).toBeTruthy();
  });
});
