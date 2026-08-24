# Implement — Codex Alpha Search + Claude hosted WebSearch

## 前置确认

- [x] 分支 `sync/upstream-v3.20.0`；父主体 S1–S8 + 已归档 `feat-pi-native-agent` 均在 HEAD 之前。
- [x] `bdeaac75` 在 `product-upstream` remote 本地可达。
- [x] `.pi/`/`.pi-subagents/` 不在提交范围。
- [x] 基线快照（回归对照）：test:unit 173 files / 1044 tests；`cargo test --lib` **2106** passed / 5 ignored（规划时误记 2083，已独立复核 `258245f4~1` 实测 2106）；Rust parity 37；web-routes **292 commands / 280 routes / 0 gaps**；locales 2637 parity；test:integration 50/54（4 PRD flakes）。

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
- [x] `proxy/server.rs`：追加四别名路由 `/alpha/search`、`/v1/alpha/search`、`/v1/v1/alpha/search`、`/codex/v1/alpha/search` → `post(handlers::handle_alpha_search)`，紧邻既有 `/responses` 四别名（现 310–313）同构排布
- [x] `proxy/handlers.rs`：新增 `pub async fn handle_alpha_search`，复用既有 `endpoint_with_query(&uri, "/alpha/search")`（handlers.rs:481）与 `handle_responses` 同形管线
- [x] `proxy/forwarder.rs`：
  - `is_codex_alpha_search = matches!(app_type, AppType::Codex) && split_endpoint_and_query(&effective_endpoint).0 == "/alpha/search"`
  - full-URL 分支：`else if is_full_url && is_codex_alpha_search { rewrite_codex_alpha_search_full_url(&base_url, passthrough_query.as_deref())? }`，插在既有 `is_full_url` 分支（现 1239/1241 附近）之前
  - `fn rewrite_codex_alpha_search_full_url`：仅接受 `/responses` 结尾 URL → 取前缀拼 `/alpha/search` + 保留 query；opaque URL 返回上游原文错误
- [x] 移植 4 个测试：`server.rs` `alpha_search_routes_forward_to_canonical_upstream`（mock upstream Router，四别名归一断言）；`forwarder.rs` `alpha_search_rewrites_known_full_responses_urls`（`/v1`、`/backend-api/codex`、含 `%2F` 自定义前缀，query 保留）+ `alpha_search_rejects_opaque_full_url_instead_of_misrouting_payload`
- [x] **红线核验**：body cap / deadline 未触碰（forwarder 改动仅 URL 派生）；`check:web-routes` 计数不变
- [x] 门禁全绿 → commit

### W2 — WebSearch 请求翻译 + markdown/引用原语（`transform_responses.rs` +3199/−293，39 测试）
- [x] 工具识别与校验：`is_anthropic_web_search_tool`、`anthropic_web_search_tool_name`、`anthropic_web_search_max_uses`（正整数校验）、`validate_anthropic_web_search_direct_mode`、`has_http_url_scheme`
- [x] 请求翻译：`anthropic_web_search_to_responses`（`allowed_domains` 保留 / `blocked_domains` 非空显式失败 / `response_inclusion`/non-direct/未验证版本 fail-closed）
- [x] 多轮重放：`responses_web_search_call_from_anthropic_blocks`、`collect_web_search_results_from_content`
- [x] 结果/错误原语（W3 消费）：`web_search_results_from_action`、`web_search_results_from_output_item`、`web_search_tool_result_error`、`web_search_max_uses_exceeded_error`
- [x] 响应聚合入口：`responses_to_anthropic_with_web_search_options`
- [x] `anthropic_to_responses` 接入上述路径（保留 fork 既有 78 行漂移适配）
- [x] ~30 个 markdown 解析函数：`markdown_code_scan`/`markdown_container_prefix`/`markdown_container_continuation_start`/`markdown_fence`/`markdown_bracket_pairs`/`markdown_list_marker_end`/`markdown_indentation_columns`/`markdown_link_label`/`markdown_link_destination`/`markdown_link_suffix_end`/`markdown_link_suffix_is_closed`/`markdown_inline_destination_end`/`is_valid_bare_markdown_destination`/`is_valid_markdown_autolink_destination`/`markdown_closing_bracket`/`normalize_markdown_reference_label`/`markdown_reference_definitions`/`markdown_reference_uses`/`markdown_link_syntax_mask`/`markdown_destination_matches_url`/`contains_markdown_link_to_url{,_with_context}`/`char_index_to_byte_offset`/`is_markdown_escaped` 等
- [x] **不可信输入红线**：全部迭代实现，无无界递归/指数回溯；只做只读扫描 + 掩码；无外部 IO。逐函数确认后在 commit message 记录。
- [x] 移植 39 个测试
- [x] 容忍本批 dead-code 警告（W3 才有调用方）；**不加** `#[allow(dead_code)]` 掩盖，W3 落地即消除
- [x] 门禁全绿 → commit

### W2.5 — 引用去重分析的二次复杂度加固（fork 侧硬化，上游无）
> **来源**：W2 移植后子代理主动上报，主会话结构性复核确认。4 个 markdown shape 为 O(n²)（非指数），最坏 `"[a]("`×n 在 128 KiB 文本上约 15 s CPU。
> **根因（已逐条定位）**：
> - `markdown_link_syntax_mask` 对每个 `](` 调 `markdown_inline_destination_end(&text[start..])`，后者在未闭合目标上 `char_indices()` 扫到文本末尾才返回 `None` → 每候选 O(n)。
> - 同函数对每个 `<` 做 `text[opening+1..].find('>')`，无 `>` 时扫到末尾。
> - `is_markdown_escaped` 的 `.rev().take_while(|b| **b == b'\\')` 在 `"\\"`×n 上每次回走 n。
> - `markdown_reference_uses` 的 definition 成员扫描。
> **已是活路径（非 W3 才接通）**：`handlers.rs:426` / `claude.rs:1043` / `streaming_responses.rs:67` → `responses_to_anthropic`(2477) → `_with_web_search_name`(2481) → `_with_web_search_options`(2488) → `output_text_with_url_citations`(1473) → `text_with_url_citations`(1321) → markdown 二次扫描。触发需上游响应带 `url_citation` 注解 + 病态 markdown（威胁模型：prompt injection 让模型回显敌意内容）。
> **PRD 约束**：carry-forward 明列「不可信 markdown 解析必须有界」。O(n²) 非无界递归也非指数回溯，字面未违约，但 15 s CPU 不满足该约束意图 → 按 fork 既有更严口径加固。

- [x] 在唯一生产入口 `text_with_url_citations`（1321）加双重上限：候选数上限（`](` 与 `<` 出现次数）+ 文本长度上限作粗粒度兜底
- [x] 超限行为 = **跳过去重分析**（`linked_urls` 视为空 → 所有 citation 照常追加）。**此处 fail-open 是正确的**：去重只是「避免重复链接」的优化，不是安全控制；退化后果是病态输入下可能多一个重复链接（外观），而 fail-closed 会丢 citation 或整响应报错，更糟。与 Alpha Search 的 fail-closed 语义相反且互不矛盾 —— 后者的降级会误投递 payload。
- [x] 常量命名与 fork 既有上限风格一致（`MAX_*_BYTES` / `MAX_*`），带注释说明为 fork 侧硬化、上游无
- [x] 回归测试：病态输入（`"[a]("`×n、`"<"`×n、`"\\"`×n）在上限内快速完成且**所有 citation 仍在输出中**；正常输入去重行为不变（既有 `test_text_with_url_citations_does_not_repeat_existing_body_link` 等仍绿）
- [x] 门禁全绿 → commit

### W3 — WebSearch 响应/流式桥 + 非流式 + usage + docs + 全量门禁
- [x] `providers/streaming_responses.rs` +6179/−1310：
  - `create_anthropic_sse_stream_from_responses_with_web_search_options`（+ `_raw` 内核）
  - 成对块：`record_web_search_call`/`WebSearchCallDisposition`/`WebSearchRecordState`/`web_search_result_events`/`take_web_search_result_events`/`reserve_web_search_result_index`/`take_open_web_search_block_stop_events`/`append_unique_web_search_results`
  - 引用保留：`BufferedCitationAnnotation`/`BufferedCitationPart`/`BufferedCitationTextState`/`StreamedTextPart`/`StreamedTextState`/`missing_message_text_parts`/`text_block_events`
  - 顺序稳定：`WebSearchResultOrderingState`/`order_anthropic_web_search_result_stream`/`anthropic_event_value`/`resolve_content_index`
  - item key 解析：`tool_item_key_from_added`/`tool_item_key_from_event`/`web_search_item_keys`/`reasoning_item_key`
  - max_uses 限流：`web_search_limit_stop_events`
  - 非流式转换：`responses_json_to_anthropic_sse`、`anthropic_ping_sse`
- [x] `proxy/handlers.rs` +297/−69：
  - `handle_claude_transform` 接线 `hosted_web_search_name` / `hosted_web_search_max_uses`（来自 W2 导出）
  - 流式路径改走 `create_anthropic_sse_stream_from_responses_with_web_search_options`（无 web-search 时保持既有无参路径）
  - `enforce_codex_web_search_limit_while_aggregating = aggregate_codex_oauth_responses_sse && hosted_web_search_max_uses.is_some()`
  - `responses_sse_stream_to_anthropic_message`（非流式聚合）
  - `should_use_claude_transform_streaming` hunk（fork 现 1333）
  - `log_usage` 记 `usage.server_tool_use.web_search_requests`（fork 现 1454）
- [x] 移植 66 + 1 个测试（streaming 66、handlers `non_streaming_codex_web_search_limit_stops_polling_upstream`）
- [x] `docs/guides/claude-codex-routing-guide-{en,ja,zh}.md`：改写第 96 行 web-search 段落为实际能力（Responses hosted `web_search` 翻译、`allowed_domains` 保留、API-key 路由 `max_tool_calls`、Codex OAuth 桥端限流 + `max_uses_exceeded`、非强制 Codex 带 `max_uses` fail-closed、`blocked_domains` 显式失败、本地 WebFetch 不受影响）。三语内容对齐，**不宣称未实现能力**。
- [x] **核验流式 citation 路径不绕过 W2.5 上限**：`BufferedCitationTextState`/`missing_message_text_parts` 等若自行调用 markdown 原语，必须走同一受保护入口或获得同一上限；否则 W3 会静默重开 W2.5 已关闭的二次复杂度面
- [x] W2 dead-code 警告清零核验
- [x] 全量门禁（含 test:integration + build:web + smoke:web-server）→ commit


## W1 结果（2026-08-24，258245f4）

W1 Codex Alpha Search 透传完成。3 文件（+395/−0）。

### hunk 锚定（按符号，非行号）
- `proxy/server.rs` +223：四别名路由紧随既有 `/grokbuild/v1/responses/compact` 之后（即 `/responses` + compact 别名块末尾）；测试模块追加在 `get_circuit_breaker_stats` 之后（本文件此前无 `#[cfg(test)]` mod）。
- `proxy/handlers.rs` +65：`handle_alpha_search` 插在 `handle_responses_compact` 与 `handle_grokbuild_responses_compact` 之间 —— 与上游完全相同的锚点对，fork 中逐字存在。
- `proxy/forwarder.rs` +107：`is_codex_alpha_search` 紧随 `(effective_endpoint, passthrough_query)` 绑定；分支置于 `else if is_full_url { append_query_to_full_url(...) }` **之前**；`rewrite_codex_alpha_search_full_url` 位于 `append_query_to_full_url` 与 `build_codex_oauth_session_headers` 之间（与上游同邻居）。

`+107`/`+223` 与上游 per-file 计数精确一致。**无任何 hunk 需要猜测插入点** —— 上游全部锚点符号在 fork 中均存在，规划标记的「forwarder 漂移 2679 行」对齐风险未实际发生。

### 与上游最终态保真度
- `rewrite_codex_alpha_search_full_url`：**字节级一致**。
- `server.rs` 测试模块：**字节级一致**。
- `handle_alpha_search`：仅两处 fork 适配 —— 加 `ctx.failover_enabled()`（双运行时 `forward_with_retry` 签名）、去掉 `ctx.outbound_model = result.outbound_model.take();`（fork `RequestContext` 无该字段，`handlers.rs:208` 已有记录）。body 解析失败保留上游 `ProxyError::InvalidRequest`（客户端畸形 body 应为 400），未跟随兄弟 handler 的 `Internal`。

### 测试（3 函数全绿）
- `proxy::forwarder::tests::alpha_search_rewrites_known_full_responses_urls`
- `proxy::forwarder::tests::alpha_search_rejects_opaque_full_url_instead_of_misrouting_payload`
- `proxy::server::tests::alpha_search_routes_forward_to_canonical_upstream`

无需 mock 适配（fork `ProxyServer::new(config, db, Option<ProxyRuntimeCtx>)` 同位置接 `None`，上游测试原样编译）。零断言删改。

**变异验证（不只信断言）**：把 `is_full_url && is_codex_alpha_search` 改成 `false && …` 后 server 测试失败（`left: 404, right: 202`）—— mock 上游只路由 `/v1/alpha/search`，该 404 正是分支所阻止的误路由到 `/v1/responses`。已复原并复验。

### 门禁（全绿）
- cargo fmt / format:check / typecheck ✓
- **check:web-routes 292 commands / 280 routes / 0 missing/mismatch/dangling/fallback —— 计数不变** ✓（硬约束满足）
- check:locales 2637 parity ✓
- desktop + web cargo check ✓
- `cargo test --lib proxy::` **1005 passed / 0 failed** ✓
- `cargo test --lib` 全量 **2109 passed / 5 ignored**（基线 2106 → +3）✓
- test:unit 173 files / 1044 tests ✓（无回归）
- Rust parity 37 ✓
- clippy 仅 2 处既有警告（`services/omo.rs`、`prompt_files.rs`，与本批无关）

### 安全核验
- 上限逐条 grep 确认不变：`MAX_RESPONSE_BODY_BYTES` 128 MiB、`JS_EXECUTION_TIMEOUT` 2s、`JS_MEMORY_LIMIT_BYTES` 16 MiB、`JS_MAX_STACK_BYTES` 256 KiB、`MAX_CODEX_CATALOG_BYTES` 32 MiB。forwarder diff 无任何 body/limit/deadline 处理行。
- 无新出站目标：派生 URL 仍在 provider 自身 host + path 前缀内，出站仍经 forwarder + `ip_guard`。
- fail-closed 逐字保留：opaque full URL 返回
  `ProxyError::ConfigError("Codex Alpha Search cannot derive /alpha/search from an opaque full URL; use a base URL or a full URL ending in /responses")`，**不发出任何请求**。
- 延期栈仍缺席（`transform_codex_anthropic.rs`/`transform_codex_chat.rs`/`streaming_codex_chat.rs`/`codex_chat_common.rs` 均不在 `providers/`）。
- 无新 Tauri 命令；`src/` 未触碰。

### 规划文档纠正
`implement.md` 前置确认原记 `cargo test --lib` 基线 2083 —— 实测 `258245f4~1` 为 **2106**（主会话独立 checkout 复核确认）。差 23 个来自本分支更早提交（P3.5 +23），非本批回归。已在前置确认行内更正。


## W2 结果（2026-08-24，a49e7af5）

W2 hosted WebSearch 请求翻译 + markdown 引用原语完成。1 文件（+3230/−284）。

### 落地方式与漂移保全
- 全批经 `git apply --3way`（`bdeaac75^..bdeaac75` 的 `transform_responses.rs`）落地，**每个 hunk 都落在上游锚点，无手工放置**。
- 未只信工具：`diff upstream_post.rs <ported>` 结果**恰为** 11 个既有 fork 漂移 hunk（内容与移植前 `diff bdeaac75^ fork` 字节一致）+ 1 个新增测试，其余逐字等于上游最终态。
- 78 行漂移构成：`build_anthropic_usage_from_responses` 的本地化文档/注释、cache 记账测试注释、fork 独有 `test_anthropic_to_responses_canonicalizes_tool_arguments`、上游有而 fork 无的 `test_build_usage_clamps_input_when_cache_exceeds_input`（既有差异，本批未引入也未移除）。均不与 W2 hunk 重叠。

### 移植内容
工具识别/校验 5 个 fn；`anthropic_web_search_to_responses` + `anthropic_to_responses` 接线（hosted 工具列表、forced-tool 过滤、`max_tool_calls` vs Codex instruction cap、`include=web_search_call.action.sources`、`map_tool_choice_to_responses` hosted 选择器）；多轮重放（`responses_web_search_call_from_anthropic_blocks`/`collect_web_search_results_from_content`/`convert_messages_to_input` 配对）；结果/错误原语 5 个；`responses_to_anthropic_with_web_search_{name,options}`；~30 个 markdown 引用原语 + 7 个支撑类型。**无 fork 侧代码适配需求**。

### fail-closed 矩阵（7 行全部有测试钉住）
| 行 | 钉住测试 |
|---|---|
| `allowed_domains` 保留 | `test_anthropic_hosted_web_search_maps_newer_direct_version_to_codex_tool`（断言 `filters.allowed_domains`） |
| `blocked_domains` 非空 → 失败 | `test_hosted_web_search_blocked_domains_fails_closed` |
| non-direct caller | `test_non_direct_web_search_callers_fail_closed` + `test_dynamic_filtering_web_search_requires_explicit_direct_caller` |
| `response_inclusion` | `test_web_search_response_inclusion_fails_closed` |
| 未验证版本 | `test_unknown_web_search_version_is_recognized_but_fails_closed` |
| `max_uses` 非正整数 | **`test_hosted_web_search_non_positive_max_uses_fails_closed` —— 新增；上游此行无测试** |
| （附加）Codex OAuth `max_uses` 无 forced hosted tool | `test_codex_auto_hosted_web_search_max_uses_fails_closed` |

新增测试解构 `ProxyError::InvalidRequest` 并断言逐字消息（`0/-1/1.5/"3"/[1]`，及显式 `null` 保持不限），不比对 `to_string()` —— fork 本地化了 Display 前缀（`无效的请求: `）。这是本批唯一命中的 fork API 差异。

### markdown 有界性审计
**无无界递归、无指数回溯 —— 红线未触**。子代理提取文件内调用图做环检测：DAG，无自递归与互递归；`markdown_bracket_pairs` 用显式 `Vec` 栈而非调用栈。文件内无 `fs`/`net`/`process`/`tokio` 引用（仅 serde_json + log），全部只读扫描 + 掩码/`Vec` 构造。

主会话独立复核：26 个 markdown 家族 fn，自递归 NONE，调用图 cycles NONE（DAG），`std::fs|std::net|std::process|tokio::|reqwest` 计数 0。

**但发现 4 个 O(n²) shape（非指数，×4/倍跨三次倍增稳定）→ 转入 W2.5 加固**，根因已逐条定位（见 W2.5 小节）。

### 测试
`transform_responses` 116 通过（fork 基线 76 + 上游 39 + 新增 1）。集合比对确认 39 个上游测试名全在，**零断言删改、零 mock 适配**（上游测试对 fork API 原样编译）。

### 门禁（全绿）
- cargo fmt / format:check / typecheck ✓
- **check:web-routes 292 commands / 280 routes / 0 gaps —— 计数不变** ✓
- check:locales 2637 parity ✓
- desktop + web cargo check ✓
- `cargo test --lib proxy::` **1045**（1005 + 40）✓
- `cargo test --lib` **2149 passed / 5 ignored**（基线 2109 → +40）✓
- test:unit 173 files / 1044 tests ✓；Rust parity 37 ✓
- clippy 无新 lint（仅 2 个预期 dead-code 警告）
- 上限逐条确认不变（128 MiB body / 2s deadline / 16 MiB heap / 256 KiB stack / 32 MiB catalog）；无新出站目标；无新命令；延期栈仍缺席

### 预期 dead-code（W3 清零）
仅 2 个，均为 handlers.rs 入口：`anthropic_web_search_tool_name`、`anthropic_web_search_max_uses`。未加 `#[allow(dead_code)]`。
其余 4 个原语 + `responses_to_anthropic_with_web_search_options` 已有文件内调用方（经 `responses_to_anthropic` → `_with_web_search_name` → `_with_web_search_options`），故不告警。**W3 结束若此处非零警告，即有 hunk 漏移。**


## W2.5 结果（2026-08-24，860b0523）

引用去重分析二次复杂度加固完成（fork 侧硬化，上游 `bdeaac75` 无对应实现）。1 文件（+323/−23）。

### 五个上限（每个对应一个实测的二次维度，缺一即留下未受约束的维度）
| 常量 | 值 | 根因 |
|---|---|---|
| `MAX_CITATION_DEDUP_TEXT_BYTES` | 32 KiB | 粗粒度兜底；同时约束 code fence / reference-definition 扫描 |
| `MAX_CITATION_DEDUP_LINK_CANDIDATES` | 256 | 每个 `](` 经 `markdown_inline_destination_end` 在未闭合目标上扫到文本末尾 |
| `MAX_CITATION_DEDUP_AUTOLINK_CANDIDATES` | 2048 | 每个 `<` 做 `text[opening+1..].find('>')`，无 `>` 时扫到末尾 |
| `MAX_CITATION_DEDUP_BACKSLASH_RUN` | 64 | `is_markdown_escaped` 的 `.rev().take_while(...)` 每次回走整段连续反斜杠 |
| `MAX_CITATION_DEDUP_CITATIONS` | 256 | 每条 citation 各做一次全文扫描，条数只受 128 MiB 响应体上限约束 |

预检 `citation_dedup_analysis_is_affordable(text, citations)` 全为 O(text) 只读预扫描，长度上限先行短路 → 预检自身不构成新开销（caps-maxed 实测 56.1 ms → 56.9 ms，差值即预检成本）。

### 跳过路径与 fail-open 论证
`analysis: Option<CitationDedupAnalysis>` 为 `None` 时：(a) `linked_urls` 空集；(b) citation 循环首个 guard 把每条推入 `fallback`，`ranged` 空 → 正文逐字节原样；(c) fallback 链接按原序追加到 `Sources:`。

**此处 fail-open 正确**：去重只是排版优化，不是安全控制。降级代价 = 正文已有链接在 `Sources:` 重复一次（外观）；fail-closed 会丢 citation 或整响应报错，更重。与 W1 Alpha Search 的 fail-closed **不矛盾** —— 后者降级会把搜索 payload 误投到非搜索端点（路由决策）。生产代码已带「请勿改成 fail-closed」注释。

### 套件外计时证据（release，同 n 对比）
| 形状 | n | 字节 | before（无上限） | after |
|---|---|---|---|---|
| `"[a]("`×n | 8192 | 32 KiB | 433 ms | 117 µs |
| `"[a]("`×n | 16384 | 64 KiB | 1.667 s | 17 µs |
| `"[a]("`×n | 32768 | 128 KiB | **6.690 s** | 116 µs |
| `"<"`×n | 32768 | 32 KiB | 56.97 ms | 45 µs |
| `"[" + "\"`×n | 32767 | 32 KiB | 342.9 ms | 60 µs |
| 1000 条 citation | 1000 | 128 KiB | 129 ms | 1.8 ms |
| 全上限打满（合法最坏） | 256 | 32 KiB | 56.1 ms | **56.9 ms** |

×4/倍增长跨三次倍增稳定确认。计时 harness 已移除（`grep Instant::now` 为 0）。据重测校准生产注释两处数字：128 KiB 由「7.4 s」改为「6.7~7.4 s」并补增长曲线；全上限最坏由「约 50 ms」改为「约 57 ms」（原值低估）。

### 测试（+7，transform_responses 116 → 123）
跳过路径（每个只越过一个维度，其余维度断言仍在上限内）：
`..._skips_dedup_for_link_candidate_flood`（`[a](`×257）、`..._skips_dedup_for_autolink_flood`（`<`×2049）、`..._skips_dedup_for_long_backslash_run`（`\`×65）、`..._skips_dedup_for_oversized_text`（>32 KiB）、`..._skips_dedup_for_citation_flood`（257 条）。

反向锚（防止上限被悄悄改小）：
`..._still_dedups_scattered_backslash_runs`（钉住「连续长度而非总数」，预检计数器必须归零）、`..._still_dedups_input_at_half_of_every_cap`（各维度压到上限一半的真实正文必须仍被分析）。

全部断言 `text_with_url_citations` 可观测输出、逐字相等而非 `contains`、**无墙钟计时断言**（避免 CI flake）。共用 helper 的第二条 citation 带 range 0..3，正常路径会就地织成内联链接 → 「跳过」与「正常去重」不可能同时通过。

**变异验证**：预检强制 `true` → 5 个 skip 测试全 FAILED；强制 `false` → 2 个反向锚 + 7 个既有 `text_with_url_citations` 测试 FAILED。文件已从备份逐字节还原（md5 一致）。

既有 14 个 `text_with_url_citations` 测试零改动并全绿（diff 的 23 行删除全部落在生产代码内，测试模块只有新增；`git show` 中 `^-.*fn test_` 计数为 0）。

### 门禁（全绿）
- cargo fmt ✓（接手时工作区生产改动本身有 2 处不符 rustfmt，已修正）
- format:check / typecheck ✓
- **check:web-routes 292 commands / 280 routes / 0 gaps —— 计数不变** ✓
- check:locales 2637 parity ✓
- desktop + web cargo check exit 0 ✓
- `cargo test --lib proxy::` **1052**（1045 → +7）✓
- `cargo test --lib` **2156 passed / 5 ignored**（2149 → +7）✓
- test:unit 173 files / 1044 tests ✓；Rust parity 37 ✓
- clippy 无新 lint
- **dead-code 仍恰为 W2 的 2 个**（`anthropic_web_search_tool_name`/`anthropic_web_search_max_uses`），无 `#[allow(dead_code)]`
- 安全上限逐条不变；无新出站目标；延期栈四文件仍缺席


## W3 结果（2026-08-24，c917d5cf）

W3 hosted WebSearch 响应/流式桥 + 非流式 + usage + docs 完成。5 文件（+6787/−1350）。**本任务最后一批。**

### 落地方式
| 文件 | 规模 | 方式 |
|---|---|---|
| `providers/streaming_responses.rs` | +6173/−1304，64 测试 | `git apply --3way`，2 处冲突均落在 fork 漂移区 |
| `proxy/handlers.rs` | +611/−43，1 上游 + 11 fork 测试 | 手工按符号锚定 |
| `docs/guides/*-{en,ja,zh}.md` | 各 +1/−1 | `--3way` 干净 |

`streaming_responses.rs` **除 4 个既有 fork 漂移 hunk（28 行，逐字符核对与移植前一致）外与上游 `bdeaac75` 字节一致**；函数名集合与上游精确相同。两处 `--3way` 冲突为上游新代码撞上 fork 更简的 `build_anthropic_usage_from_responses` 注释（源自 fork `e506f98a`）→ 取上游代码后重新贴回 fork 注释/换行，**漂移被保全而非被回退**。

`handlers.rs` 8 个上游 hunk全部按符号锚定（上游行号 → fork 行号）：`-27`→`-16`（use tree）、`-46`→`-36`、`-412`→`-294`（`tool_schema_hints` 绑定后）、`-419`→`-301`（`api_format == "openai_responses"` 选择器）、`-525`→`-388`、`-2017`→`-1404`、`-2668`→`-1567`（测试 use 列表）、`-3210`→`-1602`（与上游同一测试对之间）。

`-525` 需手工适配：fork 无 `body_looks_like_sse`/`aggregate_fallback_error`/`upstream_body_parse_error`，故在上游新的 `(headers, direct_anthropic_response, upstream_response)` 重构内保留 fork 较简的 parse 块；区域 diff 确认该内层块之外与上游字节一致。

**`log_usage` 无需改动**：上游 `@@ -2668` hunk 只是测试模块 import 列表，git hunk header 里的 `log_usage` 只是最近的前置函数名。`usage.server_tool_use.web_search_requests` 非流式来自 `transform_responses`（W2）、流式来自转换器的 `message_delta`，经既有 `TokenUsage::from_claude_response` 抵达 `log_usage`。

### Q1 修正（详见 prd.md 裁定记录）
上游 `responses_sse_stream_to_anthropic_message` 调 `transform_codex_anthropic::anthropic_sse_to_message_value`。只把**该 1 函数**按上游 post-`bdeaac75` 态（含 Q1 原欲延期的 `message_delta.usage` 整对象合并）移植为 `handlers.rs` **私有 `fn`**；`providers/` 未新增延期文件（四文件仍缺席，主会话复核）。**变异验证必需性**：回退成 `output_tokens`-only → 移植的 handlers 测试报 `usage.server_tool_use.web_search_requests` = `Null` 而非 `1`。Q1 的「后果可界定」前提在 W3 引入 fork 本地 Anthropic SSE 聚合后失效，已在 PRD 记修正。

### Check A — W2.5 上限未被绕过 ✓
流式引用代码**从不直接调用 markdown 原语**。路径：
```
create_anthropic_sse_stream_from_responses_with_web_search_options → _raw
  → BufferedCitationTextState / StreamedTextState 收集 url_citation 注解
  → render_part_pending(2) | render_message_part(3) | missing_message_text_parts(1)
  → text_with_url_citations   [transform_responses.rs:1411 — W2.5 受保护入口]
  → citation_dedup_analysis_is_affordable  [五个 W2.5 上限]
```
6 个流式调用点全部经受保护入口。主会话独立复核：`streaming_responses.rs` 中 `text_with_url_citations` 出现 7 次，直接调用 `markdown_*`/`contains_markdown_link` **计数为 0**。

**变异证明该 guard 在活路径上**：强制 `citation_dedup_analysis_is_affordable` 返回 `false` → 10 个流式测试失败（buffered-citation 与 hosted-web-search 块测试）。已还原并 md5 核对。

新增流式生产代码：无自递归、调用图无环（DAG）、零 `fs`/`net`/`process` 引用、测试外零 `http(s)://` 字面量。

### Check B — dead-code 归零 ✓
`cargo check --lib` **完全无警告**（主会话复核 warning 计数 = 0）。`anthropic_web_search_tool_name`/`anthropic_web_search_max_uses` 在 `handle_claude_transform` 有真实调用方。未加 `#[allow(dead_code)]`。web-server example 的 70 个警告全为既有 desktop-only dead code，零个涉及 W3 触碰文件。

### 测试（+76）
`cargo test --lib proxy::` 1052 → **1128**；`cargo test --lib` 2156 → **2232**。
- **64** 个流式测试 —— 上游实际增量（其自身测试数 21 → 85；规划写的「66」略有偏差）。**零 mock 适配、零断言改动**，上游测试对 fork API 原样编译。
- **1** 个 handlers 测试 `non_streaming_codex_web_search_limit_stops_polling_upstream`，与上游字节一致，含 poll 计数（3 chunk 中取 2 → 提前取消生效）与 `max_uses_exceeded` 断言。
- **11** 个 fork 测试覆盖 fork 本地聚合器。其中 10 个改编自上游 `transform_codex_anthropic` 套件（3 个原经延期的 `anthropic_response_to_responses` 断言，改为直接断言聚合后消息）；1 个为新增：`message_delta` 里的 0 不得覆盖 `message_start` 的非零 usage —— 变异验证（去掉守卫后失败，`left: 0, right: 31`）。

### docs（三语第 96 行）
按上游改写落地，每条主张都能映射到已落地且有测试钉住的代码：`web_search` 翻译与 `allowed_domains`（W2）、`max_tool_calls` 映射（`transform_responses.rs:2042`，测试 3684）、Codex OAuth instruction cap（`:2029`）+ 桥端停流与 `max_uses_exceeded`（`streaming_responses.rs:2071`，5 个 limit 测试）、非强制 fail-closed（`:2020`）、`blocked_domains` fail-closed（W2）。**未描述任何 Codex-Chat-bridge 行为**，与 `transform_codex_anthropic.rs` 缺席一致。

### 全量门禁（全绿，含 PRD 验收 build:web + smoke）
- cargo fmt / format:check / typecheck ✓
- **check:web-routes 292 commands / 280 routes / 0 gaps —— 计数不变** ✓
- check:locales 2637 parity ✓
- desktop cargo check exit 0 / **0 warnings** ✓；web-server example exit 0 ✓
- `cargo test --lib proxy::` **1128 passed / 0 failed** ✓
- `cargo test --lib` **2232 passed / 0 failed / 5 ignored** ✓
- test:unit 173 files / 1044 tests ✓；Rust parity 37 ✓
- test:integration **50/54** —— 恰为 4 个 PRD 已知 flake（ProviderList empty-state 1、SkillsPage repo/fixture 3）✓
- **build:web exit 0** ✓；**smoke:web-server exit 0** ✓
- clippy 仅 2 处既有警告
- 安全上限逐条不变（128 MiB body / 2s / 16 MiB / 256 KiB / 32 MiB + W2.5 五个 citation 上限）；无新出站目标；延期栈四文件仍缺席

首轮 test:integration 出现 5 个失败，后两轮为 4 个；多出的一个仍在既有 flaky 的 `SkillsPage.web-server` 文件内，失败从未越出两个已知 flaky 文件，且二者均不触碰 proxy 代码。


## Phase 2.2 trellis-check 结果（2026-08-24，ff067851）

最终全范围 check 判定 **PASS，无阻塞项**。发现并修复 **2 个真实缺陷**。

### 缺陷 (a) — `rewrite_codex_alpha_search_full_url` 在 dot-segment URL 上 panic / 静默误派生
`suffix` 取自 `Url::parse(..).path()`（会归一化 dot segment），但切片作用在**原始**字符串上。二者错位：

| 输入 | parsed path | upstream 行为 |
|---|---|---|
| `https://h/responses/éxxxxxx/..` | `/responses/` | cut = 31−10 = 21，byte[21] = `0xa9`（在 'é' 的 20..21 内）→ **panic** |
| `https://h/responses/x/..` | `/responses/` | prefix = `https://h/resp` → **静默误派生** |

主会话独立复核算术：以真实 suffix（`/responses`，trim 后长 10）计算 cut = 21，byte[21] = `0xa9` 确为 UTF-8 continuation byte，与子代理实测 panic 消息「index 21 is not a char boundary; inside 'é'」精确一致；ASCII 变体前缀 `https://h/resp` 亦复现。

**静默误派生是更坏的一半**：本函数契约是「只在无歧义时派生，否则拒绝」，因为猜测会把搜索 payload 投到非搜索端点。改为 `strip_suffix`（天然对齐字符边界），错位形状落回既有 fail-closed 分支；所有当前可用形状（`/v1`、`/backend-api/codex`、`%2F` 前缀、尾随 `/`、query、fragment）行为不变。新增 `alpha_search_rejects_full_url_whose_raw_form_does_not_end_with_responses`。

**该代码与上游 `bdeaac75` 字节一致 → 缺陷同样存在于上游**，值得回报上游。严重度低（operator 提供的 provider 配置，非远端输入；影响面为一次请求失败），但正落在本任务所断言的契约内，且修复只是把 fail-closed 收得更紧。

### 缺陷 (b) — `modelsDevAutoSync.test.ts` 1 ms 边界 flake
`lastSyncAt = Date.now() - INTERVAL + 1` 只留 1 ms 余量，与实现自身的 `Date.now()` 竞争 → 并行套件负载下失败（首轮 test:unit 1/1044 失败，隔离 3/3 通过）。非本任务引入（文件未触碰，源自 `0ddce4e6`）。**上游已修**（60 s 余量 + 注释）→ 移植上游原样行，**消除 fork 漂移而非新增**（已核对与 `v3.20.0:tests/lib/modelsDevAutoSync.test.ts` 第 86–88 行字节一致）。

### 七条红线（逐条独立取证，非复读报告）
1. **延期栈缺席** ✓ — 四文件均不在 `providers/`；`providers/mod.rs` 无对应 `mod`；`anthropic_sse_to_message_value` 为 `fn`（无 `pub`）于 `handlers.rs:1524`，仅 `handlers.rs` 内引用；`proxy/mod.rs` + shim 零 diff，example mirror 测试在 37 内通过。
2. **无新命令** ✓ — 292/280/0 计数不变；`src/lib/api/web-commands.ts` 在 `258245f4~1..05e9e209` 区间 **0 diff**；改动仅 5 个 `proxy/*` + 3 docs。
3. **安全上限不变** ✓ — 128 MiB / 2s / 16 MiB / 256 KiB / 32 MiB + 五个 `MAX_CITATION_DEDUP_*` 均在文档值。
4. **W2.5 上限约束所有引用路径** ✓（三法取证）— grep：`streaming_responses.rs` 与 `handlers.rs` 中 `markdown_*`/`contains_markdown_link*` 计数均为 **0**；调用图（62 个生产 fn）：唯一进入 markdown 家族的非 markdown 函数是 `text_with_url_citations`，两个昂贵入口均在 `citation_dedup_analysis_is_affordable(...).then(...)` 门内；变异：强制预检 `false` → 10 个流式测试失败。
5. **fail-closed / fail-open 双向完好且未互换** ✓ — Alpha Search 的 `?` 在构造 `url` 期间求值，拒绝时**未发出任何请求**；citation 超限时正文逐字节不变且全部 citation 仍在 `Sources:`。
6. **SSRF 面不变** ✓ — 5 个文件生产代码零新增 `http(s)://` host 字面量（仅 `forwarder.rs:1792` 一处注释）；diff 中 200+ URL 全为测试 fixture。**一处口径修正**：`ip_guard` 挂在 `http_client::get_guarded()` 与 `web_api` handler 出站，proxy forwarder 热路径按 *Web Outbound SSRF (scope C)* 契约刻意不设 guard → 准确表述是「**无新出站路径**」，而非「出站现在过 ip_guard」。
7. **不可信 markdown 有界** ✓ — 26 个 markdown fn 无自递归、调用图无环（DAG）；`transform_responses.rs` 中 `std::fs|std::net|std::process|tokio|reqwest` 计数 0。

### 移植保真度（check 独立核对，非采信报告）
- `streaming_responses.rs` 与上游 `bdeaac75` 差 **恰 28 行**（即 4 个既有漂移 hunk）。
- `transform_responses.rs` 在 W2（`a49e7af5`）后与上游差 118 行 / 16 hunk，其中**生产代码分歧仅** `build_anthropic_usage_from_responses` 的文档/注释/换行漂移（78 行，与移植前基线一致），其余为 2 个 fork 独有测试。无未解释的生产分歧。
- Q1 修正与代码一致 ✓，且 check 重新变异验证：回退合并 → 2 个 handlers 测试失败（`web_search_requests` = `Null`；fork 的 zero-clobber 测试）。还原后 28/28 通过。

### 门禁（全绿）
- cargo fmt / format:check / typecheck exit 0
- **check:web-routes 292 / 280 / 0 gaps —— 计数不变** ✓
- check:locales 2637 × en/ja/zh parity ✓
- desktop cargo check exit 0 / **0 warnings**（强制重编译确认 W2 dead-code 已清零）；web-server example exit 0（69 既有 desktop-only 警告，零个在触碰文件内）
- `cargo test --lib proxy::` **1129**（1128 + 1 新回归）✓
- `cargo test --lib` **2233 passed / 0 failed / 5 ignored** ✓
- test:unit **173 files / 1044 tests 全绿**（修 flake 前为 1 失败）✓
- Rust parity **37** ✓
- test:integration **50/54** —— 恰 4 个 PRD flake ✓
- **build:web exit 0**；**smoke:web-server exit 0** ✓
- clippy 仅 2 处既有（`services/omo.rs:958`、`prompt_files.rs:37`）

### 非阻塞观察（记录备查）
- `MAX_CITATION_DEDUP_TEXT_BYTES = 32 KiB` 意味着超过 32 KiB 的**合法**长回复会失去内联引用编织、退为 `Sources:` 列表 —— 相对上游在长输出上的真实（仅排版）分歧。按 W2.5 实测，256 候选 × 128 KiB ≈ 116 ms，若实践中出现可上调。暂不动作。
- `handlers.rs:420` 的「压缩 Codex SSE 无法强制 max_uses」fail-closed 分支无测试；上游亦无，handler 级覆盖需完整 `ProxyState` 脚手架。

## Phase 3.3 spec 更新（2026-08-24）

`.trellis/spec/frontend/quality-guidelines.md`（32 → **34** Scenario）：
- **Upstream Desktop Sync Into Web Fork** 增补两条移植红旗：(1) 禁止用 `Url::parse(..).path()` 派生的长度去切原始 URL 串（dot-segment 归一化导致错位 → panic 或静默误派生；用 `strip_suffix`/`strip_prefix`），并注明该类缺陷是从上游逐字带入且上游无对应测试；(2) 异步测试禁止对 `Date.now()` 设 ~1 ms 边界余量（上游对 `MODELS_DEV_STARTUP_SYNC_INTERVAL_MS` 采用 60 s）。
- **新增 Scenario: Degradation Direction — Fail-Closed vs Fail-Open Guards**（7 段完整）：以「降级的代价是什么」为分类依据 —— 路由/投递/授权决策 → fail closed；表现层/优化 → fail open；两者可并存不得统一；必须带「勿反转」注释；fail-open 不得丢用户可见 payload；两方向都要测试且需变异验证。
- **新增 Scenario: Deferred Upstream Stack — Private-Helper Exception**（7 段完整）：延期边界是机械判据（延期目录无新文件、无新 `mod`），而非「我们不调用那个栈」（后者会在引入消费方的那一刻失效）；单函数可作为**私有** fn 落在唯一消费方内；按上游 post-commit 态移植（含原裁定曾豁免但对消费方 load-bearing 的子 hunk）；必需性须变异验证；前提失效时**原地修正裁定并保留失效前提可审计**。

## 验证命令汇总

见每批门禁块。关键额外检查：
- **无新命令**：`pnpm check:web-routes` 保持 292 commands / 280 routes / 0 missing/mismatch/dangling/fallback。计数变化 → 误把 proxy 路由当 Web API 命令注册。
- **安全上限零退化**：`grep` 确认 128 MiB body cap（`hyper_client.rs:128` `MAX_RESPONSE_BODY_BYTES`）、2s JS deadline（`usage_script.rs:9`）、16 MiB heap（`usage_script.rs:10`）、256 KiB stack（`usage_script.rs:11`）、32 MiB catalog（`codex_config.rs:1287`）不变。
- **SSRF 面不变**：新代码无新增出站目标；出站仍经 forwarder + `ip_guard`。
- **不可信 markdown 有界**：无自递归、无指数回溯；**且经 W2.5 加固后二次复杂度受候选数/长度上限约束**（病态输入在上限内完成且 citation 不丢）。
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
