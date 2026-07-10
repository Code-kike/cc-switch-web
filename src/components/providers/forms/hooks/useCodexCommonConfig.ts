import { useState, useEffect, useCallback, useRef } from "react";
import { useTranslation } from "react-i18next";
import { parse as parseToml } from "smol-toml";
import { hasTomlCommonConfigSnippet } from "@/utils/providerConfigUtils";
import { configApi } from "@/lib/api";
import { normalizeTomlText } from "@/utils/textNormalization";
import { extractErrorMessage } from "@/utils/errorUtils";

const applyTomlSnippet = async (
  configToml: string,
  snippetToml: string,
  enabled: boolean,
): Promise<{ updatedConfig: string; error?: unknown }> => {
  try {
    const updatedConfig = await configApi.updateTomlCommonConfigSnippet(
      configToml,
      snippetToml,
      enabled,
    );
    return { updatedConfig };
  } catch (error) {
    return { updatedConfig: configToml, error };
  }
};

const LEGACY_STORAGE_KEY = "cc-switch:codex-common-config-snippet";
const DEFAULT_CODEX_COMMON_CONFIG_SNIPPET = `# Common Codex config
# Add your common TOML configuration here`;

interface UseCodexCommonConfigProps {
  codexConfig: string;
  onConfigChange: (config: string) => void;
  initialData?: {
    settingsConfig?: Record<string, unknown>;
  };
  initialEnabled?: boolean;
  selectedPresetId?: string;
}

/**
 * 管理 Codex 通用配置片段 (TOML 格式)
 * 从 config.json 读取和保存，支持从 localStorage 平滑迁移
 */
export function useCodexCommonConfig({
  codexConfig,
  onConfigChange,
  initialData,
  initialEnabled,
  selectedPresetId,
}: UseCodexCommonConfigProps) {
  const { t } = useTranslation();
  const [useCommonConfig, setUseCommonConfig] = useState(false);
  const [commonConfigSnippet, setCommonConfigSnippetState] = useState<string>(
    DEFAULT_CODEX_COMMON_CONFIG_SNIPPET,
  );
  const [commonConfigError, setCommonConfigError] = useState("");
  const [isLoading, setIsLoading] = useState(true);
  const [isExtracting, setIsExtracting] = useState(false);

  const formatConfigError = useCallback(
    (error: unknown, key: string) =>
      t(key, {
        error: extractErrorMessage(error) || t("common.unknown"),
      }),
    [t],
  );

  // 用于跟踪是否正在通过通用配置更新
  const isUpdatingFromCommonConfig = useRef(false);
  // 用于跟踪新建模式是否已初始化默认勾选
  const hasInitializedNewMode = useRef(false);
  // 用于跟踪编辑模式是否已初始化显式开关/预览
  const hasInitializedEditMode = useRef(false);
  const tomlOpSeqRef = useRef(0);
  const extractOpSeqRef = useRef(0);
  const latestCodexConfigRef = useRef(codexConfig);
  latestCodexConfigRef.current = codexConfig;
  const snippetSaveQueueRef = useRef<Promise<void>>(Promise.resolve());
  const operationIdentityRef = useRef({ selectedPresetId, initialEnabled });

  if (
    operationIdentityRef.current.selectedPresetId !== selectedPresetId ||
    operationIdentityRef.current.initialEnabled !== initialEnabled
  ) {
    operationIdentityRef.current = { selectedPresetId, initialEnabled };
    tomlOpSeqRef.current += 1;
    hasInitializedNewMode.current = false;
    hasInitializedEditMode.current = false;
  }

  const isTomlOpStale = useCallback(
    (seq: number, baseConfig: string) =>
      seq !== tomlOpSeqRef.current ||
      baseConfig !== latestCodexConfigRef.current,
    [],
  );

  // Unmount invalidates every pending TOML/save operation.
  useEffect(() => {
    return () => {
      tomlOpSeqRef.current += 1;
    };
  }, []);

  const persistCommonConfigSnippet = useCallback(
    async (value: string, seq: number): Promise<boolean> => {
      const save = snippetSaveQueueRef.current.then(() =>
        configApi.setCommonConfigSnippet("codex", value),
      );
      snippetSaveQueueRef.current = save.catch(() => undefined);

      try {
        await save;
      } catch (error) {
        if (seq === tomlOpSeqRef.current) {
          console.error("保存 Codex 通用配置失败:", error);
          setCommonConfigError(
            formatConfigError(error, "codexConfig.saveFailed"),
          );
        }
        return false;
      }

      return seq === tomlOpSeqRef.current;
    },
    [formatConfigError],
  );

  const parseCommonConfigSnippet = useCallback(
    (snippetString: string) => {
      const trimmed = snippetString.trim();
      if (!trimmed) {
        return {
          hasContent: false,
        };
      }

      try {
        const parsed = parseToml(normalizeTomlText(snippetString)) as Record<
          string,
          unknown
        >;
        return {
          hasContent: Object.keys(parsed).length > 0,
        };
      } catch (error) {
        return {
          hasContent: false,
          error: extractErrorMessage(error) || t("common.unknown"),
        };
      }
    },
    [t],
  );

  // 初始化：从 config.json 加载，支持从 localStorage 迁移
  useEffect(() => {
    let mounted = true;

    const loadSnippet = async () => {
      try {
        // 使用统一 API 加载
        const snippet = await configApi.getCommonConfigSnippet("codex");

        if (snippet && snippet.trim()) {
          if (mounted) {
            setCommonConfigSnippetState(snippet);
          }
        } else {
          // 如果 config.json 中没有，尝试从 localStorage 迁移
          if (typeof window !== "undefined") {
            try {
              const legacySnippet =
                window.localStorage.getItem(LEGACY_STORAGE_KEY);
              if (legacySnippet && legacySnippet.trim()) {
                // 迁移到 config.json
                await configApi.setCommonConfigSnippet("codex", legacySnippet);
                if (mounted) {
                  setCommonConfigSnippetState(legacySnippet);
                }
                // 清理 localStorage
                window.localStorage.removeItem(LEGACY_STORAGE_KEY);
                console.log(
                  "[迁移] Codex 通用配置已从 localStorage 迁移到 config.json",
                );
              }
            } catch (e) {
              console.warn("[迁移] 从 localStorage 迁移失败:", e);
            }
          }
        }
      } catch (error) {
        console.error("加载 Codex 通用配置失败:", error);
      } finally {
        if (mounted) {
          setIsLoading(false);
        }
      }
    };

    loadSnippet();

    return () => {
      mounted = false;
    };
  }, []);

  // 初始化时检查通用配置片段（编辑模式）
  useEffect(() => {
    if (
      !initialData?.settingsConfig ||
      isLoading ||
      hasInitializedEditMode.current
    ) {
      return;
    }

    hasInitializedEditMode.current = true;

    const parsedSnippet = parseCommonConfigSnippet(commonConfigSnippet);
    if (parsedSnippet.error) {
      if (commonConfigSnippet.trim()) {
        setCommonConfigError(parsedSnippet.error);
      }
      setUseCommonConfig(false);
      return;
    }

    const config =
      typeof initialData.settingsConfig.config === "string"
        ? initialData.settingsConfig.config
        : "";
    const inferredHasCommon = hasTomlCommonConfigSnippet(
      config,
      commonConfigSnippet,
    );

    // 优先级：显式设置的 initialEnabled > 从配置推断的值
    // 如果 initialEnabled 为 undefined，使用推断值
    const hasCommon =
      initialEnabled !== undefined ? initialEnabled : inferredHasCommon;

    // 如果应该启用通用配置但配置中还没有，则自动添加
    if (hasCommon && !inferredHasCommon && parsedSnippet.hasContent) {
      let cancelled = false;
      const seq = ++tomlOpSeqRef.current;
      void (async () => {
        const { updatedConfig, error } = await applyTomlSnippet(
          codexConfig,
          commonConfigSnippet,
          true,
        );
        if (cancelled || isTomlOpStale(seq, codexConfig)) {
          return;
        }
        if (error) {
          setCommonConfigError(extractErrorMessage(error));
          setUseCommonConfig(false);
          return;
        }

        setCommonConfigError("");
        setUseCommonConfig(true);
        isUpdatingFromCommonConfig.current = true;
        onConfigChange(updatedConfig);
        setTimeout(() => {
          isUpdatingFromCommonConfig.current = false;
        }, 0);
      })();
      return () => {
        cancelled = true;
      };
    }

    setCommonConfigError("");
    setUseCommonConfig(hasCommon);
  }, [
    codexConfig,
    commonConfigSnippet,
    initialData,
    initialEnabled,
    isLoading,
    isTomlOpStale,
    onConfigChange,
    parseCommonConfigSnippet,
  ]);

  // 新建模式：如果通用配置片段存在且有效，默认启用
  useEffect(() => {
    if (initialData || isLoading || hasInitializedNewMode.current) {
      return;
    }

    hasInitializedNewMode.current = true;

    const parsedSnippet = parseCommonConfigSnippet(commonConfigSnippet);
    if (parsedSnippet.error) {
      if (commonConfigSnippet.trim()) {
        setCommonConfigError(parsedSnippet.error);
      }
      setUseCommonConfig(false);
      return;
    }
    if (!parsedSnippet.hasContent) {
      return;
    }

    let cancelled = false;
    const seq = ++tomlOpSeqRef.current;
    void (async () => {
      const { updatedConfig, error } = await applyTomlSnippet(
        codexConfig,
        commonConfigSnippet,
        true,
      );
      if (cancelled || isTomlOpStale(seq, codexConfig)) {
        return;
      }
      if (error) {
        setCommonConfigError(extractErrorMessage(error));
        setUseCommonConfig(false);
        return;
      }

      setCommonConfigError("");
      setUseCommonConfig(true);
      isUpdatingFromCommonConfig.current = true;
      onConfigChange(updatedConfig);
      setTimeout(() => {
        isUpdatingFromCommonConfig.current = false;
      }, 0);
    })();
    return () => {
      cancelled = true;
    };
  }, [
    initialData,
    commonConfigSnippet,
    isLoading,
    codexConfig,
    isTomlOpStale,
    onConfigChange,
    parseCommonConfigSnippet,
  ]);

  // 处理通用配置开关
  const handleCommonConfigToggle = useCallback(
    async (checked: boolean) => {
      const seq = ++tomlOpSeqRef.current;
      const parsedSnippet = parseCommonConfigSnippet(commonConfigSnippet);
      if (parsedSnippet.error) {
        setCommonConfigError(parsedSnippet.error);
        setUseCommonConfig(false);
        return;
      }
      if (!parsedSnippet.hasContent) {
        setCommonConfigError(
          t("codexConfig.noCommonConfigToApply", {
            defaultValue: "通用配置片段为空或没有可写入的内容",
          }),
        );
        setUseCommonConfig(false);
        return;
      }

      const { updatedConfig, error: snippetError } = await applyTomlSnippet(
        codexConfig,
        commonConfigSnippet,
        checked,
      );
      if (isTomlOpStale(seq, codexConfig)) {
        return;
      }

      if (snippetError) {
        setCommonConfigError(extractErrorMessage(snippetError));
        setUseCommonConfig(false);
        return;
      }

      setCommonConfigError("");
      setUseCommonConfig(checked);
      // 标记正在通过通用配置更新
      isUpdatingFromCommonConfig.current = true;
      onConfigChange(updatedConfig);
      // 在下一个事件循环中重置标记
      setTimeout(() => {
        isUpdatingFromCommonConfig.current = false;
      }, 0);
    },
    [
      codexConfig,
      commonConfigSnippet,
      isTomlOpStale,
      onConfigChange,
      parseCommonConfigSnippet,
      t,
    ],
  );

  // 处理通用配置片段变化
  const handleCommonConfigSnippetChange = useCallback(
    async (value: string): Promise<boolean> => {
      const seq = ++tomlOpSeqRef.current;
      const previousSnippet = commonConfigSnippet;

      if (!value.trim()) {
        let updatedConfig = codexConfig;

        if (useCommonConfig) {
          const previousParsed = parseCommonConfigSnippet(previousSnippet);
          if (!previousParsed.error && previousParsed.hasContent) {
            const removeResult = await applyTomlSnippet(
              codexConfig,
              previousSnippet,
              false,
            );
            if (isTomlOpStale(seq, codexConfig)) {
              return false;
            }
            if (removeResult.error) {
              setCommonConfigError(extractErrorMessage(removeResult.error));
              return false;
            }
            updatedConfig = removeResult.updatedConfig;
          }
        }

        if (!(await persistCommonConfigSnippet("", seq))) {
          return false;
        }
        if (isTomlOpStale(seq, codexConfig)) {
          return false;
        }

        if (useCommonConfig) {
          isUpdatingFromCommonConfig.current = true;
          onConfigChange(updatedConfig);
          setTimeout(() => {
            isUpdatingFromCommonConfig.current = false;
          }, 0);
        }
        setUseCommonConfig(false);
        setCommonConfigSnippetState("");
        setCommonConfigError("");
        return true;
      }

      const parsedNextSnippet = parseCommonConfigSnippet(value);
      if (parsedNextSnippet.error) {
        setCommonConfigError(parsedNextSnippet.error);
        return false;
      }

      let updatedConfig = codexConfig;
      if (useCommonConfig) {
        const previousParsed = parseCommonConfigSnippet(previousSnippet);

        if (!previousParsed.error && previousParsed.hasContent) {
          const removeResult = await applyTomlSnippet(
            codexConfig,
            previousSnippet,
            false,
          );
          if (isTomlOpStale(seq, codexConfig)) {
            return false;
          }
          if (removeResult.error) {
            setCommonConfigError(extractErrorMessage(removeResult.error));
            return false;
          }
          updatedConfig = removeResult.updatedConfig;
        }

        const addResult = await applyTomlSnippet(updatedConfig, value, true);
        if (isTomlOpStale(seq, codexConfig)) {
          return false;
        }

        if (addResult.error) {
          setCommonConfigError(extractErrorMessage(addResult.error));
          return false;
        }
        updatedConfig = addResult.updatedConfig;
      }

      if (!(await persistCommonConfigSnippet(value, seq))) {
        return false;
      }
      if (isTomlOpStale(seq, codexConfig)) {
        return false;
      }

      if (useCommonConfig) {
        isUpdatingFromCommonConfig.current = true;
        onConfigChange(updatedConfig);
        setTimeout(() => {
          isUpdatingFromCommonConfig.current = false;
        }, 0);
      }

      setCommonConfigError("");
      setCommonConfigSnippetState(value);
      return true;
    },
    [
      commonConfigSnippet,
      codexConfig,
      isTomlOpStale,
      onConfigChange,
      parseCommonConfigSnippet,
      persistCommonConfigSnippet,
      useCommonConfig,
    ],
  );

  // 当配置变化时检查是否包含通用配置（但避免在通过通用配置更新时检查）
  useEffect(() => {
    if (isUpdatingFromCommonConfig.current || isLoading) {
      return;
    }
    const parsedSnippet = parseCommonConfigSnippet(commonConfigSnippet);
    if (parsedSnippet.error || !parsedSnippet.hasContent) {
      setUseCommonConfig(false);
      return;
    }
    const hasCommon = hasTomlCommonConfigSnippet(
      codexConfig,
      commonConfigSnippet,
    );
    setUseCommonConfig(hasCommon);
  }, [codexConfig, commonConfigSnippet, isLoading, parseCommonConfigSnippet]);

  // 从编辑器当前内容提取通用配置片段
  const handleExtract = useCallback(async () => {
    const seq = ++tomlOpSeqRef.current;
    const extractSeq = ++extractOpSeqRef.current;
    const baseConfig = codexConfig;
    setIsExtracting(true);
    setCommonConfigError("");

    try {
      const extracted = await configApi.extractCommonConfigSnippet("codex", {
        settingsConfig: JSON.stringify({
          config: baseConfig ?? "",
        }),
      });
      if (isTomlOpStale(seq, baseConfig)) {
        return;
      }

      if (!extracted || !extracted.trim()) {
        setCommonConfigError(t("codexConfig.extractNoCommonConfig"));
        return;
      }

      if (!(await persistCommonConfigSnippet(extracted, seq))) {
        return;
      }
      if (isTomlOpStale(seq, baseConfig)) {
        return;
      }
      setCommonConfigSnippetState(extracted);
    } catch (error) {
      if (seq === tomlOpSeqRef.current) {
        console.error("提取 Codex 通用配置失败:", error);
        setCommonConfigError(
          formatConfigError(error, "codexConfig.extractFailed"),
        );
      }
    } finally {
      if (extractSeq === extractOpSeqRef.current) {
        setIsExtracting(false);
      }
    }
  }, [
    codexConfig,
    formatConfigError,
    isTomlOpStale,
    persistCommonConfigSnippet,
    t,
  ]);

  const clearCommonConfigError = useCallback(() => {
    setCommonConfigError("");
  }, []);

  return {
    useCommonConfig,
    commonConfigSnippet,
    commonConfigError,
    isLoading,
    isExtracting,
    handleCommonConfigToggle,
    handleCommonConfigSnippetChange,
    handleExtract,
    clearCommonConfigError,
  };
}
