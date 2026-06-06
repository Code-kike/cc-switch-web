import { QueryClient } from "@tanstack/react-query";

/**
 * 全局默认值（M36 修复刷新放大）：
 *
 * 此前默认 `staleTime: 0`，意味着每次组件挂载 / 窗口聚焦 / 重连都会立刻重拉。
 * 叠加各 hook 的 `refetchInterval`（2s/5s/10s/30s）以及 SSE invalidate 之后，
 * 尤其在 web 模式下会形成持续的请求流。
 *
 * 改为 `staleTime: 30_000` 后：
 * - 窗口聚焦 / 重新挂载 / 重连只在数据已超过 30s 时才重拉，消除了与轮询周期
 *   叠加产生的重复触发（聚焦那一刻往往刚轮询过，旧逻辑会再多打一次）。
 * - `refetchInterval` 在 React Query v5 中独立于 `staleTime`，仍按各自周期轮询，
 *   因此真正需要实时的数据不受影响：proxy 状态运行时 2s 轮询、usage 仪表盘 30s
 *   轮询、failover 队列等都保持原节奏。
 * - mutation 的 `invalidateQueries` 会无视 `staleTime` 立即把数据标记为失效并重拉，
 *   所以用户操作后界面仍即时更新。
 *
 * 选 30s 是因为它与 usage 仪表盘既有的默认轮询周期
 * (`DEFAULT_REFETCH_INTERVAL_MS = 30000`) 一致——团队已认可这是可接受的陈旧度，
 * 复用该值不会让界面显得过时；需要更高实时性的查询继续用各自的
 * `refetchInterval` / 显式 `staleTime` 单独调优，而不是靠全局值。
 *
 * `refetchOnWindowFocus` 保留为 true：配合上面的 `staleTime`，只在数据确实陈旧时
 * 才在聚焦时重拉。
 */
export const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      retry: 1,
      refetchOnWindowFocus: true,
      staleTime: 30_000,
    },
    mutations: {
      retry: false,
    },
  },
});
