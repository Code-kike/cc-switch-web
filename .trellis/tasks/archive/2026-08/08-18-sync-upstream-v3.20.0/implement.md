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
- [x] `18ca2da0`/`0b5da510` 版本号 → `3.20.0`（package.json + Cargo.toml + tauri.conf.json + Cargo.lock，S1 已落地）
- [x] `af31a87b` changelog 改写为 fork 短条目，反映实际移植范围（父主体 S1–S7；3 个子任务结果待各子任务归档后由父集成 review 补入合 main 前 changelog）
- [x] 全量门禁最终跑一遍
- [x] 生产构建 `pnpm build:web`（PRD 验收 #5，exit 0，dist-web 产出）
- [x] 真实 Web 服务冒烟（smoke:web-server exit 0）
- [x] commit

## 子任务编排（父职责，非父直接实现）

- [x] 确认子任务 `feat-pi-native-agent` 规划完成并启动（S2 落地后无前置阻塞）—— 已归档 `7d839ed8`
- [x] 确认子任务 `feat-codex-alpha-websearch` 规划完成（S2/F 组落地后启动）—— 已归档 `b6f02cfc`
- [x] 确认子任务 `feat-managed-oauth-accounts` 规划完成（**依赖 S2 `d2b070c9` 落地**）—— 已归档 `e1b88f48`
- [x] 各子任务归档后，父做跨子任务集成 review —— 见下「跨子任务集成 review 结果」
- [x] 统一版本/changelog，补入三子任务结果（CHANGELOG + 三语 release notes）
- [x] 合入 `main` —— `65436f90`（`--no-ff`，与 main 历次 sync 分支同惯例），0 冲突。
      **实况更正**：`main` 合并前在 **3.18.0**，不是 PRD「已确认事实」写的 3.19.2 —— v3.19.2 那次同步
      从未合入 main，其提交在本分支上。故本次合并交付**两个上游周期**（3.18.0 → 3.20.0，147 commit）；
      同步链未断（v3.20.0 分支基于已完成的 v3.19.2 工作切出）。回滚点 `5687d4c0`。
      合并后在 main 上复跑：`cargo test --lib` **2290** / test:unit **177 files / 1089** /
      web-routes **292/280/0** / locales **2664** / `SCHEMA_VERSION` **17** / fmt+typecheck 全绿。

## 跨子任务集成 review 结果（2026-08-26，HEAD 14b8416e）

### 改动面与交集（契约三项之一：相互无冲突）

父主体 = `25e34700..cb6a1229`（38 commit，112 生产文件）。三子任务顺序落在其后：
pi `cb6a1229..7d839ed8`（127 文件）、alpha `7d839ed8..b6f02cfc`（6 文件）、oauth `b6f02cfc..HEAD`（62 文件）。

| 交集 | 文件数 | 结论 |
|---|---|---|
| pi ∩ alpha | 0 | 无冲突面 |
| alpha ∩ oauth | 0 | 无冲突面 |
| **pi ∩ oauth** | **15** | 逐文件核验：pi 建立的行零丢失。唯一变化是 `ProviderCard.tsx` 的徽章条件加 `&& !supportsOfficialRouting`（oauth 对可路由的托管卡抑制徽章，是对 pi 徽章统一的**有意收窄**，非回退） |
| **父主体 ∩ pi** | **24** | 见下符号存活核验 |
| **父主体 ∩ oauth** | **23** | 同上 |
| 父主体 ∩ alpha | 0 | alpha 只碰 proxy 转译面 |

### 父主体语义无回退（契约第三项，经审阅补测）

- **Rust**：父主体 28 个实现类 commit 新增的**全部** `fn` 在 HEAD 存活 —— 脚本化核验零缺失。
- **前端**：父主体新增的全部 `export` 符号在 HEAD 存活 —— 零缺失。
- **S2 `d2b070c9` never-clobber**（最高风险，落点 `services/proxy.rs` 被 oauth W3 净移植）：
  helper `preserve_codex_oauth_login_on_restore` + 6 个回归用例（`restore_keeps_*` / `restore_writes_*` /
  `restore_prefers_*` / `simple_restore_path_keeps_codex_oauth_login`）全部存活。
- **i18n**：父主体 2506 键 → HEAD 2664，父主体键**零丢失**；S1 删除的 `officialPartner` 未被复活。
- **定价面**：三子任务零触碰 pricing seed（S3 的 FE↔Rust 一致性不受影响）。
- **`codexProviderPresets.ts`**（父 S1 与 oauth 都改）：oauth 仅扩宽 `providerType` union + 给 OpenAI Official
  加该字段，未动 S1 的预设数据。

### Web API parity / 安全边界 / schema（契约第一、二项）

- `check:web-routes` **292/280/0**（wildcard 20 / unsupported 29 / parityExact 5）—— 与基线逐项相同。
  pi 的 9 个新命令仍注册于 `web-commands.ts` + `web_api/handlers/pi.rs` + `lib.rs`；alpha/oauth 零新命令。
- 三子任务各自的守卫互不削弱：alpha 的 `strip_suffix` fail-closed URL 推导（oauth 未碰 `forwarder.rs`）；
  oauth 的 `Provider::blocked_by_proxy_takeover` 四处分发点单一定义 + 前端 `isOfficialBlockedByTakeover`
  单一派生；pi 的 18 个 skill 安全 helper（含不跟随符号链接的 `collect_tree_entries`）。
- carry-forward 上限全部原值：128 MiB / 2s / 16 MiB / 256 KiB / 32 MiB + 五个 `MAX_CITATION_DEDUP_*`。
- `SCHEMA_VERSION` **17**；pi 的 `migrate_v16_to_v17` + `idx_session_usage_dedup_semantic` +
  `SQL_RESTORE_INDEXES` 条目完整；冒烟中新库走完 v15→v16→v17。
- 缺席项：延期 Codex Chat 栈四文件、zh-TW、ClaudeDesktop 表单、`migrate_legacy_codex_official_managed_binding`。

### 门禁（集成 review 全量跑）

`cargo test --lib` **2289** passed / 0 failed / 5 ignored；`proxy::` **1153**；Rust parity **38**；
test:unit **177 files / 1089**；test:integration **50/54**（恰 4 个白名单 fixture flake）；
locales **2664** parity；web-routes **292/280/0**；build:web 与 smoke:web-server 均 exit 0
（124 探针，8 个有意非 2xx：6 个 desktop-only 501 + 2 个预期 400）。
web-server example 警告 67 → **69**（+2 为 oauth 新增的 shim 侧 dead code，非产品路径）。

### 唯一阻塞项（已修）

发布文档滞后于实际范围：`CHANGELOG.md` 与三语 release notes 写于 `857779a7`（子任务落地前），
仍称三特性「拆分为独立子任务，单独落地」并保留「子任务范围之外」章节，en L31 还说 Pi 是
「fork 尚未承载的 provider」—— 而三者都是 HEAD 祖先且同版本 3.20.0 发布。属**已交付能力被声明为未交付**，
违反 PRD「版本与 fork 自有发布说明反映实际移植范围」与 S8「三子任务结果待归档后由父集成 review 补入」。
已改：四个文件加「新特性」小节（三特性各一条，含 fork 特有裁定），「子任务范围之外」改写为
「三个新特性的补充说明」，S2/S4 的「子任务前置/移交」措辞改为指向同版内小节。

### review 后修正：第五处未收敛的 official 判定（架构审阅指出）

`services/proxy.rs:1122` 是接管**开启**时的封禁风险警告（`proxy-official-warning`），仍内联
`category == "official"` —— 托管 Codex Official 卡现在是 fork 官方支持的路由路径（代理在服务端解析
其绑定账号），却照样收到「建议切换到第三方供应商」的警告。这与 `provider.rs` doc 里记录的
「tray 曾内联导致漂移」是同一模式：**对我们自己提供服务的路径发警告，会训练用户忽略警告**。

已改为 `provider.blocked_by_proxy_takeover(app_type_str)`，与四处切换期执行点同源（现为五处）。
新增回归 `takeover_warning_skips_managed_codex_official_but_fires_for_other_official`：托管卡不告警、
未绑定原生登录 Official 卡仍告警。**变异验证**：改回内联即该用例 FAILED（"a managed Codex Official
card is served by this fork; it must not get a ban-risk warning"），恢复后通过。
测试台新增 `inject_recording_runtime_ctx`（`ChannelEventSink` + 返回 manager 以便种入托管账号）——
这是观测 `emit_json` 的唯一途径，此前测试模块没有录制型 sink。

**其余内联判定点逐处定性为不同语义，保留**（审阅要求的逐处判定）。

> **普查模式与计数两次更正（grounded-reviewer 指正）**
>
> 一次：首次普查用 `== Some("official")` 匹配，漏掉否定式与非 `as_deref` 形态三处
> （`codex_config.rs:1412` / `:2647` / `live.rs:805`）。正确模式是
> **`category[^=]*(==|!=)\s*Some\("official"\)`**。
>
> 二次：改用正确模式后计数仍有两处差账 ——（a）`codex_config.rs:2647` 与 `:2648` 是
> **独立两处**（`==` 与 `!=` 分属 `should_write_auth` 的两个析取项），被并成一行表格；
> （b）`provider.rs` 的 4 处命中被 `rg -v` 整文件排除而**未说明理由**。
>
> **实测口径（下次同步照此核对）**：模式全命中 17 行 → 去 1 行注释
> （`provider/mod.rs:4943`）= 16 处代码 → 去 `provider.rs` 4 处**谓词自身定义**
> （`is_managed_codex_official_account_card`:120 / `is_codex_official_card`:133 /
> `codex_official_login_is_live_owned`:162 / `blocked_by_proxy_takeover`:221 ——
> 这些**就是**单一定义的实现体，不是待收敛的消费点）= **12 处消费侧，全部保留**。
>
> 本次收敛的 `proxy.rs:1130` 改用 `blocked_by_proxy_takeover(..)` 后**已不再匹配该模式**，
> 故不出现在 12 之内；下表仍列它以记录改动，但计数时它属"谓词调用"而非"内联判定"。

| 位置 | 所在函数 | 语义 | 结论 |
|---|---|---|---|
| `proxy.rs:1130` | `set_takeover_for_app` | 接管开启时的封禁风险警告 | **已改用谓词**（本次修正；已不匹配内联模式） |
| `proxy.rs:1393` | `sync_live_config_to_provider` | 保证 official 行的 auth 为空（凭据卫生） | 更宽即 fail-safe，已有 in-code 注释 → 保留 |
| `tray.rs:257` | `provider_uses_official_subscription` | app-wide 订阅缓存能否表示该卡 | **已显式对 managed Codex 返回 false** → 保留 |
| `tray.rs:282` | `format_usage_suffix` | 用量后缀优先级，委派上者 | → 保留 |
| `tray.rs:1057` | `refresh_all_usage_in_tray` | 是否发用量请求，委派上者 | → 保留 |
| `provider/mod.rs:4948` | `switch_normal` | stale live auth 清理 | 已带 W2 注释 + `target_managed_codex_account_id.is_none()` 托管豁免 → 保留 |
| `provider/mod.rs:6218` | `validate_provider_settings` | Grok Build TOML 校验严格度 | 与 Codex/接管无关 → 保留 |
| `codex_config.rs:1412` | `should_restore_codex_provider_token_for_backfill` | backfill 是否回填 token | 不同语义 → 保留 |
| `codex_config.rs:2647` | `write_codex_live_for_provider` | live auth 写入路由，第一析取项（`==` + 有登录材料） | 不同语义 → 保留 |
| `codex_config.rs:2648` | `write_codex_live_for_provider` | 同上，第二析取项（`!=`，非 official 行的写入分支） | 不同语义 → 保留 |
| `live.rs:805` | `apply_codex_managed_oauth_auth` | managed OAuth apply 的早返回；**其后紧接托管账号判定** | 不同语义、无漂移 → 保留 |
| `grok_config.rs:382` | `write_grok_provider_live` | Grok live 写入 | 与 Codex/接管无关 → 保留 |
| `stream_check.rs:88` | `stream_check_all_providers` | 不对 official 端点做未认证探测 | 更宽即 fail-safe → 保留 |

**排除项（命中模式但不属消费点）**：`provider.rs:120/133/162/221` —— 分别是
`is_managed_codex_official_account_card` / `is_codex_official_card` /
`codex_official_login_is_live_owned` / `blocked_by_proxy_takeover` 的实现体。
它们是这套语义的**单一定义所在**，收敛的目标就是让消费点调用它们；把它们计入
"待收敛内联"会自相矛盾。

前端镜像 `providerCapabilities.ts::isOfficialBlockedByTakeover` 的 doc 已补「Rust 侧为权威、含完整消费者表」
的交叉引用；`provider.rs` 的 doc 由「四处执行点 / 切换必须被拒绝」改写为**分类谓词 + 五消费者表格**
（四处切换拒绝 + 接管开启时的封禁风险警告），并声明**不得为了措辞一致而再分裂谓词** —— 否则正是
tray 与警告两次漂移的成因。四处调用点注释同步去掉过时计数。

门禁复跑：`cargo test --lib` **2290** passed / 5 ignored（+1 新用例）；`proxy::` **1154**（+1）；
parity **38**；web-routes **292/280/0**；locales **2664**；fmt / web-server example check 全绿。

### 非阻塞跟进项（不影响合 main）

- pi 徽章统一后遗留三个无引用 locale 键（`claudeCode.noRoutingSupport` / `codex.noRoutingSupport` /
  `claudeCode.needsRouting`）—— 三语齐备故 `check:locales` 绿，纯冗余。
- `README.md` L5/L17/L41 枚举 5 个应用，`AppId` 实为 8 个：`grokbuild`/`hermes` 是**既有**漂移、`pi` 是本版新增。
  只补 pi 会留下不一致，故整体留作独立 README 任务。
- `useProviderActions.ts` / `ProviderCard.tsx` / `providerCapabilities.ts` 的 Official-takeover 谓词与上游
  有意分歧（fork 无 inbound Authorization passthrough），下次同步这三个文件会冲突，须按 in-code 注释取据判定。

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
