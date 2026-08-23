# Implement — sync upstream v3.20.0（父任务，主体工作）

## 前置确认

- [ ] 确认当前分支 `sync/upstream-v3.20.0` 已从 `main`（含 v3.19.2 同步成果）切出。
- [ ] 确认 `product-upstream` remote 已 fetch，`v3.20.0` tag 本地可达。
- [ ] 确认工作树未跟踪目录 `.pi/`、`.pi-subagents/` 不在本任务提交范围。

## 执行批次（ordered checklist）

每批：cherry-pick → Web 适配 → 全量门禁 → 单独 commit。
门禁命令：
```bash
source ~/.cargo/env
cargo fmt --check
pnpm format:check
pnpm exec cargo check --manifest-path src-tauri/Cargo.toml   # web cargo check
pnpm check:web-routes
pnpm check:locales
pnpm test:unit
pnpm test:integration        # 已知环境 flake 不阻塞
cargo test --manifest-path src-tauri/Cargo.toml web_api::dual_runtime_parity::
cargo test --manifest-path src-tauri/Cargo.toml web_proxy_lifecycle::
```

### S1 — 发布/文档 + sponsor/preset/i18n churn（~20 提交）
- [ ] cherry-pick A 组：`0b5da510` `18ca2da0` `af31a87b` `a7f073e9`（改写为 fork 短条目）
- [ ] cherry-pick B 组：`0cd922c5` `52745efe` `4080a8e9` `af06356d` `9dcd3486` `e12fc623`
      `1435223b` `c6247d13` `e163a671` `eb69e492` `5b8bf1fe` `c99550e0` `5f6072ce`
      `58d92e56` `3711e1a0` `16cc0d7f`
- [ ] Web 适配：preset 增量同步到 fork provider（ctok/lionccapi/lemondata 等自有
      provider），i18n 走 fork locales 路径
- [ ] 门禁全绿 → commit

### S2 — 安全/数据完整性（优先，8 提交）
- [ ] `fd14f9c4` env-check preflight hang
- [ ] `d2b070c9` never clobber Codex ChatGPT login ⚠ 子任务 feat-managed-oauth 前置
- [ ] `c8262476` Kimi thinking injection stop
- [ ] `1f38c838` zhipu CREDIT_LIMIT quota
- [ ] `3f75bbdf` StepFun effort inference
- [ ] `ccc86298` animate routing activation
- [ ] `c9fe340b` sync/restore consistency（1012 行）⚠ SQL-restore 锁测试
- [ ] `dfb2e523` backup SQL fidelity（1302 行）⚠ SQL-restore 锁测试
- [ ] 保留 fork 既有 canonical-schema allow-list、backup/replace 锁边界、2s deadline
- [ ] DB 迁移连续性校验（从 v3.19.2 基线不跳号）
- [ ] 门禁全绿 → commit

### S3 — 定价/usage（5 提交）
- [ ] `bad9c151` DeepSeek V4 peak + Gemini 3.7 Flash
- [ ] `5602324b` DeepSeek catalog mirror extraction
- [ ] `7dc0a725` Grok 4.5 cached + Grok 4.6 + DeepSeek alias
- [ ] `3d126f45` multi-year trend tooltip
- [ ] `46f19a15` DeepSeek cache-hit tokens
- [ ] ⚠ FE-preset ↔ Rust-seed 一致性核对（防 `cost=$0`，注意 `find_model_pricing_row` 有损清洗）
- [ ] 门禁全绿 → commit

### S4 — codex/provider 功能（12 提交）
- [x] `d1c550ba` drop Goal mode toggle（-270 行）→ S4a `5a5874a5`
- [x] `6e424fd3` restore 1M context toggle → S4c `feffa171`
- [~] `0455a92c` multiple follow-login providers（829 行）→ **拆分**：
      前端独立增量（providerCapabilities 3 符号 + ProviderCard identity/useManagedAuth +
      useProviderActions supportsOfficialProxyTakeover + providerConfigUtils +
      useManagedAuth enabled + presetEntries）已落 S4b `84d54e7d`；**Rust managed-codex 事务**
      + `ProviderForm.codexManagedAccount.test.tsx`（8 用例）移交 `feat-managed-oauth-accounts`
      子任务（依赖 `a2e22f33` 的 `preflight_managed_codex_live` 等辅助函数，fork 全缺）。
- [~] `897ca892` OAuth usage queries configurable → 前端（CodexOauthQuotaFooter/subscription.ts/
      ProviderCard codexAccount identity）已落 S4b `84d54e7d`；Rust tray.rs 已落 S4c `f462a4ce`。
- [x] `a98829ba` IME-safe provider fields → S4a `5a5874a5`
- [x] `f62c854a` cancel stale device login → S4c `4e060fa5`
- [~] `d01eab97` OpenCode Zen reasoning effort → **整提交跳过（proven inapplicable）**：触及延期 Codex Chat
      reasoning 栈（`transform_codex_chat.rs` zen 映射 + `CodexChatReasoningConfig.effort_levels` +
      `infer_aggregator_platform_config` opencode.ai 条目），fork 全缺（无 `CodexChatReasoningConfig`/
      `effort_value_mode`/`effort_levels`/`map_reasoning_effort`/`apply_reasoning_options`/
      `infer_aggregator_platform_config`），前端 `codexChatReasoning`/`effortValueMode`/`reasoningLevels`/
      `mapCodexCatalogModelForForm` 亦全缺。恢复需先落地整套 ~1.1k LOC 推断表 + 类型 + `codex.rs` 基础，
      越过 S4 边界；与 S2 `3f75bbdf`/S3 `46f19a15`/S4 `9db9c56f`/`6a7da87c` 同属延期项。
- [x] `b109dcd3` Grok Build Codex copy → S4c `11cd4ba3`
- [~] `40cac1a6` per-model reasoning levels（637 行）→ **拆分**：catalog 生成数据层（`codex_config.rs`
      `apply_codex_reasoning_level_override` + spec 字段 + `types.ts` `reasoningLevels`/`defaultReasoningLevel`）
      已落 S4c `69534266`；转换层（`transform_codex_chat.rs`/`transform_codex_anthropic.rs`）+ 前端 catalog
      模型编辑器 UI 依赖延期 Codex Chat reasoning 栈，跳过。
- [x] `f748f3ac` grokbuild form align → S4c `e97e01e4`
- [x] `d9d4a660` macOS IME corruption → S4c `0277a8e1`（HermesFormFields/OpenClawFormFields/OpenCodeFormFields
      model-name + ProviderForm 3 个 provider-key 采用 ImeSafeInput，补 IME 回归测试）
- [~] `6a7da87c` grokbuild input token details → **整提交跳过（proven inapplicable）**：仅命中
      `streaming_codex_chat.rs` + `transform_codex_chat.rs` 两延期文件（`git show --stat` 双重确认无
      grokbuild/session_usage 文件），`chat_usage_to_responses_usage` 在 fork 全树无定义。委派清单
      归为“必移植”是基于提交标题而非实际文件清单的误判。
- [x] 新命令注册到 `web-commands.ts`，过 `check:web-routes`（S4c 无新命令）
- [x] 门禁全绿 → commit

### S5 — UI/refactor（14 提交）
- [~] `7e5007d5` fix(claude-desktop): clarify model configuration modes → **整提交跳过（proven inapplicable）**：仅改 fork 不存在的 `ClaudeDesktopProviderForm.tsx`（`ls` 证实 fork 无该文件）。fork 的 AppId 集合无 claude-desktop。
- [x] `580a4d7b` refactor(hermes): align provider form hierarchy → `86520729`。采用上游 Pi-style 行布局（header row + ChevronRight 展开式 + context_length 可折叠 + rateLimitDelay 直显），保留 fork S4c 的 `ImeSafeInput`（model id/name/baseUrl）与 `expandedModelKeys` Set；合并上游结构测试与 fork IME 回归。
- [x] `ec842156` refactor(opencode): clarify provider form hierarchy → `79424b99`。Extra SDK Options 由 Collapsible 改为常驻可见区（FormLabel + hint + add），Models section 加 border-l family divider；新增 2 个结构测试。
- [x] `c0050623` fix(ui): unify checkbox styles → `da6136cf`。Radix CheckboxPrimitive → 原生 `<input type=checkbox>`，保留 `onCheckedChange(boolean)` + `checked(CheckedState)` 契约；indeterminate 经 useLayoutEffect 设原生属性 + aria-checked="mixed"。全量消费方（~14 处）typecheck/test 通过，无 peer/data-[state] CSS 依赖。
- [x] `5b77da2b` style(openclaw): clarify User-Agent section hierarchy → `53c52afb`。User-Agent 行加 border-l family divider。
- [~] `619a592c` fix(claude-desktop): align provider form frame → **整提交跳过（proven inapplicable）**：同 `7e5007d5`，仅改 fork 不存在的 `ClaudeDesktopProviderForm.tsx`。
- [x] `95b95da6` refactor(openclaw): align provider model editor → `e0f07dbc`。Pi-style family 模型编辑器（header row + ChevronRight 展开式 + 原生 OpenClaw detail panel：reasoning/input-types/contextWindow/maxTokens/cost grid），保留 fork `ImeSafeInput`；ProviderActions/ProviderCard/useProviderActions 加默认模型选择流；合并结构测试与 IME 回归。
- [x] `076c2744` fix(provider): finish model dropdown consolidation → `7386a797`。HermesFormFields/OpenClawFormFields 内联 DropdownMenu 分组拣选器统一到 shared `<ModelDropdown>`；OmoFormFields ModelCombobox 与 ProfileSwitcher 加 Command label；新增 ModelDropdown.test.tsx。
- [x] `7e152d75` feat(provider): add fuzzy search to model mapping dropdown → `bcb5ae53`。**含前置**：fork 从未落地 `shared/ModelDropdown.tsx`（v3.19.2 同步跳过 `2deee109` 因其仅触 ClaudeDesktopForm），本提交连带创建该共享组件（上游最终态：Command+Popover fuzzy search + vendor keywords），并替换 ModelInputWithFetch/ClaudeFormFields Copilot block 的内联 DropdownMenu；新增 3 locale keys（en/ja/zh）。
- [x] `8673e9d8` fix(claude): align advanced options with Codex → `4c7a0b44`。Claude 高级选项 Collapsible 加 bordered card 样式 + full-width trigger + leading-relaxed hint；模型映射 divider 改 border-border-default；providerForm.apiFormat/apiFormatHint 重命名为 upstream-format 文案（en/ja/zh）。**跳过 ClaudeDesktopProviderForm hunk**（proven inapplicable）；未引入相邻 `customUserAgent*/localProxy*` 键（属 `6fd4e6f4` 未移植特性，越界）。
- [x] `bc7f5f41` fix(ui): reduce provider editor empty space → `351ae9b0`。JsonEditor 默认 rows 12→3，各表单 editor override（Codex 6/8、Common 14、Gemini 6/8、GrokBuild 12、ProviderForm 3×14）统一降至 3。
- [x] `7de63227` fix(grokbuild): add glass form container → `ce5da1b7`。GrokBuild form 加 glass rounded-xl p-6 border border-white/10 容器（fork 既有模式）。
- [x] `967daa1a` fix(skills): report missing SSOT dir as update in check_updates → `e58491bc`。Rust `local_hash_for_update_check` 先验 SSOT 目录存在再信任缓存哈希，避免换机恢复备份后缺失目录被缓存掩盖；新增 4 个回归测试。
- [~] `390102a2` fix(codex): fill DeepSeek contextWindow in OpenCode Go catalog → **整提交跳过（proven inapplicable）**：fork 无 "OpenCode Go" 预设（grep 全树 0 匹配），且 fork 的 DeepSeek 官方预设已含 `deepseek-v4-pro`/`deepseek-v4-flash` 的 `contextWindow: 1048576`（3 个 DeepSeek 模型目录条目均有 contextWindow，无缺失）。
- [x] UI 适配到 fork 组件树，保留 fork 既有样式约定
- [x] 门禁全绿 → commit（每提交全量 test:unit + format/typecheck/locales/web-routes/cargo fmt；批末 test:integration + Rust parity）

### S6 — Windows 全量适配（4 提交）
- [x] `d4fefefc` startup FOUC（前端渲染入口）→ `58474eee`
- [x] `de9af49a` Windows CLI registry detect → 适配到 `services/tool_version.rs`（fork 迁出 misc.rs）+ `commands/misc.rs::resolve_launch_cwd` 共享 `windows_shell_compatible_path` → `7678bbfe`
- [x] `3c592d93` WiX Handlebars backslash escape → `src-tauri/wix/per-user-main.wxs` → `03dd54e3`
- [x] `c39c9032` WSL atomic replace fallback（保留 fork 2s deadline + heap/stack 上限）→ `8457fd13`
- [x] 门禁全绿 → commit（4 提交各自全量门禁 + 批末 test:integration + Rust parity）

### S7 — CI 适配（2 提交，排除 2 桌面专属）
- [x] `c98cc3a9` skip-checks → fork `.github/workflows/ci.yml`（按 fork frontend/backend/web-server 3-job 分区；新增 `changes` job inline 路径过滤器，fork `index.html` 在 `src/` 被覆盖）
- [~] `36ed280d` i18n labeler glob → **整提交跳过（proven inapplicable，用户裁定 option A）**：fork 无 `.github/labeler.yml`（配置）也无 `workflows/labeler.yml`（工作流），上游 glob 修复无目标文件；`c98cc3a9` inline 过滤器已含 `src/**` 覆盖 `src/i18n/locales/**`，等价覆盖 i18n 路径。引入整套 labeler CI 是独立新决策，不夹带进 sync。
- [x] 排除 `ceef0a52`（WSL2 backend tests）、`bef46cd5`（grokBuild exclusion 文档）
- [x] 门禁全绿 → commit

### S8 — 版本 + changelog + 全量门禁
- [ ] `18ca2da0`/`0b5da510` 版本号 → `3.20.0`（package.json + Cargo.toml）
- [ ] `af31a87b` changelog 改写为 fork 短条目，反映实际移植范围（含 3 个子任务结果）
- [ ] 全量门禁最终跑一遍
- [ ] 真实 Web 服务冒烟
- [ ] commit

## 子任务编排（父职责，非父直接实现）

- [ ] 确认子任务 `feat-pi-native-agent` 规划完成并启动（S2 落地后无前置阻塞）
- [ ] 确认子任务 `feat-codex-alpha-websearch` 规划完成（S2/F 组落地后启动）
- [ ] 确认子任务 `feat-managed-oauth-accounts` 规划完成（**依赖 S2 `d2b070c9` 落地**）
- [ ] 各子任务归档后，父做跨子任务集成 review
- [ ] 统一版本/changelog，合入 `main`

## 验证命令汇总

见每批门禁块。关键额外检查：
- DB 迁移连续性：`sqlite3` schema 版本号查询，对比 v3.19.2 基线
- 定价一致性：FE-preset 与 Rust-seed 对照（防 `cost=$0`）
- Web API parity：`check:web-routes` + `web_api::dual_runtime_parity::`

## 风险点与回滚

- **单批失败**：`git reset --hard <上一批 commit>` 回滚，不影响已落地批次。
- **DB 迁移失败**：从 S2 前备份恢复；fork 既有备份/replace 锁流程。
- **子任务阻塞**：子任务独立分支，不阻塞父主体合 main（可先合主体，子任务后续跟进）。
- **CI glob 路径错误**：`36ed280d` 必须按 fork locales 路径校正，照搬上游 glob 会失效。

## review gates

- 每批 commit 前：全量门禁全绿（已知 flake 除外）。
- S2 后：DB 迁移连续性 + SQL-restore 锁测试专项确认。
- S3 后：定价一致性核对专项确认。
- S8 后：真实 Web 服务冒烟 + 子任务集成 review 后才合 main。
