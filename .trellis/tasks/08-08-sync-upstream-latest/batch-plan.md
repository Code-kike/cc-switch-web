# Batch plan — sync v3.18.0..v3.19.2 + 413c09e0

方式:selective port(直接 merge 有 435 冲突文件,不可行)。逐批 cherry-pick + Web 适配,每批过全量门禁后单独 commit。

## Excluded(桌面发布链,PRD 默认排除)
- 414b7150 ci: mirror release assets to R2
- 708b3879 ci: sha256 + publish date in download manifest
- 2b2f2cfa ci: mirror in-app updater to R2

## S1 docs/presets/sponsors/i18n(全量原样,grilling 裁定 #2)
3a9fb13a 846fbdd1 a377d793 876e9f89 b0482320 b972f0a3 bc7c8222 934a2d03 ccda04bf 993077c6 dbb26595 bfb767ae 30409878 ebbf141f 996d512f 4d3e2c35 0e604b75 5b697abc 290b65c0 b884595a a354f08a 245d180c 134bdc0e b33d300d 87b0e3fb 6b13d018 3b9d0593 c0ff89b9 b3a20e58 28529620 a4bba43f fbf52cff 425e932b 43eaf073

## S2 security fixes(优先)
ff3bc242(zip-slip/credential leaks/panics) c98913df(SQL import cross-file) 35486afd(POSIX quoting) cd17912f(prototype pollution) 6b8f3643(caps:scripts/file-reads/proxy bodies)

## S3 deeplink risk classification
6dbb944b a443eae9 19bf236e cfa90f39

## S4 proxy/bridges
6c9d444c 878c26f3(tool-result media) 13ea497a(Copilot compat) 9db9c56f(dropped tool calls) 4317bd99 3c1154be(refactors/dead code)

## S5 Grok(含上次延期项)
15d5dbe0(官方订阅配额) 3cf84ca3 cd161f44 34cbb375(Grok Build usage) c49cf96a(proxy+deeplink integrations) f07edc76(GUI upgrade)

## S6 pricing/usage/backup
9cf4ae41(Opus 5 pricing) 12b972a6(models.dev auto sync) f42534ed f38722a4(Qwen3.8 Max) 56fb46c0(rollout timeline cache) 4bfb3fc3(dedupe) 59a2bd10(interleaved counters) baf07a27(batch inserts) 668bbda9(backup single-txn ⚠ SQL-restore 锁测试)
⚠ FE-preset↔Rust-seed 定价一致性核对(cost=$0 陷阱)

## S7 codex Responses/auth/catalog
8ae1ce85(DeepSeek native Responses) 56a66eea(Volcengine) 58dd376d(Hunyuan) e3f80a98(stale auth clear) 492245dc(OAuth per-account usage, Auth Center) 413c09e0(respect user model_catalog_json)

## S8 opencode/OMO
92ca95ff(runtime models) 0345fad6(unified OMO config)

## S9 UI/management/skills/misc
f5f4281d(icon-only switcher) 9f19d8fd(searchable lists + bulk toggles) 968794e3(idle GPU) 0cb6e014(header actions) eb356e15 40b6376b(skills source dir/readme_url) 83830767(Hermes SOUL.md)

## 每批门禁
source ~/.cargo/env; cargo fmt --check; pnpm format:check; web cargo check; pnpm check:web-routes; pnpm check:locales; pnpm test:unit; pnpm test:integration(4 个已知环境 flake 不阻塞); cargo test web_api:: dual_runtime_parity:: web_proxy_lifecycle::

## S1 结果(2026-08-08,e16f3dd4)
26 ported / 8 skipped。跳过:876e9f89、934a2d03、5b697abc(重排序/对 fork 空操作)、bfb767ae(fork 无 Code0/Qiniu gemini 预设)、b884595a(fork 已移除 zh-TW)、c0ff89b9(上游 CHANGELOG 文体不适用)、245d180c(fork 无 user-manual 树)、87b0e3fb(前置为 ff3bc242 的 skill.rs TempDir 重构 → 移入 S2 末尾重试)。
判断项:模型升级(Opus 5/GPT-5.6 Sol)同步应用到 fork 自有 provider(ctok/lionccapi/lemondata);CHANGELOG 用 fork 短条目格式。
门禁:test:unit 741 通过;desktop+web cargo check 通过;format/locales/web-routes 全绿。

## S2 结果（2026-08-08，9b2bb754）

已移植 `ff3bc242`、`c98913df`、`35486afd`、`cd17912f`、`6b8f3643`，并在
`ff3bc242` 落地后成功重试 `87b0e3fb`。语义适配：

- SQL import 保留 fork 的 canonical-schema allow-list、规范化复制和 backup/replace
  锁边界，同时显式拒绝/记录 ATTACH（含 VACUUM INTO）、vtable、未知动作与越界
  PRAGMA；新增 ATTACH/VACUUM、真实导出 round-trip 回归。
- Product upstream 的独立 `deepClone.ts` 在 fork 不存在；等价保护移植到
  `providerConfigUtils.ts` 的内联 clone，并保留 merge/remove/subset 三层原型污染保护。
- `6b8f3643` 保留 managed file-write、Web SSRF/no-auth 与双运行时边界；JS runtime
  沿用 fork 更严格的 2 秒 deadline，并补 16 MiB heap/256 KiB stack；Codex catalog
  同时做 lexical+canonical symlink containment 与实际流式 32 MiB read cap；proxy raw/
  decompressed body 设 128 MiB cap，并额外修复 deflate TooLarge 不得降级到 raw fallback。
- Deeplink token-only import 现在也展示 masked usage access token 与 user id；zh-TW hunk
  因 fork 已移除该 locale 而为已证实 no-op（locale gate 只包含 en/ja/zh）。
- `session_usage_grokbuild.rs` hunk此时不可应用：S5 的 `cd161f44/34cbb375` 尚未引入该
  模块，HEAD 无 module declaration/调用点。S5 引入文件时必须以 `6b8f3643` 版本为
  安全上限，保留 50 MiB file cap、16-level traversal cap 与 skip-all-symlinks。

门禁：cargo fmt/desktop cargo check/web-server cargo check、Prettier/typecheck、
web-routes/locales、747 frontend unit、SQL restore/skill/proxy/catalog/usage-script focused
Rust tests、web_api 26、dual_runtime_parity 3、web_proxy_lifecycle 7 全绿。
`test:integration` 46/50 通过；仅命中 PRD 已知 4 个环境 fixture flake：ProviderList
import-current 1 个、SkillsPage seeded-default-repos/fixture discovery 3 个，无新增失败。

S2 check follow-up `a3a44b60`：把 GrokBuild 用量 `base_url` 修复落到 fork 实际调用的
`services/provider/usage.rs`，移除未调用的重复解析器，并补齐 Web 启动时 Gemini 泄漏
凭据 scrub 与 lifecycle 顺序断言。

## S3 结果（2026-08-08，7fea1cba..5ffc3393）

按序移植 `6dbb944b`、`a443eae9`、`19bf236e`、`cfa90f39`，无跳过提交：

- `7fea1cba` 新增纯函数风险分类：进程加载环境变量、私网/metadata endpoint、shell
  command+args；输入均按不可信 `unknown` 处理，并统一敏感值遮罩。
- `4bda59d9` 在 MCP 确认页逐项完整展示 command/args/url/env，风险逐行及汇总标记；
  provider config env 与 endpoint 同样标记；MCP 立即写入警告不再受链接作者可控的
  `enabled` 字段影响。`DeepLinkImportDialog.tsx` 冲突以 fork 的 Web paste/error handling、
  S2 usage 字段展示为基底逐块合并；已删除的 zh-TW locale hunk 为证实 no-op。
- `f1a8e512` 前端 Base64 解码与后端四种 alphabet 语义对齐，支持 URL_SAFE/NO_PAD；
  冲突保留 fork 的 `TextDecoder` fallback（不恢复 deprecated `escape/unescape`）。
- `5ffc3393` 用量脚本默认禁用，确认页完整显示可执行 JavaScript 与无条件警告；保留
  S2 的“任一 usage 字段均可见”外层 gate、token/user-id 展示及 `=== true` 徽章语义。
  冲突同时暴露并补回 v3.18.0 祖先 `d1f6c74b` 的显式 usage credential override 语义：
  provider key/endpoint 不重复持久化，只有 distinct override 经 trim/去尾斜杠后保存；
  新增 3 个回归测试。未新增 Tauri command 或 Web route。

新增/更新定向覆盖：`deeplinkRisk` 16、Base64 8、DeepLink dialog 7（含 MCP args/env
风险与脚本正文）、Rust `deeplink::tests` 33、`deeplink::provider::tests` 7，全部通过。

门禁：cargo fmt、Prettier、desktop/web cargo check、typecheck、web-routes（277 commands，
0 missing/mismatch/fallback）、locales（en/ja/zh 各 2466）、frontend unit（140 files / 773
测试）、Web `web_api::` + `dual_runtime_parity::` + `web_proxy_lifecycle::`（36）全绿。
`test:integration` 首轮 44/50（OpenCode/OpenClaw 两个同文件时序 flake）；ProviderList
隔离复跑 9/10 后确认两者通过；全量复跑稳定为 46/50，仅剩 PRD 已知 4 个 fixture
失败（ProviderList Claude official-seed empty-state 1、SkillsPage default-repo/fixture 3）。

## S4 结果（2026-08-08，3fa5ddbf..883f934a + 4 个未提交 Web 适配文件）

按序处理全部 6 个 Product commits；5 个形成正常 cherry-pick commit，1 个经证实为
整提交不适用：

- `6c9d444c` → `3fa5ddbf`：移植共享 `tool_media` walker、Responses input/tool-output
  sanitizer、Codex native Responses 反应式降级及定向回归。冲突以 fork 的
  `ProxyRuntimeCtx`、128 MiB body cap、双运行时 forwarder 为基底；上游
  `transform_codex_chat.rs` hunk 未恢复（该模块在 fork HEAD 不存在）。
- `878c26f3` → `64ba352c`：把媒体恢复扩展到 fork 实际存在的三组桥：
  Anthropic↔OpenAI Chat、Anthropic↔Responses、Claude↔Gemini；保留 Gemini shadow
  id/signature 与 usage 计费修复。`transform_codex_chat.rs` 和
  `transform_codex_anthropic.rs` 两个 modify/delete hunk 不适用：两者及其共同基础
  `codex_chat_common.rs`/`streaming_codex_chat.rs` 均不在 HEAD，且 providers/mod.rs
  无 module declaration。
- `13ea497a` → `213eaab7`：Copilot 默认使用无确认弹窗的 AUTH_TOKEN placeholder，
  仅显式 `meta.apiKeyField=ANTHROPIC_API_KEY` 时使用 API_KEY；模型 live-resolution
  后移除第三方模型的 `[1M]` marker，同时保留 fork 的 managed-auth 单键约束、
  runtime context 与原子 live writer。`copilot` 120 个定向测试及 takeover 10 个测试通过。
- `9db9c56f`：**整提交跳过（proven inapplicable）**。`git show --name-status` 证明
  唯二目标为 `streaming_codex_chat.rs` 与 `transform_codex_chat.rs`；`git ls-tree HEAD`
  两者均不存在，`src-tauri/src/proxy/providers/mod.rs` 无声明，且
  `handlers.rs` 明确记录 Codex Chat routing stack 尚未移植。因此 fork 没有该提交所修复的
  Chat→Responses “丢掉无名 tool call 后伪 completed”执行路径，也无可挂接的 dropped-call
  regression；恢复两个叶文件会缺少整套约 5.7k LOC 的既有延期基础，属于越过 S4 边界。
- `4317bd99` → `9aec867b`：统一 `proxyKeys` 和 proxy status/takeover query，保留
  `src/lib/api/adapter`（不回退成直接 Tauri invoke）及 Web event 监听。集成门禁暴露
  app-config 保存会用旧快照回写独立 failover toggle；未提交 follow-up 在
  `src/lib/query/proxy.ts` 以 dedicated toggle query 为字段 owner，并在
  `tests/hooks/useProxyQueryHooks.test.tsx` 增加防回退回归。
- `3c1154be` → `883f934a`：删除 2,576 行已取代死代码/依赖；语义冲突中保留仍被 fork
  `UsageDashboard` 使用的 `DataSourceBar` 和 `ProviderCard` 使用的
  `HealthStatusIndicator`，删除无引用的 `PromptFormModal`/旧 schemas。最终 parity gate
  证明 proxy `health` 模块也应删除；未提交 follow-up 同步删除
  `src-tauri/src/proxy/health.rs` 与 Web shim include，并保留新增 `tool_media` shim module。
  已删除的 zh-TW locale hunk仍为 no-op，三现役 locale 维持严格同构。

未新增 Tauri command/Web route。最终门禁：

- `cargo fmt --check`、desktop `cargo check`、Web example `cargo check` 全绿（Web 仅既有
  standalone shim dead-code warnings）。
- Prettier、TypeScript、web routes（277 commands；0 missing/mismatch/fallback）、locales
  （en/ja/zh 各 2441）全绿。
- frontend unit：140 files / 775 tests 全绿。
- focused Rust：tool_media 15、media_sanitizer 32、Anthropic↔Chat 77、Responses 76、
  Gemini 32、Copilot 120、managed takeover 10，全部通过。
- Web Rust：`web_api::` 26、`dual_runtime_parity::` 3、`web_proxy_lifecycle::` 7 全绿。
- `test:integration` 最终 46/50；仅 PRD 已知 4 个 fixture flake：ProviderList
  official-seed empty-state 1 个、SkillsPage default-repo/fixture discovery 3 个。S4 新暴露的
  ProxyTab failover owner 回写失败已修复并隔离/全量复跑通过。

残余风险：Codex Chat/Anthropic routing stack 仍是既有延期项，因此 `9db9c56f` 与
`6c9d444c`/`878c26f3` 对该缺失 stack 的叶 hunk仍未落地；若未来单独恢复该 stack，必须从
完整依赖基线移植并重跑 dropped-tool-call 流/非流回归。当前 4 个未提交适配文件需由 parent
作为 S4 follow-up commit 提交；本批不进入 S5。

## S5 结果（2026-08-09，Grok usage/quota follow-up）

### 提交映射与处置

- `15d5dbe0` → `96052739`：已移植。加入 Grok 官方订阅额度查询、xAI OAuth
  managed quota、Web API/command parity、ProviderCard/UsageScript/tray 路径及
  重试/过期语义；保留 fork 的无认证 Web 姿态与 adapter 路由 SSOT。
- `3cf84ca3` → `99f66eaf`：已移植。把 proxy logger、calculator 与 usage backfill
  统一到 `is_cache_inclusive_app`，将 `grokbuild` 纳入 TOTAL 输入语义，避免缓存读
  重复计费并补回填回归。
- `cd161f44` → `3c8f910a`：已移植并按安全上限适配。新增 Grok native
  `updates.jsonl` session importer，支持 turn-level idempotency、reported ticks
  优先级、partial-cost fallback、settle/takeover 去重窗口；保留 **50 MiB 单文件读
  上限、16 层遍历上限、目录/文件/链路全部跳过 symlink**。
- `34cbb375` → `4be656ae`：已移植。加入 `grok_session` data-source UI、过滤器与
  breakdown 文案；en/ja/zh 三个保留 locale 同构（已删除的 zh-TW 不恢复）。
- `c49cf96a` → `a5794a2f`：已移植并适配 fork 实际的 Grok proxy/session/deeplink
  stack，包含安全预览、native session 识别、环境变量检查及 Web integration 稳定化；
  未恢复 fork 不存在的桌面/CLI 生命周期表面。
- `f07edc76`：**排除（proven inapplicable）**。fork 已刻意移除 CLI lifecycle
  GUI/installer commands；恢复会新增未经认证的 Web installer surface，或只能暴露一个
  明知拒绝的伪 parity route，均违反本 fork 的 scope/security 边界。该提交不进入 S5，
  也不延期到 S6。

### 冲突、no-op 与额外适配

- 上游提交中的桌面专属 updater/release/installer 路径没有恢复；Web-only 行为均通过
  `web-commands.ts` 与 Axum route 对齐，`check:web-routes` 无 missing/mismatch/fallback。
- `a5794a2f` 使 failover editor 只有在对应 app takeover 生效时可写；原 integration
  fixture 只启动共享 proxy，未启用 Claude takeover，因而补了最小测试前置步骤。该测试
  适配未提交，留 parent review。
- `ProviderCard.test.tsx` 原本未 mock 新增的 `XaiOauthQuotaFooter`，导致无
  QueryClient 的测试 harness 错误；补了最小 mock，未提交，留 parent review。
- `src-tauri/src/services/provider/mod.rs` 的 Claude common-config extractor 原先
  只剥显式字段，`OPENROUTER_API_KEY` 与顶层 `apiKey/api_key` 会泄漏到共享片段；现复用
  已有 `is_sensitive_config_key` 对 env/顶层统一过滤。该安全修复未提交，留 parent
  review；不是将失败静默标为环境 flake。
- 未发现 S5 需要新增 Tauri command 的 no-op；既有 Web adapter/route 是唯一注册源。

### 强制 importer / pricing 回归

- importer 回归覆盖：turn_completed-only、逐 turn 原值累计（不做相邻差分）、空
  prompt-id index fallback、rewind stable UUID、rescan idempotency、10 分钟 settle
  window、proxy activity guard、reported `costUsdTicks`（1 tick = 1e-10 USD）与
  priced/unpriced/partial cost 分支。
- 安全上限回归明确通过：超 50 MiB `updates.jsonl` 在读取前跳过；遍历超过 16 层停止；
  directory symlink 与 file symlink 均不跟随。
- cost backfill 回归确认 `grokbuild` TOTAL 行扣除 cache-read；seeded Grok 4.5
  Build pricing 与 CLI ticks 样本一致，未出现 cost=$0。

### S5 门禁证据

- `source ~/.cargo/env; (cd src-tauri && cargo fmt --all -- --check)`：通过。
- Desktop `cargo check --all-targets --locked`：通过；最终 `cargo test --lib
  --locked`：**1874 passed / 0 failed / 2 ignored**。Claude common-config security
  focused test：**1 passed**。
- Web `cargo check --locked --no-default-features --features web-server --example
  server`：通过（67 个 standalone shim dead-code warnings，仅预期警告）。Web example
  `web_api::` + `dual_runtime_parity::` + `web_proxy_lifecycle::`：**36 passed / 0
  failed**。
- Rust S5 focused：subscription Grok **14**、session importer **19**、Grok backfill
  **1**、`grokbuild` proxy/cost/session/deeplink aggregate **47**、deeplink namespace
  **41**，均通过。
- `pnpm format:check`、`pnpm typecheck`：通过；`pnpm check:web-routes`：**278
  commands / 266 routes / missing 0 / methodMismatch 0 / parityFallback 0**；
  `pnpm check:locales`：en/ja/zh 各 **2445 keys**，严格 parity。
- S5 focused frontend：**18 files / 107 tests passed**；full `pnpm test:unit`：
  **142 files / 782 tests passed**。
- `pnpm test:integration`：**46/50 passed**。仅以下 PRD 已知四个 fixture flakes
  非阻塞：ProviderList 的 Claude official-seed empty-state/import-current 1 个；
  SkillsPage 的 default-repository/fixture discovery 3 个（automatic skills.sh
  fallback empty-state、repo skill install/update、skills.sh pagination install）。
  初次 45/50 另有 ProxyTab failover-control failure；补 takeover 前置后隔离与全量
  重跑通过，不计为 flake。

### 残余风险与工作树说明

- 已知四个 integration fixture flakes 仍是环境/fixture 时序问题；本批没有新增产品
  失败。Web example standalone dead-code warnings 与 Vitest 的 baseline-browser-mapping/
  jsdom `window.scrollTo` 诊断均不影响结果。
- 上述三个额外适配（security extractor、ProviderCard mock、ProxyTab takeover
  fixture）均保持未提交，供 parent review/合并；本 agent 未 commit/amend/push/merge，
  未进入 S6。
- 名为 **`s5 usage backfill test adaptation for fork API`** 的 redundant stash 仅做了
  read-only `git stash show` 检查，未 pop、drop、apply、修改或重建；其 patch 已由既有
  `99f66eaf` 覆盖。

## S6a progress（2026-08-09，pricing only）

### 提交映射与边界

- `9cf4ae41` → `19ecbf4e`：加入 `claude-opus-5` 内置价格
  `5/25/0.50/6.25`；冲突中保留 fork 已有 Fable/Mythos 行。
- `12b972a6` → `0ddce4e6`：移植 models.dev 自动同步、本地
  `model-pricing.json` override/tombstone、批量更新与上游单元测试。冲突保留 fork 的
  usage API 参数形态、Web adapter/event imports、异步非阻塞启动和 en/ja/zh locale
  集合；恢复该功能依赖的 picker 及其测试，已删除的 zh-TW 保持 no-op。
- `f42534ed` → `28edd813`：同步 GPT-5.6、DeepSeek alias、MiniMax M3 vendor list
  价格及 repair guards，加入 bare Claude 4.6、Codex Spark、Gemini Flash Lite、Kimi
  HighSpeed、GLM Turbo、Qwen3.6 Flash seeds；冲突保留 fork 自有 coding alias rows 和
  fork 较新的 GLM/MiMo 定价，并为 fork 既有 `qwen3.6-plus` 零 cache-read seed 增加
  `0 → 0.065` guarded repair，避免覆盖用户自定义价。
- `f38722a4` → `58ac544a`：加入 `qwen3.8-max` 内置价格
  `2/6/0.25/2.50`。
- 已确认停在 `58ac544a`；`56fb46c0` 未进入历史。本子阶段未触及 usage timeline、
  session insert 或 backup/restore。

### 定价映射与 parity 证据

- Rust seed 目标值：Opus 5=`5/25/0.50/6.25`；Qwen3.8 Max=`2/6/0.25/2.50`；
  GPT-5.6 Luna=`0.20/1.20/0.02/0.25`；Terra=`2/12/0.20/2.50`；
  DeepSeek chat/reasoner=`0.14/0.28/0.0028/0`；MiniMax M3=`0.30/1.20/0.06/0`。
- `repair_current_model_pricing` 仅匹配旧内置四元组；用户 override 与已产生的历史请求
  非零 cost 不被 vendor refresh 无条件改写。models.dev 本地文件只保存显式 override/
  tombstone，不导出整张 seed 表，避免启动时把未来内置 repair 回滚。
- 上游 executable coverage 已随 `0ddce4e6` 落入：本地文件默认惰性、override reload、
  deletion tombstone、batch overwrite、选择并发保护、离线启动错误记录、picker
  normalization/price formatting。fork 既有 `openclawPresetPricing.test.ts` 继续作为
  FE cost↔Rust seed SSOT 枚举门禁。
- S6a follow-up 已完成上述适配：`web_services.rs` 现在暴露 tauri-free
  `model_pricing`；Web usage/system router 与 `web-commands.ts` 已提供四个新增命令的
 真实 handlers（并将 get/update/delete pricing 统一转到 shared service）；
  `check:web-routes` 通过 **282 commands / missing 0 / methodMismatch 0 / parityFallback 0**。
  `ModelsDevAutoSyncPanel` 在 Web 隐藏 desktop-only open-folder 控件，桌面动作保留并有
  focused UI coverage。models.dev fetch 仍只使用编译期常量
  `https://models.dev/api.json`，响应有 JSON shape、16 MiB、provider/model/count、文本与
  非负 finite cost bounds；Web CSP 仅增列固定 `https://models.dev` origin（桌面已有
  `https:` 许可，未改为新的通用 server-side proxy，也不接受任意 URL）。
  Web route round-trip integration 覆盖 config/batch/single/result/tombstone；Qwen3.8 Max
  只加入 fork-owned Bailian/Qwen catalogs（OpenClaw Qwen Coder、OpenCode Bailian、Hermes
  两个 Bailian endpoints），并有 owner-exclusivity tests 与 Rust-seed vendor tuple guards。

### S6a Phase 2.2 review follow-up（2026-08-09）

- command manifest generator 对 models.dev config 的两个既有 Web POST system 路由增加
  显式 override；生成的 282-command manifest 与 `web-commands.ts`/Axum 精确一致，
  `gen-command-manifest.sh --check` 通过（ignored manifest 未纳入版本控制）。
- 移除 preset/seed 验收中的 Rust 源码 regex 解析：真实 Web server fresh DB API 直接读取
  runtime seed，并与导入的 OpenClaw preset objects 逐条比较所有 exact shared IDs；另钉住
  S6a 7 个完整四元组及 Qwen3.8 Max 的 OpenClaw/OpenCode/Hermes owner scope。该门禁同时
  暴露并修正 E-FlowCode `gpt-5.3-codex` 的零价 tuple。
- batch pricing 先无回填地应用待处理本地 override/tombstone，在自身事务后仅做一次 full
  backfill；standalone local sync 保持 full backfill，single update 保持 normalized
  per-model backfill 与 provider multiplier。确定性 SQL trace 回归证明“batch 前本地文件变化”
  只执行 1 次 full select 且两类行均正确计价。
- real-server harness 在 SIGTERM/SIGKILL 后递归清理 data/home（启动失败同样清理），并隔离
  HOME/USERPROFILE 与 XDG config/data/state/cache；测试 origin 使用 scoped MSW
  `passthrough()`，suite 内 unhandled traffic 为 error。models.dev parser 的 iterative
  depth/cycle/container guards 保留并扩展覆盖 size/count/NaN/Infinity/negative/固定 URL。
- 最终 review gates：Rust fmt、desktop/web cargo check、model_pricing 10、backfill 19、
  generator 1、CSP 1；Prettier/typecheck/web-routes（282，missing/mismatch/fallback=0）/
  locales（en/ja/zh 各 2491）全绿；focused frontend 4 files/28 tests、real Web pricing
  integration 1 file/3 tests 全绿。Web example 仅既有 standalone dead-code warnings。

## S6b progress（2026-08-09，usage timeline/dedupe/batch insert）

### 提交映射与适配

- `56fb46c0`：已用 no-commit cherry-pick 移植 Codex parent rollout timeline cache，
  包含 fork cutoff 复用、cache invalidation 与 replay hardening；依赖冲突只为
  `windows-sys` 增加实际所需的 `Win32_Storage_FileSystem` feature，保留 fork 现有
  版本与其余 feature 选择。
- `4bfb3fc3`：已用 no-commit cherry-pick 移植 Claude Desktop proxy/session usage
  去重。proxy logger 记录可关联 session row 的 fingerprint，usage 查询在双来源重叠时
  保留正确 owner，避免桌面代理与原生会话重复计费。
- `59a2bd10`：已用 no-commit cherry-pick 移植交错 Codex token counter 支持，覆盖
  counter stream 交错、replayed snapshot 去重及跨 source stale snapshot 隔离。
- `baf07a27`：已用 no-commit cherry-pick 移植 Codex session 批量插入、sync cursor
  preload、pricing lookup cache 与 cached statements。冲突中保留 fork 已有的
  `pricing_model`、`pricing_missing` 列写入与读取；删除随冲突误带入、但不属于该提交父级
  依赖的 dashboard helper/tests。

### S6a/S6b follow-up

- `update_model_pricing_batch` 先应用 pending local override/tombstone，但延迟其历史成本
  回填；models.dev batch 与本地 override 提交完成后只执行 **一次** full backfill。
  单模型更新仍按 shared normalization candidates 做 targeted backfill，standalone local
  sync 仍执行 full backfill，删除 tombstone 不触发无意义回填。SQLite authorizer 回归明确
  断言 batch path 只有一个 full scan，并验证 provider multiplier 与 pending local price。
- `gen-command-manifest` 为 `get_models_dev_sync_config` /
  `save_models_dev_sync_config` 加入显式 `system + POST` route override，避免名称推断错误地
  生成 config GET/PUT；回归与 `web-commands.ts`/Axum 实际路径一致。
- 两个 follow-up 已与本批其他变更一起保留在工作树；没有 commit、amend、push 或 merge。

### S6b 门禁证据

- S6b 初始全量 Rust library：**1901 passed / 0 failed / 3 ignored**；desktop
  `cargo check --all-targets --locked` 通过。新增 follow-up focused：
  `services::model_pricing::tests` **10 passed**，manifest generator **1 passed**。
- Web `cargo check --locked --no-default-features --features web-server --example server`
  通过（67 个既有 standalone shim dead-code warnings）。Web Rust：`web_api::`
  **27 passed**、`dual_runtime_parity::` **3 passed**、`web_proxy_lifecycle::`
  **7 passed**。
- frontend unit：**147 files / 820 tests passed**；TypeScript、Prettier、locale parity
  全绿。route parity：**282 commands / 270 routes / missing 0 / methodMismatch 0 /
  parityFallback 0**。
- `pnpm test:integration`：**49/53 passed**。仅 PRD 已知四个 fixture flakes：
  ProviderList Claude official-seed empty-state/import-current 1 个；SkillsPage
  default-repository/fixture discovery 3 个。新增 `ModelsDevPricing.web-server` **3/3**
  通过，无新增产品失败。

## S6c progress（2026-08-09，backup/restore batching）

### 提交映射与安全适配

- `668bbda9`：已用 no-commit cherry-pick 移植。SQL dump 按最多 200 行且最多
  1 MiB 组合 multi-row `VALUES`，超大单行独立输出；identifier 统一 quote，generated
  column 只写可插入列，trigger 延后到数据加载完成后创建，index/view 与旧单行 INSERT
  备份格式继续兼容。
- `restore_tables` 现在把全部 preserve/canonical table 恢复放入单一事务，每表只
  prepare 一次 INSERT；任一后段表失败会回滚前面所有 DELETE/INSERT，不留下半恢复状态。
- 冲突中保留 fork 的严格 canonical table/index/PRAGMA/function authorizer、trusted-schema
  copy、integrity/foreign-key check 与 safety-backup/live-replacement 单锁边界。SQLite 将
  multi-row `VALUES` 表示为内部 `AuthAction::Select`，因此只显式允许该 action；具体
  table/column read 与 function 仍分别经过既有 allow-list，未恢复 Product upstream 的
  宽松 authorizer。

### S6c 门禁证据

- `database::backup::tests`：**16 passed / 0 failed / 2 ignored**。覆盖 450 行恰好
  3 个 INSERT batch、字节上限、特殊文本/BLOB/NULL round-trip、generated/quoted
  identifiers、trigger order、index/view、insertable columns、late-failure 全事务回滚、
  sync preserve 与 `sql_import_holds_main_lock_across_safety_backup_and_replace`。
- canonical SQL compatibility filter `sql_import_`：**5 passed**（legacy v1、schema v2、
  upgraded legacy current export、失败保持旧库、单锁并发写阻塞）；v14→v15 Grok migration
  regression：**1 passed**。
- Desktop `cargo check --all-targets --locked` 与 Web example cargo check 全绿；Web 仅
  67 个既有 standalone dead-code warnings。
- focused real-Web integration：`ImportExportSection` + `BackupListSection` **3/3 passed**，
  覆盖 SQL 导入/导出 round-trip 与 binary backup create/rename/restore/delete。

### 下一批

- S7：`8ae1ce85`、`56a66eea`、`58dd376d`、`e3f80a98`、`492245dc`、`413c09e0`。

## S7 结果（2026-08-09，provider presets / Codex auth / catalog ownership）

### 提交映射与处置

- `8ae1ce85`：已用 no-commit cherry-pick 移植 DeepSeek Responses preset/catalog。
  保留 catalog lexical/canonical containment、symlink 拒绝、32 MiB 流式读取上限与
  parsed vendor-host 匹配；未恢复上游已删除但 fork 仍需保留的 Anthropic profile。
- `56a66eea`：已用 no-commit cherry-pick 移植火山方舟 Agentplan preset；只使用
  `/api/coding/v3`，并按 native Responses 能力注册。
- `58dd376d`：已用 no-commit cherry-pick 移植腾讯混元 TokenHub preset；使用中国大陆
  endpoint，并保持 Hy3 catalog 为 text-only。
- `e3f80a98`：已用 no-commit cherry-pick 移植 Codex 第三方认证清理。只有 backfill
  成功或切换到官方 provider 成功后才清除 stale auth；OAuth 凭据继续保留，空
  `config.toml` 仍是有效配置。
- `492245dc`：已用 no-commit cherry-pick 移植 Auth Center 的 per-account Codex OAuth
  quota，并补齐现有 Web command/route parity；没有新增 desktop-only fallback。
- `413c09e0`：已用 no-commit cherry-pick 移植用户自有 `model_catalog_json` 保护，并在
  fork 的 catalog containment/security 模型上完成所有权适配。

### Catalog 所有权与安全适配

- `src-tauri/src/codex_config.rs` 新增共享
  `is_cc_switch_owned_catalog_reference`，catalog resolver 与写入/清理路径复用同一判定，
  避免所有权语义漂移。
- 写入 `Some(...)` 时只认领不存在或已由 cc-switch 管理的 pointer；用户完整路径或相对
  路径指向的自有 catalog 均不覆盖。写入 `None` 时只删除 cc-switch-owned pointer，不清理
  用户自有配置。
- resolver 继续同时执行 lexical 与 canonical containment，拒绝 symlink 越界，并保留
  实际流式 32 MiB cap；测试覆盖用户完整路径、用户相对路径、pointer 缺失时认领及既有
  managed pointer 更新/清理。
- Auth Center 的 OAuth UI 与 persisted-file 断言共用 5 秒真实服务等待预算，匹配满套件
  负载下实际 1 秒 device-code polling cadence；未放宽产品端 polling 或错误语义。

### S7 门禁证据

- Rust `codex_config::tests`：**43 passed**；provider-service integration：**23 passed**。
- focused frontend：**19 passed**；focused real-Web Auth Center：**2 passed**。
- full frontend unit：**146 files / 830 tests passed**。
- desktop `cargo check --all-targets --locked`：通过；Web example cargo check：通过，仅
  **67** 个预期 standalone dead-code warnings。
- Web Rust：`web_api::` **27 passed**、`dual_runtime_parity::` **3 passed**、
  `web_proxy_lifecycle::` **7 passed**。
- `pnpm format:check`、`pnpm typecheck`：通过；Web routes：**282 commands / 270 routes**，
  missing/methodMismatch/parityFallback 均为 **0**；locales en/ja/zh 各 **2491 keys**，
  严格 parity。
- 最终 `pnpm test:integration`：**49/53 passed**。仅剩 PRD 已知四个 fixture flakes：
  ProviderList official-seed/import-current **1** 个；SkillsPage default repository/fixture
  discovery **3** 个。机器可读报告：
  `/tmp/cc-switch-web-s7-integration-results-final.json`。
- `git diff --cached --check` 通过；无 unmerged path、无 conflict marker，且不存在
  `CHERRY_PICK_HEAD`。本批没有 commit、amend、push 或 merge。

### 下一批

- S8：`92ca95ff`（OpenCode runtime models）、`0345fad6`（统一 OMO config）。

## S8 结果（2026-08-09，OpenCode runtime models / unified OMO config）

### 提交映射与 Web-first 适配

- `92ca95ff`：已用 no-commit cherry-pick 移植。OpenCode runtime model discovery
  收敛到 tauri-free `services/model_fetch.rs`，桌面 command 与真实 Web
  `GET /api/config/get-opencode-models` 共用同一实现；`web-commands.ts` 与 manifest
  override 已同步，route parity 不依赖 wildcard/fallback。前端 OMO model source 把
  OAuth/Zen runtime models 与已配置 provider models 合并，重复项由配置侧 label、variants
  与 preset metadata 优先。
- `0345fad6`：已用 no-commit cherry-pick 移植并完成三处语义冲突适配。支持统一
  `~/.omo/omo.json(c)` 的 `[opencode]` 配置，同时保留 legacy OMO/OMO Slim 路径；OpenCode
  JSON5 round-trip 继续保留注释，并由共享 path lock 与 path-specific plugin API 串行化。
  非当前 OMO variant 只更新数据库；当前 variant 先写 live config/plugin，全部成功后才
  持久化数据库，任一步失败都执行补偿回滚。

### Runtime command、文件所有权与回滚安全

- runtime execution 固定为直接执行 `opencode models`，拒绝 shell metacharacters/非法参数；
  20 秒总 deadline、stdout/stderr 各 4 MiB 上限，超时或溢出终止完整 process tree，未引入
  shell 拼接或任意命令/参数表面。真实 Web 测试使用隔离 HOME/XDG、`SHELL=sh` 与受控
  `PATH` fake executable，证明浏览器 adapter → Axum route → shared service → CLI parser
  的完整 round-trip。
- `config.rs` 新增返回实际落盘 bytes 的
  `write_json_file_with_contents` / `write_json_file_managed_with_contents`，调用方可对精确
  JSON bytes 做 rollback snapshot；restricted writer 继续拒绝 final symlink，managed writer
  只跟随现存 regular-file final symlink。Unix 新文件仍使用 `create_new + 0600`，已有 target
  保留 mode；Windows 采用 `ReplaceFileW` 原子替换语义。
- rollback snapshot 保留 regular-file / managed-symlink identity；已删除的 managed symlink
  可按原 link target 安全重建，但若目标内容或 symlink identity 在事务期间被外部并发修改，
  rollback 拒绝覆盖新状态。统一 OMO config 写入同样使用 managed writer，不把用户 dotfiles
  symlink 替换成普通文件。

### S8 门禁证据

- focused Rust：`services::omo::tests` **28 passed**、`opencode_config::tests`
  **8 passed**、`config::tests` filter **161 passed**、`services::model_fetch::tests`
  **24 passed**、`services::tool_version::tests` **12 passed**；manifest override regression
  **1 passed**。Provider OMO update regressions：current rewrite/DB-on-write-failure/plugin
  rollback **3 passed**，non-current variant persistence **1 passed**。
- frontend hook regressions新增 **3 passed**：runtime OAuth/Zen merge、配置 label/variants
  dedupe 优先、runtime discovery failure warning + configured fallback。full `pnpm test:unit`：
  **147 files / 833 tests passed**。
- real Web `AboutSection.web-server`：**4/4 passed**，包含受控 fake `opencode` 的
  `/api/config/get-opencode-models` 成功路径；未读取开发者真实 OpenCode 配置。
- `cargo fmt --check`、desktop `cargo check --all-targets --locked`、Web example cargo
  check 均通过；Web 仅 **67** 个既有 standalone shim dead-code warnings。
- Prettier、TypeScript、Web routes、locale parity 全绿。route report：**283 commands /
  271 routes / missing 0 / methodMismatch 0 / parityFallback 0**；en/ja/zh 各
  **2492 keys**。
- Web Rust：`web_api::` **27 passed**、`dual_runtime_parity::` **3 passed**、
  `web_proxy_lifecycle::` **7 passed**。
- `pnpm test:integration` machine-readable rerun：**50/54 passed**；仅 PRD 已知四个
  fixture flakes：ProviderList Claude official-seed/import-current **1** 个，SkillsPage default
  repository/fixture discovery **3** 个。报告：
  `/tmp/cc-switch-web-s8-integration-results.json`。S8 新增 route probe 与 OMO 行为无失败。
- `git diff --cached --check` 通过；无 unmerged path、无 conflict marker、无
  `CHERRY_PICK_HEAD`。本批没有 commit、amend、push 或 merge。

### 下一批

- S9：`f5f4281d`、`9f19d8fd`、`968794e3`、`0cb6e014`、`eb356e15`、
  `40b6376b`、`83830767`。

## S9 结果（2026-08-09，UI / management / Skills / Hermes）

### 提交映射与处置

- `f5f4281d`：已用 no-commit cherry-pick 移植。移除 `useAutoCompact`，应用切换器
  固定为 icon-only；保留 fork 的 Claude、Codex、Gemini、Grok Build、OpenCode、
  OpenClaw、Hermes 七应用集合。
- `9f19d8fd`：已用 no-commit cherry-pick 移植。Prompts、MCP、Skills 管理页新增搜索、
  应用计数与批量启停；批量操作继续串行执行并暴露 busy/partial-failure 状态，header
  action 与 panel imperative API 同步。保留 fork 的 Web invoke/event adapter 与七应用
  类型；上游 `zh-TW` locale hunk 按既有删除决定省略，en/ja/zh 同步新增键。
- `968794e3`：已用 no-commit cherry-pick 移植。常驻 `animate-pulse` 改为仅活动窗口每
  3 秒短暂触发的 `status-heartbeat`，Web/dev 由 browser focus 事件驱动，Tauri 由窗口
  focus 事件补充；`prefers-reduced-motion` 下禁用 opacity heartbeat。fork 不暴露 Claude
  Desktop route toggle，因此只省略该单一 desktop-only hunk。
- `0cb6e014`：已用 no-commit cherry-pick 移植并适配 icon-only 七应用 switcher。header
  将 switcher 与 actions 分槽布局；`ResizeObserver` 根据可用宽度把溢出应用放入“更多”
  菜单，当前 active app 即使原本落入溢出区也会保留在可见位。未引入 Claude Desktop
  surface。
- `eb356e15`：已用 no-commit cherry-pick 移植。Skills 安装源目录必须以真实存在的
  `SKILL.md` 为锚点；拒绝同名但无 `SKILL.md` 的 wrapper，并能递归解析嵌套 catalog
  skill 或仓库根 skill。
- `40b6376b`：已用 no-commit cherry-pick 移植。新下载 Skill 的 `readme_url` 优先由
  canonical resolved source 推导仓库内 `SKILL.md` 路径，旧 `readme_url` 与 `directory`
  仅作兼容 fallback，避免 skills.sh 只给末级 skillId 时生成 404 链接；保留 S8 已有的
  metadata 并发更新与卸载保护。
- `83830767`：已用 no-commit cherry-pick 移植。Rust prompt path 与 frontend filename
  SSOT 均把 Hermes 从 `AGENTS.md` 改为 `SOUL.md`，其他共享 `AGENTS.md` 的应用不变。

### Fork 适配与回归覆盖

- 保持无 Claude Desktop surface、无 `zh-TW`、Web event/API adapter 与七个 fork app；
  未用 desktop-only 代码替换 Web 路径，未新增 route parity fallback。
- 新增 `tests/components/AppSwitcher.test.tsx`，覆盖窄 header 下 active app 可见及从
  overflow menu 切换并持久化；新增 `tests/components/promptFilename.test.ts`，锁定
  Hermes `SOUL.md` 与 OpenClaw `AGENTS.md`。Skills 的 wrapper/nested source、resolved
  doc path 与 fallback 均有 Rust regression。
- Phase 3.3 将可复用的 Skills source/readme contract 写入
  `.trellis/spec/frontend/quality-guidelines.md`：源目录以 `SKILL.md` 为锚点，freshly
  resolved repository-relative doc path 优先于旧 `readme_url` 与 directory fallback。
- S9 全部提交叠加在既有 staged S6-S8 工作上，没有 commit、amend、push、merge，且
  未修改或纳入 `.pi/`、`.pi-subagents/`。

### S9 门禁证据

- Prettier、TypeScript、`cargo fmt --check`、desktop
  `cargo check --all-targets --locked` 与 Web example cargo check 全部通过；Web example
  仅保留已记录的 **67** 个 standalone shim dead-code warnings。
- Web routes：**283 commands / 271 routes / missing 0 / methodMismatch 0 /
  parityFallback 0**。locale parity：en/ja/zh 各 **2505 keys**。
- full frontend unit：**158 files / 942 tests passed**。focused suites：Skills Rust
  **39 passed**、MCP DAO **4 passed**、MCP rollback integration **1 passed**、frontend
  `App.test.tsx` **23 passed**；Hermes Rust/frontend filename regressions通过。
- Web Rust：`web_api::` **27 passed**、`dual_runtime_parity::` **3 passed**、
  `web_proxy_lifecycle::` **7 passed**。
- `pnpm test:integration` machine-readable rerun：**21 files / 54 tests，50 passed、
  4 failed**。失败仍严格等于 PRD 已知环境 fixtures：ProviderList Claude official
  seed 导致 import-current empty-state 预期失败 **1** 个；SkillsPage default repository /
  fixture discovery **3** 个。报告：
  `/tmp/cc-switch-web-s9-integration-results.json`。S9 AppSwitcher、heartbeat、nested Skill
  source/readme URL 与 Hermes filename 未出现新增失败。
- 最终 hygiene：无 unmerged path、无 conflict marker、无 `.git/CHERRY_PICK_HEAD`；
  `git diff --cached --check` 通过。
