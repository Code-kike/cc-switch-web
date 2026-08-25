# PRD — Codex Alpha Search + Claude hosted WebSearch

> 父任务：`08-18-sync-upstream-v3.20.0`。本子任务独立规划/实现/归档。

## 范围

- `bdeaac75` fix(proxy): support Codex Alpha Search and Claude hosted WebSearch (#5681) — 9 文件 / **+10,041 / −1,691**

上游提交合并两条**互相独立**的联网搜索路径：

**A. Codex Alpha Search 透传**（~330 行）
把 `POST /alpha/search` 及本地别名注册为到所选 Codex provider 的语义透传，复用既有 provider 选择、模型映射、鉴权、retry/failover、日志。full-URL provider 仅从**无歧义的 `/responses` URL** 派生同级 `/alpha/search` 端点，否则 fail-closed。

**B. Claude hosted WebSearch → OpenAI Responses 桥**（~9,400 行）
把 Claude Code 的 hosted WebSearch 工具桥接到 Responses hosted `web_search` 工具与 Codex OAuth 后端：请求翻译（对不可表达约束 fail-closed）、成对 `server_tool_use` / `web_search_tool_result` 块、引用保留、`max_uses` 强制、多轮重放、`usage.server_tool_use.web_search_requests`。

## 已确认事实（代码库调研）

### 基线对齐（决定可移植性的关键）

| 文件 | 上游 `bdeaac75^` | fork 现状 | 上游 `bdeaac75` | fork 漂移 |
|---|---|---|---|---|
| `providers/streaming_responses.rs` | 2197 | 2195 | 7066 | **28 行** |
| `providers/transform_responses.rs` | 2402 | 2410 | 5308 | **78 行** |
| `proxy/server.rs` | 405 | 390 | 628 | **61 行** |
| `proxy/handlers.rs` | 3353 | 1760 | 3581 | 2287 行 |
| `proxy/forwarder.rs` | 5076 | 4349 | 5183 | 2679 行 |
| `providers/transform_codex_anthropic.rs` | 存在 | **不存在** | — | 全缺 |

- 本提交主体（`streaming_responses.rs` +6179、`transform_responses.rs` +3199）所在文件与上游 pre-commit 基线**几乎一致**（28/78 行漂移）→ fork 完整持有 Responses streaming stack，主体可移植。
- `handlers.rs`/`forwarder.rs` 漂移大是 fork Web-first 改造的既有结果（`ProxyRuntimeCtx`、双运行时 forwarder、128 MiB body cap），**不是缺失基线**；本提交对二者的改动量小（+297/−69、+107/−0），按 hunk 逐个对齐即可。

### fork 既有前置（全部就位）
- `handlers.rs:481` `endpoint_with_query(&uri, endpoint)` — Alpha Search handler 直接复用。
- `forwarder.rs:1025` `is_full_url` provider meta 读取 + full-URL 分支（1239/1241）。
- `transform_responses.rs:292` `pub fn anthropic_to_responses`、`streaming_responses.rs:293` `pub fn create_anthropic_sse_stream_from_responses` — 两个改造入口都在。
- `server.rs` 已有 `/responses` 四别名路由表（310–313），Alpha Search 四别名同构追加。
- `docs/guides/claude-codex-routing-guide-{en,ja,zh}.md` 第 96 行「Web search is unavailable」原句在三语中逐字存在 → 文档改写可直接落地。

### fork 无既有 web-search 支持
`grep -rn "web_search\|server_tool_use\|alpha/search" src-tauri/src/proxy/` 仅命中 `transform_codex_responses_xai_sanitize.rs:50` 的 xAI 工具名白名单 —— 无功能重叠、无碰撞。

### 无新命令
9 个文件均在 `src-tauri/src/proxy/` 与 `docs/`，**不触及** `commands/`、`web_api/`、`src/lib/api/web-commands.ts`、`src/`。→ 本子任务**不新增 Tauri 命令**，`check:web-routes` 计数应保持 292 commands / 280 routes 不变。PRD stub 原写「新命令注册 web-commands.ts」为误设，已按调研纠正。

### 测试规模
上游随本提交新增 **105 个测试**：`streaming_responses.rs` 66、`transform_responses.rs` 39、`forwarder.rs` 2、`handlers.rs` 1、`server.rs` 1（含 mock upstream Router 的 `alpha_search_routes_forward_to_canonical_upstream`）。

### 新增安全面：不可信 Markdown 解析
B 侧为保留引用，新增约 30 个 markdown 解析函数（code fence/容器前缀/bracket pair/reference definition/autolink 目标校验）处理**模型输出**这一不可信内容。`markdown_bracket_pairs` 等为迭代实现；`anthropic_web_search_to_responses` 对 `max_uses` 做正整数校验。移植时须核验无无界递归/回溯。

## 前置依赖
- 父主体 S2/F 组 proxy 基线已落地（`d2b070c9` never clobber login 等就位）。
- 无跨子任务依赖；与 `feat-managed-oauth-accounts` 无文件重叠。

## 约束（carry-forward）
- proxy raw/decompressed body 保留 fork 既有 **128 MiB cap**、2s JS deadline、16 MiB heap / 256 KiB stack；本提交对 `forwarder.rs` 仅做 URL 派生，不得触碰 body 处理上限。
- 不引入 SSRF：WebSearch 桥只做工具语义翻译，出站仍走既有 forwarder + `ip_guard`；`allowed_domains`/`blocked_domains` 必须保持 fail-closed 语义。
- 不可信 markdown 解析必须有界（无无界递归/指数回溯）。
- `transform_codex_anthropic.rs` hunk 丢弃（Q1 裁定，见下）。
- `.pi/`、`.pi-subagents/` 不得修改或提交。
- zh-TW 不存在，无相关 hunk。

## 验收标准
- [x] **A** Codex Alpha Search：4 条本地别名路由（`/alpha/search`、`/v1/alpha/search`、`/v1/v1/alpha/search`、`/codex/v1/alpha/search`）透传到所选 Codex provider 的 canonical `/alpha/search`；full-URL provider 仅从 `/responses` 结尾 URL 派生，opaque full URL **fail-closed 拒绝**且不误发 payload。
- [x] **B** Claude hosted WebSearch：请求翻译（`allowed_domains` 保留；`blocked_domains` 非空 → 显式失败；non-direct caller / `response_inclusion` / 未验证版本 → fail-closed）；响应侧成对 `server_tool_use` + `web_search_tool_result` 块、引用保留、`max_uses` 强制（API-key 路由映射 `max_tool_calls`；Codex OAuth 路由改由桥端限流 + `max_uses_exceeded`）、多轮重放、`usage.server_tool_use.web_search_requests` 计数。
- [x] **无新命令**：`check:web-routes` 保持 292 commands / 280 routes / 0 missing/mismatch/dangling/fallback（计数不变即为正确）。
- [x] proxy 安全上限零退化：128 MiB body cap、2s deadline、16 MiB heap、256 KiB stack、32 MiB catalog cap。
- [x] 不可信 markdown 解析有界（并经 W2.5 加固使二次复杂度受五个上限约束）；无新增**出站路径**（口径修正：`ip_guard` 挂在 `http_client::get_guarded()` 与 `web_api` 出站，proxy forwarder 热路径按 scope-C 契约刻意不设 guard）。
- [x] 上游测试全量移植并全绿（不删测试、零断言删改）。实测上游增量为 **103**（streaming 64 而非规划估的 66；forwarder 2 / handlers 1 / server 1 / transform 39 无误差），另加 fork 侧新增 **20** 个（W2 1 + W2.5 7 + W3 11 + check 1）。
- [x] 全量门禁：test:unit 全绿（非 flake 项）、test:integration（4 PRD flakes 外全绿）、Rust parity（`web_api::`/`dual_runtime_parity::`/`web_proxy_lifecycle::`）、web-routes、locales parity、build:web exit 0、smoke:web-server exit 0；与父主体及已归档 pi 子任务无回归。
- [x] docs：三语 `claude-codex-routing-guide` 第 96 行 web-search 段落按实际能力改写（不宣称未实现能力）。
- [x] `transform_codex_anthropic.rs` 整文件延期记录在案，且 Q1 前提在 W3 失效后已**原地修正**（单函数以私有 fn 落在 `handlers.rs`，延期栈四文件仍缺席，必需性经变异验证）。

## 裁定记录（brainstorm 2026-08-24，用户授权采纳推荐方案）

- **Q1 `transform_codex_anthropic.rs` (+33/−16)**：**整文件跳过（延期栈不恢复），但其中 1 个函数在 W3 被证明必需 → 已按下述修正落地**。
  - 原裁定（2026-08-24 brainstorm）：跳过整个 hunk。理由是该文件属 fork 明确延期的 Codex Chat routing stack（连同 `codex_chat_common.rs`/`streaming_codex_chat.rs`/`transform_codex_chat.rs` 均不存在），恢复需先移植约 5.7k LOC 基座；功能后果限于 Codex Chat → Anthropic 桥的 usage 上报精度，而 fork 无该桥。
  - **W3 修正（2026-08-24，`c917d5cf`）**：原裁定的「fork 无该桥所以无影响」前提**在 W3 引入 fork 本地 Anthropic SSE 聚合器后失效**。上游 `responses_sse_stream_to_anthropic_message`（非流式路径）调用 `transform_codex_anthropic::anthropic_sse_to_message_value`。子代理只把**该 1 个函数**按上游 post-`bdeaac75` 状态（即含 Q1 原本要延期的 `message_delta.usage` 整对象合并）移植为 `handlers.rs` 内的**私有 helper**，`providers/` **未新增任何延期文件**（四文件仍全部缺席，已复核）。
  - **必需性经变异验证**：把该合并回退成「只取 `output_tokens`」后，移植的 handlers 测试失败 —— `usage.server_tool_use.web_search_requests` 变为 `Null` 而非 `1`。即该 hunk 对 hosted WebSearch 的 usage 计数是 load-bearing，不是装饰。
  - 净结果：延期栈边界未破（无新文件、无新模块声明、helper 为 `fn` 私有），但 Q1 的「后果可界定」表述在 W3 之后不再准确，故此处修正记录。
- **Q2 执行分批**：3 批 —— **W1** Alpha Search 透传（独立、~330 行）→ **W2** WebSearch 请求翻译 + markdown/引用原语（`transform_responses.rs`，39 测试）→ **W3** WebSearch 响应/流式桥 + 非流式 + usage + docs + 全量门禁（`streaming_responses.rs` 66 测试 + `handlers.rs`）。W2 提供 W3 流式层消费的转换原语，故必须先行；W1 与 B 侧无共享代码，可证明独立，先落以建立较简路径。
