import { describe, expect, it } from "vitest";

import { getPromptFilename } from "@/components/prompts/promptFilename";

describe("getPromptFilename", () => {
  it("uses Hermes' SOUL.md while preserving the shared AGENTS.md filename", () => {
    expect(getPromptFilename("hermes")).toBe("SOUL.md");
    expect(getPromptFilename("openclaw")).toBe("AGENTS.md");
  });
});
