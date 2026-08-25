import type { AppId } from "@/lib/api";
import type { ProviderCategory } from "@/types";
import { providerSchema } from "@/lib/schemas/provider";
import { parseOmoOtherFieldsObject } from "@/types/omo";

/**
 * Typed reasons the submitted config cannot be assembled/validated.
 * The caller maps these to the existing hard-reject toast messages so the
 * happy-path UX is unchanged.
 */
export type AssembleSettingsConfigError =
  | "codexAuthInvalid"
  | "geminiConfigInvalid"
  | "settingsConfigInvalid";

export type AssembleSettingsConfigResult =
  | { ok: true; settingsConfig: string }
  | { ok: false; error: AssembleSettingsConfigError };

export interface AssembleSettingsConfigParams {
  appId: AppId;
  category: ProviderCategory | undefined;
  isAnyOmoCategory: boolean;
  /** Raw provider name; trimmed before being synced into the config. */
  name: string;
  /**
   * The form textarea value. Used as-is (trimmed) for claude /
   * opencode-non-omo / openclaw / hermes.
   */
  textareaSettingsConfig: string;
  // codex hook state
  codexAuth?: string;
  codexConfig?: string;
  // gemini hook state
  geminiEnv?: string;
  geminiConfig?: string;
  envStringToObj?: (value: string) => Record<string, string>;
  /** Official managed Codex cards persist only the accountId binding. */
  stripCodexOfficialAuth?: boolean;
  // omo draft state
  omoAgents?: Record<string, unknown>;
  omoCategories?: Record<string, unknown>;
  omoOtherFieldsStr?: string;
}

/**
 * Assembles the settingsConfig string that will ACTUALLY be submitted and
 * validates it.
 *
 * For codex / gemini / OMO the submitted config is rebuilt from per-CLI hook
 * state rather than the zod-validated textarea, so this is the single place
 * where "what we validate" equals "what we save" (M40). It runs the same
 * schema rule the form resolver uses against the final (name-synced) string,
 * and returns a typed error instead of silently falling back to a divergent
 * config — invalid JSON is never silently persisted.
 */
export function assembleSettingsConfig(
  params: AssembleSettingsConfigParams,
): AssembleSettingsConfigResult {
  const {
    appId,
    category,
    isAnyOmoCategory,
    name,
    textareaSettingsConfig,
    codexAuth = "",
    codexConfig = "",
    geminiEnv = "",
    geminiConfig = "",
    envStringToObj,
    omoAgents = {},
    omoCategories = {},
    omoOtherFieldsStr = "",
    stripCodexOfficialAuth = false,
  } = params;

  let settingsConfig: string;

  if (appId === "codex") {
    try {
      const authJson = stripCodexOfficialAuth ? {} : JSON.parse(codexAuth);
      settingsConfig = JSON.stringify({
        auth: authJson,
        config: codexConfig ?? "",
      });
    } catch {
      return { ok: false, error: "codexAuthInvalid" };
    }
  } else if (appId === "gemini") {
    try {
      const envObj = envStringToObj ? envStringToObj(geminiEnv) : {};
      const configObj = geminiConfig.trim() ? JSON.parse(geminiConfig) : {};
      settingsConfig = JSON.stringify({ env: envObj, config: configObj });
    } catch {
      // M40: previously this silently fell back to the textarea, dropping the
      // user's (invalid) Gemini edits and saving a stale value. Hard-reject
      // instead so a divergent config is never silently saved.
      return { ok: false, error: "geminiConfigInvalid" };
    }
  } else if (
    appId === "opencode" &&
    (category === "omo" || category === "omo-slim")
  ) {
    const omoConfig: Record<string, unknown> = {};
    if (Object.keys(omoAgents).length > 0) {
      omoConfig.agents = omoAgents;
    }
    if (category === "omo" && Object.keys(omoCategories).length > 0) {
      omoConfig.categories = omoCategories;
    }
    if (omoOtherFieldsStr.trim()) {
      let otherFields: Record<string, unknown> | undefined;
      try {
        otherFields = parseOmoOtherFieldsObject(omoOtherFieldsStr);
      } catch {
        otherFields = undefined;
      }
      if (otherFields) {
        omoConfig.otherFields = otherFields;
      }
    }
    settingsConfig = JSON.stringify(omoConfig);
  } else {
    settingsConfig = textareaSettingsConfig.trim();
  }

  settingsConfig = syncProviderNameIntoSettingsConfig(
    appId,
    settingsConfig,
    name.trim(),
    isAnyOmoCategory,
  );

  // Final safety net: validate the EXACT string we will save with the same
  // rule the form resolver applies to the textarea.
  const validation =
    providerSchema.shape.settingsConfig.safeParse(settingsConfig);
  if (!validation.success) {
    return { ok: false, error: "settingsConfigInvalid" };
  }

  return { ok: true, settingsConfig };
}

/**
 * Mirrors the provider name into the config payload so the saved config and
 * the form name stay consistent (claude → `ui.displayName`, opencode → `name`).
 * No-op for other CLIs / OMO categories, and a no-op on parse failure.
 */
export function syncProviderNameIntoSettingsConfig(
  appId: AppId,
  settingsConfig: string,
  providerName: string,
  isOmoCategory: boolean,
): string {
  if (!providerName) {
    return settingsConfig;
  }

  if (appId !== "claude" && appId !== "opencode") {
    return settingsConfig;
  }

  if (appId === "opencode" && isOmoCategory) {
    return settingsConfig;
  }

  try {
    const parsed = JSON.parse(settingsConfig) as Record<string, unknown>;
    if (!parsed || Array.isArray(parsed)) {
      return settingsConfig;
    }

    if (appId === "claude") {
      const ui =
        parsed.ui && typeof parsed.ui === "object" && !Array.isArray(parsed.ui)
          ? { ...(parsed.ui as Record<string, unknown>) }
          : {};
      ui.displayName = providerName;
      parsed.ui = ui;
    } else {
      parsed.name = providerName;
    }

    return JSON.stringify(parsed, null, 2);
  } catch {
    return settingsConfig;
  }
}
