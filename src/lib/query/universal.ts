import { useQuery } from "@tanstack/react-query";
import { universalProvidersApi } from "@/lib/api";
import type { UniversalProvidersMap } from "@/types";

/**
 * 统一供应商（Universal Provider）查询键。
 *
 * 单一来源：UniversalProviderPanel 以及 App 层的 `universal-provider-synced`
 * 事件失效都应指向 `universalProviderKeys.all`，避免面板自行 useState/手动
 * fetch 导致外部同步后 UI 不刷新（M42）。
 */
export const universalProviderKeys = {
  all: ["universalProviders"] as const,
};

/**
 * 获取所有统一供应商
 */
export const useUniversalProvidersQuery = () =>
  useQuery<UniversalProvidersMap>({
    queryKey: universalProviderKeys.all,
    queryFn: async () => (await universalProvidersApi.getAll()) ?? {},
  });
