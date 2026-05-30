import { useEffect } from "react";
import { listen } from "@/lib/api/event-adapter";
import { useQueryClient } from "@tanstack/react-query";
import { usageKeys } from "@/lib/query/usage";

/**
 * 监听后端 `usage-log-recorded` 事件，收到后立刻 invalidate 所有
 * UsageDashboard 相关查询，让用户无需等待 30s 轮询周期。
 *
 * 后端在 `proxy_request_logs` 写入新行时会 emit 该事件（200ms 防抖合并），
 * 来源覆盖代理日志、Claude/Codex/Gemini 会话同步、启动归档。
 *
 * 事件通过运行时中立的 `event-adapter` 订阅：桌面端走 Tauri 事件总线监听，
 * Web 端走 `GET /api/events` 的 SSE。**禁止**直接 import
 * `@tauri-apps/api/event`，否则 Web 模式拿不到事件。
 *
 * 该 hook 只挂在 UsageDashboard 上，避免在主界面其他位置无意义触发；
 * 离开页面时自动取消监听。
 */
export function useUsageEventBridge() {
  const queryClient = useQueryClient();

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let disposed = false;

    (async () => {
      const off = await listen("usage-log-recorded", () => {
        // invalidate 整个 usage 命名空间：summary / trends / providerStats /
        // modelStats / logs 全部跟着重拉
        queryClient.invalidateQueries({ queryKey: usageKeys.all });
      });

      if (disposed) {
        off();
      } else {
        unlisten = off;
      }
    })();

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [queryClient]);
}
