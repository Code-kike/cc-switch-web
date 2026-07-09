import {
  useQuery,
  type UseQueryResult,
  keepPreviousData,
} from "@tanstack/react-query";
import { useRef } from "react";
import {
  providersApi,
  settingsApi,
  usageApi,
  sessionsApi,
  type AppId,
} from "@/lib/api";
import type {
  Provider,
  Settings,
  UsageResult,
  SessionMeta,
  SessionMessage,
} from "@/types";
import { usageKeys } from "@/lib/query/usage";
import { extractErrorMessage } from "@/utils/errorUtils";

const sortProviders = (
  providers: Record<string, Provider>,
): Record<string, Provider> => {
  const sortedEntries = Object.values(providers)
    .sort((a, b) => {
      const indexA = a.sortIndex ?? Number.MAX_SAFE_INTEGER;
      const indexB = b.sortIndex ?? Number.MAX_SAFE_INTEGER;
      if (indexA !== indexB) {
        return indexA - indexB;
      }

      const timeA = a.createdAt ?? 0;
      const timeB = b.createdAt ?? 0;
      if (timeA === timeB) {
        return a.name.localeCompare(b.name, "zh-CN");
      }
      return timeA - timeB;
    })
    .map((provider) => [provider.id, provider] as const);

  return Object.fromEntries(sortedEntries);
};

export interface ProvidersQueryData {
  providers: Record<string, Provider>;
  currentProviderId: string;
}

export interface UseProvidersQueryOptions {
  isProxyRunning?: boolean; // 代理服务是否运行中
}

export const useProvidersQuery = (
  appId: AppId,
  options?: UseProvidersQueryOptions,
): UseQueryResult<ProvidersQueryData> => {
  const { isProxyRunning = false } = options || {};

  return useQuery({
    queryKey: ["providers", appId],
    placeholderData: keepPreviousData,
    // 当代理服务运行时，每 10 秒刷新一次供应商列表
    // 这样可以自动反映后端熔断器自动禁用代理目标的变更
    refetchInterval: isProxyRunning ? 10000 : false,
    queryFn: async () => {
      let providers: Record<string, Provider> = {};
      let currentProviderId = "";

      try {
        providers = await providersApi.getAll(appId);
      } catch (error) {
        console.error("获取供应商列表失败:", error);
      }

      try {
        currentProviderId = await providersApi.getCurrent(appId);
      } catch (error) {
        console.error("获取当前供应商失败:", error);
      }

      return {
        providers: sortProviders(providers),
        currentProviderId,
      };
    },
  });
};

export const useSettingsQuery = (): UseQueryResult<Settings> => {
  return useQuery({
    queryKey: ["settings"],
    queryFn: async () => settingsApi.get(),
  });
};

export interface UseUsageQueryOptions {
  enabled?: boolean;
  autoQueryInterval?: number; // 自动查询间隔（分钟），0 表示禁用
}

export interface UsageLikeResult {
  success: boolean;
  error?: string | null;
}

export interface LastGoodSnapshot<T> {
  data: T;
  at: number;
}

export type LastGoodUsage = LastGoodSnapshot<UsageResult>;

export const KEEP_LAST_GOOD_MS = 10 * 60 * 1000;

export function isTransientUsageError(result: UsageLikeResult): boolean {
  if (result.success) return false;
  const error = result.error?.toLowerCase() ?? "";
  if (!error) return false;

  if (
    error.includes("network error") ||
    error.includes("request failed") ||
    error.includes("请求失败") ||
    error.includes("failed to read response") ||
    error.includes("读取响应失败")
  ) {
    return true;
  }

  const httpMatch = error.match(/http\s+(\d{3})/);
  if (httpMatch) {
    const status = Number(httpMatch[1]);
    return (status >= 500 && status <= 599) || status === 429;
  }

  return false;
}

export interface ResolveDisplayUsageOptions {
  rejected?: boolean;
  keepMs?: number;
}

export function resolveDisplayUsage<T extends UsageLikeResult>(
  raw: T | undefined,
  dataUpdatedAt: number,
  prevLastGood: LastGoodSnapshot<T> | null,
  now: number,
  options: ResolveDisplayUsageOptions = {},
): {
  data: T | undefined;
  lastQueriedAt: number | null;
  lastGood: LastGoodSnapshot<T> | null;
} {
  const { rejected = false, keepMs = KEEP_LAST_GOOD_MS } = options;

  if (rejected && raw?.success) {
    const lastGood = { data: raw, at: dataUpdatedAt || now };
    if (now - lastGood.at < keepMs) {
      return { data: raw, lastQueriedAt: lastGood.at, lastGood };
    }
    return { data: undefined, lastQueriedAt: lastGood.at, lastGood };
  }

  let lastGood = prevLastGood;

  if (raw?.success) {
    const snapshot = { data: raw, at: dataUpdatedAt || now };
    return { data: raw, lastQueriedAt: snapshot.at, lastGood: snapshot };
  }

  if (!raw) {
    return {
      data: undefined,
      lastQueriedAt: lastGood?.at ?? null,
      lastGood,
    };
  }

  if (isTransientUsageError(raw) && lastGood && now - lastGood.at < keepMs) {
    return {
      data: lastGood.data,
      lastQueriedAt: lastGood.at,
      lastGood,
    };
  }

  const shouldClearLastGood = !isTransientUsageError(raw);
  return {
    data: raw,
    lastQueriedAt: dataUpdatedAt || now,
    lastGood: shouldClearLastGood ? null : lastGood,
  };
}

export const useUsageQuery = (
  providerId: string,
  appId: AppId,
  options?: UseUsageQueryOptions,
) => {
  const { enabled = true, autoQueryInterval = 0 } = options || {};

  // 计算 staleTime：如果有自动刷新间隔，使用该间隔；否则默认 5 分钟
  // 这样可以避免切换 app 页面时重复触发查询
  const staleTime =
    autoQueryInterval > 0
      ? autoQueryInterval * 60 * 1000 // 与刷新间隔保持一致
      : 5 * 60 * 1000; // 默认 5 分钟

  const lastGoodRef = useRef<{
    key: string;
    snap: LastGoodSnapshot<UsageResult> | null;
  }>({ key: "", snap: null });
  const scopeKey = `${appId}:${providerId}`;
  if (lastGoodRef.current.key !== scopeKey) {
    lastGoodRef.current = { key: scopeKey, snap: null };
  }

  const query = useQuery<UsageResult>({
    queryKey: usageKeys.script(providerId, appId),
    queryFn: async () => usageApi.query(providerId, appId),
    enabled: enabled && !!providerId,
    refetchInterval:
      autoQueryInterval > 0
        ? Math.max(autoQueryInterval, 1) * 60 * 1000 // 最小1分钟
        : false,
    refetchIntervalInBackground: true, // 后台也继续定时查询
    refetchOnWindowFocus: false,
    retry: 1,
    retryDelay: 1500,
    staleTime, // 使用动态计算的缓存时间
    gcTime: 10 * 60 * 1000, // 缓存保留 10 分钟（组件卸载后）
  });
  const { data, lastQueriedAt, lastGood } = resolveDisplayUsage(
    query.data,
    query.dataUpdatedAt,
    lastGoodRef.current.snap,
    Date.now(),
    { rejected: query.isError },
  );
  lastGoodRef.current.snap = lastGood;

  return {
    ...query,
    data:
      data ??
      (query.isError
        ? {
            success: false,
            error: extractErrorMessage(query.error) || undefined,
          }
        : undefined),
    lastQueriedAt,
  };
};

export const useSessionsQuery = () => {
  return useQuery<SessionMeta[]>({
    queryKey: ["sessions"],
    queryFn: async () => sessionsApi.list(),
    staleTime: 30 * 1000,
  });
};

export const useSessionMessagesQuery = (
  providerId?: string,
  sourcePath?: string,
) => {
  return useQuery<SessionMessage[]>({
    queryKey: ["sessionMessages", providerId, sourcePath],
    queryFn: async () => sessionsApi.getMessages(providerId!, sourcePath!),
    enabled: Boolean(providerId && sourcePath),
    staleTime: 30 * 1000,
  });
};
