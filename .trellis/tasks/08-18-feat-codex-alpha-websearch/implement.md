# Implement — Codex Alpha Search + Claude hosted WebSearch

## 前置确认

- [ ] 分支 `sync/upstream-v3.20.0`；父主体 S1–S8 + 已归档 `feat-pi-native-agent` 均在 HEAD 之前。
- [ ] `bdeaac75` 在 `product-upstream` remote 本地可达。
- [ ] `.pi/`/`.pi-subagents/` 不在提交范围。
- [ ] 基线快照（回归对照）：test:unit 173 files / 1044 tests；`cargo test --lib` 2083 passed / 5 ignored；Rust parity 37；web-routes **292 commands / 280 routes / 0 gaps**；locales 2637 parity；test:integration 50/54（4 PRD flakes）。

## 移植方法

逐 hunk selective port（取上游 `bdeaac75` 最终态对齐 fork）。`transform_codex_anthropic.rs` hunk 丢弃（PRD Q1）。

门禁命令：
```bash
source ~/.cargo/env
(cd src-tauri && cargo fmt --all -- --check)
pnpm format:check
pnpm typecheck
pnpm check:web-routes        # 必须保持 292/280/0 —— 计数变化即误引入命令
pnpm check:locales
pnpm exec cargo check --manifest-path src-tauri/Cargo.toml
(cd src-tauri && cargo check --no-default-features --features web-server --example server)
cargo test --manifest-path src-tauri/Cargo.toml --lib proxy::
pnpm test:unit
cargo test --manifest-path src-tauri/Cargo.toml --no-default-features --features web-server --example server -- web_api:: dual_runtime_parity:: web_proxy_lifecycle::
```
W3 追加：`pnpm test:integration`、`pnpm build:web`、`pnpm smoke:web-server`。

## 执行批次（ordered checklist）

### W1 — Codex Alpha Search 透传（~330 行，独立）
- [ ] `proxy/server.rs`：追加四别名路由 `/alpha/search`、`/v1/alpha/search`、`/v1/v1/alpha/search`、`/codex/v1/alpha/search` → `post(handlers::handle_alpha_search)`，紧邻既有 `/responses` 四别名（现 310–313）同构排布
- [ ] `proxy/handlers.rs`：新增 `pub async fn handle_alpha_search`，复用既有 `endpoint_with_query(&uri, "/alpha/search")`（handlers.rs:481）与 `handle_responses` 同形管线
- [ ] `proxy/forwarder.rs`：
  - `is_codex_alpha_search = matches!(app_type, AppType::Codex) && split_endpoint_and_query(&effective_endpoint).0 == "/alpha/search"`
  - full-URL 分支：`else if is_full_url && is_codex_alpha_search { rewrite_codex_alpha_search_full_url(&base_url, passthrough_query.as_deref())? }`，插在既有 `is_full_url` 分支（现 1239/1241 附近）之前
  - `fn rewrite_codex_alpha_search_full_url`：仅接受 `/responses` 结尾 URL → 取前缀拼 `/alpha/search` + 保留 query；opaque URL 返回上游原文错误
- [ ] 移植 4 个测试：`server.rs` `alpha_search_routes_forward_to_canonical_upstream`（mock upstream Router，四别名归一断言）；`forwarder.rs` `alpha_search_rewrites_known_full_responses_urls`（`/v1`、`/backend-api/codex`、含 `%2F` 自定义前缀，query 保留）+ `alpha_search_rejects_opaque_full_url_instead_of_misrouting_payload`
- [ ] **红线核验**：body cap / deadline 未触碰（forwarder 改动仅 URL 派生）；`check:web-routes` 计数不变
- [ ] 门禁全绿 → commit

### W2 — WebSearch 请求翻译 + markdown/引用原语（`transform_responses.rs` +3199/−293，39 测试）
- [ ] 工具识别与校验：`is_anthropic_web_search_tool`、`anthropic_web_search_tool_name`、`anthropic_web_search_max_uses`（正整数校验）、`validate_anthropic_web_search_direct_mode`、`has_http_url_scheme`
- [ ] 请求翻译：`anthropic_web_search_to_responses`（`allowed_domains` 保留 / `blocked_domains` 非空显式失败 / `response_inclusion`/non-direct/未验证版本 fail-closed）
- [ ] 多轮重放：`responses_web_search_call_from_anthropic_blocks`、`collect_web_search_results_from_content`
- [ ] 结果/错误原语（W3 消费）：`web_search_results_from_action`、`web_search_results_from_output_item`、`web_search_tool_result_error`、`web_search_max_uses_exceeded_error`
- [ ] 响应聚合入口：`responses_to_anthropic_with_web_search_options`
- [ ] `anthropic_to_responses` 接入上述路径（保留 fork 既有 78 行漂移适配）
- [ ] ~30 个 markdown 解析函数：`markdown_code_scan`/`markdown_container_prefix`/`markdown_container_continuation_start`/`markdown_fence`/`markdown_bracket_pairs`/`markdown_list_marker_end`/`markdown_indentation_columns`/`markdown_link_label`/`markdown_link_destination`/`markdown_link_suffix_end`/`markdown_link_suffix_is_closed`/`markdown_inline_destination_end`/`is_valid_bare_markdown_destination`/`is_valid_markdown_autolink_destination`/`markdown_closing_bracket`/`normalize_markdown_reference_label`/`markdown_reference_definitions`/`markdown_reference_uses`/`markdown_link_syntax_mask`/`markdown_destination_matches_url`/`contains_markdown_link_to_url{,_with_context}`/`char_index_to_byte_offset`/`is_markdown_escaped` 等
- [ ] **不可信输入红线**：全部迭代实现，无无界递归/指数回溯；只做只读扫描 + 掩码；无外部 IO。逐函数确认后在 commit message 记录。
- [ ] 移植 39 个测试
- [ ] 容忍本批 dead-code 警告（W3 才有调用方）；**不加** `#[allow(dead_code)]` 掩盖，W3 落地即消除
- [ ] 门禁全绿 → commit

### W3 — WebSearch 响应/流式桥 + 非流式 + usage + docs + 全量门禁
- [ ] `providers/streaming_responses.rs` +6179/−1310：
  - `create_anthropic_sse_stream_from_responses_with_web_search_options`（+ `_raw` 内核）
  - 成对块：`record_web_search_call`/`WebSearchCallDisposition`/`WebSearchRecordState`/`web_search_result_events`/`take_web_search_result_events`/`reserve_web_search_result_index`/`take_open_web_search_block_stop_events`/`append_unique_web_search_results`
  - 引用保留：`BufferedCitationAnnotation`/`BufferedCitationPart`/`BufferedCitationTextState`/`StreamedTextPart`/`StreamedTextState`/`missing_message_text_parts`/`text_block_events`
  - 顺序稳定：`WebSearchResultOrderingState`/`order_anthropic_web_search_result_stream`/`anthropic_event_value`/`resolve_content_index`
  - item key 解析：`tool_item_key_from_added`/`tool_item_key_from_event`/`web_search_item_keys`/`reasoning_item_key`
  - max_uses 限流：`web_search_limit_stop_events`
  - 非流式转换：`responses_json_to_anthropic_sse`、`anthropic_ping_sse`
- [ ] `proxy/handlers.rs` +297/−69：
  - `handle_claude_transform` 接线 `hosted_web_search_name` / `hosted_web_search_max_uses`（来自 W2 导出）
  - 流式路径改走 `create_anthropic_sse_stream_from_responses_with_web_search_options`（无 web-search 时保持既有无参路径）
  - `enforce_codex_web_search_limit_while_aggregating = aggregate_codex_oauth_responses_sse && hosted_web_search_max_uses.is_some()`
  - `responses_sse_stream_to_anthropic_message`（非流式聚合）
  - `should_use_claude_transform_streaming` hunk（fork 现 1333）
  - `log_usage` 记 `usage.server_tool_use.web_search_requests`（fork 现 1454）
- [ ] 移植 66 + 1 个测试（streaming 66、handlers `non_streaming_codex_web_search_limit_stops_polling_upstream`）
- [ ] `docs/guides/claude-codex-routing-guide-{en,ja,zh}.md`：改写第 96 行 web-search 段落为实际能力（Responses hosted `web_search` 翻译、`allowed_domains` 保留、API-key 路由 `max_tool_calls`、Codex OAuth 桥端限流 + `max_uses_exceeded`、非强制 Codex 带 `max_uses` fail-closed、`blocked_domains` 显式失败、本地 WebFetch 不受影响）。三语内容对齐，**不宣称未实现能力**。
- [ ] W2 dead-code 警告清零核验
- [ ] 全量门禁（含 test:integration + build:web + smoke:web-server）→ commit

## 验证命令汇总

见每批门禁块。关键额外检查：
- **无新命令**：`pnpm check:web-routes` 保持 292 commands / 280 routes / 0 missing/mismatch/dangling/fallback。计数变化 → 误把 proxy 路由当 Web API 命令注册。
- **安全上限零退化**：`grep` 确认 128 MiB body cap（`hyper_client.rs:128` `MAX_RESPONSE_BODY_BYTES`）、2s JS deadline（`usage_script.rs:9`）、16 MiB heap（`usage_script.rs:10`）、256 KiB stack（`usage_script.rs:11`）、32 MiB catalog（`codex_config.rs:1287`）不变。
- **SSRF 面不变**：新代码无新增出站目标；出站仍经 forwarder + `ip_guard`。
- **不可信 markdown 有界**：无自递归、无指数回溯。
- **延期栈零回潮**：`ls src-tauri/src/proxy/providers/` 不出现 `transform_codex_anthropic.rs`/`transform_codex_chat.rs`/`streaming_codex_chat.rs`/`codex_chat_common.rs`。

## review gates

- 每批 commit 前：全量门禁全绿（test:unit 必须全绿，非 flake 项）。
- W1 后：四别名路由归一 + opaque full URL fail-closed 专项确认；forwarder hunk 对齐方式已验证。
- W2 后：fail-closed 矩阵（`blocked_domains`/non-direct/`response_inclusion`/未验证版本/`max_uses` 非正整数）逐项测试确认；markdown 解析有界性逐函数确认。
- W3 后：max_uses 双路由语义（API-key `max_tool_calls` vs Codex OAuth 桥端限流）+ 成对块/引用保留/多轮重放/usage 计数专项确认；真实 Web 服务冒烟。
- 全部完成后：并入父任务跨子任务集成 review。

## 风险点与回滚

- **单批失败**：`git reset --hard <上一批 commit>`。
- **最大对齐风险 = `forwarder.rs`（fork 漂移 2679 行）**：W1 先在 +107 行小改动上验证对齐方式，再进 B 侧。若 hunk 无法干净对齐，停下报告而非猜测插入点。
- **`handlers.rs` 漂移 2287 行**：W3 的 +297/−69 分布在 8 个 hunk（27/46/412/419/525/915/2017/2668/3210 行附近）。fork 对应符号已确认存在（`handle_claude_transform`:262、`should_use_claude_transform_streaming`:1333、`log_usage`:1454），按符号而非行号定位。
- **W2 dead-code**：本批导出函数暂无调用方属预期；不得用 `#[allow(dead_code)]` 掩盖，W3 落地即消除。若 W3 后仍有残留，说明有 hunk 漏移。
- **不可信 markdown 解析引入 DoS**：若发现任何无界递归/指数回溯，停下报告，不得直接落地。
- **误注册 Web API 命令**：proxy Axum 路由 ≠ Web API 命令。`check:web-routes` 计数是硬约束。
