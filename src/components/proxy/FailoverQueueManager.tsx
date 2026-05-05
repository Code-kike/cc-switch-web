/**
 * 故障转移队列管理组件
 *
 * 允许用户管理代理模式下的故障转移队列，支持：
 * - 添加/移除供应商
 * - 队列顺序基于首页供应商列表的 sort_index
 */

import { useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import {
  Plus,
  Trash2,
  Loader2,
  Info,
  AlertTriangle,
  RotateCcw,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { Switch } from "@/components/ui/switch";
import { Alert, AlertDescription } from "@/components/ui/alert";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { cn } from "@/lib/utils";
import type { FailoverQueueItem } from "@/types/proxy";
import type { AppId } from "@/lib/api";
import {
  useFailoverQueue,
  useAvailableProvidersForFailover,
  useAddToFailoverQueue,
  useRemoveFromFailoverQueue,
  useAutoFailoverEnabled,
  useSetAutoFailoverEnabled,
  useProviderHealth,
  useResetCircuitBreaker,
} from "@/lib/query/failover";
import { extractErrorMessage } from "@/utils/errorUtils";

interface FailoverQueueManagerProps {
  appType: AppId;
  disabled?: boolean;
}

export function FailoverQueueManager({
  appType,
  disabled = false,
}: FailoverQueueManagerProps) {
  const { t } = useTranslation();
  const [selectedProviderId, setSelectedProviderId] = useState<string>("");

  const formatActionError = (
    error: unknown,
    key:
      | "proxy.failoverQueue.addFailed"
      | "proxy.failoverQueue.removeFailed"
      | "proxy.failoverQueue.loadFailed",
    defaultValue: string,
  ) => {
    const detail =
      extractErrorMessage(error) ||
      t("common.unknown", { defaultValue: "未知错误" });
    return t(key, {
      detail,
      defaultValue: `${defaultValue}: {{detail}}`,
    });
  };

  // 故障转移开关状态（每个应用独立）
  const { data: isFailoverEnabled = false } = useAutoFailoverEnabled(appType);
  const setFailoverEnabled = useSetAutoFailoverEnabled();

  // 查询数据
  const {
    data: queue,
    isLoading: isQueueLoading,
    error: queueError,
  } = useFailoverQueue(appType);
  const { data: availableProviders, isLoading: isProvidersLoading } =
    useAvailableProvidersForFailover(appType);

  // Mutations
  const addToQueue = useAddToFailoverQueue();
  const removeFromQueue = useRemoveFromFailoverQueue();

  // 切换故障转移开关
  const handleToggleFailover = (enabled: boolean) => {
    setFailoverEnabled.mutate({ appType, enabled });
  };

  // 添加供应商到队列
  const handleAddProvider = async () => {
    if (!selectedProviderId) return;

    try {
      await addToQueue.mutateAsync({
        appType,
        providerId: selectedProviderId,
      });
      setSelectedProviderId("");
      toast.success(
        t("proxy.failoverQueue.addSuccess", "已添加到故障转移队列"),
        { closeButton: true },
      );
    } catch (error) {
      toast.error(
        formatActionError(
          error,
          "proxy.failoverQueue.addFailed",
          "添加失败",
        ),
      );
    }
  };

  // 从队列移除供应商
  const handleRemoveProvider = async (providerId: string) => {
    try {
      await removeFromQueue.mutateAsync({ appType, providerId });
      toast.success(
        t("proxy.failoverQueue.removeSuccess", "已从故障转移队列移除"),
        { closeButton: true },
      );
    } catch (error) {
      toast.error(
        formatActionError(
          error,
          "proxy.failoverQueue.removeFailed",
          "移除失败",
        ),
      );
    }
  };

  if (isQueueLoading) {
    return (
      <div className="flex items-center justify-center p-8">
        <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
      </div>
    );
  }

  if (queueError) {
    return (
      <Alert variant="destructive">
        <AlertTriangle className="h-4 w-4" />
        <AlertDescription>
          {formatActionError(
            queueError,
            "proxy.failoverQueue.loadFailed",
            "加载故障转移队列失败",
          )}
        </AlertDescription>
      </Alert>
    );
  }

  return (
    <div className="space-y-4">
      {/* 自动故障转移开关 */}
      <div className="flex items-center justify-between p-4 rounded-lg bg-muted/50 border border-border/50">
        <div className="space-y-0.5">
          <div className="flex items-center gap-2">
            <span className="text-sm font-medium">
              {t("proxy.failover.autoSwitch", {
                defaultValue: "自动故障转移",
              })}
            </span>
            {isFailoverEnabled && (
              <span className="px-2 py-0.5 text-xs rounded-full bg-emerald-500/20 text-emerald-600 dark:text-emerald-400">
                {t("common.enabled", { defaultValue: "已开启" })}
              </span>
            )}
          </div>
          <p className="text-xs text-muted-foreground">
            {t("proxy.failover.autoSwitchDescription", {
              defaultValue:
                "开启后将立即切换到队列 P1，并在请求失败时自动切换到队列中的下一个供应商",
            })}
          </p>
        </div>
        <Switch
          checked={isFailoverEnabled}
          onCheckedChange={handleToggleFailover}
          disabled={disabled || setFailoverEnabled.isPending}
        />
      </div>

      {/* 说明信息 */}
      <Alert className="border-blue-500/40 bg-blue-500/10">
        <Info className="h-4 w-4" />
        <AlertDescription className="text-sm">
          {t(
            "proxy.failoverQueue.info",
            "队列顺序与首页供应商列表顺序一致。当请求失败时，系统会按顺序依次尝试队列中的供应商。",
          )}
        </AlertDescription>
      </Alert>

      {/* 添加供应商 */}
      <div className="flex items-center gap-2">
        <Select
          value={selectedProviderId}
          onValueChange={setSelectedProviderId}
          disabled={disabled || isProvidersLoading}
        >
          <SelectTrigger className="flex-1">
            <SelectValue
              placeholder={t(
                "proxy.failoverQueue.selectProvider",
                "选择供应商添加到队列",
              )}
            />
          </SelectTrigger>
          <SelectContent>
            {availableProviders?.map((provider) => (
              <SelectItem key={provider.id} value={provider.id}>
                {provider.name}
                {provider.notes && (
                  <span className="ml-1 text-xs text-muted-foreground">
                    ({provider.notes})
                  </span>
                )}
              </SelectItem>
            ))}
            {(!availableProviders || availableProviders.length === 0) && (
              <div className="px-2 py-4 text-center text-sm text-muted-foreground">
                {t(
                  "proxy.failoverQueue.noAvailableProviders",
                  "没有可添加的供应商",
                )}
              </div>
            )}
          </SelectContent>
        </Select>
        <Button
          onClick={handleAddProvider}
          disabled={disabled || !selectedProviderId || addToQueue.isPending}
          size="icon"
          variant="outline"
          aria-label={t("common.add", "添加")}
        >
          {addToQueue.isPending ? (
            <Loader2 className="h-4 w-4 animate-spin" />
          ) : (
            <Plus className="h-4 w-4" />
          )}
        </Button>
      </div>

      {/* 队列列表 */}
      {!queue || queue.length === 0 ? (
        <div className="rounded-lg border border-dashed border-muted-foreground/40 p-8 text-center">
          <p className="text-sm text-muted-foreground">
            {t(
              "proxy.failoverQueue.empty",
              "故障转移队列为空。添加供应商以启用自动故障转移。",
            )}
          </p>
        </div>
      ) : (
        <div className="space-y-2">
          {queue.map((item, index) => (
            <QueueItem
              key={item.providerId}
              item={item}
              appType={appType}
              index={index}
              disabled={disabled}
              onRemove={handleRemoveProvider}
              isRemoving={removeFromQueue.isPending}
            />
          ))}
        </div>
      )}

      {/* 队列说明 */}
      {queue && queue.length > 0 && (
        <p className="text-xs text-muted-foreground">
          {t(
            "proxy.failoverQueue.orderHint",
            "队列顺序与首页供应商列表顺序一致，可在首页拖拽调整顺序。",
          )}
        </p>
      )}
    </div>
  );
}

interface QueueItemProps {
  item: FailoverQueueItem;
  appType: AppId;
  index: number;
  disabled: boolean;
  onRemove: (providerId: string) => void;
  isRemoving: boolean;
}

function QueueItem({
  item,
  appType,
  index,
  disabled,
  onRemove,
  isRemoving,
}: QueueItemProps) {
  const { t } = useTranslation();
  const { data: health, error: healthError, isLoading: isHealthLoading } =
    useProviderHealth(item.providerId, appType, !disabled);
  const resetCircuitBreaker = useResetCircuitBreaker();

  const handleResetCircuit = async () => {
    try {
      await resetCircuitBreaker.mutateAsync({
        providerId: item.providerId,
        appType,
      });
      toast.success(
        t("proxy.failoverQueue.resetSuccess", {
          defaultValue: "Circuit breaker has been reset",
        }),
        { closeButton: true },
      );
    } catch (error) {
      toast.error(
        t("proxy.failoverQueue.resetFailed", {
          detail: extractErrorMessage(error),
          defaultValue: "Failed to reset circuit breaker: {{detail}}",
        }),
      );
    }
  };

  const healthLabel = (() => {
    if (disabled) {
      return null;
    }

    if (healthError) {
      return t("proxy.failoverQueue.healthUnavailable", {
        defaultValue: "Unavailable",
      });
    }

    if (isHealthLoading) {
      return t("common.loading", { defaultValue: "Loading" });
    }

    if (!health) {
      return t("proxy.failoverQueue.healthUnknown", {
        defaultValue: "Unknown",
      });
    }

    if (!health.is_healthy) {
      return t("proxy.failoverQueue.healthTripped", {
        defaultValue: "Tripped",
      });
    }

    if (health.consecutive_failures > 0) {
      return t("proxy.failoverQueue.healthWarning", {
        defaultValue: "Warning",
      });
    }

    return t("proxy.failoverQueue.healthHealthy", {
      defaultValue: "Healthy",
    });
  })();

  const healthToneClass = (() => {
    if (disabled) {
      return "bg-muted text-muted-foreground";
    }
    if (healthError) {
      return "bg-muted text-muted-foreground";
    }
    if (!health) {
      return "bg-muted text-muted-foreground";
    }
    if (!health.is_healthy) {
      return "bg-destructive/10 text-destructive";
    }
    if (health.consecutive_failures > 0) {
      return "bg-amber-500/15 text-amber-700 dark:text-amber-400";
    }
    return "bg-emerald-500/15 text-emerald-700 dark:text-emerald-400";
  })();

  return (
    <div
      className={cn(
        "flex items-center gap-3 rounded-lg border bg-card p-3 transition-colors",
      )}
    >
      {/* 序号 */}
      <div className="flex h-6 w-6 items-center justify-center rounded-full bg-muted text-xs font-medium">
        {index + 1}
      </div>

      {/* 供应商名称 */}
      <div className="flex-1 min-w-0">
        <span className="text-sm font-medium truncate block">
          {item.providerName}
          {item.providerNotes && (
            <span className="ml-1 text-xs text-muted-foreground">
              ({item.providerNotes})
            </span>
          )}
        </span>
        {healthLabel ? (
          <div className="mt-1 flex flex-wrap items-center gap-2 text-xs">
            <span
              className={cn(
                "inline-flex items-center rounded-full px-2 py-0.5 font-medium",
                healthToneClass,
              )}
            >
              {healthLabel}
            </span>
            {health && health.consecutive_failures > 0 ? (
              <span className="text-muted-foreground">
                {t("proxy.failoverQueue.failureCount", {
                  count: health.consecutive_failures,
                  defaultValue: "{{count}} failures",
                })}
              </span>
            ) : null}
          </div>
        ) : null}
        {!disabled && health?.last_error ? (
          <p className="mt-1 text-xs text-muted-foreground truncate">
            {t("proxy.failoverQueue.lastError", {
              detail: health.last_error,
              defaultValue: "Last error: {{detail}}",
            })}
          </p>
        ) : null}
      </div>

      <div className="flex items-center gap-1">
        <Button
          variant="ghost"
          size="sm"
          className="h-8 px-2 text-xs text-muted-foreground"
          onClick={handleResetCircuit}
          disabled={disabled || resetCircuitBreaker.isPending}
          title={t("proxy.failoverQueue.resetCircuit", {
            defaultValue: "Reset Circuit",
          })}
        >
          {resetCircuitBreaker.isPending ? (
            <Loader2 className="h-3.5 w-3.5 animate-spin" />
          ) : (
            <RotateCcw className="h-3.5 w-3.5" />
          )}
          <span className="ml-1">
            {t("proxy.failoverQueue.resetCircuit", {
              defaultValue: "Reset Circuit",
            })}
          </span>
        </Button>

        {/* 删除按钮 */}
        <Button
          variant="ghost"
          size="icon"
          className="h-8 w-8 text-muted-foreground hover:text-destructive"
          onClick={() => onRemove(item.providerId)}
          disabled={disabled || isRemoving}
          aria-label={t("common.delete", "删除")}
        >
          {isRemoving ? (
            <Loader2 className="h-4 w-4 animate-spin" />
          ) : (
            <Trash2 className="h-4 w-4" />
          )}
        </Button>
      </div>
    </div>
  );
}
