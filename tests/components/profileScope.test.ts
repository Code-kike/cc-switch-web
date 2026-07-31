import { describe, expect, it } from "vitest";
import {
  APP_PROFILE_SCOPE,
  hasScopeSnapshot,
} from "@/components/profiles/scope";
import type { Profile } from "@/lib/api/profiles";

const profile = (overrides: Partial<Profile["payload"]> = {}): Profile => ({
  id: "p1",
  name: "Project",
  payload: {
    providers: { claude: null, codex: null },
    mcp: { claude: null, codex: null },
    skills: { claude: null, codex: null },
    prompts: { claude: null, codex: null },
    ...overrides,
  },
});

describe("profile scope mirror", () => {
  it("supports only Claude and Codex app tabs", () => {
    expect(APP_PROFILE_SCOPE).toEqual({ claude: "claude", codex: "codex" });
    expect(APP_PROFILE_SCOPE.gemini).toBeUndefined();
    expect(APP_PROFILE_SCOPE.opencode).toBeUndefined();
    expect(APP_PROFILE_SCOPE.openclaw).toBeUndefined();
    expect(APP_PROFILE_SCOPE.hermes).toBeUndefined();
  });

  it("distinguishes an uncaptured slot from a captured empty snapshot", () => {
    const uncaptured = profile();
    expect(hasScopeSnapshot(uncaptured, "claude")).toBe(false);
    expect(hasScopeSnapshot(uncaptured, "codex")).toBe(false);

    const claudeCapturedEmpty = profile({
      mcp: { claude: [], codex: null },
    });
    expect(hasScopeSnapshot(claudeCapturedEmpty, "claude")).toBe(true);
    expect(hasScopeSnapshot(claudeCapturedEmpty, "codex")).toBe(false);

    const codexCaptured = profile({
      providers: { claude: null, codex: "codex-provider" },
    });
    expect(hasScopeSnapshot(codexCaptured, "claude")).toBe(false);
    expect(hasScopeSnapshot(codexCaptured, "codex")).toBe(true);
  });
});
