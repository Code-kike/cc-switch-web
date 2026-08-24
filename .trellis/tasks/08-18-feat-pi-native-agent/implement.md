# Implement — feat-pi-native-agent（pi native coding agent + session usage）

## 前置确认

- [ ] 确认当前分支 `sync/upstream-v3.20.0`，父主体 S1–S8 已落地（HEAD `cb6a1229`）。
- [ ] 确认 `84e75ad2`/`40d747c0` 在 `product-upstream` remote 本地可达。
- [ ] 确认工作树未跟踪 `.pi/`/`.pi-subagents/` 不在本任务提交范围。

## 移植方法

逐文件 selective port（取上游最终态，逐文件落到 fork，丢弃 ClaudeDesktop/zh-TW hunk）。不直接 cherry-pick 156 文件单提交。

每批：移植 → 丢弃 ClaudeDesktop/zh-TW hunk → Web 适配（web-commands.ts + Axum handler）→ 全量门禁 → 单独 commit。

门禁命令（沿用父任务）：
```bash
source ~/.cargo/env
(cd src-tauri && cargo fmt --all -- --check)
pnpm format:check
pnpm typecheck
pnpm check:web-routes        # 0 missing/mismatch/fallback
pnpm check:locales           # en/ja/zh parity
pnpm exec cargo check --manifest-path src-tauri/Cargo.toml
(cd src-tauri && cargo check --no-default-features --features web-server --example server)
pnpm test:unit               # MUST fully green (not flake-eligible)
cargo test --manifest-path src-tauri/Cargo.toml --no-default-features --features web-server --example server -- web_api:: dual_runtime_parity:: web_proxy_lifecycle::
pnpm test:integration        # 4 PRD flakes allowed
pnpm build:web               # PRD acceptance
pnpm smoke:web-server
```

## 执行批次（ordered checklist）

### P1 — Rust 后端核心
- [x] `pi_config/mod.rs`（纯新增）：本地 models.json/settings.json 读写 + revision + MODELS_FILE_LOCK + `MAX_PI_FILE_BYTES=1MiB` + test_support
- [x] `services/pi_state.rs`（纯新增）：PiCurrentState 只读服务
- [x] `services/pi_prompt_files.rs`（纯新增）：AGENTS.md/SYSTEM.md/prompts 模板读写 + PiAgentsFileGuard
- [x] `services/provider/pi.rs`（纯新增）：list/add/update/delete/remove/enable + strip_unsupported_pi_metadata
- [x] `commands/pi.rs`（纯新增）：get_pi_current_state/update_pi_provider_usage_script/get_pi_session_discovery
- [x] `commands/prompt.rs`（修改）：加 6 个 pi prompt 命令（get/replace/delete_pi_prompt_file + list/upsert/delete_pi_prompt_template）
- [x] `session_manager/providers/pi.rs`（纯新增）：scan_sessions/load_messages/session_roots/session_discovery（本地文件，无进程）
- [x] `session_manager/providers/mod.rs`：加 `pub mod pi`
- [x] `session_manager/mod.rs`：scan_sessions 加 `pi::scan_sessions` handle + load_messages/delete_session/session_roots dispatch 加 "pi"
- [x] `session_manager/terminal/mod.rs`：`shell_escape` 由 `fn` 改 `pub(crate)`（1 行可见性）
- [x] `app_config.rs`：AppType enum 加 `Pi` + as_str/all/from_str/is_additive_mode；**丢弃 ClaudeDesktop arm**
- [x] `services/provider/mod.rs`：dispatch 加 `AppType::Pi` arm（list/add/update/delete/remove/enable/credentials/validate）；**丢弃 ClaudeDesktop arm**
- [x] `services/mod.rs`：加 `pub mod pi_prompt_files` + `pub(crate) mod pi_state`
- [x] `services/prompt.rs`：dispatch 加 `AppType::Pi` → pi_prompt_files；**丢弃 ClaudeDesktop arm**
- [x] `prompt_files.rs`：`prompt_file_path(AppType::Pi)` → AGENTS.md；**丢弃 ClaudeDesktop arm**
- [x] `services/skill.rs`：pi skills 集成（+1141 行，最大单文件）；**丢弃 ClaudeDesktop arm**（4 处）
- [x] `services/mcp.rs`/`database/dao/mcp.rs`：pi mcp gate（Pi 无 native MCP → false）；**丢弃 ClaudeDesktop arm**
- [x] `services/provider/live.rs`：pi live config 路径；**丢弃 ClaudeDesktop arm**
- [x] `services/stream_check.rs`/`settings.rs`/`proxy/providers/mod.rs`：pi arm（`Pi => return None` 等）；**丢弃 ClaudeDesktop arm**
- [x] `services/model_fetch.rs`：加 request_headers + api_format header 构建 + MAX_REQUEST_HEADERS=64 + secret scrub（无 ClaudeDesktop 依赖）
- [x] `commands/mod.rs`：加 `mod pi` + `pub(crate) use pi::*`
- [x] `lib.rs`：注册 9 个 pi 命令 + `mod pi_config` + startup import_pi_providers_from_live
- [x] `provider.rs`：resolve_credentials 加 `AppType::Pi` native baseUrl/apiKey
- [x] `error.rs`/`config.rs`/`deeplink/{parser,provider,tests}.rs`：pi 相关适配（如 deeplink provider 识别 pi）
- [x] Web handlers：`web_api/handlers/` 加 pi 路由（9 命令）+ `web-commands.ts` 注册
- [x] 门禁全绿 → commit

### P2 — 前端 provider UI
- [x] `lib/api/types.ts`：AppId union 加 `"pi"`
- [x] `config/appConfig.tsx`：APP_IDS/SKILLS_APP_IDS/MCP_APP_IDS 加 "pi"（Pi 无 MCP → 不加 MCP_APP_IDS）+ APP_ICON_MAP 加 pi + PROXY_APP_IDS/ADDITIVE_APP_IDS 加 pi + isProxyAppId/isAdditiveAppId；**丢弃 claude-desktop**
- [x] `config/piModelCatalog.ts`/`piProviderPresets.ts`/`piThinkingProfiles.ts`（纯新增）
- [x] `lib/api/pi.ts`/`lib/query/pi.ts`（纯新增）：pi provider API + query hooks
- [x] `lib/piPromptSlug.ts`/`lib/piPromptTemplate.ts`（纯新增）
- [x] `components/providers/forms/PiProviderForm.tsx`（纯新增，2038 行）
- [x] `components/providers/forms/ProviderForm.tsx`：`appId === "pi"` → PiProviderForm；**丢弃 claude-desktop**
- [x] `components/providers/forms/helpers/requestHeaders.ts`/`RequestHeadersEditor.tsx`/`StructuredOptionsEditor.tsx`（纯新增）
- [x] `components/providers/ProviderStatusBadge.tsx`（纯新增）
- [x] `components/providers/{ProviderActions,ProviderCard,ProviderList,AddProviderDialog,EditProviderDialog}.tsx`：pi 集成；**丢弃 claude-desktop**
- [x] `components/providers/forms/ProviderPresetSelector.tsx`：pi preset；**丢弃 claude-desktop**
- [x] `components/providers/forms/OpenCodeFormFields.tsx`/`helpers/opencodeFormUtils.ts`/`EndpointSpeedTest.tsx`：requestHeaders 集成
- [x] `hooks/useProviderActions.ts`：pi provider action 适配
- [x] `App.tsx`：pi 路由 + usePiCurrentState + handleEnablePiProvider + proxyAppId 逻辑；**丢弃 claude-desktop**（8 处）
- [x] `components/AppSwitcher.tsx`：pi tab；**丢弃 claude-desktop**
- [x] `components/settings/{AppVisibilitySettings,DirectorySettings,SettingsPage,AboutSection,ProxyTabContent}.tsx`：pi settings；**丢弃 claude-desktop**
- [x] `config/appConfig.tsx` i18n apps/pi 键
- [x] `icons/extracted/index.ts`：加 pi SVG icon
- [x] 门禁全绿 → commit

### P3 — 前端 prompts/skills/sessions UI
- [ ] `components/prompts/PiPromptPanel.tsx`/`PiNativePromptResources.tsx`/`PromptLibrary.tsx`（纯新增）
- [ ] `components/prompts/PromptPanel.tsx`：加 `appId === "pi"` → PiPromptPanel dispatch；**不移植非 pi 路径 PromptLibrary 重构**（保留 fork 现有 ManagementListSearch 列表）
- [ ] `components/prompts/PromptFormPanel.tsx`：pi 用 AGENTS.md + 不 trim
- [ ] `components/prompts/PromptListItem.tsx`：pi 适配
- [ ] `lib/api/prompts.ts`：pi prompt API dispatch
- [ ] `hooks/usePromptActions.ts`：pi prompt actions
- [ ] `components/skills/UnifiedSkillsPanel.tsx`：pi skills（Pi 无 native MCP registry → gate）
- [ ] `components/sessions/SessionManagerPage.tsx`：pi session 集成
- [ ] `components/mcp/UnifiedMcpPanel.tsx`：pi mcp gate（Pi 无 MCP）
- [ ] `components/proxy/{ProxyPanel,ProxyToggle,FailoverQueueManager,FailoverToggle}.tsx`：pi 不参与 proxy（gate/排除）
- [ ] `components/UsageFooter.tsx`/`UsageScriptModal.tsx`/`usage/{ModelsDevAutoSyncPanel,ModelsDevPickerDialog,PricingConfigPanel,UsageHero}.tsx`：pi usage script 集成
- [ ] `hooks/{useDirectorySettings,useDragSort,useSettingsForm,useSettings,useProxyStatus}.ts`：pi 适配
- [ ] `lib/api/{deeplink,model-fetch,skills,index}.ts`/`lib/query/{failover,index,mutations}.ts`/`lib/modelsDevPricing.ts`：pi 集成
- [ ] `utils/errorUtils.ts`/`types.ts`：pi error + type
- [ ] 门禁全绿 → commit

### P4 — 40d747c0 session usage + docs + i18n 收口 + 全量门禁
- [ ] `services/session_usage_pi.rs`（纯新增，1496 行）：pi session JSONL importer + token/cost 解析 + dedup
- [ ] `services/session_usage.rs`：sync_all_unlocked 加 "Pi" merge_sync_step
- [ ] `services/mod.rs`：加 `pub mod session_usage_pi`
- [ ] `database/schema.rs`：SCHEMA_VERSION 16→17 + `migrate_v16_to_v17`（session_usage_dedup_ledger）+ 测试
- [ ] `database/{backup,mod}.rs`：v17 备份适配
- [ ] `services/usage_stats.rs`：pi session data source 适配
- [ ] `session_manager/providers/pi.rs`：40d747c0 增量（如已随 P1 落地则跳过）
- [ ] `types/usage.ts`：AppType 加 `"pi"` + KNOWN_APP_TYPES 加 "pi"
- [ ] `components/usage/UsageDashboard.tsx`：APP_FILTER_OPTIONS 含 pi（KNOWN_APP_TYPES 驱动）+ appFilter i18n
- [ ] `components/usage/UsageHero.tsx`：TITLE_THEMES 加 pi（fuchsia）
- [ ] `i18n/locales/{en,ja,zh}.json`：pi 顶层命名空间 + provider/settings/apps/appSwitcher 散键 + usage.appFilter.pi；**丢弃 zh-TW**
- [ ] `docs/pi-native-contract-zh.md`（纯新增，60 行）：移植行为契约
- [ ] 全量门禁最终跑一遍（含 build:web + smoke:web-server）
- [ ] commit



## P1 结果（2026-08-24，18340719）

P1 Rust 后端核心完成。36 文件改动（+3779/−46），7 新增文件。

### 实施要点
- 6 个纯新增 Rust 文件（pi_config/mod.rs、services/{pi_state,pi_prompt_files,provider/pi}.rs、commands/pi.rs、session_manager/providers/pi.rs）+ web_api/handlers/pi.rs（9 Axum routes）。
- app_config.rs：AppType::Pi 枚举 + 10 处 match arm（MCP/Skills/CommonConfigSnippets/mcp_for/prompts/migration）；SkillApps 加 pi:bool 字段；McpRoot 加 pi:McpConfig 空槽位。
- settings.rs：pi_config_dir 字段 + get_pi_override_dir + resolve_override_path 改 pub(crate)。
- error.rs：Conflict variant（pi_config revision 冲突）。
- terminal/mod.rs：shell_escape 改 pub(crate)（pi session_manager 复用）。
- provider/mod.rs：mod pi + update_pi_usage_script + normalize_usage_script_credential_overrides helper（复用 fork 既有 extract_provider_usage_credentials）+ ProviderService 6 方法 Pi 早返回。
- 40 处 AppType::Pi 非穷尽 match（14 文件）按上游 84e75ad2 落地，全丢弃 ClaudeDesktop arm。
- 9 个 pi 命令完整注册：commands/prompt.rs 补 6 prompt 命令 + lib.rs generate_handler! + web-commands.ts + Axum handlers。
- Web build 适配：examples/server.rs 加 #[path] mod pi_config;，examples/web_services.rs 加 pi_prompt_files + pi_state。

### 门禁（全绿）
- cargo fmt / format:check / typecheck ✓
- check:web-routes 292 commands / 0 missing/mismatch/dangling ✓
- check:locales en/ja/zh 各 2506 parity ✓
- desktop cargo check + web cargo check ✓（0 errors）
- test:unit 171 files / 1002 tests ✓
- cargo test --lib 2060 passed / 0 failed / 5 ignored ✓
- Rust parity 37 passed ✓（web_api::/dual_runtime_parity::/web_proxy_lifecycle::）
- pi focused 28 passed ✓（pi::/pi_config::/pi_prompt_files::）
- build:web exit 0（23.13s）+ smoke:web-server exit 0 ✓

### Carry-forward
- ClaudeDesktop 零回潮（仅既有 usage logger/stats 兼容残留 + profile.rs 注释）。
- zh-TW 零回潮（fork 无 zh-TW.json）。
- .pi/ .pi-subagents/ untracked 未提交。
- 安全上限：pi_config MAX_PI_FILE_BYTES=1MiB 保留；2s JS deadline/body limit 未触碰。


## P2 结果（2026-08-24，a7fac324）

P2 前端 provider UI 完成。59 文件改动（+6454/−127）。

### 实施要点
- 12 个纯新增前端文件：`config/{piModelCatalog,piProviderPresets,piThinkingProfiles}.ts`、`lib/{api/pi,query/pi,piPromptSlug,piPromptTemplate}.ts`、`components/providers/forms/{PiProviderForm,RequestHeadersEditor,StructuredOptionsEditor,helpers/requestHeaders}.tsx`、`components/providers/ProviderStatusBadge.tsx`。
- `lib/api/types.ts` AppId union 加 `"pi"`；`config/appConfig.tsx` APP_IDS/SKILLS_APP_IDS 加 pi + `McpAppId = Exclude<AppId,"openclaw"|"pi">`（Pi 无 MCP）+ APP_ICON_MAP pi SVG。
- `ProviderList`：pi 成员身份经 `usePiCurrentState` 折入既有 `isProviderInConfig`；`isPiAuthoritativeStateReady` gate + amber 读失败 `role="alert"` 提示；`supportsFailover = appId !== "pi"`。
- `ProviderActions`/`ProviderCard`：pi 纳入 `isAdditiveMode`；新增 `isStateChangeProtected`（冻结主按钮 + 阻断删除 + `pi.current.stateUnavailableHint`）；3 个路由徽章迁移到 `ProviderStatusBadge`。
- `App.tsx`：`usePiCurrentState` + `handleEnablePiProvider` + pi `onSwitch`/`onRemoveFromConfig` dispatch + `translatePiProviderMutationError` + pi 全局默认警告 memo + pi live-id 复制分支；proxy/failover toggle 与 MCP 头部按钮对 pi 隐藏。
- Q4 model_fetch 通用增强随本批落地：`services/model_fetch.rs` request_headers + api_format（复用 `ClaudeAdapter::get_auth_headers` 单一真理源）+ `MAX_REQUEST_HEADERS=64` / name 256B / value 16KiB 上限；`commands/model_fetch.rs` + `web_api/handlers/config.rs` 参数透传。
- i18n：`pi` 命名空间（empty/provider/form/current，各 60 键）+ 散键（`common.collapse`/`notifications.removeFromConfigFailed`/`confirm.piDefaultProviderWarning`/`settings.piConfigDir`/`settings.browsePlaceholderPi`/`apps.pi`/`provider.needsRouting`/`provider.noRoutingSupport`）；en/ja/zh 各 2574 parity；zh-TW 未创建。

### 证实不适用（未移植，有据）
- `FullScreenPanel contentClassName`：fork 无该 prop。
- `ProviderPresetSelector` FormLabel→Label：fork `FormLabel` 显式容忍 `<FormField>` 外使用（`ui/form.tsx:44-51`）。
- `ProviderActions isRemovalProtected`：上游为 OpenClaw/Hermes 重构，pi 传 false；保留 fork `disableRemoveFromConfig`。
- `EditProviderDialog` submit-ready 管线：pi `isSubmitReady = isEdit || presetSelected`，edit 恒真。
- `useProviderHealth` enabled gate：fork 已有 `healthEnabled = isProxyRunning && isInFailoverQueue`。
- `settings.piConfigDirDescription` / `oneClickInstallHint`：fork DirectoryInput 恒传 `description={undefined}`；install hint 列表本就未含 OpenClaw/Hermes。
- 未引入 `ADDITIVE_APP_IDS`/`PROXY_APP_IDS`/`isAdditiveAppId`/`isProxyAppId`（上游专有构件），沿用 fork 内联列表风格。

### 门禁（全绿）
- cargo fmt / format:check / typecheck ✓
- check:web-routes 292 commands / 280 routes / 0 missing/mismatch/dangling/fallback ✓
- check:locales en/ja/zh 各 2574 parity ✓
- desktop + web cargo check ✓（0 errors）
- test:unit **171 files / 1008 tests** ✓（+6 新 pi 测试）
- Rust parity 37 passed ✓；`model_fetch` focused 28 passed ✓

### Carry-forward
- ClaudeDesktop：`grep src/` 6 处全为 HEAD 既有残留（`ensureClaudeDesktopOfficialSeed` 3 处 + `notifications.proxyReasonClaudeDesktop` 3 locale），零新增。
- zh-TW 零回潮；`.pi/`/`.pi-subagents/` 未 stage。

## 验证命令汇总

见每批门禁块。关键额外检查：
- DB 迁移连续性：v16→v17 不跳号；`migrate_v16_to_v17` 测试绿
- Web API parity：`check:web-routes` 0 missing/mismatch/fallback（pi 9 命令注册）
- ClaudeDesktop/zh-TW 零回潮：`grep -r "ClaudeDesktop\|zh-TW" src-tauri/src/ src/i18n/` 为空
- Carry-forward 红线：2s/16MiB/256KiB/128MiB/32MiB 安全上限不退化

## review gates

- 每批 commit 前：全量门禁全绿（test:unit 必须全绿，非 flake 项）。
- P1 后：AppType::Pi dispatch 完整性 + web-commands.ts 9 命令注册 + check:web-routes 0 gap。
- P4 后：DB v16→v17 迁移连续性 + pi session usage importer 回归 + 真实 Web 服务冒烟。
- 全部完成后：父任务跨子任务集成 review（Web API parity、安全边界、与主体无冲突回归）+ 统一 changelog 后合 main。

## 风险点与回滚

- **单批失败**：`git reset --hard <上一批 commit>` 回滚。
- **DB 迁移失败**：从 P4 前备份恢复；fork 既有备份/replace 锁流程。
- **ClaudeDesktop hunk 误带入**：每批 commit 前 grep 确认 `ClaudeDesktop`/`claude-desktop` 零回潮。
- **PromptPanel 非 pi 路径误重构**：P3 只取 pi dispatch hunk，不删 ManagementListSearch/filteredPromptEntries。
- **子任务阻塞父主体**：子任务在同分支，但独立 PR；可先合父主体，子任务后续跟进（PRD 已述）。
