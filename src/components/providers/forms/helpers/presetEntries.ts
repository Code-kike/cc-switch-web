import type { AppId } from "@/lib/api";
import {
  providerPresets,
  type ProviderPreset,
} from "@/config/claudeProviderPresets";
import {
  codexProviderPresets,
  type CodexProviderPreset,
} from "@/config/codexProviderPresets";
import {
  geminiProviderPresets,
  type GeminiProviderPreset,
} from "@/config/geminiProviderPresets";
import {
  opencodeProviderPresets,
  type OpenCodeProviderPreset,
} from "@/config/opencodeProviderPresets";
import {
  openclawProviderPresets,
  type OpenClawProviderPreset,
} from "@/config/openclawProviderPresets";
import {
  hermesProviderPresets,
  type HermesProviderPreset,
} from "@/config/hermesProviderPresets";

export type PresetEntry = {
  id: string;
  preset:
    | ProviderPreset
    | CodexProviderPreset
    | GeminiProviderPreset
    | OpenCodeProviderPreset
    | OpenClawProviderPreset
    | HermesProviderPreset;
};

/**
 * Build the per-app preset list with stable synthetic ids (`<app>-<index>`).
 * Extracted verbatim from ProviderForm's `presetEntries` useMemo (L26
 * conservative split): pure mapping over the static preset arrays, no hooks
 * or component state. The claude branch keeps its filter-then-index ordering
 * so ids stay aligned with the visible (non-hidden) presets.
 */
export function buildPresetEntries(appId: AppId): PresetEntry[] {
  if (appId === "codex") {
    return codexProviderPresets.map<PresetEntry>((preset, index) => ({
      id: `codex-${index}`,
      preset,
    }));
  } else if (appId === "gemini") {
    return geminiProviderPresets.map<PresetEntry>((preset, index) => ({
      id: `gemini-${index}`,
      preset,
    }));
  } else if (appId === "opencode") {
    return opencodeProviderPresets.map<PresetEntry>((preset, index) => ({
      id: `opencode-${index}`,
      preset,
    }));
  } else if (appId === "openclaw") {
    return openclawProviderPresets.map<PresetEntry>((preset, index) => ({
      id: `openclaw-${index}`,
      preset,
    }));
  } else if (appId === "hermes") {
    return hermesProviderPresets.map<PresetEntry>((preset, index) => ({
      id: `hermes-${index}`,
      preset,
    }));
  }
  return providerPresets
    .filter((p) => !p.hidden)
    .map<PresetEntry>((preset, index) => ({
      id: `claude-${index}`,
      preset,
    }));
}
