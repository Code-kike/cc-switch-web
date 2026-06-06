import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { proxyApi } from "@/lib/api/proxy";
import { toast } from "sonner";
import { useTranslation } from "react-i18next";
import type { GlobalProxyConfig, AppProxyConfig } from "@/types/proxy";
import { extractErrorMessage } from "@/utils/errorUtils";

// ========== 代理服务器控制 Hooks ==========
//
// 注意（M37）：代理状态/总开关/接管状态等查询的唯一来源是
// `@/hooks/useProxyStatus`（canonical）。本文件历史上重复实现了一整套
// `useProxyStatus` / `useIsProxyRunning` / `useStartProxyServer` 等 hook，
// 它们与 canonical 共用同样的 query key（如 `["proxyStatus"]` /
// `["proxyTakeoverStatus"]`）却带不同的轮询配置，是一处"双观察者各自为战"的
// 隐患，且在应用代码中零引用——已全部删除。接管状态请从
// `useProxyStatus()` 返回的 `takeoverStatus` 读取，不要再开第二个查询。
//
// 本文件仅保留仍被 ProxyPanel / AutoFailoverConfigPanel 引用的接管开关与
// 全局/应用级代理配置 hook。

/**
 * 设置应用接管状态
 */
export function useSetProxyTakeoverForApp() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ appType, enabled }: { appType: string; enabled: boolean }) =>
      proxyApi.setProxyTakeoverForApp(appType, enabled),
    onSuccess: () => {
      // 接管状态由 canonical `useProxyStatus` 的 `["proxyTakeoverStatus"]`
      // 查询持有，失效它即可让 ProxyPanel 立即反映新状态。
      queryClient.invalidateQueries({ queryKey: ["proxyTakeoverStatus"] });
    },
  });
}

// ========== v3+ 全局/应用级配置 Hooks ==========

/**
 * 获取全局代理配置
 */
export function useGlobalProxyConfig() {
  return useQuery({
    queryKey: ["globalProxyConfig"],
    queryFn: () => proxyApi.getGlobalProxyConfig(),
  });
}

/**
 * 更新全局代理配置
 */
export function useUpdateGlobalProxyConfig() {
  const queryClient = useQueryClient();
  const { t } = useTranslation();
  const formatError = (error: unknown) =>
    extractErrorMessage(error) || t("common.unknown");

  return useMutation({
    mutationFn: (config: GlobalProxyConfig) =>
      proxyApi.updateGlobalProxyConfig(config),
    onSuccess: () => {
      toast.success(t("proxy.settings.toast.saved"), { closeButton: true });
      queryClient.invalidateQueries({ queryKey: ["globalProxyConfig"] });
      queryClient.invalidateQueries({ queryKey: ["proxyStatus"] });
    },
    onError: (error: unknown) => {
      toast.error(
        t("proxy.settings.toast.saveFailed", { error: formatError(error) }),
      );
    },
  });
}

/**
 * 获取指定应用的代理配置
 */
export function useAppProxyConfig(appType: string) {
  return useQuery({
    queryKey: ["appProxyConfig", appType],
    queryFn: () => proxyApi.getProxyConfigForApp(appType),
    enabled: !!appType,
  });
}

/**
 * 更新指定应用的代理配置
 */
export function useUpdateAppProxyConfig() {
  const queryClient = useQueryClient();
  const { t } = useTranslation();
  const formatError = (error: unknown) =>
    extractErrorMessage(error) || t("common.unknown");

  return useMutation({
    mutationFn: (config: AppProxyConfig) =>
      proxyApi.updateProxyConfigForApp(config),
    onSuccess: (_, variables) => {
      toast.success(t("proxy.settings.toast.saved"), { closeButton: true });
      queryClient.invalidateQueries({
        queryKey: ["appProxyConfig", variables.appType],
      });
      queryClient.invalidateQueries({ queryKey: ["circuitBreakerConfig"] });
    },
    onError: (error: unknown) => {
      toast.error(
        t("proxy.settings.toast.saveFailed", { error: formatError(error) }),
      );
    },
  });
}
