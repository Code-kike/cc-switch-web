import { describe, expect, it } from "vitest";
import { buildPresetEntries } from "@/components/providers/forms/helpers/presetEntries";
import { providerPresets } from "@/config/claudeProviderPresets";
import { codexProviderPresets } from "@/config/codexProviderPresets";

describe("buildPresetEntries", () => {
  it("assigns app-prefixed sequential ids per app", () => {
    const codex = buildPresetEntries("codex");
    expect(codex.length).toBe(codexProviderPresets.length);
    codex.forEach((entry, index) => {
      expect(entry.id).toBe(`codex-${index}`);
    });
  });

  it("filters hidden claude presets before indexing", () => {
    const claude = buildPresetEntries("claude");
    const visible = providerPresets.filter((p) => !p.hidden);
    expect(claude.length).toBe(visible.length);
    expect(claude[0]?.id).toBe("claude-0");
    claude.forEach((entry) => {
      expect((entry.preset as { hidden?: boolean }).hidden).not.toBe(true);
    });
  });
});
