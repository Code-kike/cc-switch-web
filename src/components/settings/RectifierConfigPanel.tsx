import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { Switch } from "@/components/ui/switch";
import { Label } from "@/components/ui/label";
import { Alert, AlertDescription } from "@/components/ui/alert";
import {
  settingsApi,
  type RectifierConfig,
  type OptimizerConfig,
} from "@/lib/api/settings";
import { extractErrorMessage } from "@/utils/errorUtils";

export function RectifierConfigPanel() {
  const { t } = useTranslation();
  const [config, setConfig] = useState<RectifierConfig>({
    enabled: true,
    requestThinkingSignature: true,
    requestThinkingBudget: true,
  });
  const [optimizerConfig, setOptimizerConfig] = useState<OptimizerConfig>({
    enabled: false,
    thinkingOptimizer: true,
    cacheInjection: true,
    cacheTtl: "1h",
  });
  const [isLoading, setIsLoading] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;

    async function loadConfig() {
      setIsLoading(true);
      setLoadError(null);

      const [rectifierResult, optimizerResult] = await Promise.allSettled([
        settingsApi.getRectifierConfig(),
        settingsApi.getOptimizerConfig(),
      ]);

      if (cancelled) return;

      const errors: string[] = [];

      if (rectifierResult.status === "fulfilled") {
        setConfig(rectifierResult.value);
      } else {
        console.error("Failed to load rectifier config:", rectifierResult.reason);
        const detail =
          extractErrorMessage(rectifierResult.reason) || t("common.unknown");
        errors.push(
          t("settings.advanced.rectifier.loadFailed", {
            error: detail,
          }),
        );
      }

      if (optimizerResult.status === "fulfilled") {
        setOptimizerConfig(optimizerResult.value);
      } else {
        console.error("Failed to load optimizer config:", optimizerResult.reason);
        const detail =
          extractErrorMessage(optimizerResult.reason) || t("common.unknown");
        errors.push(
          t("settings.advanced.optimizer.loadFailed", {
            error: detail,
          }),
        );
      }

      setLoadError(errors.length > 0 ? errors.join("\n") : null);
      setIsLoading(false);
    }

    void loadConfig();

    return () => {
      cancelled = true;
    };
  }, [t]);

  const handleChange = async (updates: Partial<RectifierConfig>) => {
    const previousConfig = config;
    const newConfig = { ...previousConfig, ...updates };
    setConfig(newConfig);
    try {
      await settingsApi.setRectifierConfig(newConfig);
    } catch (e) {
      console.error("Failed to save rectifier config:", e);
      const detail = extractErrorMessage(e) || t("common.unknown");
      toast.error(
        t("settings.advanced.rectifier.saveFailed", {
          error: detail,
        }),
      );
      setConfig(previousConfig);
    }
  };

  const handleOptimizerChange = async (updates: Partial<OptimizerConfig>) => {
    const previousConfig = optimizerConfig;
    const newConfig = { ...previousConfig, ...updates };
    setOptimizerConfig(newConfig);
    try {
      await settingsApi.setOptimizerConfig(newConfig);
    } catch (e) {
      console.error("Failed to save optimizer config:", e);
      const detail = extractErrorMessage(e) || t("common.unknown");
      toast.error(
        t("settings.advanced.optimizer.saveFailed", {
          error: detail,
        }),
      );
      setOptimizerConfig(previousConfig);
    }
  };

  if (isLoading) return null;

  if (loadError) {
    return (
      <Alert variant="destructive">
        <AlertDescription className="whitespace-pre-line">
          {loadError}
        </AlertDescription>
      </Alert>
    );
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div className="space-y-0.5">
          <Label>{t("settings.advanced.rectifier.enabled")}</Label>
          <p className="text-xs text-muted-foreground">
            {t("settings.advanced.rectifier.enabledDescription")}
          </p>
        </div>
        <Switch
          checked={config.enabled}
          onCheckedChange={(checked) => handleChange({ enabled: checked })}
        />
      </div>

      <div className="space-y-4">
        <h4 className="text-sm font-medium text-muted-foreground">
          {t("settings.advanced.rectifier.requestGroup")}
        </h4>
        <div className="flex items-center justify-between pl-4">
          <div className="space-y-0.5">
            <Label>{t("settings.advanced.rectifier.thinkingSignature")}</Label>
            <p className="text-xs text-muted-foreground">
              {t("settings.advanced.rectifier.thinkingSignatureDescription")}
            </p>
          </div>
          <Switch
            checked={config.requestThinkingSignature}
            disabled={!config.enabled}
            onCheckedChange={(checked) =>
              handleChange({ requestThinkingSignature: checked })
            }
          />
        </div>
        <div className="flex items-center justify-between pl-4">
          <div className="space-y-0.5">
            <Label>{t("settings.advanced.rectifier.thinkingBudget")}</Label>
            <p className="text-xs text-muted-foreground">
              {t("settings.advanced.rectifier.thinkingBudgetDescription")}
            </p>
          </div>
          <Switch
            checked={config.requestThinkingBudget}
            disabled={!config.enabled}
            onCheckedChange={(checked) =>
              handleChange({ requestThinkingBudget: checked })
            }
          />
        </div>
      </div>

      <div className="border-t pt-6 mt-6">
        <div className="space-y-1 mb-4">
          <h3 className="text-sm font-medium">
            {t("settings.advanced.optimizer.title")}
          </h3>
          <p className="text-xs text-muted-foreground">
            {t("settings.advanced.optimizer.description")}
          </p>
        </div>

        <div className="space-y-4">
          <div className="flex items-center justify-between">
            <div className="space-y-0.5">
              <Label>{t("settings.advanced.optimizer.enabled")}</Label>
            </div>
            <Switch
              checked={optimizerConfig.enabled}
              onCheckedChange={(checked) =>
                handleOptimizerChange({ enabled: checked })
              }
            />
          </div>

          <div className="space-y-4 pl-4">
            <div className="flex items-center justify-between">
              <div className="space-y-0.5">
                <Label>
                  {t("settings.advanced.optimizer.thinkingOptimizer")}
                </Label>
                <p className="text-xs text-muted-foreground">
                  {t(
                    "settings.advanced.optimizer.thinkingOptimizerDescription",
                  )}
                </p>
              </div>
              <Switch
                checked={optimizerConfig.thinkingOptimizer}
                disabled={!optimizerConfig.enabled}
                onCheckedChange={(checked) =>
                  handleOptimizerChange({ thinkingOptimizer: checked })
                }
              />
            </div>

            <div className="flex items-center justify-between">
              <div className="space-y-0.5">
                <Label>{t("settings.advanced.optimizer.cacheInjection")}</Label>
                <p className="text-xs text-muted-foreground">
                  {t("settings.advanced.optimizer.cacheInjectionDescription")}
                </p>
              </div>
              <Switch
                checked={optimizerConfig.cacheInjection}
                disabled={!optimizerConfig.enabled}
                onCheckedChange={(checked) =>
                  handleOptimizerChange({ cacheInjection: checked })
                }
              />
            </div>

            {optimizerConfig.cacheInjection && (
              <div className="flex items-center justify-between">
                <div className="space-y-0.5">
                  <Label>{t("settings.advanced.optimizer.cacheTtl")}</Label>
                </div>
                <select
                  className="h-9 rounded-md border border-input bg-background px-3 text-sm"
                  value={optimizerConfig.cacheTtl}
                  disabled={
                    !optimizerConfig.enabled || !optimizerConfig.cacheInjection
                  }
                  onChange={(e) =>
                    handleOptimizerChange({ cacheTtl: e.target.value })
                  }
                >
                  <option value="5m">
                    {t("settings.advanced.optimizer.cacheTtl5m")}
                  </option>
                  <option value="1h">
                    {t("settings.advanced.optimizer.cacheTtl1h")}
                  </option>
                </select>
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
