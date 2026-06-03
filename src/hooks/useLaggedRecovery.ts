import { useEffect } from "react";
import { listen } from "@/lib/api/event-adapter";
import { useQueryClient } from "@tanstack/react-query";
import { isWebMode } from "@/lib/api/adapter";
import { usageKeys } from "@/lib/query/usage";
import { subscriptionKeys } from "@/lib/query/subscription";

/**
 * Web SSE 接收端积压恢复。
 *
 * 当后端的事件广播通道 lag（订阅者来不及消费、被 broadcast channel 丢弃）时，
 * `GET /api/events` 会发出一条 `lagged` SSE，`event-adapter` 把它映射成内部
 * 订阅键 `__lagged`。此时客户端缓存可能已经错过若干增量事件，唯一安全的做法
 * 是把受影响的查询命名空间全部 invalidate，让 React Query 重新拉取权威数据。
 *
 * 仅在 **Web 模式** 生效：桌面端走 Tauri 事件总线，不存在 SSE 积压，因此该 hook
 * 在 Tauri 下直接 no-op。事件依旧通过运行时中立的 `event-adapter` 订阅，离开
 * 时自动取消监听。
 */
export function useLaggedRecovery() {
  const queryClient = useQueryClient();

  useEffect(() => {
    // 桌面端没有 SSE 积压问题，直接跳过订阅
    if (!isWebMode()) return;

    let unlisten: (() => void) | undefined;
    let disposed = false;

    (async () => {
      const off = await listen("__lagged", () => {
        // 错过了增量事件，把主要查询命名空间全部 invalidate 触发重拉
        queryClient.invalidateQueries({ queryKey: usageKeys.all });
        queryClient.invalidateQueries({ queryKey: ["providers"] });
        queryClient.invalidateQueries({ queryKey: ["proxyStatus"] });
        queryClient.invalidateQueries({ queryKey: subscriptionKeys.all });
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
