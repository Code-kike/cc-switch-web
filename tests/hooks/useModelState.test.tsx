import { act, renderHook } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { useModelState } from "@/components/providers/forms/hooks/useModelState";

describe("useModelState", () => {
  it("hydrates the Claude Code subagent model from env", () => {
    const settingsConfig = JSON.stringify({
      env: {
        ANTHROPIC_MODEL: "fallback-model",
        CLAUDE_CODE_SUBAGENT_MODEL: "subagent-model",
      },
    });

    const { result } = renderHook(() =>
      useModelState({ settingsConfig, onConfigChange: vi.fn() }),
    );

    expect(result.current.claudeModel).toBe("fallback-model");
    expect(result.current.subagentModel).toBe("subagent-model");
  });

  it("writes and clears the Claude Code subagent model env field", () => {
    let latestConfig = JSON.stringify({
      env: {
        ANTHROPIC_MODEL: "fallback-model",
      },
    });
    const onConfigChange = vi.fn((config: string) => {
      latestConfig = config;
    });

    const { result } = renderHook(() =>
      useModelState({
        settingsConfig: latestConfig,
        onConfigChange,
      }),
    );

    act(() => {
      result.current.handleModelChange(
        "CLAUDE_CODE_SUBAGENT_MODEL",
        "subagent-model[1M]",
      );
    });

    let env = JSON.parse(latestConfig).env;
    expect(env.ANTHROPIC_MODEL).toBe("fallback-model");
    expect(env.CLAUDE_CODE_SUBAGENT_MODEL).toBe("subagent-model[1M]");

    act(() => {
      result.current.handleModelChange("CLAUDE_CODE_SUBAGENT_MODEL", "");
    });

    env = JSON.parse(latestConfig).env;
    expect(env.CLAUDE_CODE_SUBAGENT_MODEL).toBeUndefined();
  });
});
