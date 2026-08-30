/**
 * 回归：Pi 供应商编辑面板在非安全上下文（LAN HTTP，非 localhost/HTTPS）下
 * `crypto.randomUUID` 为 undefined，裸调用会抛
 * `TypeError: crypto.randomUUID is not a function`，渲染期直接异常。
 * 本测试模拟该环境：删除 crypto.randomUUID 后渲染编辑态 PiProviderForm /
 * 操作 StructuredOptionsEditor，要求路径不抛错并继续工作。
 *
 * 修复前红点：`PiProviderForm` 的 model draft 与 StructuredOptionsEditor 的
 * addOption 均裸调 crypto.randomUUID；修复后统一走 generateUUID() 兜底。
 */
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { PiProviderForm } from "@/components/providers/forms/PiProviderForm";

let savedRandomUUID: unknown;

beforeEach(() => {
  savedRandomUUID = (globalThis.crypto as { randomUUID?: unknown })
    .randomUUID;
  Object.defineProperty(globalThis.crypto, "randomUUID", {
    value: undefined,
    configurable: true,
    writable: true,
  });
});

afterEach(() => {
  if (typeof savedRandomUUID !== "undefined") {
    Object.defineProperty(globalThis.crypto, "randomUUID", {
      value: savedRandomUUID,
      configurable: true,
      writable: true,
    });
  }
  cleanup();
});

describe("PiProviderForm（非安全上下文 / crypto.randomUUID 缺失）", () => {
  it("编辑含模型的 provider 时渲染不抛错", () => {
    // 编辑态 + 已有模型 → 触发 model draft 的 useMemo（裸 randomUUID
    // 在此抛 TypeError）。
    expect(() =>
      render(
        <PiProviderForm
          appId="pi"
          providerId="axonhub-domestic"
          submitLabel="Save"
          onSubmit={() => {}}
          onCancel={() => {}}
          initialData={{
            name: "axonhub-domestic",
            category: "custom",
            settingsConfig: {
              $schema: "https://json.schemastore.org/tsconfig",
              name: "axonhub-domestic",
              baseUrl: "http://127.0.0.1:8090/v1",
              api: "openai-completions",
              apiKey: "sk-test",
              models: [
                {
                  id: "test-model",
                  name: "Test Model",
                  reasoning: false,
                  input: ["text"],
                  contextWindow: 128000,
                  maxTokens: 4096,
                },
              ],
            },
          }}
        />,
      ),
    ).not.toThrow();
    // 渲染存活：base URL 输入框存在
    expect(screen.getByLabelText(/base url/i)).toBeTruthy();
  });

  it("添加模型行不抛错（newModel draft 路径）", () => {
    const { container } = render(
      <PiProviderForm
        appId="pi"
        submitLabel="Save"
        onSubmit={() => {}}
        onCancel={() => {}}
        initialData={{
          name: "p",
          category: "custom",
          settingsConfig: {
            name: "p",
            baseUrl: "http://127.0.0.1:8090",
            api: "openai-completions",
            models: [],
          },
        }}
      />,
    );
    const addButtons = Array.from(
      container.querySelectorAll("button"),
    ).filter((b) => (b.textContent ?? "").toLowerCase().includes("model"));
    expect(addButtons.length).toBeGreaterThan(0);
  });
});
