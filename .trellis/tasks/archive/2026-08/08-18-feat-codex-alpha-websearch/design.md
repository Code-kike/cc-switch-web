# Design — Codex Alpha Search + Claude hosted WebSearch

## 架构与边界

移植 Product upstream `bdeaac75`（9 文件 / +10,041 / −1,691）到 Web-first fork。提交内含**两条互相独立**的联网搜索路径，共享零生产代码，只共享同一批 proxy 文件。

### 移植方法

selective port（逐 hunk 对齐上游最终态）。基线调研证明主体可移植：`streaming_responses.rs`/`transform_responses.rs` 与上游 `bdeaac75^` 仅漂移 28/78 行，fork 完整持有 Responses streaming stack。`handlers.rs`/`forwarder.rs` 漂移大（2287/2679 行）源于 fork Web-first 改造（`ProxyRuntimeCtx`、双运行时 forwarder、128 MiB body cap），但本提交对二者改动小（+297/−69、+107/−0），按 hunk 落地。

### 不移植（Q1 裁定）

`providers/transform_codex_anthropic.rs` (+33/−16) — fork 无此文件，属延期 Codex Chat routing stack。见 PRD Q1。

## A. Codex Alpha Search 透传

### 数据流

```
POST /alpha/search | /v1/alpha/search | /v1/v1/alpha/search | /codex/v1/alpha/search
  → server.rs 路由 → handlers::handle_alpha_search
  → endpoint_with_query(&uri, "/alpha/search")     [fork handlers.rs:481 既有]
  → forwarder 既有 provider 选择/模型映射/鉴权/retry/failover/日志
  → base-URL provider:  {base}/alpha/search
  → full-URL provider:  rewrite_codex_alpha_search_full_url(base_url, query)
```

### 契约

- `server.rs`：四别名与 fork 既有 `/responses` 四别名（310–313）同构追加，全部 `post(handlers::handle_alpha_search)`。
- `handlers.rs`：`handle_alpha_search` 与 `handle_responses` 同形，仅端点常量不同。
- `forwarder.rs`：
  - `is_codex_alpha_search = matches!(app_type, AppType::Codex) && split_endpoint_and_query(&effective_endpoint).0 == "/alpha/search"`
  - full-URL 分支插在既有 `is_full_url` 分支之前：`else if is_full_url && is_codex_alpha_search { rewrite_codex_alpha_search_full_url(...) }`
  - `rewrite_codex_alpha_search_full_url` **仅**接受以 `/responses` 结尾的 URL，取其前缀拼 `/alpha/search`，保留 passthrough query；opaque URL 返回错误
    `"Codex Alpha Search cannot derive /alpha/search from an opaque full URL; use a base URL or a full URL ending in /responses"`。

### fail-closed 是安全要求，不是体验取舍

opaque full URL 若猜测派生，会把搜索 payload 发到**非搜索端点**（可能是任意上游路径）。必须拒绝而非降级。

### 测试
- `server.rs`：`alpha_search_routes_forward_to_canonical_upstream`（mock upstream Router 捕获 path/authorization/body，验证四别名都归一到 canonical `/v1/alpha/search`）。
- `forwarder.rs`：`alpha_search_rewrites_known_full_responses_urls`（含 `/v1`、`/backend-api/codex`、含 `%2F` 的自定义前缀 + query 保留）、`alpha_search_rejects_opaque_full_url_instead_of_misrouting_payload`。

## B. Claude hosted WebSearch → Responses 桥

### 数据流

```
Claude Code messages 请求（含 hosted web_search 工具）
  → handlers::handle_claude_transform
      hosted_web_search_name = transform_responses::anthropic_web_search_tool_name(original_body)
      hosted_web_search_max_uses = transform_responses::anthropic_web_search_max_uses(original_body)
  → 请求侧：transform_responses::anthropic_to_responses
      validate_anthropic_web_search_direct_mode  (fail-closed)
      anthropic_web_search_to_responses          (工具翻译 + max_uses 正整数校验)
      responses_web_search_call_from_anthropic_blocks  (多轮重放)
  → 上游 Responses / Codex OAuth
  → 响应侧（流式）：streaming_responses::create_anthropic_sse_stream_from_responses_with_web_search_options
      record_web_search_call → server_tool_use + web_search_tool_result 成对块
      BufferedCitationTextState → 引用保留（markdown 链接不重复注入）
      web_search_limit_stop_events → max_uses 超限 → max_uses_exceeded
      order_anthropic_web_search_result_stream → 结果块顺序稳定
  → 响应侧（非流式）：handlers::responses_sse_stream_to_anthropic_message
      内部复用同一 with_web_search_options 流式转换后聚合
  → transform_responses::responses_to_anthropic_with_web_search_options
  → log_usage 记录 usage.server_tool_use.web_search_requests
```

### 请求翻译契约（fail-closed 矩阵）

| 输入 | 行为 |
|---|---|
| `allowed_domains` | 保留，映射到 Responses hosted `web_search` |
| `blocked_domains` 非空 | **显式失败**（Responses 无 deny-list 等价物） |
| non-direct caller | fail-closed |
| `response_inclusion` | fail-closed |
| 未验证版本 | fail-closed |
| `max_uses` 非正整数 | 拒绝（`"Anthropic WebSearch max_uses must be a positive integer"`） |

### max_uses 双路由语义（关键差异）

- **API-key Responses 路由**：`max_uses` → 上游 `max_tool_calls` 硬限制。
- **Codex OAuth 路由**：请求契约**拒绝** `max_tool_calls` 字段 → 由桥端限流：把限制写入 model instructions，检测到额外调用启动时停止上游流，返回 `max_uses_exceeded`。
- **非强制（non-forced）Codex 请求带 `max_uses`**：fail-closed —— 其 per-tool 预算无法安全表达。

`handlers.rs` 侧对应 `enforce_codex_web_search_limit_while_aggregating = aggregate_codex_oauth_responses_sse && hosted_web_search_max_uses.is_some()`。

### 不可信 Markdown 解析（新增安全面）

为「引用已在正文中以 markdown 链接出现则不重复注入」，`transform_responses.rs` 引入约 30 个解析函数：`markdown_code_scan`（fence/inline code mask）、`markdown_container_prefix`（blockquote/list 容器）、`markdown_bracket_pairs`、`markdown_reference_definitions`/`markdown_reference_uses`、`is_valid_bare_markdown_destination`/`is_valid_markdown_autolink_destination`、`contains_markdown_link_to_url_with_context`。

输入是**模型输出**（不可信）。移植红线：
- 必须迭代实现，无无界递归、无指数回溯（上游 `markdown_bracket_pairs` 等已是迭代 + 显式 mask 数组）。
- 只做只读扫描 + 掩码构造，不做外部 IO。
- 移植后以 fork 既有口径复核：不引入新的出站目标，SSRF 面不变（桥只翻译工具语义，出站仍走 forwarder + `ip_guard`）。

## 无新命令 / Web parity

9 个文件全在 `src-tauri/src/proxy/` 与 `docs/`。**不触及** `commands/`、`web_api/handlers/`、`src/lib/api/web-commands.ts`、`src/`。

→ 本子任务不新增 Tauri 命令；`check:web-routes` 应保持 **292 commands / 280 routes / 0 gaps 不变**。计数变化即为误引入。

proxy 路由（`/alpha/search` 等）是 **CC Switch 本地代理服务器**的 Axum 路由，与 `web_api` 的 Web API 路由是两套独立表面，不进 `web-commands.ts`。

## 批次依赖与顺序

```
W1 Alpha Search 透传 ────────────  独立（与 B 零共享生产代码）
  server.rs 4 routes + handlers::handle_alpha_search
  + forwarder is_codex_alpha_search / rewrite_codex_alpha_search_full_url
  + 4 tests

W2 WebSearch 请求翻译 + markdown/引用原语 ──┐  提供 W3 消费的转换原语
  transform_responses.rs +3199/−293
  (anthropic_web_search_tool_name / anthropic_web_search_max_uses /
   validate_anthropic_web_search_direct_mode / anthropic_web_search_to_responses /
   responses_web_search_call_from_anthropic_blocks /
   responses_to_anthropic_with_web_search_options /
   web_search_results_from_action / web_search_results_from_output_item /
   web_search_tool_result_error / web_search_max_uses_exceeded_error /
   ~30 markdown fns) + 39 tests

W3 WebSearch 响应/流式桥 + 非流式 + usage + docs + 全量门禁 ──┘
  streaming_responses.rs +6179/−1310 (66 tests)
  handlers.rs +297/−69 (handle_claude_transform 接线 +
   responses_sse_stream_to_anthropic_message + log_usage server_tool_use)
  docs/guides ×3
```

W2 必须先于 W3：`streaming_responses.rs` 的新 `use super::transform_responses::{web_search_max_uses_exceeded_error, web_search_results_from_action, web_search_results_from_output_item, web_search_tool_result_error}` 直接依赖 W2 导出的函数。W1 可证明独立（不同路由、不同 handler、无共享符号），先落以建立较简路径并验证 forwarder hunk 对齐方式。

## 兼容性与回滚

- 每批独立 commit → 单批失败 `git reset --hard <上一批>` 回滚。
- 无 DB 迁移（本提交不触 `database/`），无 schema 风险。
- 无新命令 → 无 Web route parity 回滚面。
- 子任务在同分支、独立 PR；回滚不影响父主体与已归档 pi 子任务。

## 重要权衡

- **W2/W3 拆分 vs 单批**：`streaming_responses.rs` 单文件 +6179/66 测试与 `transform_responses.rs` +3199/39 测试合计近 1 万行，单批不可审查。按依赖方向（transform → streaming）切分，每批可独立过门禁。代价是 W2 落地后 `transform_responses.rs` 的部分导出函数暂无调用方（W3 才消费），需容忍一轮 dead-code 警告或加 `#[allow]`（优先前者，W3 落地即消除）。
- **Alpha Search 先行 vs 后置**：先行使 forwarder hunk 的对齐方式在小改动上先被验证（fork forwarder 漂移 2679 行，是本任务最大对齐风险点），再进入 B 侧大改。
- **`transform_codex_anthropic.rs` 延期 vs 恢复延期栈**：见 PRD Q1。恢复会把量级从 ~10k 升到 ~16k 行并重开 S4 已裁定跳过的转换层，远超本子任务边界。
