import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { usageApi } from "@/lib/api/usage";
import { resolveUsageRange } from "@/lib/usageRange";
import type { LogFilters, UsageRangeSelection } from "@/types/usage";

const DEFAULT_REFETCH_INTERVAL_MS = 30000;

type UsageQueryOptions = {
  refetchInterval?: number | false;
  refetchIntervalInBackground?: boolean;
};

type RequestLogsQueryArgs = {
  filters: LogFilters;
  range: UsageRangeSelection;
  page?: number;
  pageSize?: number;
  options?: UsageQueryOptions;
};

type RequestLogsKey = {
  preset: UsageRangeSelection["preset"];
  customStartDate?: number;
  customEndDate?: number;
  appType?: string;
  providerName?: string;
  model?: string;
  statusCode?: number;
};

// Query keys
export const usageKeys = {
  all: ["usage"] as const,
  summary: (
    preset: UsageRangeSelection["preset"],
    customStartDate: number | undefined,
    customEndDate: number | undefined,
    appType?: string,
  ) =>
    [
      ...usageKeys.all,
      "summary",
      preset,
      customStartDate ?? 0,
      customEndDate ?? 0,
      appType ?? "all",
    ] as const,
  summaryByApp: (
    preset: UsageRangeSelection["preset"],
    customStartDate: number | undefined,
    customEndDate: number | undefined,
  ) =>
    [
      ...usageKeys.all,
      "summary-by-app",
      preset,
      customStartDate ?? 0,
      customEndDate ?? 0,
    ] as const,
  trends: (
    preset: UsageRangeSelection["preset"],
    customStartDate: number | undefined,
    customEndDate: number | undefined,
    appType?: string,
  ) =>
    [
      ...usageKeys.all,
      "trends",
      preset,
      customStartDate ?? 0,
      customEndDate ?? 0,
      appType ?? "all",
    ] as const,
  providerStats: (
    preset: UsageRangeSelection["preset"],
    customStartDate: number | undefined,
    customEndDate: number | undefined,
    appType?: string,
  ) =>
    [
      ...usageKeys.all,
      "provider-stats",
      preset,
      customStartDate ?? 0,
      customEndDate ?? 0,
      appType ?? "all",
    ] as const,
  modelStats: (
    preset: UsageRangeSelection["preset"],
    customStartDate: number | undefined,
    customEndDate: number | undefined,
    appType?: string,
  ) =>
    [
      ...usageKeys.all,
      "model-stats",
      preset,
      customStartDate ?? 0,
      customEndDate ?? 0,
      appType ?? "all",
    ] as const,
  logs: (key: RequestLogsKey, page: number, pageSize: number) =>
    [
      ...usageKeys.all,
      "logs",
      key.preset,
      key.customStartDate ?? 0,
      key.customEndDate ?? 0,
      key.appType ?? "",
      key.providerName ?? "",
      key.model ?? "",
      key.statusCode ?? -1,
      page,
      pageSize,
    ] as const,
  detail: (requestId: string) =>
    [...usageKeys.all, "detail", requestId] as const,
  pricing: () => [...usageKeys.all, "pricing"] as const,
  limits: (providerId: string, appType: string) =>
    [...usageKeys.all, "limits", providerId, appType] as const,
  script: (providerId: string, appType: string) =>
    [...usageKeys.all, providerId, appType] as const,
};

/**
 * `usage` 命名空间中由 `proxy_request_logs` 表派生的"仪表盘聚合"分区标识。
 *
 * 这些是 UsageDashboard 直接渲染、且会随新日志行写入而变化的查询；它们的
 * query key 形如 `["usage", <section>, ...]`，第二段是下列固定标识之一。
 */
export const USAGE_LOG_DERIVED_SECTIONS = [
  "summary",
  "summary-by-app",
  "trends",
  "provider-stats",
  "model-stats",
  "logs",
] as const;

/**
 * 判断某个 query key 是否属于"日志派生的仪表盘聚合"分区（M38）。
 *
 * `usage-log-recorded` 事件（`proxy_request_logs` 写入新行）的 payload 为空
 * （见 `src-tauri/src/usage_events.rs`），无法按 provider/app 精准定位，但可以
 * 排除明显与该事件无关、重拉纯属浪费的查询：
 * - 按供应商的脚本查询 `usageKeys.script` = `["usage", providerId, appType]`，
 *   其数据来自外部计费脚本（`usageApi.query` 发起的外部 API 调用），与
 *   `proxy_request_logs` 无关；用 `usageKeys.all`（`["usage"]`）做前缀失效会把
 *   它们一并重拉，触发昂贵且无意义的外部请求。
 * - `pricing` / `limits` / `detail` 同样不随新增日志行变化。
 *
 * 因此该事件只 invalidate 本函数命中的聚合查询。注意：`__lagged` 兜底恢复
 * （见 `useLaggedRecovery`）仍会 invalidate 整个命名空间，不受此约束。
 */
export function isUsageLogDerivedKey(queryKey: readonly unknown[]): boolean {
  return (
    queryKey[0] === usageKeys.all[0] &&
    typeof queryKey[1] === "string" &&
    (USAGE_LOG_DERIVED_SECTIONS as readonly string[]).includes(queryKey[1])
  );
}

// Hooks
export function useUsageSummary(
  range: UsageRangeSelection,
  appType?: string,
  options?: UsageQueryOptions,
) {
  const effectiveAppType = appType === "all" ? undefined : appType;
  return useQuery({
    queryKey: usageKeys.summary(
      range.preset,
      range.customStartDate,
      range.customEndDate,
      appType,
    ),
    queryFn: () => {
      const { startDate, endDate } = resolveUsageRange(range);
      return usageApi.getUsageSummary(startDate, endDate, effectiveAppType);
    },
    refetchInterval: options?.refetchInterval ?? DEFAULT_REFETCH_INTERVAL_MS,
    refetchIntervalInBackground: options?.refetchIntervalInBackground ?? false,
  });
}

export function useUsageSummaryByApp(
  range: UsageRangeSelection,
  options?: UsageQueryOptions,
) {
  return useQuery({
    queryKey: usageKeys.summaryByApp(
      range.preset,
      range.customStartDate,
      range.customEndDate,
    ),
    queryFn: () => {
      const { startDate, endDate } = resolveUsageRange(range);
      return usageApi.getUsageSummaryByApp(startDate, endDate);
    },
    refetchInterval: options?.refetchInterval ?? DEFAULT_REFETCH_INTERVAL_MS,
    refetchIntervalInBackground: options?.refetchIntervalInBackground ?? false,
  });
}

export function useUsageTrends(
  range: UsageRangeSelection,
  appType?: string,
  options?: UsageQueryOptions,
) {
  const effectiveAppType = appType === "all" ? undefined : appType;
  return useQuery({
    queryKey: usageKeys.trends(
      range.preset,
      range.customStartDate,
      range.customEndDate,
      appType,
    ),
    queryFn: () => {
      const { startDate, endDate } = resolveUsageRange(range);
      return usageApi.getUsageTrends(startDate, endDate, effectiveAppType);
    },
    refetchInterval: options?.refetchInterval ?? DEFAULT_REFETCH_INTERVAL_MS,
    refetchIntervalInBackground: options?.refetchIntervalInBackground ?? false,
  });
}

export function useProviderStats(
  range: UsageRangeSelection,
  appType?: string,
  options?: UsageQueryOptions,
) {
  const effectiveAppType = appType === "all" ? undefined : appType;
  return useQuery({
    queryKey: usageKeys.providerStats(
      range.preset,
      range.customStartDate,
      range.customEndDate,
      appType,
    ),
    queryFn: () => {
      const { startDate, endDate } = resolveUsageRange(range);
      return usageApi.getProviderStats(startDate, endDate, effectiveAppType);
    },
    refetchInterval: options?.refetchInterval ?? DEFAULT_REFETCH_INTERVAL_MS,
    refetchIntervalInBackground: options?.refetchIntervalInBackground ?? false,
  });
}

export function useModelStats(
  range: UsageRangeSelection,
  appType?: string,
  options?: UsageQueryOptions,
) {
  const effectiveAppType = appType === "all" ? undefined : appType;
  return useQuery({
    queryKey: usageKeys.modelStats(
      range.preset,
      range.customStartDate,
      range.customEndDate,
      appType,
    ),
    queryFn: () => {
      const { startDate, endDate } = resolveUsageRange(range);
      return usageApi.getModelStats(startDate, endDate, effectiveAppType);
    },
    refetchInterval: options?.refetchInterval ?? DEFAULT_REFETCH_INTERVAL_MS,
    refetchIntervalInBackground: options?.refetchIntervalInBackground ?? false,
  });
}

export function useRequestLogs({
  filters,
  range,
  page = 0,
  pageSize = 20,
  options,
}: RequestLogsQueryArgs) {
  const key: RequestLogsKey = {
    preset: range.preset,
    customStartDate: range.customStartDate,
    customEndDate: range.customEndDate,
    appType: filters.appType,
    providerName: filters.providerName,
    model: filters.model,
    statusCode: filters.statusCode,
  };

  return useQuery({
    queryKey: usageKeys.logs(key, page, pageSize),
    queryFn: () => {
      const effectiveFilters = { ...filters, ...resolveUsageRange(range) };
      return usageApi.getRequestLogs(effectiveFilters, page, pageSize);
    },
    refetchInterval: options?.refetchInterval ?? DEFAULT_REFETCH_INTERVAL_MS, // 每30秒自动刷新
    refetchIntervalInBackground: options?.refetchIntervalInBackground ?? false,
  });
}

export function useRequestDetail(requestId: string) {
  return useQuery({
    queryKey: usageKeys.detail(requestId),
    queryFn: () => usageApi.getRequestDetail(requestId),
    enabled: !!requestId,
  });
}

export function useModelPricing() {
  return useQuery({
    queryKey: usageKeys.pricing(),
    queryFn: usageApi.getModelPricing,
  });
}

export function useProviderLimits(
  providerId: string,
  appType: string,
  enabled: boolean = true,
) {
  return useQuery({
    queryKey: usageKeys.limits(providerId, appType),
    queryFn: () => usageApi.checkProviderLimits(providerId, appType),
    enabled: enabled && !!providerId && !!appType,
  });
}

export function useUpdateModelPricing() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (params: {
      modelId: string;
      displayName: string;
      inputCost: string;
      outputCost: string;
      cacheReadCost: string;
      cacheCreationCost: string;
    }) =>
      usageApi.updateModelPricing(
        params.modelId,
        params.displayName,
        params.inputCost,
        params.outputCost,
        params.cacheReadCost,
        params.cacheCreationCost,
      ),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: usageKeys.pricing() });
    },
  });
}

export function useDeleteModelPricing() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (modelId: string) => usageApi.deleteModelPricing(modelId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: usageKeys.pricing() });
    },
  });
}
