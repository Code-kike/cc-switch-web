import { useEffect } from "react";
import { listen } from "@/lib/api/event-adapter";
import { useQueryClient } from "@tanstack/react-query";
import { isUsageLogDerivedKey } from "@/lib/query/usage";

/**
 * 监听后端 `usage-log-recorded` 事件，收到后立刻 invalidate UsageDashboard 的
 * 聚合查询，让用户无需等待 30s 轮询周期。
 *
 * 后端在 `proxy_request_logs` 表写入新行时会 emit 该事件（200ms 防抖合并），
 * 来源覆盖代理日志、Claude/Codex/Gemini 会话同步、启动归档。该事件 payload 为空
 * （见 `src-tauri/src/usage_events.rs`），无法携带具体的 provider/app。
 *
 * **失效范围（M38）**：只 invalidate 由 `proxy_request_logs` 派生的聚合查询
 * （summary / summary-by-app / trends / provider-stats / model-stats / logs，见
 * `isUsageLogDerivedKey`），**不**触碰按供应商的脚本查询
 * `["usage", providerId, appType]`。后者的数据来自外部计费脚本，与日志行无关，
 * 若一并重拉会触发昂贵且无意义的外部 API 调用。此前用 `usageKeys.all` 前缀失效
 * 会把整个 `usage` 命名空间（含脚本查询）全部重拉，每 200ms 一次事件即放大成
 * 大量冗余请求。
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
        // 仅 invalidate 由 proxy_request_logs 派生的仪表盘聚合查询，
        // 跳过按供应商的脚本查询（外部计费调用，与日志行无关）。
        queryClient.invalidateQueries({
          predicate: (query) => isUsageLogDerivedKey(query.queryKey),
        });
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
