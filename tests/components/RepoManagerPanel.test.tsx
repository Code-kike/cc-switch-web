import "@testing-library/jest-dom";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { RepoManagerPanel } from "@/components/skills/RepoManagerPanel";

const openExternalMock = vi.fn();

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, options?: Record<string, string>) => {
      if (key === "skills.repo.duplicateWarning") {
        return `${key}:${options?.owner}/${options?.name}`;
      }
      return key;
    },
  }),
}));

vi.mock("@/lib/api", () => ({
  settingsApi: {
    openExternal: (...args: unknown[]) => openExternalMock(...args),
  },
}));

vi.mock("@/components/common/FullScreenPanel", () => ({
  FullScreenPanel: ({ children, title }: any) => (
    <div>
      <h2>{title}</h2>
      {children}
    </div>
  ),
}));

vi.mock("@/components/ui/button", () => ({
  Button: ({ children, ...props }: any) => <button {...props}>{children}</button>,
}));

vi.mock("@/components/ui/input", () => ({
  Input: (props: any) => <input {...props} />,
}));

vi.mock("@/components/ui/label", () => ({
  Label: ({ children, ...props }: any) => <label {...props}>{children}</label>,
}));

function deferred<T>() {
  let resolve!: (value: T | PromiseLike<T>) => void;
  let reject!: (reason?: unknown) => void;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });

  return { promise, resolve, reject };
}

function renderPanel(overrides: Partial<React.ComponentProps<typeof RepoManagerPanel>> = {}) {
  const onAdd = vi.fn().mockResolvedValue(undefined);
  const onRemove = vi.fn().mockResolvedValue(undefined);
  const onClose = vi.fn();

  render(
    <RepoManagerPanel
      repos={[
        {
          owner: "foo",
          name: "bar",
          branch: "main",
          enabled: true,
        },
      ]}
      skills={[]}
      onAdd={onAdd}
      onRemove={onRemove}
      onClose={onClose}
      {...overrides}
    />,
  );

  return { onAdd, onRemove, onClose };
}

describe("RepoManagerPanel", () => {
  beforeEach(() => {
    openExternalMock.mockReset();
  });

  it("warns when the repo already exists and disables the form while adding", async () => {
    const addRequest = deferred<void>();
    const onAdd = vi.fn().mockReturnValue(addRequest.promise);

    renderPanel({ onAdd });

    fireEvent.change(screen.getByLabelText("skills.repo.url"), {
      target: { value: "foo/bar" },
    });
    fireEvent.change(screen.getByLabelText("skills.repo.branch"), {
      target: { value: "develop" },
    });

    expect(screen.getByRole("status")).toHaveTextContent(
      "skills.repo.duplicateWarning:foo/bar",
    );

    fireEvent.click(screen.getByRole("button", { name: "skills.repo.add" }));

    await waitFor(() => {
      expect(onAdd).toHaveBeenCalledWith({
        owner: "foo",
        name: "bar",
        branch: "develop",
        enabled: true,
      });
    });

    expect(screen.getByLabelText("skills.repo.url")).toBeDisabled();
    expect(screen.getByLabelText("skills.repo.branch")).toBeDisabled();
    expect(
      screen.getByRole("button", { name: "common.saving" }),
    ).toBeDisabled();

    addRequest.resolve();

    await waitFor(() => {
      expect(
        screen.getByRole("button", { name: "skills.repo.add" }),
      ).toBeEnabled();
    });
    expect(screen.getByLabelText("skills.repo.url")).toHaveValue("");
    expect(screen.getByLabelText("skills.repo.branch")).toHaveValue("");
  });

  it("shows an inline error when adding a repo fails", async () => {
    const onAdd = vi.fn().mockRejectedValue(new Error("repo add failed"));

    renderPanel({ onAdd });

    fireEvent.change(screen.getByLabelText("skills.repo.url"), {
      target: { value: "demo/skills" },
    });
    fireEvent.click(screen.getByRole("button", { name: "skills.repo.add" }));

    await waitFor(() => {
      expect(screen.getByRole("alert")).toHaveTextContent("repo add failed");
    });
    expect(
      screen.getByRole("button", { name: "skills.repo.add" }),
    ).toBeEnabled();
  });

  it("shows row-level pending and error feedback when removing a repo fails", async () => {
    const removeRequest = deferred<void>();
    const onRemove = vi.fn().mockReturnValue(removeRequest.promise);

    renderPanel({ onRemove });

    fireEvent.click(screen.getByRole("button", { name: "common.delete" }));

    await waitFor(() => {
      expect(onRemove).toHaveBeenCalledWith("foo", "bar");
    });
    expect(
      screen.getByRole("button", { name: "common.deleting" }),
    ).toBeDisabled();

    removeRequest.reject(new Error("repo remove failed"));

    await waitFor(() => {
      expect(screen.getByRole("alert")).toHaveTextContent("repo remove failed");
    });
    expect(
      screen.getByRole("button", { name: "common.delete" }),
    ).toBeEnabled();
  });
});
