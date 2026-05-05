import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { isWebMode } from "@/lib/api/adapter";
import { getServerPlatform } from "@/lib/api/path-adapter";
import { settingsApi, type AppId } from "@/lib/api";
import type { SettingsFormState } from "./useSettingsForm";

type AppDirectoryKey =
  | "claude"
  | "codex"
  | "gemini"
  | "opencode"
  | "openclaw"
  | "hermes";
type DirectoryKey = "appConfig" | AppDirectoryKey;

export interface ResolvedDirectories {
  appConfig: string;
  claude: string;
  codex: string;
  gemini: string;
  opencode: string;
  openclaw: string;
  hermes: string;
}

// Single source of truth for per-app directory metadata.
const APP_DIRECTORY_META: Record<
  AppId,
  { key: AppDirectoryKey; defaultFolder: string }
> = {
  claude: { key: "claude", defaultFolder: ".claude" },
  codex: { key: "codex", defaultFolder: ".codex" },
  gemini: { key: "gemini", defaultFolder: ".gemini" },
  opencode: { key: "opencode", defaultFolder: ".config/opencode" },
  openclaw: { key: "openclaw", defaultFolder: ".openclaw" },
  hermes: { key: "hermes", defaultFolder: ".hermes" },
};

const DIRECTORY_KEY_TO_SETTINGS_FIELD: Record<
  AppDirectoryKey,
  keyof SettingsFormState
> = {
  claude: "claudeConfigDir",
  codex: "codexConfigDir",
  gemini: "geminiConfigDir",
  opencode: "opencodeConfigDir",
  openclaw: "openclawConfigDir",
  hermes: "hermesConfigDir",
};

const sanitizeDir = (value?: string | null): string | undefined => {
  if (!value) return undefined;
  const trimmed = value.trim();
  return trimmed.length > 0 ? trimmed : undefined;
};

export interface UseDirectorySettingsProps {
  settings: SettingsFormState | null;
  onUpdateSettings: (updates: Partial<SettingsFormState>) => void;
}

export interface UseDirectorySettingsResult {
  appConfigDir?: string;
  resolvedDirs: ResolvedDirectories;
  isLoading: boolean;
  initialAppConfigDir?: string;
  updateDirectory: (app: AppId, value?: string) => void;
  updateAppConfigDir: (value?: string) => void;
  browseDirectory: (app: AppId) => Promise<void>;
  browseAppConfigDir: () => Promise<void>;
  resetDirectory: (app: AppId) => Promise<void>;
  resetAppConfigDir: () => Promise<void>;
  resetAllDirectories: (overrides?: ResolvedAppDirectoryOverrides) => void;
}

export type ResolvedAppDirectoryOverrides = Partial<
  Record<AppDirectoryKey, string | undefined>
>;

const EMPTY_RESOLVED_DIRECTORIES: ResolvedDirectories = {
  appConfig: "",
  claude: "",
  codex: "",
  gemini: "",
  opencode: "",
  openclaw: "",
  hermes: "",
};

const normalizeDefaultDirectories = (
  defaults?: Partial<Record<keyof ResolvedDirectories, string | null | undefined>>,
): ResolvedDirectories => ({
  appConfig: sanitizeDir(defaults?.appConfig) ?? "",
  claude: sanitizeDir(defaults?.claude) ?? "",
  codex: sanitizeDir(defaults?.codex) ?? "",
  gemini: sanitizeDir(defaults?.gemini) ?? "",
  opencode: sanitizeDir(defaults?.opencode) ?? "",
  openclaw: sanitizeDir(defaults?.openclaw) ?? "",
  hermes: sanitizeDir(defaults?.hermes) ?? "",
});

const resolveDesktopDefaultDirectories = async (): Promise<ResolvedDirectories> => {
  try {
    const path = await import("@tauri-apps/api/path");
    const home = await path.homeDir();

    return normalizeDefaultDirectories({
      appConfig: await path.join(home, ".cc-switch"),
      claude: await path.join(home, ".claude"),
      codex: await path.join(home, ".codex"),
      gemini: await path.join(home, ".gemini"),
      opencode: await path.join(home, ".config/opencode"),
      openclaw: await path.join(home, ".openclaw"),
      hermes: await path.join(home, ".hermes"),
    });
  } catch (error) {
    console.error(
      "[useDirectorySettings] Failed to resolve desktop default directories",
      error,
    );
    return EMPTY_RESOLVED_DIRECTORIES;
  }
};

const resolveDefaultDirectories = async (): Promise<ResolvedDirectories> => {
  if (!isWebMode()) {
    return resolveDesktopDefaultDirectories();
  }

  try {
    const platform = await getServerPlatform();
    return normalizeDefaultDirectories(platform.defaultPaths);
  } catch (error) {
    console.error(
      "[useDirectorySettings] Failed to load server default directories",
      error,
    );
    return EMPTY_RESOLVED_DIRECTORIES;
  }
};

/**
 * useDirectorySettings - 目录管理
 * 负责：
 * - appConfigDir 状态
 * - resolvedDirs 状态
 * - 目录选择（browse）
 * - 目录重置
 * - 默认值计算
 */
export function useDirectorySettings({
  settings,
  onUpdateSettings,
}: UseDirectorySettingsProps): UseDirectorySettingsResult {
  const { t } = useTranslation();

  const [appConfigDir, setAppConfigDir] = useState<string | undefined>(
    undefined,
  );
  const [resolvedDirs, setResolvedDirs] = useState<ResolvedDirectories>(
    EMPTY_RESOLVED_DIRECTORIES,
  );
  const [isLoading, setIsLoading] = useState(true);

  const defaultsRef = useRef<ResolvedDirectories>(EMPTY_RESOLVED_DIRECTORIES);
  const initialAppConfigDirRef = useRef<string | undefined>(undefined);

  // 加载目录信息
  useEffect(() => {
    let active = true;
    setIsLoading(true);

    const load = async () => {
      try {
        const [
          overrideRaw,
          claudeDir,
          codexDir,
          geminiDir,
          opencodeDir,
          openclawDir,
          hermesDir,
          defaultDirs,
        ] = await Promise.all([
          settingsApi.getAppConfigDirOverride(),
          settingsApi.getConfigDir("claude"),
          settingsApi.getConfigDir("codex"),
          settingsApi.getConfigDir("gemini"),
          settingsApi.getConfigDir("opencode"),
          settingsApi.getConfigDir("openclaw"),
          settingsApi.getConfigDir("hermes"),
          resolveDefaultDirectories(),
        ]);

        if (!active) return;

        const normalizedOverride = sanitizeDir(overrideRaw ?? undefined);

        defaultsRef.current = defaultDirs;

        setAppConfigDir(normalizedOverride);
        initialAppConfigDirRef.current = normalizedOverride;

        setResolvedDirs({
          appConfig: normalizedOverride ?? defaultDirs.appConfig,
          claude: claudeDir || defaultDirs.claude,
          codex: codexDir || defaultDirs.codex,
          gemini: geminiDir || defaultDirs.gemini,
          opencode: opencodeDir || defaultDirs.opencode,
          openclaw: openclawDir || defaultDirs.openclaw,
          hermes: hermesDir || defaultDirs.hermes,
        });
      } catch (error) {
        console.error(
          "[useDirectorySettings] Failed to load directory info",
          error,
        );
      } finally {
        if (active) {
          setIsLoading(false);
        }
      }
    };

    void load();
    return () => {
      active = false;
    };
  }, []);

  const updateDirectoryState = useCallback(
    (key: DirectoryKey, value?: string) => {
      const sanitized = sanitizeDir(value);
      if (key === "appConfig") {
        setAppConfigDir(sanitized);
      } else {
        onUpdateSettings({
          [DIRECTORY_KEY_TO_SETTINGS_FIELD[key]]: sanitized,
        });
      }

      setResolvedDirs((prev) => {
        const next = sanitized ?? defaultsRef.current[key];
        // Same-ref early-return: unchanged value shouldn't cascade renders
        // through the settings tree.
        if (prev[key] === next) return prev;
        return { ...prev, [key]: next };
      });
    },
    [onUpdateSettings],
  );

  const updateAppConfigDir = useCallback(
    (value?: string) => {
      updateDirectoryState("appConfig", value);
    },
    [updateDirectoryState],
  );

  const updateDirectory = useCallback(
    (app: AppId, value?: string) => {
      updateDirectoryState(APP_DIRECTORY_META[app].key, value);
    },
    [updateDirectoryState],
  );

  const browseDirectory = useCallback(
    async (app: AppId) => {
      const key = APP_DIRECTORY_META[app].key;
      const settingsField = DIRECTORY_KEY_TO_SETTINGS_FIELD[key];
      const currentValue =
        (settings?.[settingsField] as string | undefined) ?? resolvedDirs[key];

      try {
        const picked = await settingsApi.selectConfigDirectory(currentValue);
        const sanitized = sanitizeDir(picked ?? undefined);
        if (!sanitized) return;
        updateDirectoryState(key, sanitized);
      } catch (error) {
        console.error("[useDirectorySettings] Failed to pick directory", error);
        toast.error(
          t("settings.selectFileFailed", {
            defaultValue: "选择目录失败",
          }),
        );
      }
    },
    [settings, resolvedDirs, t, updateDirectoryState],
  );

  const browseAppConfigDir = useCallback(async () => {
    const currentValue = appConfigDir ?? resolvedDirs.appConfig;
    try {
      const picked = await settingsApi.selectConfigDirectory(currentValue);
      const sanitized = sanitizeDir(picked ?? undefined);
      if (!sanitized) return;
      updateDirectoryState("appConfig", sanitized);
    } catch (error) {
      console.error(
        "[useDirectorySettings] Failed to pick app config directory",
        error,
      );
      toast.error(
        t("settings.selectFileFailed", {
          defaultValue: "选择目录失败",
        }),
      );
    }
  }, [appConfigDir, resolvedDirs.appConfig, t, updateDirectoryState]);

  const resetDirectory = useCallback(
    async (app: AppId) => {
      const key = APP_DIRECTORY_META[app].key;
      if (!defaultsRef.current[key]) {
        defaultsRef.current = await resolveDefaultDirectories();
      }
      updateDirectoryState(key, undefined);
    },
    [updateDirectoryState],
  );

  const resetAppConfigDir = useCallback(async () => {
    if (!defaultsRef.current.appConfig) {
      defaultsRef.current = await resolveDefaultDirectories();
    }
    updateDirectoryState("appConfig", undefined);
  }, [updateDirectoryState]);

  const resetAllDirectories = useCallback(
    (overrides?: ResolvedAppDirectoryOverrides) => {
      setAppConfigDir(initialAppConfigDirRef.current);
      setResolvedDirs({
        appConfig:
          initialAppConfigDirRef.current ?? defaultsRef.current.appConfig,
        claude: overrides?.claude ?? defaultsRef.current.claude,
        codex: overrides?.codex ?? defaultsRef.current.codex,
        gemini: overrides?.gemini ?? defaultsRef.current.gemini,
        opencode: overrides?.opencode ?? defaultsRef.current.opencode,
        openclaw: overrides?.openclaw ?? defaultsRef.current.openclaw,
        hermes: overrides?.hermes ?? defaultsRef.current.hermes,
      });
    },
    [],
  );

  return {
    appConfigDir,
    resolvedDirs,
    isLoading,
    initialAppConfigDir: initialAppConfigDirRef.current,
    updateDirectory,
    updateAppConfigDir,
    browseDirectory,
    browseAppConfigDir,
    resetDirectory,
    resetAppConfigDir,
    resetAllDirectories,
  };
}
