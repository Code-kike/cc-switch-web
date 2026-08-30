# Review Findings — v3.20.0 Web 全量回归

基线：`25e34700..f0b468ad`（302 文件 / 256 source+test）。
事实源：当前源码 + 运行中的 systemd 服务 + 真实数据库 + 浏览器。

---

## F1 [P0 · 已修复] Codex 会话用量自 8/14 起永久停止导入

**症状（线上复现）**：每分钟扫描 518 文件、重复输出 118 条 `deferred` WARN、`导入 0 条`；
Claude/Pi 正常。`proxy_request_logs` 中 Codex 最新记录停在 **2026-08-14 01:47**，
而 Claude/Pi 均为当日 —— Codex 用量视图静默停更 16 天。

**根因 1（数据丢失）**：`ParentTokenTimeline::signatures_before` 以
`parent.max_timestamp < child fork cutoff` 判定「父 rollout 尚未写到 fork 时刻」。
该守卫用于捕捉**仍在写入**的父文件，但无法与**早已完结**的父文件区分。
`codex resume` 旧会话正是后者，真实数据佐证：

| | 文件 | 时间 |
|---|---|---|
| 父 | `rollout-2026-07-29T00-17-10` | 最后一行 `2026-07-29T16:03:47.666Z`（mtime 相同） |
| 子 | `rollout-2026-08-30T01-19-29` | fork cutoff `2026-08-29T17:19:30.059Z` |

父文件已停写一个月，条件永久成立 → 这些会话的用量**永不导入**且每分钟重解析。

**根因 2（日志放大）**：`mark_deferred` 仅在 pending 条目变化时告警，但重试分支先
`caches.pending.remove()` 再让其重新 insert，旧值恒为 `None` → 每轮都判定为首次 deferral。

**修复**：`PARENT_SETTLED_AFTER_NANOS`（120s，远高于同步周期）—— 超过该静默期的父文件视为已定型，
其时间线被接受；仍在写入的父文件继续 defer（提前接受会缩短 replay prefix 并**重复计费**）。
重试分支不再清除 pending 条目。

**变异验证**：`parent_stamp_is_settled` 恒 false → `test_settled_parent_...` FAILED；
恢复 `pending.remove` → `test_unchanged_retryable_...` FAILED。

**线上效果**：新二进制首轮 `导入 271 条`，Codex 最新记录 `2026-08-14` → `2026-08-30`；
告警从「每分钟 26 条」变为「首轮一次，之后 0」。

**Commit**：`17cfab54`

---

## F2 [P1 · 已修复] 回填成本后未清除 `pricing_missing`，行被永久排除出聚合

**契约**：`usage_rollup::rollup_and_prune` 的 M4 注释声明，定价缺失的行保留下来
「以便在补齐 model_pricing 后重算」。该查询用 `pricing_missing = 0` 同时门控**聚合**与**清理**。

**缺陷**：重算确实发生（查询期 `maybe_backfill_log_costs` 于 `usage_stats.rs:1423/1466`，
以及定价更新时的 `backfill_missing_usage_costs*`），但 UPDATE 只写成本列。标志保持 1 →
刚获得正确成本的行**仍被排除出每一次 rollup**，且**永不清理**，detail 表无限增长。

**真实数据**：`claude-opus-5` 232 行（2026-08-08~08-25）成本 $0 且 `pricing_missing=1`，
而该模型的种子在本次同步中已存在（`schema.rs:1624`，$5/$25）。

**修复**：在同一 UPDATE 中置 `pricing_missing = 0`。安全性依据：定价为 `None` 时函数提前返回，
走到写入即代表定价已解析。

**线上效果**：浏览用量列表触发惰性回填后，生产库 `claude-opus-5` 的 `still_flagged` 由 232 → **0**，
1246 行已定价，恢复计入 **$1351.41**（副本对照实验中该区间恢复 $637.69）。

**Commit**：`dbbb5654`（含架构审阅要求的全局计数器移除）

---

## 已审阅无发现（有证据）

| 面 | 证据 |
|---|---|
| 首屏 404 | 仅 `/favicon.ico`；应用从未引用，SPA 深链接 `/settings` `/usage` `/sessions` 均 200 |
| CSP `eval` 告警 | 全 dist 无 `eval(`/`new Function(` 调用语法（命中项为 JS 解析器关键字表）；Chrome 以 `--disable-extensions` 启动 → 系 DevTools 快照自身的 `Runtime.evaluate` 触发 |
| 应用页 Claude/Codex/Hermes/Pi | 全部渲染；Pi 正确显示 4 个 provider 与用量，并按设计隐藏 MCP/Universal Provider |
| 工具页 Skills/Prompts/Universal Provider/Session Manager/MCP/Settings | 6/6 正常打开，零新增失败请求 |
| `queryproviderusage` 500 | 响应体为 `error sending request for url (https://api.moril1.com/...)`，外部供应商不可达的传输层失败；按 spec「transient → Err → 前端 reject 以便重试并保留 last-good」属设计行为 |
| 路由/schema/上限 | 292 commands / 280 routes / 0 gaps；`SCHEMA_VERSION` 17；`integrity_check = ok` |

---

## 有界且不可逆（记录，不伪造修复）

- **历史 rollup 成本泄漏**：修复前已冻结进 `usage_daily_rollups` 的失败请求成本，源 detail 已删，
  无法精确重算。上限：39 行含失败、636 次失败、≤$197.10。见 `f0b468ad`。
- **`gpt-5.4-xhigh-px`（827 行 / 7880 万 tokens）与 `codex-auto-review`（9 行）**：无对应种子，
  成本确实未知，保持 `pricing_missing=1` 被排除出聚合是正确的（不猜价）。
- **92 个 `MissingParent` deferral**：3 个父 rollout 在磁盘上确不存在（各 0 文件）。
  replay prefix 无法计算，导入会重复计费 → 按会计正确性 fail-closed 保留；该分支为早退，
  不重解析、不重告警，成本有界。

---

## 覆盖状态（诚实记录）

已深审并取证：session usage 全链（Codex/Pi importer、detail 聚合、rollup、定价查找与回填）、
浏览器全部应用页与工具页、Web API 路由契约、部署运行时。

**尚未逐文件走完** 302 文件基线中的其余部分（S2 `backup.rs`/`sync_protocol.rs` 深审、
S4/S5 provider 表单细节、Alpha WebSearch 与 Managed OAuth 的逐 hunk 复核）。
这些面由现有回归测试覆盖（`cargo test --lib` 2296、test:unit 177/1089、parity 38、
integration、smoke 全绿），但未做本轮的人工语义复核。

---

## F3 [P0 · 已修复] LAN HTTP 访问下 Pi 编辑弹窗整页崩溃

**用户报障**：Pi 页编辑 axonhub-国产 渠道即显示「界面遇到了问题 / 重新加载界面」。

**根因**：`crypto.randomUUID` 仅在 secure context（HTTPS/localhost）下存在。用户经
LAN IP `http://100.75.197.120:3010`（非 secure）访问时其为 undefined；而
`PiProviderForm.tsx:272/292`（model draft）与 `StructuredOptionsEditor.tsx:108`
（addOption）绕过既有 `utils/uuid.ts` 的 `generateUUID()`（已含 getRandomValues
兜底）裸调 `crypto.randomUUID()`，渲染期抛出 → ErrorBoundary 整页接管。

日志佐证：`~/.cc-switch/logs/frontend.log` 的 error_boundary 记录与堆栈精确
指向 `index-*.js` useMemo + Array.map（model draft）。

**修复**：三处改调 `generateUUID()`，不新建 helper（code-reuse 指南）。

**变异验证**：`tests/components/PiProviderForm.insecureContext.test.tsx` 删除
`crypto.randomUUID` 后渲染编辑表单 —— 修复前 TypeError（红），修复后渲染通过（绿）。
`tests/utils/uuid.test.ts` 钉住 fallback 的 UUID v4 形状 + 原生路径回归。

**部署终验（关键：非 127.0.0.1）**：在 LAN IP `100.75.197.120:3010` 上核实
`isSecureContext=false` 且 `randomUUID` 不存在；Pi 页 Edit axonhub-国产 →
表单正常渲染、Add header / Add compatibility 子组件正常、Save PUT 200。
服务侧 `frontend.log` 自部署后无新 error_boundary 记录。

**Commit**：`571257bd`（前端修复）+ 前置 `b1677ac7`（dao 搬迁收尾）。

### Side note — 已验证无缺陷

- `navigator.clipboard` 链路已有兜底：`lib/clipboard.ts` 先走后端 copy_text_to_clipboard，
  失败回退 navigator.clipboard（非 secure 下同样可用）。本问题未波及。

## CODEX DAO 分层搬迁（architecture-review 驱动收尾）

按 implement.md C1 补完「纯搬迁 commit」：`reset_codex_usage`（含 SAVEPOINT 事务）/
`load_codex_sync_cursors`/`insert_codex_session_row_on_conn`/`is_rollout_filename`/
`is_codex_cursor_path` 收入 `database/dao/session_usage_codex.rs`，`impl Database` 块
移回 dao（符合 database/mod.rs 架构注释「持久化归 dao/」）。服务层保留 scanner /
parser / replay 与账务推导，成本计算以参数传入 DAO。行为锁定测试（2296）原样通过，
无任何断言改动。pricing lookup（find_codex_pricing）有意留在 service 层——它是定价
域匹配策略，待与 gemini/pi/session-usage 的相似查询收拢为 model_pricing 共享 helper
时再统一（下一独立小 commit）。
