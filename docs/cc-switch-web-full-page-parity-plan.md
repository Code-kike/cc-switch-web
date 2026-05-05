# cc-switch Web 全页面扫平同步计划

## Summary

目标是让当前 `cc-switch` Web UI 的所有可见页面不再出现 404、空 handler 或未实现错误。每个入口必须满足以下三种状态之一：

- Web 版功能可用。
- Web UI 明确禁用并说明原因。
- 后端返回明确的 Web 专属错误码，而不是落到通用 404。

当前审计基线分三层：

- 初始缺口：`src/lib/api/web-commands.ts` 共 259 个命令，约 76 个命中后端路由，25 个显式 `unsupported`，158 个已映射但没有 Web route。
- 当前路由覆盖：`pnpm check:web-routes -- --list-parity` 为 `259 commands / 247 routes / 20 wildcardRoutes / 30 unsupported / webReplacements 7 / missing 0 / parityExact 0 / parityFallback 0`。
- 计划推进进度：相较初稿阶段，real route 已从 206 增至 247，`parityFallback` 已从 40 清零到 0。
- 完成判定：`missing 0`、`parityExact 0` 和 `parityFallback 0` 说明 route 分类层已经收口；7 条 `webReplacements` 已逐条绑定到浏览器替代链路和 rendered-page 证据，`Manual Regression Matrix` 当前为 `72 pass / 72 total`。剩余重点是发布前人工复核、远程安全补强和新增入口防回退，而不是 P0/P1 基础功能开发。

注意：`missing 0` 只代表“不再落到通用 404”。如果命中 `src-tauri/src/web_api/handlers/parity.rs` 的 catch-all fallback，它仍然只是明确返回 `WEB_NOT_SUPPORTED`、`WEB_UPLOAD_REQUIRED` 或 `WEB_DESKTOP_ONLY`，不能算功能扫平完成。

开发范围只包含 `/home/orion/Workspace/github/cc-switch-web`，不改动 `Code-kike/cc-switch-web`。

## Current Audit Snapshot

审计时间：2026-05-05。

当前已经达到的状态：

- 前端命令总数为 259。
- 显式 unsupported 命令为 30。
- 显式 web replacement 命令为 7。
- 已注册精确 route 为 247。
- wildcard route 为 20。
- 通用 404 缺口为 0。
- 精确命中 parity handler 的 route 为 0。
- 仍落入 parity wildcard fallback 的 route 为 0。

这说明“页面不会直接 404”的 route 分类阶段已经完成，route 层不再依赖 parity catch-all，也不再把文件替代型命令误计入 parity 待收口集合。结合当前真实 `web-server` smoke、rendered-page 测试套件和 72 行回归矩阵，当前可见 P0/P1 主链路已经具备页面级证据；后续重点不是继续增加 catch-all 或补基础 handler，而是发布前复核、远程安全补强和新增入口防回退。

## Current Route And Runtime Closure

按 2026-05-05 当前进度，这份计划已经不需要再回到“大面积补 route”或 P0/P1 主链路开发阶段，发布前复核与后续维护应围绕以下三类风险执行：

- `显式 Web 语义页面闭环`：`get_circuit_breaker_stats -> WEB_NOT_SUPPORTED`、lightweight mode `WEB_DESKTOP_ONLY` 这类接口已经从 route 层收口；其中 `/settings/proxy` 的 config-only / runtime-unavailable 文案已由 `tests/integration/ProxyTabContent.web-server.test.tsx` 跑通，`/about` 的 Web 环境检查、环境检测失败 alert 与更新语义也已收口。当前剩余重点已从“页面没语义”转成长尾人工矩阵与极端异常态补验。
- `Skills 搜索与仓库边角`：`UnifiedSkillsPanel` 主链路、`RepoManagerPanel` 的 owner/name 重复覆盖提示，以及 add/remove pending/失败反馈已经有 focused + rendered-page 证据；2026-05-05 暴露出来的 `SkillsPage` source-selection 回归也已收口，现在无 repo 时会从自动 `repos` fallback 到 `skills.sh`，repo 重新出现时若此前只是自动 fallback 会回切 `repos`，而用户显式选中的 `skills.sh` 会继续保持；同一套真实 `web-server` 测试现在也验证了初始无 repo 时的自动 `skills.sh` fallback、手动切回 `repos` 后的空仓库态和添加仓库入口。同日又补了两条 skills.sh 状态机修复：当新查询仍在请求中时，页面不再短暂回显上一条查询的旧结果，也不会先误显示 `noResults`；当输入被清空或缩短到不足 2 个字符时，旧结果与旧错误态会立即清空并回到 idle placeholder。`UnifiedSkillsPanel` 的 import/toggle/uninstall/restore/ZIP install 失败提示也已接到 `formatSkillError()`，`SkillsPage` 自身的 install failure 现在也会先走 `extractErrorMessage()` 再交给 `formatSkillError()`，不再把 plain object error 压扁成 `[object Object]`。`tests/components/UnifiedSkillsPanel.test.tsx` 现已补上 ZIP picker 取消态的 silent no-op。`tests/components/SkillsPage.test.tsx` 与 `tests/integration/SkillsPage.web-server.test.tsx` 都已恢复通过；同一套真实 `web-server` 页面测试现在也覆盖 skills.sh 分页、空结果、503 异常可见、detail 展示、trim 后搜索，以及同 query 重试恢复。后续重点转成人工矩阵补验。
- `页面壳层与容器级联动回归`：`Prompts`、`Sessions`、`MCP`、`Workspace`、`OpenClaw`、`Hermes`、`Auth Center` 这些页面的主链路大都已有 rendered-page 证据；其中 Prompt create/edit 的 Gemini 真实页面流也已通过“本地立即更新 + silent reload”方式收口，不再受全量 reload 阻塞，Prompt live-file 读取异常也已有真实 `web-server` 页面 toast detail 证据，MCP 删除确认取消也已补到真实页面层。剩余重点转成错误态文案、筛选器联动、失败态和人工矩阵补验。`JsonEditor` 的通用 format/parse 失败也已统一收敛到 `jsonEditor.invalidJson`，不再把 raw `SyntaxError` 直出到页面。

其中 `ProviderList` 已从当前 blocker 退到长尾维护项：`tests/integration/ProviderList.web-server.test.tsx` 现已覆盖 `Live Config / DB Only`、provider limits、provider usage stats、OpenClaw/Hermes rendered import，以及 diagnostics error badges 的真实 rendered-page 路径；`tests/components/ProviderCard.test.tsx` 则继续兜底 health/limits/usage/error split 语义，`tests/components/ProviderHealthBadge.test.tsx` 又补上了 health 正向态与熔断态的最小组件覆盖。此前暴露的 React Query `act(...)` 和 `<button>` 嵌套 warning 也已经清理，后续主要保留长尾 app 手工矩阵。

`UsageDashboard` 这一轮也已经从“主链路可用但测试噪音较大”收口到“页面级证据稳定”：图表补上初始尺寸，`ResizeObserver` 测试基座会返回正尺寸，request detail dialog 也已补可访问性描述；`tests/components/UsageDashboard.test.tsx` 现已额外覆盖 tab 选择在 app filter / refresh interval / date range 变化后不会回退到 `logs`，`tests/integration/UsageDashboard.web-server.test.tsx` 也已补上 session import 之后的 app/date/refresh 过滤器、logs/providers/models 多面板联动与真实后端数据回显。当前剩余工作主要转成人工矩阵与少量长尾空态/失败态补验，不再是 Recharts/Dialog warning 清理或主链路联动缺口。

2026-05-05 同日还额外收口了一批已经挂载在主页面上的 raw-error 漏洞：`WebdavSyncSection` 的 test/save/upload/download 失败 toast、`useSettings` 的 auto-save/manual save 失败、`useGlobalProxy` 的 save/test/scan failure、`src/lib/api/globalProxy.ts` 的 invoke 错误解包、`ProviderList` 的“导入当前配置”失败、`EnvWarningBanner` 的删除环境变量失败、`/settings/proxy` 下 `RectifierConfigPanel` 的 load/save failure、Auth Center 链路中 `useManagedAuth` 的 start-login / poll / logout / remove / set-default failure、MCP 表单链路里 `McpFormModal` / `useMcpValidation` 的 JSON/TOML parse failure，以及 providers 主链路里 `useStreamCheck` / `EndpointSpeedTest` 的 thrown-error/save-failure 路径，都已经统一改成 `extractErrorMessage()` 路径或可见 alert，不再把 plain object 错误显示成空白或 `[object Object]`。`JsonEditor` 也已改为用 `jsonEditor.invalidJson` 兜底，不再泄漏 raw `SyntaxError.message`。对应 focused 证据为 `tests/components/WebdavSyncSection.test.tsx`、`tests/integration/WebdavSyncSection.web-server.test.tsx`、`tests/hooks/useSettings.test.tsx`、`tests/hooks/useGlobalProxy.test.tsx`、`tests/lib/globalProxy.test.ts`、`tests/components/ProviderList.test.tsx`、`tests/components/EnvWarningBanner.test.tsx`、`tests/components/RectifierConfigPanel.test.tsx`、`tests/hooks/useManagedAuth.test.tsx`、`tests/hooks/useMcpValidation.test.tsx`、`tests/components/McpFormModal.test.tsx`、`tests/hooks/useStreamCheck.test.tsx`、`tests/components/EndpointSpeedTest.test.tsx` 与 `tests/components/JsonEditor.test.tsx`。

当前代码审计还额外确认了一类“不要误判成页面 blocker”的遗留项：`src/components/proxy/CircuitBreakerConfigPanel.tsx` 与 `src/hooks/useProxyConfig.ts` 这类旧路径目前未见挂载到主 Web 页面；它们更适合作为后续低优先级死代码/一致性清理，而不应挤占当前可见页面收口的开发顺序。

本轮复扫 `src/` 后，剩余 raw-error grep 命中进一步缩窄到两个非页面 blocker：`src/components/proxy/CircuitBreakerConfigPanel.tsx` 仍保留 `String(error)`，但当前未发现挂载引用；`src/utils/providerConfigUtils.ts` 则是工具层返回 `error: String(e)`，不属于当前 mounted 页面直接 toast/alert 的高优先级链路。后续若重新挂载 `CircuitBreakerConfigPanel` 或让该工具层错误直达 UI，再单独并入页面级扫平。

因此，当前的执行重点应该是“把剩余可见页面补齐证据并收口交互”，而不是继续追求 route 数字变化。

当前已经验证通过的静态门禁：

- `pnpm check:web-routes -- --list-parity`
- `pnpm typecheck`
- `pnpm build:web`
- `cargo check --manifest-path src-tauri/Cargo.toml --no-default-features --features web-server --example server`

当前已经验证通过的真实运行时门禁：

- `pnpm smoke:web-server` 已具备可复跑脚本，能使用临时 `CC_SWITCH_DATA_DIR` 启动真实 `web-server`，并稳定覆盖 `/api/health`、`/`、`/api/settings/get-settings`、`/api/proxy/get-proxy-status`、`/api/backups/list-db-backups`、`/api/mcp/get-mcp-servers`、`/api/prompts/get-prompts?app=claude`、`/api/usage/get-usage-summary`、`/api/usage/get-usage-data-sources`、`/api/system/check_for_updates`、`/api/system/get_tool_versions`、Copilot/Codex `auth-get-status`，以及 `open_app_config_folder -> WEB_DESKTOP_ONLY`、`export_config_to_file -> WEB_UPLOAD_REQUIRED` 等基础探针。
- 2026-05-04 同日已经把这条 smoke 扩展到真实文件流探针：`export-config-download`、`import-config-upload`、`import-prompt-upload` 与 `prompts-claude-after-upload`，不再只测 route 是否存在，而是开始验证浏览器 upload/download 的后端闭环。
- 同一轮也已把 Providers 的“导入当前配置”扩展成六类 app 的真实探针：Claude / Codex / Gemini 走 `/api/config/import-default-config`，OpenCode / OpenClaw / Hermes 分别走各自的 live import route，并在导入后立即回查 `/api/providers/get-providers` 确认数据库侧结果。
- 在此基础上，`pnpm smoke:web-server` 还新增了三条代表性写回/切换探针：Claude 当前 provider 更新后会直接写回 `~/.claude/settings.json`，Claude 新增第二个 provider 后调用 `switch_provider` 也会写回 live `settings.json` 并更新 current provider，OpenCode 的 live-managed provider 更新后会直接同步回 `~/.config/opencode/opencode.json`。这让 Providers 的 runtime 验证从“只会导入”前进到“至少已有代表性写回与切换闭环”。
- 同日也已把这条 smoke 扩展到真实 Sessions 与 MCP 探针：脚本现在会在隔离 `CC_SWITCH_TEST_HOME` 下预置 Claude / Codex / Gemini / OpenCode / Hermes 的 live MCP 配置，以及三条真实 Codex session JSONL，验证 unified MCP import/merge、legacy Claude MCP read/config、app toggle、set enabled、unified/legacy upsert/delete、config route upsert/delete，以及 sessions list/messages/delete/delete many 和 `launch-session-terminal -> WEB_DESKTOP_ONLY`。
- 同一轮还把 Batch D 的 runtime evidence 接进了真服 smoke：脚本现在会额外预置 `~/.openclaw/workspace/AGENTS.md`、`~/.openclaw/workspace/memory/2026-03-04.md`、`~/.openclaw/openclaw.json` 的 `env/tools/agents.defaults`、以及 `~/.hermes/config.yaml` 的 `memory:` 段和 `~/.hermes/memories/{MEMORY,USER}.md`，并验证 Workspace read/write、Daily Memory list/read/write/search/delete、`open_workspace_directory -> WEB_DESKTOP_ONLY`、OpenClaw env/tools/agents defaults 读写与 health scan、Hermes memory get/set/limits/toggle，以及 DeepLink parse/merge/import 的真实闭环。
- 同日还继续把 Batch E 的 Proxy / Failover 运行时证据接进真服 smoke：脚本现在会在 Claude provider 切换完成后验证 `get_proxy_takeover_status`、`get_proxy_config_for_app` / `update_proxy_config_for_app`、failover queue 的 empty-queue guard / available providers / add / remove、`set_auto_failover_enabled` 的持久化，以及 `get_circuit_breaker_stats -> WEB_NOT_SUPPORTED` 的 Web 占位语义。过程中还额外修正了一个后端兼容问题：`ProxyTakeoverStatus` Rust 返回值已补齐 `hermes` 字段，与前端类型保持一致。
- 2026-05-04 同日也把 Batch E 的 Usage / Subscription 运行时证据接进真服 smoke：脚本现在会额外预置隔离 `~/.codex/archived_sessions/smoke-usage-session.jsonl`，验证 `sync-session-usage -> usage-data-sources -> usage-summary -> usage-trends -> request logs -> request detail` 的真实链路，以及 `get_model_pricing` / `update_model_pricing` / `delete_model_pricing`、`get_subscription_quota` 的 not_found / parse_error 态、`get_balance` / `get_coding_plan_quota` 的确定性无外网错误态、`testUsageScript -> BAD_REQUEST` 的结构化错误语义。这一轮还额外暴露并修正了一个真实后端兼容问题：`src-tauri/src/services/usage_stats.rs` 中 `get_request_detail()` 的 SQL 现在已显式使用 `l.created_at` 等限定列，不再因为 join `providers` 后的歧义列而返回 500。
- 2026-05-05 又把 `UsageDashboard` 中已挂载的 pricing 与 session sync 错误态一起收口：`PricingConfigPanel` 的 query error、不成功的 app-defaults 读写、`PricingEditModal` 的保存失败，以及 `DataSourceBar` 的 session sync 失败现在都统一走 `extractErrorMessage()` 与翻译文案/description，不再在页面或 toast 中裸出 `String(error)` 或丢失后端 detail；对应 focused tests 为 `tests/components/PricingConfigPanel.test.tsx`、`tests/components/PricingEditModal.test.tsx`、`tests/components/DataSourceBar.test.tsx`，并通过 `tests/integration/UsageDashboard.web-server.test.tsx` 复验 mounted 页面未回归。
- 这轮 MCP smoke 同时暴露并修正了两个 Web 兼容问题：`src-tauri/src/web_api/handlers/mcp.rs` 中 `validate_mcp_command` 现按 `POST` JSON body 读取 `cmd`，不再错误依赖 query string；`src-tauri/src/commands/mcp.rs` 与对应 Web handler 返回的 `McpConfigResponse` 已统一序列化为前端期望的 `configPath` camelCase 字段。
- 其中 “导出后立刻回传导入” 的空数据 roundtrip 问题已经修复：`src-tauri/src/database/backup.rs` 不再拒绝空 provider / 空 MCP 的合法导出 SQL，且 `src-tauri/tests/import_export_sync.rs` 已补回归测试。
- Prompt 上传导入的 live-file 副作用也已在 2026-05-04 收口：`src-tauri/src/services/prompt.rs` 里的 `PromptService::upsert_prompt()` 现在只会在“原本 enabled -> 现在 disabled”且确实已无任何启用 prompt 时才清空 live 文件；新导入、新建或编辑 disabled prompt 不再误写真实 `CLAUDE.md / AGENTS.md / GEMINI.md`。
- 这条修复已有专门 Rust 回归测试：`cargo test --manifest-path src-tauri/Cargo.toml --test prompt_service` 通过，`src-tauri/tests/prompt_service.rs` 已覆盖“导入 disabled prompt 不触碰现有 live 文件”和“禁用最后一个 enabled prompt 时仍会清空 live 文件”两条语义。
- 因此，这条运行时 smoke 目前的正确结论是“基础探针、文件流、Providers、Sessions、MCP、Workspace / OpenClaw / Hermes / DeepLink、Proxy / Failover、Usage / Subscription 扩展探针均已收口并可复跑”；它本身仍只是 API/route 级 smoke，但当前已经由 `tests/integration/*.web-server.test.tsx` 的 rendered-page 套件和 `Manual Regression Matrix` 补齐逐页面交互证据。

已经从 parity fallback 移出的批次：

- WebDAV：`webdav_sync_save_settings`、`webdav_test_connection`、`webdav_sync_upload`、`webdav_sync_download`、`webdav_sync_fetch_remote_info` 已由 `settings.rs` 与 `webdav.rs` 提供真实 handler。
- Usage / quota：`get_balance`、`get_coding_plan_quota`、`testUsageScript` 已由 `subscription.rs` 与 `usage.rs` 提供真实 handler。
- Provider diagnostics：`queryProviderUsage`、`stream_check_provider`、`stream_check_all_providers` 已由 `providers.rs` 提供真实 handler。
- Config / stream / plugin / env：`get_claude_config_status`、`get_claude_common_config_snippet`、`set_claude_common_config_snippet`、`get_common_config_snippet`、`set_common_config_snippet`、`extract_common_config_snippet`、`fetch_models_for_config`、`get_stream_check_config`、`save_stream_check_config`、`read_claude_plugin_config`、`apply_claude_plugin_config`、`delete_env_vars` 已转为真实 handler。
- Auth / accounts：`auth_start_login`、`auth_poll_for_account`、`auth_list_accounts`、`auth_get_status`、`auth_remove_account`、`auth_set_default_account`、`auth_logout` 已由 `auth.rs` 直连 Copilot / Codex OAuth manager。
- System / tooling：`apply_claude_onboarding_skip`、`clear_claude_onboarding_skip`、`check_for_updates`、`get_tool_versions`、`test_api_endpoints`、`reset_circuit_breaker`、`get_circuit_breaker_stats` 已由 `system.rs` 提供真实 Web 语义或显式 Web 降级语义。
- Lightweight mode：`enter_lightweight_mode`、`exit_lightweight_mode`、`is_lightweight_mode` 已从 wildcard fallback 移出，当前改为显式 `WEB_DESKTOP_ONLY` 语义。

当前 route 分类中仍需要页面级验收的显式 Web 文件替代命令为：

- Web 文件替代：`export_config_to_file`、`import_config_from_file`、`import_prompt_from_file`、`install_skills_from_zip`、`open_file_dialog`、`open_zip_file_dialog`、`save_file_dialog`。

7 条文件流命令 `export_config_to_file`、`import_config_from_file`、`import_prompt_from_file`、`install_skills_from_zip`、`open_file_dialog`、`open_zip_file_dialog`、`save_file_dialog` 已在 `src/lib/api/web-commands.ts` 中改为显式 `webReplacement`，不再继续计入 parity 审计；对应的浏览器替代证据由 `tests/lib/web-file-replacements.test.ts`、`tests/hooks/useImportExport.test.tsx`、`tests/hooks/usePromptActions.test.tsx`、`tests/components/UnifiedSkillsPanel.test.tsx` 与 `pnpm smoke:web-server` 的 upload/download 探针共同覆盖。

5 条桌面专属命令 `open_app_config_folder`、`open_config_folder`、`open_provider_terminal`、`open_workspace_directory`、`pick_directory` 已在 `src/lib/api/web-commands.ts` 中改为显式 `unsupported`，由 adapter 在 Web 下直接抛 `WebNotSupportedError`，不再继续算作 parity 收口项；对应的 UI 降级证据分别由 `tests/components/DirectorySettings.test.tsx`、`tests/components/ProviderList.test.tsx`、`tests/components/WorkspacePanels.test.tsx` 与 `tests/lib/adapter.test.ts` 覆盖，其中 `tests/integration/DirectorySettings.web-server.test.tsx` 又把 Settings / Advanced / 配置目录页的 Web 禁用浏览 + 手动路径保存 / reload 回显补到了真实 rendered-page 层。

当前 route 层剩余问题已经不再是 wildcard fallback 或 parity route，而是这 7 条显式 `webReplacement` 文件流命令的产品语义与前端交互是否真正收口：

- 文件替代型 parity route 必须确认 UI 端已经完整切到浏览器 upload/download，而不是仍然先走桌面 dialog 命令再在前端二次补救。当前 Config Import/Export、Prompt Import 与 Skills ZIP Install 都已经具备 rendered-page 证据；2026-05-05 复扫 `open_file_dialog` / `open_zip_file_dialog` / `save_file_dialog` 调用点后，实际入口已收敛到 Settings 导入导出、Prompt 导入与 Skills ZIP 这三组已覆盖页面，后续主要保留人工回归，防止新增壳层绕回桌面链路。
- `check_for_updates`、`get_circuit_breaker_stats`、`lightweight mode` 这类已经落地为“显式 Web 语义”的接口，当前前两者已具备对应页面证据；lightweight mode 在当前 Web UI 中未发现挂载入口，后续主要防止新增入口绕过桌面专属语义。

## Current UI Scope And Gap Baseline

当前“全页面扫平”的范围应以现有 Web UI 可见入口为准，而不是只看后端命令数量。当前主视图来自 `src/App.tsx`：

- `providers`
- `settings`
- `prompts`
- `skills`
- `skillsDiscovery`
- `mcp`
- `agents`
- `universal`
- `sessions`
- `workspace`
- `openclawEnv`
- `openclawTools`
- `openclawAgents`
- `hermesMemory`

其中 `settings` 还拆成 6 个可见 tab：

- `general`
- `proxy`
- `auth`
- `advanced`
- `usage`
- `about`

`advanced` tab 当前至少还有 6 个可见分组需要单独验收：

- 配置目录
- 配置导入导出
- 数据库备份恢复
- WebDAV 云同步
- 模型测试配置
- 日志配置

因此，后续回归不能只盯住你已经发现的 agent 导入、skills、提示词、会话管理。按现有页面结构，以下入口都必须持续保留页面级证据，防止新增功能重新引入 Web 缺口：

- Auth Center 已经具备 focused tests、真实 `web-server` smoke 和本地假认证服务闭环，不再属于 route/runtime blocker；远程访问安全提示、轮询取消和失败重试也已进入页面证据，后续保留为人工矩阵复核。
- Providers 页不能只以“添加供应商成功”判定完成，还要覆盖导入当前配置、live 状态读取、切换写回、usage/health/limits、stream check、模型拉取、通用配置片段。
- Settings 页的 `proxy`、`usage`、`advanced` 都是显式入口，因此 proxy/failover、usage dashboard、备份恢复、WebDAV、模型测试、日志配置必须按页面闭环验收；其中 `get_circuit_breaker_stats` 的页面占位语义已收口，lightweight mode 当前未发现 Web 入口，后续主要保留回归防护与人工矩阵。
- `agents`、`universal`、`mcp`、`workspace`、`openclaw*`、`hermesMemory` 都是一级视图，不能等 Skills/Prompts/Sessions 做完后再模糊处理。
- `sessions` 与 `mcp` 已不再属于“基础 route/runtime CRUD 未补齐”的范畴：`pnpm smoke:web-server` 现在已覆盖 sessions list/messages/delete/delete many 与 MCP unified/legacy read/upsert/delete/enable/validate/config route，真实页面测试也已覆盖 sessions page flow、MCP import/add/edit/toggle/delete、壳层工具栏接线、浏览器交互和错误提示。
- `skills` 与 `prompts` 也不再是“基础功能完全未做”的状态；它们当前已完成主链路，skills.sh 空结果、网络异常/retry、初始空仓库 fallback、trim 查询、同 query retry、Prompt 导入/创建/编辑/启停/删除和 live-file IO 异常已有真实页面或 focused 证据。
- 所有文件交互型入口都要确认已经切到 Web upload/download，而不是仅靠 parity route 包装桌面 file dialog；当前 SQL 导入/导出、Prompt 文件导入和 Skills ZIP 安装都已具备浏览器替代证据，`open_file_dialog`、`open_zip_file_dialog`、`save_file_dialog` 也已在矩阵中绑定到浏览器替代链路。
- 所有桌面专属入口都要做前后端双重降级：UI 隐藏或禁用，加上结构化错误码；只在 API fallback 返回错误还不够。

### 基础功能同步开发清单

除了已经明确暴露出来的 `agents` 导入当前配置、`skills`、`prompts`、`sessions` 之外，后续同批必须同步补齐的基础能力还有：

- `providers` 主链路：添加、编辑、删除、切换、导入当前配置、读取 live 状态、写回配置、usage / health / limits、stream check、模型拉取、common config snippets。
- `settings` / `proxy` / `usage` / `advanced`：代理和 failover、usage dashboard、数据库备份恢复、配置导入导出、WebDAV、模型测试、日志配置、更新和 about 的 Web 语义。
- 一级功能页：`agents`、`universal`、`mcp`、`workspace`、`openclaw*`、`hermesMemory`、`auth` 必须各自有可用的页面级 smoke 和明确错误态。
- 文件交互页：所有 `open_file` / `open_zip` / `save_file` 入口都必须改成浏览器 upload/download，不允许回落桌面文件选择器。
- 桌面专属能力：如果 Web 版不能做，必须显式禁用并说明原因，不能只保留半成品按钮。
- 页面壳层验收：空态、加载态、取消态、失败态、权限不足态都要统一走结构化提示。

## Full Page Delivery Matrix

为了避免后续只按接口或模块推进，而忽略“页面本身是否能用”，这里把当前所有可见 Web 入口固化成页面矩阵。后续每一批开发、测试和 smoke 记录都应直接引用这一节。

### 一级视图矩阵

| 视图              | 当前页面组件             | Web 最低交付范围                                                                                                                      | 建议批次    |
| ----------------- | ------------------------ | ------------------------------------------------------------------------------------------------------------------------------------- | ----------- |
| `providers`       | `ProviderList`           | 六类 app 的 provider CRUD、当前 provider 切换、导入当前配置、live 状态读取、usage/health/limits、stream check、模型拉取、通用配置片段 | Batch B     |
| `settings`        | `SettingsPage`           | 六个 tab 都能打开，且各 tab 的显式入口不出现 404、空 handler 或未捕获异常                                                             | Batch A + E |
| `prompts`         | `PromptPanel`            | 列表、创建/编辑、删除、启停、当前文件读取、浏览器上传导入                                                                             | Batch C     |
| `skills`          | `UnifiedSkillsPanel`     | 已安装列表、启停、卸载、从 apps 导入、ZIP 安装、备份恢复、更新检查/更新                                                               | Batch C     |
| `skillsDiscovery` | `SkillsPage`             | repo discover/search/install 主流程可用，和已安装技能页联动一致                                                                       | Batch C     |
| `mcp`             | `UnifiedMcpPanel`        | unified MCP 与 legacy Claude MCP 的 list/read/upsert/delete/enable/validate                                                           | Batch C     |
| `agents`          | `AgentsPanel`            | 当前是显式 `Coming Soon` 占位页；若继续保留可见入口，则必须保持“稳定占位 + 无死按钮 + 文案明确”，否则应隐藏入口                       | Batch D     |
| `universal`       | `UniversalProviderPanel` | 统一供应商 getAll/upsert/delete/sync，且同步结果能反馈到各 app                                                                        | Batch B     |
| `sessions`        | `SessionManagerPage`     | 会话列表、消息读取、单删、批删、resume Web 降级语义                                                                                   | Batch C     |
| `workspace`       | `WorkspaceFilesPanel`    | workspace files / daily memory 的 read/write/list/search/delete，且有服务端路径边界                                                   | Batch D     |
| `openclawEnv`     | `EnvPanel`               | OpenClaw env 默认项读写、校验、保存                                                                                                   | Batch D     |
| `openclawTools`   | `ToolsPanel`             | OpenClaw tools 默认项读写、旧字段迁移与保存                                                                                           | Batch D     |
| `openclawAgents`  | `AgentsDefaultsPanel`    | OpenClaw agents 默认项读写、旧 model 配置清理与保存                                                                                   | Batch D     |
| `hermesMemory`    | `HermesMemoryPanel`      | Hermes memory get/set、启停与保存链路；并明确“服务端本机地址不等于远程浏览器机器”                                                     | Batch D     |

### Settings 子页矩阵

| `settings` 子页 | 当前范围                                                            | Web 最低交付范围                                                                   | 建议批次    |
| --------------- | ------------------------------------------------------------------- | ---------------------------------------------------------------------------------- | ----------- |
| `general`       | 语言、主题、显示项、技能存储位置等基础设置                          | autosave / restart required 语义明确，所有基础设置可读可写                         | Batch A     |
| `proxy`         | proxy、takeover、upstream、failover 相关面板                        | 配置、状态、takeover status、URL test、app proxy/failover 参数可读可写；本地 proxy 启停与 takeover 写入保留 desktop-only 提示，runtime stats 提供明确 Web 占位反馈 | Batch E     |
| `auth`          | `AuthCenterPanel`                                                   | login / poll / default account / logout / remove account 主流程可走通，或明确限制  | Batch B     |
| `advanced`      | 配置目录、配置导入导出、数据库备份恢复、WebDAV、模型测试、日志配置  | 目录类入口完成 Web 降级，文件流走 upload/download，运维项有最少一条闭环 smoke      | Batch A + E |
| `usage`         | dashboard、logs、detail、pricing、session sync、data sources、quota | 各面板可打开、核心数据链路可刷新，空态/失败态/无权限态有可见反馈                   | Batch E     |
| `about`         | 版本、工具版本、更新检查、环境检查、release notes                   | Web 模式按服务端语义展示版本与环境，更新行为改为下载页或 release 页面              | Batch A     |

### 占位页与非功能页规则

可见页面不一定都要在当前阶段实现完整功能，但必须符合以下规则之一：

- 页面已具备真实 Web 功能，并可被 smoke 证明。
- 页面明确是占位页或待开发页，文案稳定、没有会误导用户点击的死按钮。
- 页面在 Web 下被隐藏或替换成明确说明，而不是保留半成品入口。

这条规则当前首先适用于 `agents` 这类占位页。只要页面仍可见，就必须被纳入计划和回归矩阵；不能因为“暂时没有功能”就从扫平范围中排除。

## Strategy

本计划按“全页面扫平”推进，而不是只补最常用功能。优先级由用户可见入口决定：

1. 前端已有入口的功能必须先归类并处理。
2. 能复用现有 Tauri command/service 的，Web handler 只做参数解析、错误映射和 Web 特有文件流转。
3. 桌面专属能力不强行搬到 Web，必须在 UI 或 API 层明确标识不支持。
4. Web 文件交互使用浏览器上传/下载替代桌面文件选择器。
5. 安全完整化延后，但 server 默认仍只监听 `127.0.0.1`，不新增默认局域网暴露行为。

## Execution Order

本计划按“先基础闭环，再高级能力，再安全硬化”的顺序推进。不要把所有 route 都注册到 parity fallback 后就视为完成。

### Phase 0：路由扫平与错误语义

目标是让所有前端命令都有可审计状态。

- `pnpm check:web-routes` 必须持续通过。
- 每个命令必须被分类为 real handler、Web UI replacement、explicit unsupported 或 parity fallback。
- 当前 `parityFallback` 已经清零；后续新增命令不允许重新引入对 parity catch-all 的依赖。
- Web 错误必须统一返回可识别 code，不允许裸字符串错误穿透到 UI。

### Phase 1：P0 基础功能闭环

这些是远程 Web 版能否实际替代桌面版的最低门槛，应优先实现 real handler。

- 各 agent 导入当前配置：Claude、Codex、Gemini、OpenCode、OpenClaw、Hermes 的 live/default config import。
- Providers 基础 CRUD 与切换：添加、编辑、删除、启用、切换、读取当前配置、同步 live 状态。
- Universal Provider：列表、读取、保存、删除、同步到各 app。
- Skills：已安装列表、安装、卸载、启停、扫描、从 apps 导入、仓库管理、更新检查、更新、备份恢复。
- Prompts：列表、创建/编辑、删除、启停、读取当前文件、浏览器上传导入。
- Sessions：列表、消息读取、删除单个、批量删除；终端启动必须改为 Web 明确提示或返回可复制命令。
- MCP：统一 MCP 接口和旧 Claude MCP 兼容接口都要可用，至少覆盖 read/upsert/delete/enable/validate。
- Settings 基础数据维护：数据库备份、恢复、重命名、删除；配置导入导出。
- Web 文件流：SQL 导入、SQL 导出、Skill ZIP 安装、Prompt 文件导入全部改为浏览器 upload/download。

### Phase 1.5：route 层收口后的页面级收口批次

这一阶段不再是“清零 fallback”，因为 route 层已经完成清零。当前任务改为把已经存在的真实 route、显式 Web 语义和前端替代交互真正接到页面上，避免出现“接口在、页面半通”。

已完成批次：

- WebDAV：`webdav_sync_save_settings`、`webdav_test_connection`、`webdav_sync_upload`、`webdav_sync_download`、`webdav_sync_fetch_remote_info`。
- Usage quota：`get_balance`、`get_coding_plan_quota`、`testUsageScript`。
- Provider diagnostics：`queryProviderUsage`、`stream_check_provider`、`stream_check_all_providers`。
- Config snippets / stream / plugin / env：`get_common_config_snippet`、`set_common_config_snippet`、`extract_common_config_snippet`、`get_claude_common_config_snippet`、`set_claude_common_config_snippet`、`get_claude_config_status`、`read_claude_plugin_config`、`apply_claude_plugin_config`、`get_stream_check_config`、`save_stream_check_config`、`fetch_models_for_config`、`delete_env_vars`。
- Auth / accounts：`auth_get_status`、`auth_list_accounts`、`auth_logout`、`auth_poll_for_account`、`auth_remove_account`、`auth_set_default_account`、`auth_start_login`。
- Auth Center 页面收口：`tests/components/AuthCenterPanel.test.tsx` 已覆盖登录入口、device code 轮询态与多账号管理主分支；`tests/components/CopilotAuthSection.test.tsx` 已补 enterprise domain 必填/域名归一化、初始登录 pending 禁用、device code 复制/取消轮询、选中账号删除后清空外部选择；`tests/components/CodexOAuthSection.test.tsx` 已补初始登录 pending 禁用、FAST mode 开关、device code 复制/取消轮询、失败重试与默认账号/移除/注销操作；同轮 `tests/hooks/useManagedAuth.test.tsx` 又补上 hook 层结构化错误解包，确认 start-login 与 poll failure 不再把 plain object 错误降级成 `[object Object]`。`tests/integration/AuthCenterPanel.web-server.test.tsx` 现已通过本地假认证服务在真实 `web-server` 上验证 Copilot device flow 的 pending -> cancel -> success、Codex OAuth 登录、添加第二账号、默认账号切换、移除、失败重试与 logout all；对应页面也已补 remote Web 安全提示，避免把远程浏览器误导成“本机桌面登录”。
- System / tooling：`apply_claude_onboarding_skip`、`clear_claude_onboarding_skip`、`check_for_updates`、`get_tool_versions`、`test_api_endpoints`、`reset_circuit_breaker`、`get_circuit_breaker_stats`。
- Lightweight mode：`enter_lightweight_mode`、`exit_lightweight_mode`、`is_lightweight_mode` 已转成显式 Web 桌面专属语义。
- Settings SQL 导入：`src/lib/api/settings.ts` 与 `src/hooks/useImportExport.ts` 已改成单次浏览器文件选择 + 直接 multipart upload，不再在 Web 下二次触发文件选择。
- Skills ZIP 安装：`src/lib/api/skills.ts`、`src/hooks/useSkills.ts`、`src/components/skills/UnifiedSkillsPanel.tsx` 已改成单次浏览器文件选择 + 直接上传，不再保留双 picker 链路。
- Web 文件替代 API 证据：`tests/lib/web-file-replacements.test.ts` 已补 focused tests，直接覆盖 `settingsApi / promptsApi / skillsApi` 的 Web 分支，确认 SQL 导入/导出、Prompt 导入、Skills ZIP 安装在浏览器下走 `pickWebFile` 与 `webUpload/webDownload`，且不会回退到桌面 `invoke` 命令。
- Skills Discovery 页面收口：`tests/components/SkillsPage.test.tsx` 已补 focused test，覆盖 `SkillsPage` 顶部工具栏通过 imperative handle 触发 refresh / repo manager、仓库模式安装技能、skills.sh 搜索分页累积、旧查询结果在新查询加载期间不再闪回，以及 repo manager 的 add/remove success toast；`tests/integration/SkillsPage.web-server.test.tsx` 进一步在临时 `web-server` 上真实验证了 repo manager add -> discover -> install -> UnifiedSkillsPanel 已安装行出现 -> check updates -> update -> uninstall 后 SkillsPage 卡片回退到 install。`skillsDiscovery` 已不再只有 `UnifiedSkillsPanel` 一侧的证据。
- Repo manager 边角收口：`src/components/skills/RepoManagerPanel.tsx` 现已补上重复仓库覆盖提示、add form pending/失败反馈，以及 remove 行级 pending/失败反馈；`src/components/skills/SkillsPage.tsx` 的 repo add/remove 也不再吞掉错误，而会同时保留 toast 与向 panel 回抛失败。`tests/components/RepoManagerPanel.test.tsx` 与 `tests/components/SkillsPage.test.tsx` 已分别覆盖这些 focused 语义。
- Prompts 页面收口：`src/components/prompts/PromptPanel.tsx` 已补上显式导入入口，包含 toolbar 导入、空状态导入按钮和当前文件内容展示，`import_prompt_from_file` 不再只停留在 hook 层实现。2026-05-04 这一轮又补上 Web 取消文件选择时的静默 no-op：`src/lib/api/prompts.ts` 在浏览器下若未选中文件会返回 `null`，`src/hooks/usePromptActions.ts` 不再把这类用户取消操作误报成 `prompts.importFailed`；同时 `PromptPanel / PromptFormPanel / PromptFormModal` 已统一复用共享 prompt filename 映射，修复 `gemini` 当前文件提示误显示成 `AGENTS.md` 的 UI 错误。2026-05-05 又进一步把保存链路收口为“本地立即更新 + 后台 silent reload”，修复 Gemini 编辑保存后列表与当前文件内容回显落后的页面抖动；同日 `usePromptActions` 的失败态也补到了结构化 detail：保存/删除/启停/导入失败 toast 现在会携带 `extractErrorMessage`，当前 live prompt 文件缺失继续静默，但真实读取异常会单独提示 `prompts.currentFileLoadFailed`，不再被无声吞掉。
- Providers 页面收口：`src/App.tsx` 已在 Providers 顶部工具栏补上 `provider.importCurrent`，避免“导入当前配置”只在空状态下可见；`src/lib/providers/import-current-config.ts` 已提取共享导入逻辑，统一默认配置与 live import 分流。2026-05-04 这一轮又进一步把返回值语义从布尔值收紧为 `imported / no-change`，修复“非空列表点导入却提示 `provider.noProviders`”的误导反馈，当前改为专门的 `provider.importCurrentNoChanges` 提示。同日 `scripts/smoke-web-server.mjs` 也已补上六类 app 的真实导入探针，使用隔离 `CC_SWITCH_TEST_HOME` 预置 live 配置后，分别验证 Claude / Codex / Gemini / OpenCode / OpenClaw / Hermes 的导入链路可走通。
- Sessions 删除链路收口：`src/lib/api/adapter.ts` 已支持按命令声明将 `DELETE` 参数放入 JSON body；`delete_sessions`、`delete_env_vars` 已切到 body 语义，修复 Web 批量删除会话命中后端但参数丢失的问题。2026-05-05 又进一步把批删反馈链路收口为“成功项本地立即移除、失败项保留且继续选中、success/error toast 立即反馈”，不再等待 `invalidateQueries` 完成；`tests/components/SessionManagerPage.test.tsx` 已覆盖部分成功部分失败的恢复语义。
- 桌面专属 UI 降级第一批：`src/components/settings/DirectorySettings.tsx` 中目录浏览按钮在 Web 下禁用并提示手动输入服务端路径；`src/components/workspace/WorkspaceFilesPanel.tsx` 与 `src/components/workspace/DailyMemoryPanel.tsx` 中“打开目录”改为只展示路径文本；`src/components/providers/ProviderList.tsx` 在 Web 下不再暴露打开终端操作；`src/components/sessions/SessionManagerPage.tsx` 在 Web 下将会话恢复语义改为复制 resume 命令，而不是尝试启动本地终端。`tests/integration/DirectorySettings.web-server.test.tsx` 现已进一步在真实 `web-server` 上验证 `advanced / 配置目录` 的 browse disabled、手动输入 override、保存后 restart prompt，以及 reload 回显。
- About 页面 Web 语义收口：`src/components/settings/AboutSection.tsx` 已改为在 Web 模式下展示“服务端环境检查”而不是按浏览器 OS 隐藏工具卡片，避免“Windows 浏览器访问 Linux/macOS 服务器时看不到服务端工具状态”；同时已补 Web 模式的更新/安装提示，并修复版本为空字符串时只显示裸 `v` 的 UI 边界。`src/lib/updater.ts` 现会在 Web 模式下读取 `/api/health` 返回的版本信息，`src/lib/api/updater-adapter.ts` 则已改成走 `GET /api/system/get_update_info`，由 `src-tauri/src/services/web_update.rs` 在服务端获取最新 release 并与当前包版本比较，避免远程浏览器直接请求 GitHub 并把“能连上 latest release”误判成“必有更新”。`tests/components/AboutSection.test.tsx` 已补 focused test，覆盖 up-to-date toast、Web 下载页打开、release notes、WSL shell 刷新，以及“浏览器报告 Windows 时仍展示服务端工具/安装面板”的远程访问语义；同轮又补上 release notes / check-update 的 structured failure detail。`tests/integration/AboutSection.web-server.test.tsx` 现也已在真实 `web-server` 基座上验证当前 release notes 跳转与发现新版本后打开下载页的 rendered-page 闭环。
- Settings / Global Proxy / EnvWarningBanner 错误态收口：`src/hooks/useSettings.ts` 的 auto-save/manual save 失败、`src/hooks/useGlobalProxy.ts` 的 save/test/scan failure、`src/lib/api/globalProxy.ts` 的 structured invoke error unwrap、以及 `src/components/env/EnvWarningBanner.tsx` 的删除失败提示都已统一走 `extractErrorMessage()`，避免在 mounted 页面上继续把结构化错误显示成 `[object Object]`；对应 focused tests 为 `tests/hooks/useSettings.test.tsx`、`tests/hooks/useGlobalProxy.test.tsx`、`tests/lib/globalProxy.test.ts` 与 `tests/components/EnvWarningBanner.test.tsx`。
- Proxy query hooks 错误态收口：`src/lib/query/proxy.ts` 中 `useSwitchProxyProvider`、legacy `useProxyConfig`、`useUpdateGlobalProxyConfig` 与 `useUpdateAppProxyConfig` 的失败提示也都已统一走 `extractErrorMessage()`，避免 `settings/proxy` 页在 plain object error 下退化成空白或 `[object Object]`；对应 focused tests 为 `tests/hooks/useProxyQueryHooks.test.tsx`，并通过 `tests/components/ProxyTabContent.test.tsx` 复验容器未回归。
- UsageScriptModal 错误态收口：`src/components/UsageScriptModal.tsx` 的脚本测试与格式化失败现在也走 `extractErrorMessage()`，避免 Providers 页里的用量脚本弹窗把结构化错误降级成空白；对应 focused tests 为 `tests/components/UsageScriptModal.test.tsx`。
- SkillsPage install 错误态收口：`src/components/skills/SkillsPage.tsx` 的安装失败现在会先走 `extractErrorMessage()`，再交给 `formatSkillError()` 做文案映射；对应 focused test 为 `tests/components/SkillsPage.test.tsx`。
- 基础 server smoke 收口：`scripts/smoke-web-server.mjs` 已补成可复跑脚本，并在真实运行中先后暴露并修正了三个问题：临时 `CC_SWITCH_DATA_DIR` 未透传到数据库路径、`/sessions/launch-session-terminal` 重复 route panic、冷启动编译时间超过原 120s 等待窗口。当前脚本已改为默认 300000ms 启动超时，并能稳定跑完基础探针矩阵。随后同日又扩到真实 Sessions + MCP runtime smoke，并进一步修正了 `validate_mcp_command` 的 `POST` body 解析与 `McpConfigResponse` 的 `configPath` camelCase 序列化兼容问题。

下一批应优先做的是发布前复核和新增入口防回退，而不是继续补 route：

- 7 个 `webReplacements` 命令已复扫：`export_config_to_file`、`import_config_from_file`、`import_prompt_from_file`、`install_skills_from_zip`、`open_file_dialog`、`open_zip_file_dialog`、`save_file_dialog` 均保持显式 replacement，且当前调用点已收敛到已覆盖页面。
- 继续把文件流作为回归项保留；`config import/export`、`prompt import` 与 `skills zip install` 现都已有 rendered-page 证据，`tests/lib/web-file-replacements.test.ts`、`tests/hooks/useImportExport.test.tsx`、`tests/hooks/usePromptActions.test.tsx` 与 `tests/components/UnifiedSkillsPanel.test.tsx` 已确认取消态/错误态不会回退到桌面 file dialog。
- 继续防止 `get_circuit_breaker_stats` 的占位语义和 lightweight mode 的桌面专属语义在后续页面改动中回退；当前不再把这两项视为主开发 blocker。
- Skills Discovery / Prompts / Sessions / MCP 当前主链路和已发现边角缺口都已补到 focused 或 rendered-page 证据；后续重点是人工矩阵复核和新增入口防回退。Sessions 这边此前挂着的“搜索/筛选联动”里，`Hermes` provider filter 选项缺失已补齐并补回 focused test；批删参数丢失、部分失败恢复、即时反馈、MCP 删除取消态、Prompt live-file 读取异常，以及 skills.sh 清空输入后的取消/回退语义已不再是当前主 blocker。
- 继续把桌面专属入口的 UI 级隐藏或禁用作为回归项，避免新增按钮只在点击后才暴露结构化错误。
- `ProviderList` 后续只保留长尾维护项，以长尾 app 手工矩阵为主；provider health 基础状态语义现已补到 `ProviderCard` + `ProviderHealthBadge` focused tests。

### 当前已确认 blocker 与剩余收口项（2026-05-05）

- 当前没有新的 page/runtime 阻断型 regression。此前 `SkillsPage` 在 repo add/discover 后停留 `skills.sh` 的 source-selection 回归已经修复，并已通过 `tests/components/SkillsPage.test.tsx` 与 `tests/integration/SkillsPage.web-server.test.tsx` 复验。
- 当前剩余工作已经收敛为发布前人工矩阵与新增入口防回退，而不是主 happy-path 修复：`skills` 的 source 切换、旧结果闪回、清空输入后的取消态、skills.sh 空结果、skills.sh 网络异常/retry、空仓库态、trim 查询和同 query retry 已有真实页面证据；`prompts` 的极端 live-file IO 异常已补到真实页面证据；`sessions` 这边批删部分失败恢复、即时反馈，以及 `Hermes` provider filter 缺口都已收口；`mcp` 删除确认取消态已补到真实页面层。
- `proxy/about` 的主要 `WEB_NOT_SUPPORTED` / `WEB_DESKTOP_ONLY` 页面语义已经有真实页面或 focused 证据；`providers` 继续补长尾 app 人工矩阵，`usage` 则主要转成人工矩阵与少量更长尾空态/失败态补验。

### 当前页面证据与剩余复核项（按页面优先级）

这一节用于回答“除了 agent 导入、skills、提示词、会话管理之外，还缺哪些基础功能”。结论是：route 与主链路已经收口，剩余工作主要是发布前人工矩阵、长尾场景复核和新增入口防回退。

| 优先级 | 页面 / 面板 | 当前状态 | 剩余复核 / 维护项 | 收口判定 |
| ------ | ----------- | -------- | ---------------------- | -------- |
| P0 | `settings / proxy / ProxyTabContent` + lightweight mode 语义 | failover queue、auto switch toggle、app config save、runtime-stats placeholder 的 focused tests 与真实 `web-server` smoke 已接通；2026-05-05 又把 `src/lib/query/proxy.ts` 下的 provider switch、legacy/global/app proxy config save failure 统一收口成 `extractErrorMessage()` 路径；`tests/integration/ProxyTabContent.web-server.test.tsx` 也已把 config-only / runtime-unavailable 文案跑通 | 当前不再是基础功能 blocker；`get_circuit_breaker_stats` 的 Web 占位态已稳定，lightweight mode 在 Web 侧未发现实际可挂载入口，保留为桌面专属语义即可 | `tests/integration/ProxyTabContent.web-server.test.tsx`、`tests/hooks/useProxyQueryHooks.test.tsx` 与相关组件测试通过，人工矩阵转为长尾复核 |
| P1 | `providers / ProviderList` | 六类导入、Claude/OpenCode live 写回、Endpoint Speed Test、`导入当前配置 -> 编辑 -> 切换 -> live 写回`、`fetch_models_for_config`、Claude `common config snippet` 提取都已有 rendered-page 证据；真实页面侧也已补齐 `Live Config / DB Only`、provider limits、provider usage stats 与 diagnostics error badges，`ProviderCard` focused tests 继续兜底 health/limits/usage/error split，`ProviderHealthBadge` 也已补上 health 正向态与熔断态覆盖。2026-05-05 又把 Claude/Codex/Gemini 通用配置片段的 save/extract failure、`EndpointSpeedTest` 的测速失败提示、编辑态自定义端点 save-failure detail、`useStreamCheck` 的 thrown-error 提示，以及 `ProviderList` 自身“导入当前配置”失败提示统一收口成 `extractErrorMessage()` 路径，不再裸出原始异常或空白 detail | 基础功能已收口；此前的 React Query `act(...)` 与 `<button>` 嵌套 warning 已清理，剩余以长尾 app 手工矩阵为主，不再属于基础功能 blocker | `tests/integration/ProviderList.web-server.test.tsx`、`tests/hooks/useCommonConfigSave.test.tsx`、`tests/hooks/useStreamCheck.test.tsx`、`tests/components/EndpointSpeedTest.test.tsx`、`tests/integration/EndpointSpeedTest.web-server.test.tsx`、`tests/components/ProviderCard.test.tsx`、`tests/components/ProviderHealthBadge.test.tsx` 与 `tests/components/ProviderList.test.tsx` 已通过 |
| P1 | `settings / auth / AuthCenterPanel` | focused tests、真实 `web-server` smoke 和本地假认证服务都已覆盖 login/poll/default/logout/remove 主分支、远程安全提示、轮询取消和失败重试 | 主链路已收口；剩余主要是人工矩阵复核 | `tests/integration/AuthCenterPanel.web-server.test.tsx` 已通过 |
| P1 | `settings / advanced / WebDAV` | focused tests、runtime smoke 与真实 rendered-page smoke 都已接通 | 主链路 save/test/upload/download/remote info/error toast/数据库恢复 已收口；2026-05-05 又复验了 WebDAV failure toast 的 structured detail 路径。剩余工作主要是 Settings 容器级组合验收与人工矩阵补验 | `tests/components/WebdavSyncSection.test.tsx` 与 `tests/integration/WebdavSyncSection.web-server.test.tsx` 已通过 |
| P1 | `settings / advanced / Model Test + Log Config` | focused tests、runtime save/read 与真实 rendered-page smoke 都已接通；`LogConfigPanel` load 失败不再静默回退默认值，`ModelTestConfigPanel` load/save 失败也已补 detail | 主链路 load/save/reload 已收口；剩余工作主要是 Settings 容器级组合验收与人工矩阵补验 | `tests/integration/AdvancedConfigPanels.web-server.test.tsx` 已通过 |
| P1 | `settings / general` | 真实 rendered-page smoke 已覆盖 language `autosave -> backend 持久化 -> localStorage -> reload 回显`；`SkillStorageLocationSettings` focused tests 已覆盖确认迁移、直接迁移、部分失败与 detail toast；`ThemeSettings` / `AppVisibilitySettings` / `SkillSyncMethodSettings` / `WindowSettings` / `TerminalSettings` 也已补上 focused tests；`SettingsPage` 的 restart prompt 现在也已补上结构化失败 detail，不再只给开发态快捷提示 | 主链路最小持久化路径已收口；剩余主要是人工矩阵 | `tests/integration/SettingsGeneral.web-server.test.tsx` 已通过 |
| P1 | `settings / usage / UsageDashboard` | focused tests、runtime smoke 与真实 `web-server` 页面 smoke 已覆盖 session import、logs/detail、data sources、pricing、app/date/refresh 过滤器、多 tab 联动，以及 filter 更新后保持当前 tab；2026-05-05 又补齐了 `PricingConfigPanel` / `PricingEditModal` 的结构化 load/save failure 提示，并把 `DataSourceBar` 的 session sync 失败 detail 固定到 toast description | 基础功能已收口；剩余工作主要是人工矩阵与少量更长尾空态/失败态补验，不再属于基础功能 blocker | `tests/components/UsageDashboard.test.tsx`、`tests/components/PricingConfigPanel.test.tsx`、`tests/components/PricingEditModal.test.tsx`、`tests/components/DataSourceBar.test.tsx`、`tests/integration/UsageDashboard.web-server.test.tsx` 已通过 |
| P1 | `workspace` + `openclaw*` + `hermesMemory` | runtime smoke、focused tests 与真实 rendered-page 组合 smoke 已覆盖 Workspace file / Daily Memory 编辑保存、Daily Memory 搜索结果删除后的 list reload / active search refresh、OpenClaw Env/Tools 保存 reload、OpenClaw agents.defaults 迁移回显，以及 Hermes memory 的远程提示、保存与启停；`HermesMemoryPanel` focused test 也已补上 user tab 的独立保存/启停参数 | 主链路路径提示、保存反馈、reload 回显、Daily Memory search/delete、OpenClaw Env/Tools 页面写回和 Web 下 desktop-only / remote-only 语义已收口；后续更多是人工矩阵与极端态补验 | `tests/components/DailyMemoryPanel.test.tsx`、`tests/components/HermesMemoryPanel.test.tsx`、`tests/integration/WorkspaceOpenClawHermes.web-server.test.tsx` 已通过 |
| P1 | `universal / UniversalProviderPanel` | focused tests、App 壳层入口和真实 `web-server` 页面 smoke 都已接通 | 主链路 create/edit/delete/sync 已收口；剩余工作主要是极端路径、文案和人工矩阵补验 | `tests/integration/UniversalProviderPanel.web-server.test.tsx` 已通过 |
| P1 | `skills` + `skillsDiscovery` | `UnifiedSkillsPanel` install/import/restore 主链路和 repo manager 的重复覆盖提示、add/remove pending/失败反馈都已收口；`SkillsPage` 的 repo/skills.sh source 切换状态机、清空/缩短输入后的取消态、skills.sh 空结果、skills.sh 503 异常可见与 retry 恢复、trim 查询与同 query retry，以及初始空仓库 fallback / 手动 repo 空态都已修复并恢复 real-server 证据 | 后续继续补与已安装技能页的人工矩阵和更深组合；不再有 source-selection、clear-input cancel、skills.sh empty-result、skills.sh network retry、empty-repo 或 normalized-query blocker | `tests/integration/UnifiedSkillsPanel.web-server.test.tsx`、`tests/components/RepoManagerPanel.test.tsx`、`tests/components/SkillsPage.test.tsx`、`tests/integration/SkillsPage.web-server.test.tsx` 已通过 |
| P1 | `prompts` + `sessions` + `mcp` | 主链路基本都已有 rendered-page 证据；Prompt create/edit 已补到真实 `web-server` 的 Gemini 变体，保存链路现已支持本地立即更新 + silent reload，失败 toast 也已统一带出 `extractErrorMessage` detail，当前 prompt 文件缺失保持静默而真实读取异常会显式提示，且 live `CLAUDE.md` 读取异常现在也已补到真实 rendered-page 证据；Sessions 批删参数丢失、部分失败恢复、即时反馈，以及 `Hermes` provider filter 缺口也已收口；MCP add/edit 也已补到真实 `web-server` 的 live config 写回链路，`UnifiedMcpPanel` 的 import/toggle/delete 失败提示现也已切到 `extractErrorMessage/translateMcpBackendError` 风格，不再停留在 `common.error + String(error)`，删除确认取消态也已补到真实页面层，同轮 `McpFormModal` / `useMcpValidation` 的 JSON/TOML parse failure 也已统一接到 `extractErrorMessage()`，`import_mcp_from_apps` 现在也不再吞掉所有来源失败后误报 no-op：部分成功保持成功 count，全部失败时返回结构化 MCP detail | 主链路与已知 edge cases 已收口；剩余主要是人工矩阵复核和新增入口防回退 | 不要求重做主链路，优先保持矩阵与新增入口同步 |
| P2 | `agents` | 当前为稳定占位页 | 如果继续保留入口，只需维持稳定占位、无死按钮、文案明确；若准备进入开发，再单独立项，不应与本轮基础扫平混在一起 | 占位稳定或明确隐藏 |

当前最需要避免的误判有两类：

- `ProviderList`、`Prompts`、`Skills`、`Sessions`、`MCP`、`workspace/openclaw/hermes`、`AuthCenterPanel` 现在都更像“主链路已基本闭环、边角仍需补验”，不应该再与真正还缺显式 Web 页面语义的 `circuit breaker stats / lightweight mode` 放在同一 blocker 桶里；后者现已进入桌面专属/占位语义收口阶段，而不是基础功能缺口。
- `pnpm smoke:web-server` 全绿不等于所有页面都已完成，但 `ProviderList` 当前的核心 CRUD / writeback / `model fetch` / Claude `common config snippet` / diagnostics，以及 `SkillsPage` 当前的 repo discover/install/update/uninstall 主流程，都已经拥有 focused + rendered-page 双层证据，后续开发重心应从主链路修复转向显式 Web 语义和长尾边角态。
- `WEB_NOT_SUPPORTED` / `WEB_DESKTOP_ONLY` 已有 API 语义，也不等于页面收口。只要用户在页面上还会遇到“为什么这里不能用”而没有稳定文案、禁用态或占位态，这一页就还不能标记完成。

并行保留的产品风险：

- `get_circuit_breaker_stats` 当前是显式 `WEB_NOT_SUPPORTED`，但 `/settings/proxy` 的 runtime-unavailable 占位文案已经有真实页面证据；后续主要防止新增入口绕过这层占位。
- Lightweight mode 当前是显式 `WEB_DESKTOP_ONLY`；当前代码审计未发现挂载到 Web 页面的实际入口，后续主要防止未来新增入口时绕过桌面专属语义。
- `UsageDashboard` 当前的跨面板联动、tab 保持语义、session sync failure detail 都已经补齐，后续主要保留人工矩阵与少量更长尾空态/失败态补验，不再是当前 blocker。
- Auth 远程安全仍未完成；如果未来考虑局域网或公网暴露，不能长期保持 permissive middleware。

### 下一轮全页面扫平执行板

为避免继续按“模块感觉”推进，而漏掉实际已经挂载到 Web 的页面，这里按 `src/App.tsx` 当前可见视图重新盘点一次。当前主壳层共 14 个一级 view：

- `providers`
- `settings`
- `prompts`
- `skills`
- `skillsDiscovery`
- `mcp`
- `agents`
- `universal`
- `sessions`
- `workspace`
- `openclawEnv`
- `openclawTools`
- `openclawAgents`
- `hermesMemory`

再加上 `SettingsPage` 的 6 个显式子页：

- `general`
- `proxy`
- `auth`
- `advanced`
- `usage`
- `about`

因此当前“全页面扫平”最少要按 20 个可见页面桶管理，而不是只盯 `providers/prompts/skills/sessions` 这几页。按 2026-05-05 当前证据，下一轮建议直接按下面顺序推进：

| 顺序 | 页面桶 | 当前结论 | 下一步动作 |
| ---- | ------ | -------- | ---------- |
| 1 | `skillsDiscovery` | 主 happy-path、skills.sh 空结果、skills.sh 网络异常/retry、空仓库态、trim 查询与同 query retry 都已闭环，当前是最明确的长尾缺口集中地 | 继续补人工矩阵与更深组合，优先做真实 rendered-page 证据 |
| 2 | `prompts` + `sessions` + `mcp` | route/runtime 与主页面主链路都已通；MCP 删除取消态已补到 rendered-page 证据 | 不重做 happy-path，转补失败态、筛选组合和人工矩阵；重点是页面壳层文案与恢复行为 |
| 3 | `settings/advanced` + `settings/usage` + `settings/proxy` | 功能面最碎，虽然大多已有 smoke，但仍最容易漏页面级组合问题 | 继续按“真实页面入口 -> 操作 -> toast/alert/detail -> reload 回显”补组合证据，并把 desktop-only/runtime-unavailable 语义固定下来 |
| 4 | `providers` + `universal` | 主链路已闭环，剩余主要是 app 维度长尾组合 | 以人工矩阵为主，补少量 live import/writeback/diagnostics 边角，不再回到大面积实现阶段 |
| 5 | `workspace` + `openclaw*` + `hermesMemory` | 主链路已有 focused/runtime/rendered-page 三层证据 | 转做人机视角矩阵，确认远程语义、路径提示、search/delete/save failure 等边角保持稳定 |
| 6 | `settings/auth` + `settings/about` | 页面级证据已基本收口 | 保留远程访问安全语义、下载页跳转、超时/失败重试等长尾回归 |
| 7 | `agents` | 不是功能页，而是可见占位页 | 维持稳定占位或隐藏入口；除非单独立项，否则不混入本轮功能开发 |

这张执行板对应的判断标准也要同步收紧：

- 只要页面已经挂载在 `App.tsx` 或 `SettingsPage` 的显式 tab 里，就必须在文档里有归类，不允许落成“以后再说”的隐式待办。
- 已经拥有 focused test、runtime smoke 和 rendered-page smoke 的页面，不再回退成“重新实现主流程”；下一步只补真实缺失的 edge cases、人工矩阵和显式 Web 语义。
- 仍缺 rendered-page 证据的剩余事项，要优先补“用户肉眼能看到的失败/恢复/禁用语义”，而不是继续堆底层 route。
- `agents` 这类占位页也算页面交付范围；占位稳定本身就是一种交付状态，但必须在文档里明确写出，不得默认为未完成功能页。

### 下一轮收口出口条件

文档层面，只有同时满足下面几条，才能把“全页面扫平同步计划”标成接近完成：

- `App.tsx` 当前 14 个一级 view 与 `SettingsPage` 6 个 tab 都已经在本计划里有明确归类：功能闭环、显式 Web 降级、或稳定占位。
- `skillsDiscovery` 的剩余 blocker 已从“真实页面网络异常/retry 缺口”进一步降到“人工矩阵/长尾补验”，不再保留网络异常或 retry 的空白证据。
- `prompts`、`sessions`、`mcp`、`settings/advanced`、`settings/usage` 不再有“页面可见但缺少失败态/取消态/组合态说明”的空白项。
- 所有 desktop-only / web-unavailable 能力都已经在页面层有稳定文案、禁用态或占位态，而不是只能依赖 API 返回错误码。
- 后续待办主要剩人工矩阵、外网 happy-path mock 取舍和安全加固，而不再剩“基础页面功能没接上”的问题。

### Phase 2：P1 运维与新版扩展能力

这些能力不一定阻塞“能用”，但如果页面已经可见，就应同步补齐，否则用户会频繁遇到空状态或 unsupported。

- Usage：dashboard、request logs、request detail、model pricing、session usage sync、data sources。
- Proxy：proxy config、status、start/stop、takeover、upstream proxy、proxy URL test。
- Failover：队列、auto failover、circuit breaker config/stats/reset。
- WebDAV：保存设置、连接测试、上传、下载、remote info。
- Subscription / Balance / Coding Plan：订阅状态、余额、套餐额度。
- OMO / OMO Slim：当前状态、读取本地配置、停用。
- OpenClaw：workspace files、daily memory、env/tools/agents defaults。
- Hermes：memory get/set 与 Web UI 行为。
- DeepLink：Web 粘贴解析导入，替代 OS 协议注册。

### Phase 3：P2 桌面专属降级与安全硬化

这些能力不要求与桌面完全一致，但必须给用户明确语义。

- 桌面 OS 集成：托盘、窗口、剪贴板、开文件夹、开终端、插件自动修改、deep link 协议注册。
- 远程暴露前安全：auth、CSRF、session、rate limit、CORS allowlist、访问审计。
- 默认行为保持保守：server 默认监听 `127.0.0.1`，不自动暴露到局域网。

## Key Changes

### 1. 覆盖门禁

建立命令到 Web route 的覆盖检查，要求所有前端可见命令必须满足以下之一：

- 在 Axum handler 中注册 route。
- 在 `web-commands.ts` 中显式 unsupported。
- Web UI 已隐藏、禁用或替换为 Web 专属交互。

该检查应覆盖：

- `src/lib/api/web-commands.ts`
- `src-tauri/src/web_api/routes.rs`
- `src-tauri/src/web_api/handlers/*.rs`

门禁输出必须同时被用于两类审计：

- Route audit：确认没有 missing route。
- Function audit：确认 P0/P1 页面没有被 parity catch-all 长期兜底。
- Regression audit：每次新增 `web-commands.ts` 命令时，必须同步新增 handler、unsupported 标记或 UI replacement 说明。

建议把 `pnpm check:web-routes -- --list-parity` 的输出作为 PR / 提交说明的一部分。只要 `parityFallback` 中出现 P0/P1 页面命令，就不能标记为完成。

### 2. Providers 与主页面能力

补齐供应商相关 Web 功能：

- Claude/Codex/Gemini 的 `import_default_config`。
- OpenCode/OpenClaw/Hermes live import 的回归验证。
- `read_live_provider_settings` 与 `sync_current_providers_live`。
- Universal Provider 的 get/getAll/upsert/delete/sync。
- custom endpoints 的 get/add/remove/update last used。
- provider usage query、provider health、provider stats、provider limits。
- model fetch 与 stream check。
- OMO / OMO Slim 当前状态、读取本地文件、停用。

桌面专属的打开终端、托盘刷新在 Web 模式中改为明确 no-op 或 `WEB_DESKTOP_ONLY`。

验收重点：

- “添加供应商”已经能成功不能代表 Providers 页面完成，还必须验证从本机已有配置导入、切换后写回对应 CLI 配置、live 状态刷新、usage/health/limits 可读。
- “导入当前配置”不能只在空状态可见；非空 provider 列表场景也必须保留入口，这一项当前已补到页面工具栏，并且运行时 smoke 已覆盖 Claude/Codex/Gemini/OpenCode/OpenClaw/Hermes 六类导入链路。后续剩余的是更高一层的页面级收口：继续把 provider 切换写回、读取 live 状态、usage/health/limits 和错误提示分流扩成更完整的页面 smoke；当前至少已经用 Claude / OpenCode 三条代表性 probe 覆盖了写回与切换 live 配置这一步，并已在真实页面上覆盖 `fetch_models_for_config`、Claude `common config snippet` 提取、Claude/Codex/Gemini 通用配置片段的 save/extract failure structured detail，以及 `EndpointSpeedTest` 的测速失败 structured detail。
- 每个 app 的配置路径差异必须由后端 service 处理，前端不硬编码本机路径。
- 导入当前配置失败时要区分“文件不存在”“格式无法解析”“Web 不支持”和“服务端读写失败”。

### 3. Prompts、Sessions、Skills、MCP

补齐扩展入口：

- Prompts：get/upsert/delete/enable/import/read-current-file。
- Sessions：list/get messages/delete/delete many；launch terminal 在 Web 中明确不支持或返回可复制命令。
- Skills：installed/backups/install/uninstall/restore/toggle/scan/import/discover/check updates/update/migrate/search/repo add/remove/zip install。
- MCP：统一接口、legacy Claude MCP 兼容接口、read config、validate command、set enabled、config route upsert/delete 已经全部接入 runtime smoke；剩余需要继续收口的是页面级 import/add/edit/toggle/delete 与错误提示。

验收重点：

- Skills 不能只支持新增仓库；已安装技能的启停、卸载、恢复、更新检查、ZIP 导入都必须可完成。
- Prompt 文件导入入口已经补到页面层；后续仍需确认所有触发路径都只走浏览器 upload，不再依赖桌面 file dialog 命令链。
- Sessions 的消息读取是基础功能；launch terminal 是桌面专属能力，Web 只能禁用或提供可复制命令。
- Sessions 的批量删除不能只依赖 query string 兼容；当前 Web adapter 已补齐 `DELETE` body 语义，`pnpm smoke:web-server` 也已覆盖单删、批删和消息读取三条链路；后续只剩页面级搜索、选择集、失败恢复和壳层工具栏验收。
- MCP 新旧接口要同时保留，因为前端可能存在历史 API 调用路径；当前 runtime smoke 已覆盖 validate/read/upsert/delete/enable/config route，后续应把重点转到页面级表单、toggle、删除确认和错误态验收。

### 4. Settings 与运维页面

补齐设置页可见能力：

- database backup create/list/restore/rename/delete。
- config import/export。
- WebDAV test/upload/download/fetch remote info/save settings。
- proxy config、proxy status、proxy takeover、upstream proxy status、proxy URL test。
- failover queue、auto failover、circuit breaker config/stats/reset。
- usage dashboard、request logs、request detail、model pricing、session usage sync、data sources。
- subscription/balance/coding plan quota。

验收重点：

- 设置页每个 tab 打开不能出现空 handler、404 或未捕获异常。
- 数据备份恢复必须使用临时目录做 smoke test，避免破坏真实 `~/.cc-switch`。
- WebDAV、proxy、failover、usage 属于“页面可见基础运维能力”，不应只靠 fallback 返回 unsupported。

### 5. Web 文件交互替代方案

新增 Web 专属文件交互：

- SQL 导入：浏览器 multipart upload。
- SQL 导出：浏览器 download。
- Skill ZIP 安装：浏览器 multipart upload。
- Prompt 文件导入：浏览器 upload。
- 配置目录选择：改为服务端路径输入、展开 `~`、存在性校验。

保留普通 JSON invoke 风格用于非文件命令；只有上传/下载走 direct fetch helper。

需要避免的交互问题：

- Web ZIP 安装不能先触发一次 `openZipFileDialog()` 再在 `installFromZip()` 里再次触发 `pickWebFile()`。
- API adapter 应提供一次性 upload helper，UI 层只触发一次浏览器文件选择。
- 后端不应假设服务端能打开用户浏览器所在机器的任意路径。

### 6. OpenClaw、Hermes、Workspace、DeepLink

补齐新版本入口：

- OpenClaw workspace files 与 daily memory 的 read/write/list/search/delete。
- OpenClaw env/tools/agents defaults 的 Web 回归测试。
- Hermes memory 的 get/set 和 Web UI 行为明确化。
- DeepLink 改为 Web 粘贴解析导入，不依赖 `ccswitch://` 协议注册。

验收重点：

- Workspace / OpenClaw 文件能力必须有路径边界，避免 Web 页面任意读写服务端文件系统。
- DeepLink Web 版应接受粘贴文本或 URL 字符串，解析后进入同一套 merge/import 流程。
- Hermes 如果只是跳转本地 `127.0.0.1` 服务，需要明确这是服务端本机地址，不一定等于远程浏览器所在机器。

### 7. 桌面专属能力

以下能力不作为 Web 基础同步目标，必须统一处理为禁用或明确提示：

- 系统托盘。
- 窗口控制。
- 剪贴板写入。
- 自动启动。
- 打开系统文件夹。
- 打开本地终端。
- VSCode/Claude 插件自动修改。
- OS deep link 协议注册。

建议错误码：

- `WEB_DESKTOP_ONLY`
- `WEB_UPLOAD_REQUIRED`
- `WEB_NOT_CONFIGURED`
- `WEB_NOT_SUPPORTED`

## Page-Level Checklist

执行规则：

- 每个页面 checklist 都要记录 handler 类型：real、upload/download replacement、explicit unsupported、desktop-only、parity fallback。
- P0/P1 页面不允许用 parity fallback 作为完成状态。
- 桌面专属能力必须在 UI 上禁用或展示明确说明，不能让用户点击后才看到裸错误。

### Providers / Agent 页面

- Claude：添加、编辑、删除、切换、导入当前配置、读取 live 设置、写回配置。
- Codex：添加、编辑、删除、切换、导入当前配置、读取 live 设置、写回配置。
- Gemini：添加、编辑、删除、切换、导入当前配置、读取 live 设置、写回配置。
- OpenCode：添加、编辑、删除、切换、live import 回归。
- OpenClaw：添加、编辑、删除、切换、live import、workspace/defaults 回归。
- Hermes：添加、编辑、删除、切换、live import、memory 行为。
- Universal Provider：跨 app 共享配置 CRUD 与同步。
- Custom Endpoints：读取、添加、删除、last used 更新。
- Provider usage script：查询、测试、错误展示、禁用无脚本 provider 的入口。
- Provider diagnostics：单 provider stream check、全量 stream check、模型拉取、health/stats/limits。
- Common config snippets：读取、提取、保存通用片段，Claude 专属片段路径保持兼容。
- Stream check config：读取、保存、实际检测结果回写 UI。

### Skills 页面

- 已安装列表与状态读取。
- 安装、卸载、启用、禁用。
- 从 app 扫描并导入。
- skill repos 的 add/remove/list。
- discover/search。
- check updates/update/migrate。
- skill backups 的 list/delete/restore。
- ZIP 安装走浏览器 upload。

### Prompts 页面

- Prompt 列表、详情、创建、编辑、删除。
- 启用/禁用与当前配置同步。
- 当前 prompt 文件读取。
- 从文件导入走浏览器 upload。
- 失败 toast 需要带后端 detail；缺失 current file 保持静默，真实读取异常要显式提示。

### Sessions 页面

- 会话列表。
- 会话消息读取。
- 删除单个会话。
- 批量删除会话。
- 终端恢复入口 Web 降级为明确提示或可复制命令。

### MCP 页面

- 统一 MCP server list/read/upsert/delete。
- legacy Claude MCP status/config/read/upsert/delete。
- validate MCP command。
- set enabled。
- config MCP route 的 upsert/delete。

### Settings 页面

- 数据库备份 create/list/restore/rename/delete。
- 配置导入导出 upload/download。
- 配置目录设置改为服务端路径输入和校验。
- WebDAV settings/test/upload/download/remote info。
- Proxy settings/status/start/stop/takeover/upstream/test URL。
- Failover queue/auto failover/circuit breaker。
- Usage dashboard/logs/detail/pricing/session sync/data sources。
- Subscription/balance/coding plan quota。
- Tool versions / update check / endpoint test。
- Claude onboarding skip 状态应用与清除。
- Claude plugin config 读取与 Web 降级处理。
- Lightweight mode 状态读取与切换，或明确隐藏入口。
- Env var 删除能力的服务端白名单与风险提示。

### Auth / Account 页面

- 当前登录状态。
- 账号列表。
- start login / poll login 流程。
- 设置默认账号。
- logout / remove account。
- 如果 Web 暂不实现账号体系，页面入口必须隐藏或统一显示 `WEB_NOT_SUPPORTED`，不能保留 parity fallback。

### Workspace / OpenClaw / Hermes / DeepLink

- Workspace daily memory read/write/list/search/delete。
- Workspace file read/write/list/search/delete。
- OpenClaw env/tools/agents defaults。
- Hermes memory get/set。
- DeepLink paste parse/merge/import。

### System / Desktop-only 页面入口

- 文件选择、ZIP 选择、保存文件：改为浏览器 upload/download。
- 打开配置目录、打开 workspace 目录：Web 中禁用，必要时显示服务端路径文本。
- 打开 provider/session 终端：Web 中禁用或返回可复制命令。
- 插件自动修改、系统更新安装、托盘、窗口、剪贴板、自动启动：统一 desktop-only 语义。

## Public API / Interface Changes

新增 Web 文件接口：

- `POST /api/config/import-config-upload`
- `GET /api/config/export-config-download`
- `POST /api/skills/install-skills-upload`
- `POST /api/prompts/import-prompt-upload`

普通命令继续沿用现有 `/api/<resource>/<action>` JSON API。

前端 API 层新增 direct fetch helper，用于：

- multipart upload。
- file download。
- 将后端 Web 错误码转换为用户可读提示。

`web-commands.ts` 不手工维护；需要通过 manifest/generator 或覆盖表保证 route 与 unsupported 状态一致。

## Current Page-Level Remaining Work

在 2026-05-05 这一轮完成审计后，本计划的可见页面主链路已经达到当前完成判定。当前剩余工作集中在发布前人工复核、远程安全补强和新增入口防回退，而不是缺少后端路由或页面级主链路：

- `webReplacements` 7 个命令已经逐个记录到矩阵：4 条用户可见主文件流 `config import/export`、`prompt import`、`skills zip install` 具备 rendered-page 证据，3 条底层文件对话框桥接命令 `open_file_dialog`、`open_zip_file_dialog`、`save_file_dialog` 已确认只通过浏览器 pick/download 替代链路进入这些页面。当前剩余更偏向人工回归与新增入口防回退，而不是基础文件流开发。
- 配置导入导出这一侧已经补上真实页面级证据：`tests/integration/ImportExportSection.web-server.test.tsx` 现已在临时 `web-server` 上真实点击渲染后的 `settings.exportConfig` 按钮，并进一步验证 `settings.selectConfigFile -> settings.import` 的页面闭环，覆盖浏览器 download、动态文件名、SQL blob 内容、A/B provider 快照回滚，以及 live `~/.claude/settings.json` 跟随导入恢复。Settings SQL 文件流不再是当前主 blocker。
- SQL 导入与 Skills ZIP 安装的双文件选择问题已经修正。`import_config_from_file` 现在不再只依赖 focused component/hook 测试与 `pnpm smoke:web-server` 的 API/runtime probe；`tests/integration/ImportExportSection.web-server.test.tsx` 已复用 `tests/integration/PromptPanel.web-server.test.tsx` 的 test-only multipart shim，把 rendered-page 上传稳定落到真实 Rust handler，而不需要改生产上传逻辑。
- Prompt 页面这侧的真实 rendered-page `web-server` smoke 已经补齐：`tests/integration/PromptPanel.web-server.test.tsx` 现已在临时 `web-server` 上真实验证 `prompts.import -> 列表刷新 -> enable -> 当前 live 文件写入 -> disable -> live 文件清空 -> delete`，以及 live `CLAUDE.md` 读取异常时显示 `prompts.currentFileLoadFailed` 并带出结构化 detail。因此 Prompt 不再是 Batch C 的主 blocker，后续更多是人工矩阵和少量错误态文案的补充验收。
- Auth Center 这一页已经从“只有 focused tests”推进到真实 rendered-page 验收：除组件级 device code 登录入口、enterprise domain 校验、polling copy/cancel、FAST mode 开关、默认账号切换、logout/remove 和失败重试外，`tests/integration/AuthCenterPanel.web-server.test.tsx` 也已在真实 `web-server` + 本地假认证服务下跑通 device flow pending/cancel/success、多账号、默认账号写回、移除与 logout all。当前剩余工作已从主 happy-path 收敛到远程访问安全语义、超时/取消边界与人工矩阵补验。
- Settings 这组页面当前已经有一批真实 rendered-page 基座：`general` 已打通 language `autosave -> backend 持久化 -> localStorage -> reload`，并补上 `SkillStorageLocationSettings` 的 focused 回归，覆盖“有已安装 skills 时先确认、无已安装 skills 时直接迁移、部分迁移 warning、失败 detail toast”；`ThemeSettings` / `AppVisibilitySettings` / `SkillSyncMethodSettings` / `WindowSettings` / `TerminalSettings` 也已补成 focused tests；`advanced` 已打通 `ModelTestConfigPanel` / `LogConfigPanel` 的 load/save/reload 与 `WebDAV` 的 `save -> test -> upload -> remote info -> download restore`，其中 `LogConfigPanel` 现已补上“load 失败时显示 destructive alert 而不是静默回退到默认配置”，`ModelTestConfigPanel` 的 load/save 失败也已统一带出 extracted detail；`about` 已打通更新检查、工具版本卡片与服务端环境检测失败 alert。当前剩余重点已从 circuit breaker stats、轻量模式这类页面语义缺口，进一步收敛到 Settings 容器级人工矩阵、restart-required 组合验收与长尾空态/失败态补验；`usage` 当前的筛选器联动、多 tab 回显和 rendered-page 基座也已收口，剩余主要转成人工矩阵与长尾空态/失败态补验。
- Proxy / Failover 这页当前已经明确成“配置可远程编辑、运行时保持本机专属”的 Web 语义：`src-tauri/src/services/proxy_web.rs` 仍把 `start()`、`start_with_takeover()`、`set_takeover_for_app()` 和 `switch_proxy_target()` 视为 web-server unavailable，因此当前目标不是强行在远程浏览器里启动本机代理，而是把 config-only / desktop-only 降级、禁用态和页面级 smoke 一起补齐。`tests/integration/ProxyTabContent.web-server.test.tsx` 现已在真实 `web-server` 渲染基座上跑通 failover provider `Select`、queue add/remove、auto switch toggle、app config save 以及 runtime-unavailable / config-only 文案验收，这一页不再是 Batch E 的当前 blocker。
- Usage 这页当前也已经从“首次远程打开看不到导入入口”收口到真实 rendered-page 基座：`src/components/usage/DataSourceBar.tsx` 不再在 data sources 为空时直接 `return null`，因此首次远程访问仍会显示 empty-state 文案与 `Import Sessions`；`tests/integration/UsageDashboard.web-server.test.tsx` 现已进一步在真实临时 `web-server` 上验证 empty state -> session import -> data source refresh -> request detail dialog，以及 app filter / date range / refresh interval 与 logs/providers/models 多面板联动；`tests/components/UsageDashboard.test.tsx` 也已补上 filter 更新后保持当前 tab 的 focused 回归。这一页不再是 Batch E 的当前 blocker，剩余工作主要收敛到人工矩阵与少量长尾空态/失败态补验。
- 桌面专属按钮的前端降级已完成一轮收口：`open_app_config_folder`、`open_config_folder`、`open_provider_terminal`、`open_workspace_directory`、`pick_directory` 现已在 Web adapter 中标成显式 `unsupported`，并由 Directory / Provider / Workspace focused tests 证明 Web 页面不会直接暴露这些入口；剩余仍需继续核对其它能从 Web 页面触发的 OS 集成动作是否也满足同一规则。
- `prompts`、`sessions` 与 `mcp` 当前都已经有真实 rendered-page `web-server` smoke，`skills` 与 `skillsDiscovery` 这两页也已有真实页面 smoke 证明 `openInstallFromZip`、import/toggle/uninstall/restore、repo manager add/discover/install、cross-panel linkage、check updates/update、skills.sh 分页安装、空结果、skills.sh 异常态/retry、trim 查询、同 query retry，以及初始空仓库 fallback / 手动 repo 空态都可走通；Prompt live-file 读取异常也已补到真实页面 toast detail。`workspace/openclaw/hermes` 这一组现在也已由 `tests/integration/WorkspaceOpenClawHermes.web-server.test.tsx` 补上真实页面证据，覆盖 Workspace file / Daily Memory 的路径提示与编辑保存回显、OpenClaw Env/Tools 保存写回与 reload、OpenClaw agents.defaults 的 legacy timeout 迁移与 reload、以及 Hermes memory 的远程提示、保存与启停；`tests/components/HermesMemoryPanel.test.tsx` 也已补上 user tab 的独立保存/启停参数回归。当前剩余焦点已进一步收敛到 Skills 人工矩阵和更深组合，以及 Prompt 与 MCP 的人工矩阵补验；`ProviderList` 则降级为长尾维护项，不再列为主 blocker。`circuit breaker stats` 与 lightweight mode 不再列为当前 Web 页面开发 blocker。
- `RepoManagerPanel` 当前已不再只有 URL 解析校验和添加错误文案：重复仓库覆盖提示、add form pending/失败反馈，以及 remove 行级 pending/失败反馈都已补上，并由 focused tests 覆盖。`skillsDiscovery` 后续应转向搜索/筛选极端场景，而不是继续把 repo manager 基础交互列为 blocker。
- 已补一组针对已修收口点的前端测试，并已继续扩到页面级组合场景。当前已覆盖 `useImportExport`、Skills ZIP 安装链路、Skills 从 apps 导入默认映射、PromptPanel 导入入口、Session 消息读取 query / Web resume 语义 / 批量删除 adapter 语义、UnifiedMcpPanel 的 import/toggle/delete 主链路、ProxyTabContent 的首次启用确认 / 故障转移确认 / 桌面模式停止态禁用提示 / Web runtime-control unavailable + failover config-only + runtime-stats 占位文案、FailoverQueueManager 的队列 health/reset 交互、AutoFailoverConfigPanel 的加载/校验/保存/disabled 态、Provider Web 终端入口隐藏、Provider 当前配置导入链路、DirectorySettings Web 禁用状态、Workspace/DailyMemory 服务端路径提示与禁用打开目录语义、DeepLinkImportDialog 的 Web 手动粘贴解析/导入链路与 parse/merge/import 失败 detail 提示、HermesMemoryPanel 的打开配置/保存/启停主链路与 Web 远程提示/禁用语义、AgentsPanel 的稳定多语言占位页、OpenClaw `Env/Tools/Agents` 三页的保存主链路与关键边界（unsupported profile、legacy timeout 迁移、清空旧 model 配置），以及 Usage 页的 `RequestLogTable` 详情入口、`RequestDetailPanel` 内容/关闭/未找到反馈、`DataSourceBar` 来源展示与 session sync 反馈；`tests/integration/App.test.tsx` 现也已补上 `deeplink.pasteImport` 顶部按钮到 `DeepLinkImportDialog` imperative handle 的壳层接线回归，`tests/integration/DeepLinkImportDialog.web-server.test.tsx` 则在临时 `web-server` 上真实验证了 paste -> parse -> import -> providers 列表落库的页面级闭环，`tests/integration/BackupListSection.web-server.test.tsx` 则在同一基座上真实验证了 backup manager 的 create -> rename -> restore -> delete 页面闭环，`tests/integration/ImportExportSection.web-server.test.tsx` 则把 config import/export 的浏览器 upload/download 路径真实跑到了渲染页面层，`tests/integration/PromptPanel.web-server.test.tsx` 则把 Prompt 面板的 import -> enable -> live file write -> disable -> live file clear -> delete 跑到了真实临时 `web-server` 与隔离 `~/.claude/CLAUDE.md` 副作用层，`tests/integration/UnifiedMcpPanel.web-server.test.tsx` 则进一步把 MCP 面板的 import -> app toggle -> delete 跑到了真实临时 `web-server` 与 live MCP 配置文件写回，`tests/integration/SessionManagerPage.web-server.test.tsx` 则把 sessions 页的列表渲染 -> 详情切换 -> Web resume 复制 -> 批量删除 -> JSONL 文件清理跑到了真实临时 `web-server`，`tests/integration/UnifiedSkillsPanel.web-server.test.tsx` 则把 Skills 主面板的 `openInstallFromZip -> multipart upload -> installed skill 可见 -> SSOT/live skill file write` 与 `openImport -> toggle -> uninstall -> restore` 都跑到了真实临时 `web-server` 与隔离 live/SSOT 文件副作用层，`tests/integration/WorkspaceOpenClawHermes.web-server.test.tsx` 则把 Workspace / Daily Memory / OpenClaw Env / OpenClaw Tools / OpenClaw AgentsDefaults / Hermes Memory 收口到了同一套真实 `web-server` 基座上。`tests/integration/ProxyTabContent.web-server.test.tsx` 现也已在同一套真实 `web-server` 基座上跑通 failover provider `Select`、queue add/remove、auto switch toggle、app config save 与 Web runtime-degradation 文案，因此这一页现已从 blocked 进入 pass。
- App 壳层测试基座当前已稳定；后续若继续补顶部工具栏或容器级集成测试，应沿用现有 desktop/Web mode mock，不要回退到直接把 `window.__TAURI_INTERNALS__` 伪造成空对象的旧写法。
- 基础 API 级 `pnpm smoke:web-server` 与真实页面级 rendered smoke 都已经落地并跑通；文档中的临时数据目录方案后续继续作为发布前手工复核和新增入口防回退的执行方式。

## Immediate Delivery Queue

这一段是“从现在开始应该怎么排开发”的直接执行顺序，不再按抽象模块描述，而按最容易影响远程 Web 可用性的页面收口顺序推进。

### Batch A：Settings / About / Web Replacement 收口

这是早期最适合先做的一批，因为它们已经有 route 或已有半成品语义，容易在页面上残留桌面逻辑。当前这一批已经收口，后续只保留人工回归和新增入口防回退。

- `About` 页更新检查这一轮已经从“浏览器直连 GitHub”收口到“服务端返回真实更新元数据”：Web 下 `src/lib/api/updater-adapter.ts` 现改读 `GET /api/system/get_update_info`，`src-tauri/src/services/web_update.rs` 负责抓取 latest release 并与当前包版本比较，`POST /api/system/check_for_updates` 也同步返回真实 `available`。因此 Web 下如果发现新版本，当前行为已经稳定为打开下载页或 release 页面，而不是调用 relaunch 语义。
- `About` 页当前已不再缺更新/工具版本主链路的 rendered-page 证据：`tests/integration/AboutSection.web-server.test.tsx` 现已在真实 `web-server` 上验证版本徽标、服务端环境/安装提示、release notes、server-backed update download，以及 `get_tool_versions` 返回的四张工具卡片、本地版本/最新版本徽标和页面 refresh 行为。因此 Batch A 当前剩余工作已转向人工回归记录，而不是 `About` 主流程缺口。
- `tests/integration/ImportExportSection.web-server.test.tsx` 现在已经同时把 `settings.exportConfig` 与 `settings.selectConfigFile -> settings.import` 跑到了渲染页面层，确认浏览器 download、rendered-page upload、provider snapshot 回滚、backupId 展示与 live `~/.claude/settings.json` 恢复都能成立；`tests/integration/UnifiedSkillsPanel.web-server.test.tsx` 也已把 Skills ZIP install 的 rendered-page upload 跑到真实 Rust handler。7 条 `webReplacements` 现已在矩阵中逐条绑定到页面或浏览器替代证据，后续保留为人工回归项。
- 当前 7 个 `webReplacements` 命令已经逐个复核并记录收口状态；现阶段已确认 `export_config_to_file`、`import_config_from_file`、`import_prompt_from_file` 与 `install_skills_from_zip` 都有 rendered-page 证据，`open_file_dialog`、`open_zip_file_dialog`、`save_file_dialog` 这三类桥接命令的实际触发路径也已收敛到 Settings 导入导出、Prompt 导入与 Skills ZIP 这些已覆盖壳层：
- 浏览器替代：`export_config_to_file`、`import_config_from_file`、`import_prompt_from_file`、`install_skills_from_zip`、`open_file_dialog`、`open_zip_file_dialog`、`save_file_dialog`
- 5 条桌面专属命令 `open_app_config_folder`、`open_config_folder`、`open_provider_terminal`、`open_workspace_directory`、`pick_directory` 已在 Web adapter 中改成显式 `unsupported`，不再继续算作 parity 收口项。
- 验收要求是“从页面上找不到会误触发桌面行为的入口”，以及“剩余文件流不会回退到桌面 dialog 命令链”，而不是单纯让 parity route 返回 `WEB_UPLOAD_REQUIRED` 或 `WEB_DESKTOP_ONLY`。

### Batch B：Providers / Agents / Auth 基础闭环

这批决定远程 Web 是否真的能替代桌面做日常切换。

- 六类 agent 的“导入当前配置”真实 smoke 已补入 `pnpm smoke:web-server`：Claude、Codex、Gemini、OpenCode、OpenClaw、Hermes 都已验证到“读取 live/default config -> 导入数据库 -> 导入后 providers 列表可见”这一层。
- Providers 不能只验证新增成功，还要覆盖读取 live 配置、切换当前 provider、写回配置、错误提示分流。当前这部分不仅已有 runtime smoke 的三条代表性写回/切换探针，页面级证据也已经扩展到 `import current -> edit -> switch/live writeback`、OpenCode `Live Config / DB Only`、provider limits、provider usage stats 与 diagnostics error badges；`ProviderCard` + `ProviderHealthBadge` focused tests 也已补齐基础 health 状态语义。剩余更多是长尾 app 手工矩阵与 warning 清理，而不是基础 CRUD 缺口。
- `EndpointSpeedTest` 现在已经不只是 focused test：除 `tests/components/EndpointSpeedTest.test.tsx` 外，`tests/integration/EndpointSpeedTest.web-server.test.tsx` 也已在真实 `web-server` 上验证 `test_api_endpoints` 自动择优、服务端失败态回显，以及编辑态自定义端点通过 `get_custom_endpoints` / `add_custom_endpoint` / `remove_custom_endpoint` 做真实 diff 持久化；这部分现已可直接记为 Providers 页面级闭环证据。
- `AuthCenterPanel`、`CopilotAuthSection`、`CodexOAuthSection` 已补 focused test，覆盖 `start login` 入口、enterprise domain gating、`poll` 期间 copy/cancel、`set default`、`logout`、`remove account`、Codex FAST mode 与失败重试；真实临时数据目录下的 device flow smoke 也已补齐主链路。
- 如果某些账号体系在 Web 下仍不准备开放，页面必须直接隐藏入口或稳定显示 `WEB_NOT_SUPPORTED`，不能保留“点了才知道不行”的半成品状态。

### Batch C：Prompts / Skills / Sessions / MCP 页面闭环

这批是你已经发现缺口最多的一组，但它们不能孤立开发，要按“页面主流程能走通”来验收。

- `Prompts`：列表、创建/编辑、启停、删除、读取当前文件、浏览器导入。
- `Skills`：已安装列表、启停、卸载、ZIP 安装、从 apps 导入、repo 管理、更新检查、更新、备份恢复；主面板的列表级失败提示已补到结构化 skills error 映射。
- `Sessions`：列表、消息读取、单删、批删、Web resume 降级语义已具备 runtime smoke；后续重点是页面级搜索、选择集、失败恢复和壳层工具栏接线验收。
- `MCP`：统一接口与 legacy Claude MCP 兼容接口已验证到 read/upsert/delete/enable/validate/config route 级别；列表页 import/toggle/delete 失败提示也已补到提取 detail 并映射 MCP 后端错误文案，删除确认取消态也已补到真实页面层。后续重点收敛为人工矩阵和更长尾失败文案验收。
- `Prompts` 当前不再缺 import 或 create/edit 主链路；`tests/integration/PromptPanel.web-server.test.tsx` 已在临时 `web-server` 上验证 `import -> enable -> current file write -> disable -> live file clear -> delete`，并进一步补上 Gemini 变体的 `openAdd -> create -> edit -> enable -> live GEMINI.md writeback`。`tests/hooks/usePromptActions.test.tsx` 现也已把 `load/save/delete/disable/import` 的失败 detail、current-file 缺失静默，以及真实 current-file 读取异常提示都锁住；同一 real-server 页面测试现在也覆盖 live `CLAUDE.md` 读取异常的 `prompts.currentFileLoadFailed` toast detail。Prompt 剩余更偏向人工矩阵。
- `Skills` 当前不再缺主面板基础链路；`tests/integration/UnifiedSkillsPanel.web-server.test.tsx` 已证明 `openInstallFromZip` 与 import/toggle/uninstall/restore 都可以跑通，且 `tests/components/UnifiedSkillsPanel.test.tsx` 现也已覆盖结构化 ZIP 错误翻译与 toggle fallback，不再停留在通用错误 toast。`SkillsPage` 的 repo discovery/install/update 主链路、`searchSource` 状态机、skills.sh 新查询加载期间“旧结果不闪回、不中途误报空结果”、清空/缩短输入后旧结果与旧错误态立即撤销、真实 `web-server` 下 skills.sh 空结果、skills.sh 503 错误/detail/retry 恢复、trim 查询与同 query retry，以及初始空仓库 fallback / 手动 repo 空态也已在 2026-05-05 收口。`skillsDiscovery` 后续重点已转向人工矩阵与更深组合。
- 这一批当前不再以“补基础 handler”为目标；`Sessions` 与 `MCP` 的 route/runtime CRUD、页面壳层接线和浏览器侧验收已经完成，剩余工作是人工矩阵复核和新增入口防回退。
- 2026-05-04 当前这一批又新增了五条真实页面 smoke：`tests/integration/PromptPanel.web-server.test.tsx` 已在临时 `web-server` 上验证 `prompts.import -> 列表刷新 -> enable -> live file write -> disable -> live file clear -> delete`，并直接校验隔离 `~/.claude/CLAUDE.md` 的真实副作用；随后又补上 Gemini 变体的 `openAdd -> create -> edit -> enable -> live GEMINI.md writeback`，以及 live `CLAUDE.md` 读取异常时的 `prompts.currentFileLoadFailed` toast detail。`tests/integration/UnifiedMcpPanel.web-server.test.tsx` 已在同一基座上验证 `openImport -> 列表落库 -> codex toggle -> delete`，并进一步补上 `openAdd -> save -> edit -> live .claude.json / .codex/config.toml writeback`，以及删除确认取消后后端记录保持不变；`tests/integration/SessionManagerPage.web-server.test.tsx` 已验证排序后的会话列表、详情切换、Web resume 复制、批量删除，以及底层 Codex session JSONL 文件被真实删除；`tests/integration/UnifiedSkillsPanel.web-server.test.tsx` 已验证 `openInstallFromZip -> multipart upload -> installed skill 可见 -> live/SSOT file write`，以及 `openImport -> import from apps -> codex toggle -> uninstall -> restore from backup`；`tests/integration/SkillsPage.web-server.test.tsx` 也已在同一套 real-server 基座上重新验证 `openRepoManager -> add repo -> discover -> install -> UnifiedSkillsPanel 已安装行联动 -> check updates -> update -> uninstall`、skills.sh 分页安装、skills.sh 空结果、skills.sh 503 错误/detail/retry 恢复，以及初始无 repo 时自动进入 `skills.sh`、手动切回 `repos` 后显示空仓库态，并直接校验隔离 `dataDir/skills` / live `~/.claude/skills` 的文件副作用。因此 `Prompts` / `Sessions` / `MCP` / `SkillsDiscovery` 已主要收敛到少量人工矩阵补验。
- 这一批完成的标志不是单个接口通，而是各自页面上至少一条主链路可完整走通且无未捕获异常。

### Batch D：Workspace / OpenClaw / Hermes / DeepLink

这批属于新版能力和远程使用差异最大的区域，必须尽早做“能不能安全落地”的判断。

- `Workspace` 与 `Daily Memory` 要验证 read/write/list/search/delete 主流程，同时明确服务端路径边界。
- `OpenClaw` 的 `env/tools/agents defaults` 要逐页核对，不只看 handler 是否存在。
- `Hermes` 需要明确 memory get/set 与 `open_hermes_web_ui` 的 Web 语义，尤其要提示“服务端本机地址不等于远程浏览器所在机器”。
- `DeepLink` Web 版要以粘贴文本或 URL 解析导入为主，不再把 OS 协议注册当成前提。
- 2026-05-05 当前这一批的 focused tests、runtime smoke 与 rendered-page 证据都已补上：`tests/components/WorkspacePanels.test.tsx` 已覆盖 Workspace / Daily Memory 的 Web 手动路径提示；同轮新增的 `tests/components/WorkspaceFileEditor.test.tsx` 与 `tests/components/DailyMemoryPanel.test.tsx` 又把 Workspace / Daily Memory 的 load/save failure detail、Daily Memory search result delete -> list reload -> active search refresh，以及 delete failure detail 收口到 `extractErrorMessage()` 路径，不再在可达页面上只给无 detail 的泛化 toast。`tests/components/OpenClawPanels.test.tsx` 已覆盖 env/tools/agents defaults 的保存与迁移语义，`tests/integration/WorkspaceOpenClawHermes.web-server.test.tsx` 也已把 OpenClaw Env/Tools 的 rendered save -> live `openclaw.json` writeback -> reload 回显补到真实 `web-server` 页面层。`tests/components/HermesMemoryPanel.test.tsx` 已覆盖 Hermes memory 的保存/启停、user tab 独立参数与远程提示语义，`tests/components/DeepLinkImportDialog.test.tsx` 已覆盖 paste parse/import 以及 parse/merge/import 三条失败链路的 structured detail，`tests/integration/App.test.tsx` 也已补上 Web 顶部 `deeplink.pasteImport` 入口到 dialog handle 的壳层接线，`tests/integration/DeepLinkImportDialog.web-server.test.tsx` 进一步把这条链路跑到了真实临时 `web-server`；同一套 real-server 页面 smoke 基座现在也已覆盖 `tests/integration/BackupListSection.web-server.test.tsx` 与 `tests/integration/WorkspaceOpenClawHermes.web-server.test.tsx`。`pnpm smoke:web-server` 仍继续承担这四页对应的真实后端链路探针。当前剩余工作已不再是基础 route/runtime 缺口，而是最终逐页人工验收、更深的跨面板联动，以及少量 edge-case 页面补验。

### Batch E：Proxy / Failover / Usage / Subscription 运维页

这些页面虽然不一定是首次使用入口，但它们已经在 UI 中可见，因此不能长期处于“接口有了、页面半通”的状态。

- `Proxy`：配置、状态、takeover status、upstream、test URL，以及 app 级 proxy/failover 参数编辑。
- `Failover`：队列、auto failover、reset、circuit breaker config。
- `src-tauri/src/services/proxy_web.rs` 当前已明确 Web 运行时边界：`start()`、`start_with_takeover()`、`set_takeover_for_app()` 和 `switch_proxy_target()` 仍返回 web-server unavailable；可保留的是状态读取、takeover status、全局 proxy 配置、app 级 proxy/failover 配置，以及 failover 队列与阈值持久化。因此 Batch E 的目标不是把 Web 做成“远程启动本机代理”，而是让“可编辑配置”和“不可用运行时”两层语义在页面上清楚分离。
- `FailoverQueueManager` 已补 focused test，覆盖 auto failover 开关、队列新增/删除、队列项健康状态、熔断 reset、空列表提示，以及 add/remove/load 失败时提取 detail 而不是把异常对象直接 `String(error)` 暴露给用户；`AutoFailoverConfigPanel` 已补 focused test，覆盖配置加载、重置、范围校验、保存、disabled 态，以及 load/save 失败 detail 文案；`ProxyTabContent` 已补 Web runtime-control unavailable、failover config-only 与 runtime-stats placeholder 文案，明确说明 web-server mode 不提供本地 proxy runtime / app takeover / live breaker counters；`tests/integration/ProxyTabContent.web-server.test.tsx` 现又进一步把 failover provider `Select`、queue add/remove、auto switch toggle 与 app config save 跑到了真实页面层。
- 2026-05-04 当前这一批的 Proxy / Failover runtime smoke 也已开始补入：`pnpm smoke:web-server` 现已验证 `get_proxy_takeover_status`、Claude app proxy config 的读取与更新、failover queue 的 empty-queue guard / add / remove、`set_auto_failover_enabled` 的持久化，以及 `get_circuit_breaker_stats -> WEB_NOT_SUPPORTED`。这一轮同时暴露并修正了 `ProxyTakeoverStatus` 缺少 `hermes` 字段的后端兼容问题。
- `get_circuit_breaker_stats` 当前虽仍是显式 `WEB_NOT_SUPPORTED` 语义，但 failover 页面已补上 Web placeholder 与替代路径说明，避免用户把“没有 runtime stats”误判成加载失败；对应 server smoke 现已纳入文档后部的通过项。
- `tests/integration/ProxyTabContent.web-server.test.tsx` 已经把这一页拉到真实 `web-server` rendered-page 基座，并稳定验证 runtime-unavailable / config-only 文案、failover provider `Select`、queue add/remove、auto switch toggle 和 app config save；Batch E 这条页面 smoke 当前已不再是显式待办。
- `Usage`：dashboard、logs、detail、pricing、session sync、data sources。
- `UsageDashboard` 已补 focused test，覆盖 app filter、refresh interval、date range、pricing 面板挂载，以及切换到 `providers/models` 后在 dashboard 级 filter 更新时保持当前 tab；`PricingConfigPanel` 已补 focused test，覆盖默认倍率/来源加载保存和 add/edit/delete 入口；`DataSourceBar` 已补 focused test，覆盖 data sources 展示、session sync 导入/已最新/失败反馈与空来源态；`RequestLogTable` / `RequestDetailPanel` 已补 focused test，覆盖分页 reset、详情入口挂载、详情弹层渲染/关闭和请求不存在反馈。2026-05-04 同日 `pnpm smoke:web-server` 也已补上 `sync-session-usage`、`usage-data-sources-after-session-sync`、`usage-summary-after-session-sync`、`usage-trends-after-session-sync`、`request-logs-after-session-sync`、`request-detail-after-session-sync`、`request-detail-not-found`、`model-pricing-list`、`model-pricing-upsert`、`model-pricing-delete`。当前剩余工作已不再是 Usage runtime 缺 handler，也不再是跨面板联动缺口，而是人工可见结果矩阵与少量长尾空态/失败态验收。
- `Subscription`：余额、coding plan quota、订阅状态。
- `SubscriptionQuotaView` / `SubscriptionQuotaFooter` 已补 focused test，覆盖 not_found / parse_error 静默、expired 提示、success tier 展示、inline tier 过滤和 refresh；2026-05-04 同日 `pnpm smoke:web-server` 也已补上 `subscription-quota-claude-not-found`、`subscription-quota-gemini-parse-error`、`balance-unknown-provider`、`coding-plan-unknown-provider` 与 `usage-script-invalid-app` 的确定性运行时证据。当前剩余工作已收敛到 OAuth 绑定态、真实 provider card 结果与外网 happy-path mock/人工验收。
- 这批页面要特别注意“无数据”和“Web 不支持”的可见反馈，不能让用户只看到空白面板。

### Batch F：最终完成审计

当前完成审计已经跑完，结论是“可见页面主链路扫平，发布前继续保留人工复核和新增入口防回退”：

- 用临时数据目录启动 Web server 后，runtime smoke 已覆盖基础探针、文件流、Providers、Sessions、MCP、Workspace / OpenClaw / Hermes / DeepLink、Proxy / Failover、Usage / Subscription。
- 每个可见页面至少保留一条实际 smoke 或 rendered-page 证据，`Manual Regression Matrix` 当前为 `72 pass / 72 total`，包含 command、route、handler 类型、用户可见结果和错误码语义。
- 这次完成判定不依赖单一门禁：`missing 0`、`parityFallback 0`、`pnpm typecheck`、`pnpm build:web` 和 manifest 全映射都只是组合证据的一部分。
- 当前未发现仍需要人工解释“这个按钮其实别点”的 P0/P1 可见入口；桌面专属能力已统一成禁用、隐藏、浏览器替代或结构化 Web 错误语义。

## Batch Exit Criteria

为了避免后续继续出现“接口补完了但页面还没收口”的情况，每一批都按同一组退出条件执行：

- 至少补一条和该批页面主流程对应的测试，优先是组件测试或集成测试。
- 对涉及 Web 特有语义的行为补断言：浏览器 upload/download、`window.open`、desktop-only toast、disabled state。
- 把该批涉及的页面写入 `Manual Regression Matrix`，不能只在 PR 描述里口头说明。
- 如果是桌面专属降级项，必须同时满足“UI 上禁用/隐藏”和“API 返回结构化错误码”。
- 如果是文件流替代项，必须确认最终链路不再触发桌面 file dialog 命令。

## Completion Criteria

本计划完成时必须同时满足以下条件：

- `pnpm check:web-routes` 通过，且 P0/P1 命令不再依赖 parity catch-all。
- `pnpm typecheck` 通过。
- `pnpm build:web` 通过。
- `cargo check --no-default-features --features web-server --example server` 通过。
- 用临时数据目录启动 Web server 后，页面级 checklist 的 P0 操作可实际完成。
- 所有桌面专属入口都有统一 Web 提示，不出现未捕获异常。
- 文件上传/下载路径均使用浏览器 upload/download，不再调用桌面 file dialog。

截至本次文档更新，上述完成条件已经具备证据：静态门禁通过，基础 `pnpm smoke:web-server` 与扩展文件流探针通过，20 个 rendered-page `web-server` 测试文件共 49 条测试通过，`Manual Regression Matrix` 为 `72 pass / 72 total`。当前不再存在 P0/P1 基础功能 blocker；剩余风险应作为发布前人工复核、远程安全补强和后续新增入口防回退处理。

## Test Plan

### Static Checks

- 命令覆盖检查：所有前端可见命令必须有 route、unsupported 或 UI 替代。
- parity fallback 审计：P0/P1 route 不能只命中 `handlers/parity.rs`。
- `pnpm typecheck`
- `pnpm build:web`
- `cargo check --no-default-features --features web-server --example server`

当前结果：

- `pnpm check:web-routes -- --list-parity`：`259 commands / 247 routes / 20 wildcardRoutes / 30 unsupported / webReplacements 7 / missing 0 / parityExact 0 / parityFallback 0`
- `pnpm typecheck`：通过
- `pnpm build:web`：通过
- `cargo check --manifest-path src-tauri/Cargo.toml --no-default-features --features web-server --example server`：通过
- `cargo test --manifest-path src-tauri/Cargo.toml --test prompt_service`：通过；`src-tauri/tests/prompt_service.rs` 已覆盖 disabled prompt 导入不触碰 live 文件，以及“最后一个 enabled prompt -> disabled”时的清空语义
- `cargo test --manifest-path src-tauri/Cargo.toml --test mcp_commands`：通过；兼容层 MCP 命令回归仍可跑
- `cargo test --manifest-path src-tauri/Cargo.toml --test skill_sync`：通过；`src-tauri/tests/skill_sync.rs` 已覆盖 import selection、卸载前备份、restore 到当前 app、disabled/orphaned symlink 清理等 skills 同步语义
- `pnpm smoke:web-server`：通过；当前脚本会为 server 注入隔离 `CC_SWITCH_TEST_HOME`，覆盖 Claude / Codex / Gemini `/api/config/import-default-config`，OpenCode `/api/providers/import-opencode-providers-from-live`，OpenClaw `/api/openclaw/import-openclaw-providers-from-live`，Hermes `/api/hermes/import-hermes-providers-from-live`，对应的 `/api/providers/get-providers` 回查，Claude / OpenCode 三条代表性 provider 更新与切换写回 live 配置探针，advanced config 的 stream-check / log-config get/save 持久化链路，Backup create/list/rename/restore/delete，Sessions / MCP / Workspace / OpenClaw / Hermes / DeepLink / Proxy / Failover 的真实 runtime CRUD 与 Web 降级语义，以及 Usage / Subscription 的 session sync、logs/detail、pricing CRUD、quota fallback 与 deterministic error-state 探针
- Providers 页面级补充证据：`tests/components/EndpointSpeedTest.test.tsx` 已覆盖 `test_api_endpoints`、`get_custom_endpoints`、`add_custom_endpoint`、`remove_custom_endpoint` 的主要 UI 流程；`tests/integration/EndpointSpeedTest.web-server.test.tsx` 已进一步在临时 `web-server` 上真实验证 `test_api_endpoints` 自动择优、服务端失败态回显，以及编辑态自定义端点 add/remove diff 经真实 Web API 持久化；`tests/lib/import-current-config.test.ts` 已覆盖 Claude/Codex/Gemini 默认导入和 OpenCode/OpenClaw/Hermes live import 的共享分流与 `no-change` 语义；`tests/components/ProviderList.test.tsx` 已补 OpenCode 空状态导入；`tests/components/ProviderCard.test.tsx` 已覆盖 card 级 live-config status、health 指示器、usage 摘要、limits query gating 和错误态分流标签；`tests/integration/App.test.tsx` 已补 Providers 顶部工具栏对六个 app 的组合回归，以及“无新配置可导入”时的 info toast 反馈；`tests/integration/ProviderList.web-server.test.tsx` 现已进一步在真实 `web-server` 上验证 Claude 的 `import current -> edit -> switch`、OpenCode 的 `import current -> edit`、OpenCode additive provider 的 `Live Config / DB Only` 状态、card 级 provider limits、provider usage stats，以及故意注入 `/api/providers/get-provider-health`、`/api/providers/check-provider-limits`、`/api/providers/get-provider-stats` 失败后的 split diagnostics error badges，同时在编辑弹窗内通过真实 Web API 执行 `fetch_models_for_config` 与 Claude `common config snippet` 提取页面闭环，并顺带修正 `ProviderForm` 提交时名称未同步回 `ui.displayName` / `settingsConfig.name` 的真实缺口
- Universal Provider 页面级补充证据：`src/components/universal/UniversalProviderPanel.tsx` 已补 dedicated create 入口，修复该一级页面与 Add Provider 的 universal tab 只能看/改不能新建的问题；`src/App.tsx` 现已在 Claude/Codex/Gemini 的 providers 工具栏补上进入 `universal` 视图的壳层入口，并在不支持的 app 下直接隐藏；`src/components/universal/UniversalProviderFormModal.tsx` 现会等待 `onSave / onSaveAndSync` 的异步结果，只有成功时才关闭，且编辑模式重新暴露了独立 `save` 与 `save-and-sync` 两条路径，避免“只能同步、不能只保存”的页面语义缺口；`tests/components/UniversalProviderPanel.test.tsx` 已覆盖空状态新增入口、新建后自动 sync 到各 app，以及编辑时只 upsert 不误触发 sync；`tests/components/UniversalProviderFormModal.test.tsx` 已覆盖普通保存失败、编辑模式单独保存，以及 save-and-sync 失败时保持模态框/确认框不关闭；`tests/integration/App.test.tsx` 已覆盖该入口从 App 壳层可达，以及切到 OpenCode 后入口消失；`tests/integration/UniversalProviderPanel.web-server.test.tsx` 现已进一步在真实 `web-server` 上验证 create -> auto-sync -> edit(save only) -> manual sync -> delete，并直接回查 Claude/Codex/Gemini 生成的子 provider 配置与删除联动
- Prompts 页面级补充证据：`tests/components/PromptPanel.test.tsx` 已覆盖 toolbar `openImport`、toolbar `openAdd`、空状态导入按钮，以及 `gemini -> GEMINI.md` 的当前文件提示；`tests/hooks/usePromptActions.test.tsx` 已覆盖导入成功后的 reload/success toast、Web 取消文件选择时的静默 no-op、`load/save/delete/disable/import` 失败 detail，以及 current-file 缺失静默与真实读取异常提示；`tests/integration/App.test.tsx` 已补 App 顶部 Prompts 工具栏到 `PromptPanel` imperative handle 的接线路径，确认页面壳层的导入/新增入口也已真正接通；`tests/integration/PromptPanel.web-server.test.tsx` 进一步在临时 `web-server` 上真实验证了 Claude 的 import -> enable -> live `~/.claude/CLAUDE.md` 写入 -> disable -> live 文件清空 -> delete、Gemini 变体的 create -> edit -> enable -> live `~/.gemini/GEMINI.md` 写回，以及 live `CLAUDE.md` 读取异常时页面显示 `prompts.currentFileLoadFailed` 且错误 detail 不退化成空白或 `[object Object]`
- Skills 页面级补充证据：`tests/components/UnifiedSkillsPanel.test.tsx` 已覆盖 `openImport`、`openInstallFromZip` 与 OpenClaw 导入默认启用映射；`tests/integration/App.test.tsx` 现已补 App 顶部 Skills 工具栏到 `UnifiedSkillsPanel` imperative handle 的接线路径，确认 `restore from backup`、`install from zip`、`import from apps` 三个壳层入口都已真正接通，其中 `install_skills_from_zip` 对应的页面级 browser upload 入口也因此具备了壳层证据。2026-05-04 同日 `tests/integration/UnifiedSkillsPanel.web-server.test.tsx` 又进一步在临时 `web-server` 上真实验证了 `openInstallFromZip -> multipart upload -> installed skill 可见 -> dataDir/skills/page-smoke-zip-skill/SKILL.md -> live ~/.claude/skills/page-smoke-zip-skill/SKILL.md`，以及 `openImport -> import from apps -> codex toggle -> uninstall -> restore from backup`，并直接回查 `/api/skills/get-installed-skills`、`/api/backups/get-skill-backups`、隔离 `dataDir/skills` SSOT 与 live `~/.claude/skills` / `~/.codex/skills` 的文件副作用；这一轮也顺带修正了 `src-tauri/src/services/skill.rs` 中 `~/.agents` 路径未走 `get_home_dir()`、导致 real-server 测试会泄漏真实 home 的隔离问题
- Skills Discovery 页面级补充证据：`tests/components/SkillsPage.test.tsx` 已覆盖 App toolbar 依赖的 `refresh/openRepoManager` imperative handle、仓库模式安装技能、skills.sh 搜索分页累积、repo/skills.sh source 切换与 retry，以及“新 skills.sh 查询加载期间旧结果不闪回、不中途误报空结果”、清空/缩短输入后旧结果与旧错误态立即撤销；`tests/components/RepoManagerPanel.test.tsx` 又补上重复仓库覆盖提示、add form pending/失败反馈、remove 行级 pending/失败反馈。`tests/integration/SkillsPage.web-server.test.tsx` 现已在临时 `web-server` 上完整验证 repo manager add -> discover -> install -> UnifiedSkillsPanel 联动 -> check updates -> update -> uninstall、skills.sh 分页安装、skills.sh 空结果、skills.sh 503 错误/detail/retry 恢复，以及初始无 repo 时自动进入 `skills.sh`、手动切回 `repos` 后显示空仓库态和添加仓库入口；2026-05-05 暴露的 `searchSource` 语义回归也已修复，repo add/discover 后会正确回显 repo 结果，而显式选中的 `skills.sh` 仍保持不变。为支撑这条 real-server smoke，`src-tauri/src/services/skill.rs` 现在也支持通过 `CC_SWITCH_SKILL_ARCHIVE_BASE_URL`、`CC_SWITCH_SKILL_DOC_BASE_URL` 与 `CC_SWITCH_SKILLS_SH_API_BASE_URL` 注入本地镜像/假服务地址，避免页面 smoke 依赖外网 GitHub / skills.sh 可用性
- MCP 页面级补充证据：`tests/components/UnifiedMcpPanel.test.tsx` 已覆盖 toolbar `openImport`、toolbar `openAdd`、统一列表里的 app toggle，以及删除确认主链路；`tests/integration/App.test.tsx` 已补 App 顶部 MCP 工具栏到 `UnifiedMcpPanel` imperative handle 的接线路径，确认 import/add 不只是面板内部按钮可用；`tests/components/McpFormModal.test.tsx` 与 `tests/hooks/useMcpValidation.test.tsx` 现又额外覆盖 plain-object parser error 的结构化解包；`tests/integration/UnifiedMcpPanel.web-server.test.tsx` 进一步在临时 `web-server` 上真实验证了 import -> codex toggle -> delete、`openAdd -> save -> edit` 的 live `~/.claude.json` / `~/.codex/config.toml` 写回结果，以及删除确认取消后后端 MCP server 仍保留、页面行仍可见；`src-tauri/tests/mcp_commands.rs` 现也覆盖 `import_mcp_from_all_apps` 的全失败 detail 与部分成功保留语义，避免 Web import 把损坏配置误报成没有可导入项
- Sessions 页面级补充证据：`tests/components/SessionManagerPage.test.tsx` 已覆盖当前 `appId` 默认过滤、`Hermes` provider filter 选项与切换、消息读取、单删、批删部分失败恢复、搜索过滤后的选择集收敛，以及 Web resume 复制语义；其中 2026-05-05 新增的 focused test 进一步确认“成功项立即移除、失败项保留并继续选中、success/error toast 立即显示”这一批删反馈语义，同轮又补上了 `Hermes` filter 缺口回归。`tests/integration/SessionManagerPage.web-server.test.tsx` 进一步在临时 `web-server` 上真实验证了列表渲染、详情切换、Web resume 复制、批量删除，以及底层 Codex session JSONL 文件确实被删除
- Auth 页面级补充证据：`tests/components/AuthCenterPanel.test.tsx` 已覆盖登录入口、多账号管理主分支与 device code 轮询态；`tests/components/CopilotAuthSection.test.tsx` 已覆盖 enterprise domain gating、初始登录 pending 禁用、device code copy/cancel、默认账号/移除/注销和选中账号删除时的外部选择清理；`tests/components/CodexOAuthSection.test.tsx` 已覆盖初始登录 pending 禁用、FAST mode 开关、device code copy/cancel、失败重试与默认账号/移除/注销；`tests/hooks/useManagedAuth.test.tsx` 则补上了 hook 层 plain-object error 解包。`tests/integration/AuthCenterPanel.web-server.test.tsx` 进一步在真实 `web-server` + 本地假认证服务下验证 Copilot pending/cancel/success、Codex 登录、多账号、默认账号切换、移除、失败重试与 logout all，并直接回查 `copilot_auth.json` / `codex_oauth_auth.json` 的落盘结果
- About 页面级补充证据：`tests/components/AboutSection.test.tsx` 已覆盖 up-to-date toast、Web 下载页打开、release notes、WSL shell 刷新，以及“浏览器报告 Windows 时仍展示服务端工具/安装面板”的远程 Web 语义；同轮还补上了 release notes 打开失败、检查更新失败，以及服务端工具版本加载失败时的 structured detail / alert。`tests/lib/updater.test.ts` 已补 `getCurrentVersion()` 在 Web 模式下读取 `/api/health` 版本与失败兜底；`tests/lib/updater-adapter.test.ts` 已补 Web 下 `GET /api/system/get_update_info` 的适配与失败回退；`tests/integration/AboutSection.web-server.test.tsx` 已进一步在临时 `web-server` 上真实验证版本徽标来自 `/api/health`、服务端环境/安装提示可见、当前 release notes 跳转、发现更新后通过服务端元数据打开 `html_url` 下载页，以及 `get_tool_versions` 返回的四张工具卡片、本地版本/最新版本徽标和 refresh 行为。对应页面当前已完成 Web 更新与工具版本卡片的 rendered-page 证据收口，Batch A 剩余工作已转向 `webReplacements` 壳层触发路径和手工回归
- Settings 壳层页面级补充证据：`tests/components/SettingsDialog.test.tsx` 已覆盖 `proxy / auth / usage / about` 四个一级 tab 在 Web 下的面板挂载，确保 `SettingsPage` 不会只保留 tab 标题而漏掉对应页面壳层接线
- Settings general 页面级补充证据：`tests/integration/SettingsGeneral.web-server.test.tsx` 已在真实 `web-server` 上验证 `LanguageSettings` 的 `click -> autoSave -> settingsApi.save -> localStorage -> remount reload` 闭环，确认 `general` 页至少已有一条真实持久化路径，而不再只是壳层挂载证据
- Settings general focused 补充证据：`tests/components/SkillStorageLocationSettings.test.tsx` 已补上 `SkillStorageLocationSettings` 的确认弹窗、无 skill 直接迁移、部分迁移 warning 与失败 detail toast，避免 `general` 页这一块继续停留在 `String(error)` 级别
- Settings general focused 补充证据：`tests/components/ThemeSettings.test.tsx` 已覆盖主题切换、localStorage 持久化和根节点 class 更新；`tests/components/AppVisibilitySettings.test.tsx` 已覆盖 app 可见性切换与最后一个可见 app 的禁用保护
- Settings general focused 补充证据：`tests/components/SkillSyncMethodSettings.test.tsx` 已覆盖 `auto -> symlink` 的默认显示语义、symlink hint，以及 `copy/symlink` 两条切换分支
- Settings general focused 补充证据：`tests/components/WindowSettings.test.tsx` 已覆盖 Web 下隐藏 desktop-only 开关，以及 Linux desktop 下 `launchOnStartup / silentStartup / minimizeToTray / useAppWindowControls` 的显示与回调；`tests/components/TerminalSettings.test.tsx` 已覆盖 Linux 默认终端、macOS fallback 以及平台选项切换回调
- Settings restart prompt 补充证据：`tests/components/SettingsDialog.test.tsx` 现已覆盖 `requiresRestart` 下的 dev-mode 快捷重启与 production 分支的 restart failure structured detail，确保 `settings.restartFailed` 不再吞掉后端错误细节
- Settings WebDAV 页面级补充证据：`tests/integration/WebdavSyncSection.web-server.test.tsx` 已在真实 `web-server` 上联动本地假 WebDAV 服务，验证 `save -> auto test -> upload -> remote manifest info -> download restore` 的页面闭环，并额外覆盖一条 `fetch remote info` 失败 toast；测试同时直接回查远端 `manifest.json/db.sql/skills.zip` 与本地 provider 数据恢复，不再只是 runtime probe
- Backup 页面级补充证据：`tests/components/BackupListSection.test.tsx` 已覆盖 advanced backup 分组的策略设置、create、rename、restore、delete 主流程，确认 `BackupListSection` 在 Web 下的操作按钮、确认对话框和 toast 反馈都已接线；`tests/integration/BackupListSection.web-server.test.tsx` 进一步在临时 `web-server` 上真实验证了 create -> rename -> restore -> delete 的页面闭环
- Import / Export 页面级补充证据：`tests/components/ImportExportSection.test.tsx` 已覆盖文件未选中、文件已选中、导入中、导入成功与失败等基础 UI 状态；`tests/hooks/useImportExport.test.tsx` 已覆盖浏览器 `File` 透传、导入成功后的 backupId 回写、导出成功 toast、异常态，以及“用户取消保存路径时静默 no-op、不再误报错误 toast”的取消语义；同一轮 thrown error 也已统一切到 `extractErrorMessage()` 路径。`tests/integration/ImportExportSection.web-server.test.tsx` 进一步在临时 `web-server` 上真实验证了 `settings.exportConfig` 的浏览器 download、时间戳文件名与非空 SQL blob，以及 `settings.selectConfigFile -> settings.import` 的 rendered-page upload、provider A/B 快照回滚、backupId 展示与 live `~/.claude/settings.json` 恢复。Import / Export 页面当前已具备导入与导出的真实页面级证据
- Advanced Config 页面级补充证据：`tests/components/ModelTestConfigPanel.test.tsx` 已覆盖 model-test 配置的 load/save、空数字输入回退默认值，以及 load/save 失败 detail；`tests/components/LogConfigPanel.test.tsx` 已覆盖日志配置的 load、level 保存、enabled toggle、失败回滚，以及 load 失败时显示 destructive alert；`tests/integration/AdvancedConfigPanels.web-server.test.tsx` 现已进一步在真实 `web-server` 上验证 `ModelTestConfigPanel` 与 `LogConfigPanel` 的 load -> save -> reload 页面闭环，并直接回查 `/api/config/get-stream-check-config` 与 `/api/config/get-log-config` 的持久化结果
- Failover 页面级补充证据：`tests/components/FailoverQueueManager.test.tsx` 已覆盖 auto failover 开关、队列 add/remove、队列项健康状态、熔断 reset、空状态，以及 add/remove/load 失败 detail 文案；`tests/components/AutoFailoverConfigPanel.test.tsx` 已覆盖配置加载、重置、范围校验、保存、disabled 态，以及 load/save 失败 detail 文案；`tests/components/ProxyTabContent.test.tsx` 已覆盖首次启用确认、故障转移确认、桌面模式停止态禁用提示，以及 Web runtime-control unavailable / failover config-only / runtime-stats 占位文案；`tests/integration/ProxyTabContent.web-server.test.tsx` 现已进一步在真实 `web-server` 页面基座上验证 failover provider `Select`、queue add/remove、auto switch toggle 与 app config save，当前记为 pass
- Usage dashboard 页面级补充证据：`tests/components/UsageDashboard.test.tsx` 已覆盖 dashboard 级 app filter、refresh interval、date range、pricing panel 挂载，以及 filter/range/refresh 更新后保持当前 `providers/models` tab；与 `tests/components/RequestLogTable.test.tsx`、`tests/components/DataSourceBar.test.tsx` 联跑通过
- Usage logs/detail/session sync 页面级补充证据：`tests/components/RequestLogTable.test.tsx` 已覆盖分页 reset 与 request detail 入口挂载；`tests/components/RequestDetailPanel.test.tsx` 已覆盖详情渲染、关闭按钮和 not_found 反馈；`tests/components/DataSourceBar.test.tsx` 已覆盖 data source chips、session sync imported/up-to-date、失败 detail toast 与空来源态
- Usage rendered-page 页面级补充证据：`tests/integration/UsageDashboard.web-server.test.tsx` 已进一步在临时 `web-server` 上真实验证 empty-state 下 `Import Sessions` 入口始终可见、Codex archived session usage 导入后 `codex_session` data source 可见、request logs 列表刷新、request detail dialog 打开与真实 requestId 渲染，以及 app filter / refresh interval / date range 与 logs/providers/models 多面板联动；这条证据也直接覆盖了远程首次使用时“没有 sources 就看不到导入入口”的回归
- Pricing 页面级补充证据：`tests/components/PricingConfigPanel.test.tsx` 已覆盖默认成本倍率/来源加载保存，以及 model pricing 的 add/edit/delete 入口；与 `tests/components/UsageDashboard.test.tsx`、`tests/components/RequestLogTable.test.tsx` 联跑通过
- Subscription 页面级补充证据：`tests/components/SubscriptionQuotaFooter.test.tsx` 已覆盖 quota footer 的缺凭据静默、expired、success、inline 过滤和 refresh；与 `tests/components/ProviderList.test.tsx` 联跑通过
- 定向前端回归：`tests/lib/adapter.test.ts`、`tests/hooks/useImportExport.test.tsx`、`tests/components/PromptPanel.test.tsx`、`tests/components/ProviderList.test.tsx`、`tests/components/SessionManagerPage.test.tsx`、`tests/components/UnifiedSkillsPanel.test.tsx`、`tests/components/DirectorySettings.test.tsx`、`tests/components/SettingsDialog.test.tsx`、`tests/hooks/useProviderActions.test.tsx`、`tests/lib/web-side-effects.test.ts` 已通过
- Desktop-only adapter 收口证据：`tests/lib/adapter.test.ts` 已回归 `open_app_config_folder`、`open_config_folder`、`open_provider_terminal`、`open_workspace_directory`、`pick_directory` 在 Web 模式下会直接抛 `WebNotSupportedError` 且不会发 HTTP 请求；这 5 条命令现已从 `parityExact` 审计里移出
- Route audit 收口证据：`scripts/check-web-route-coverage.mjs` 现已显式区分 `unsupported`、`webReplacements`、`parityExact` 与 `parityFallback`；当前输出为 `30 unsupported / 7 webReplacements / 0 parityExact / 0 parityFallback`
- Web 文件流替代回归：`tests/lib/web-file-replacements.test.ts` 已覆盖 `settingsApi.openFileDialog/saveFileDialog/importConfigFromFile/exportConfigToFile`、`promptsApi.importFromFile`、`skillsApi.openZipFileDialog/installFromZip` 的 Web 分支，确认这 7 条 `webReplacements` 不会再误走桌面 `save/open file dialog` 或 ZIP 安装 `invoke`
- App 壳层集成回归：`tests/integration/App.test.tsx` 已通过，当前保留的 stderr 仅为故意构造的 duplicate provider 错误分支日志，不代表未处理异常

### Server Smoke Tests

当前已经具备一条可复跑的基础 smoke 命令：

```bash
pnpm smoke:web-server
```

该脚本会：

- 自动创建临时 `CC_SWITCH_DATA_DIR`
- 自动创建临时 `CC_SWITCH_TEST_HOME`，并预置 live provider / MCP / sessions fixture
- 复用 `dist-web`
- 启动 `cargo run --manifest-path src-tauri/Cargo.toml --no-default-features --features web-server --example server`
- 探测 `/api/health` 就绪后执行一组基础探针
- 校验至少两类 Web 降级语义：`WEB_DESKTOP_ONLY` 与 `WEB_UPLOAD_REQUIRED`

如遇冷启动编译较慢，可临时提高等待窗口：

```bash
CC_SWITCH_SMOKE_STARTUP_TIMEOUT_MS=600000 pnpm smoke:web-server
```

2026-05-04 当前已确认通过的基础探针：

- `health`
- `spa-root`
- `settings`
- `proxy-status`
- `backups`
- `mcp-servers`
- `prompts-claude`
- `usage-summary`
- `usage-data-sources`
- `check-for-updates`
- `tool-versions`
- `auth-status-copilot`
- `auth-status-codex`
- `desktop-only-open-app-config-folder`
- `upload-required-export-config`

2026-05-04 同步新增的 Desktop-only 契约探针：

- `desktop-only-open-config-folder`
- `desktop-only-open-provider-terminal`
- `desktop-only-pick-directory`

2026-05-04 同步新增的 Advanced Config 扩展探针：

- `stream-check-config`
- `stream-check-config-save`
- `stream-check-config-after-save`
- `log-config`
- `log-config-set`
- `log-config-after-save`

2026-05-04 同步新增的 Backup 扩展探针：

- `backup-create`
- `backups-after-create`
- `backup-rename`
- `backups-after-rename`
- `backup-restore`
- `backups-after-restore`
- `backup-delete`
- `backups-after-delete`

2026-05-04 同步新增的文件流扩展探针：

- `export-config-download`
- `import-config-upload`
- `import-prompt-upload`
- `prompts-claude-after-upload`

2026-05-04 同步新增的 Providers 导入扩展探针：

- `import-default-claude`
- `providers-claude-after-import`
- `import-default-codex`
- `providers-codex-after-import`
- `import-default-gemini`
- `providers-gemini-after-import`
- `switch-claude-provider-writes-live`
- `import-opencode-from-live`
- `providers-opencode-after-import`
- `update-claude-current-provider-writes-live`
- `update-opencode-live-managed-provider-writes-live`
- `import-openclaw-from-live`
- `providers-openclaw-after-import`
- `import-hermes-from-live`
- `providers-hermes-after-import`

2026-05-04 同步新增的 MCP 扩展探针：

- `claude-mcp-status`
- `validate-mcp-command`
- `import-mcp-from-apps`
- `mcp-servers-after-import`
- `read-claude-mcp-config`
- `get-mcp-config-claude`
- `toggle-mcp-app-disable-codex-live`
- `set-mcp-enabled-restore-codex-live`
- `upsert-mcp-server-writes-live`
- `delete-mcp-server-removes-live`
- `upsert-claude-mcp-server-live`
- `delete-claude-mcp-server-live`
- `upsert-mcp-server-in-config-live`
- `delete-mcp-server-in-config-live`

2026-05-04 同步新增的 Sessions 扩展探针：

- `sessions-list`
- `session-messages`
- `delete-session`
- `delete-sessions-batch`
- `session-launch-terminal-desktop-only`

2026-05-04 同步新增的 Batch D 扩展探针：

- `workspace-read-file`
- `workspace-write-file`
- `workspace-open-directory-desktop-only`
- `daily-memory-list`
- `daily-memory-read`
- `daily-memory-write`
- `daily-memory-search`
- `daily-memory-delete`
- `openclaw-get-env`
- `openclaw-set-env`
- `openclaw-get-tools`
- `openclaw-scan-health-initial`
- `openclaw-set-tools`
- `openclaw-get-agents-defaults`
- `openclaw-set-agents-defaults`
- `hermes-memory-limits`
- `hermes-memory-read`
- `hermes-memory-write`
- `hermes-memory-disable`
- `deeplink-parse-provider`
- `deeplink-merge-provider-config`
- `deeplink-import-provider-unified`
- `providers-openclaw-after-deeplink-import`

2026-05-04 同步新增的 Batch E（Proxy / Failover）扩展探针：

- `proxy-takeover-status`
- `proxy-config-claude-initial`
- `failover-queue-claude-initial`
- `failover-enable-codex-without-queue-blocked`
- `failover-available-providers-claude`
- `failover-add-claude-provider`
- `proxy-update-config-for-claude`
- `failover-enable-claude`
- `failover-runtime-stats-web-not-supported`
- `failover-disable-and-remove-claude`

2026-05-04 同步新增的 Batch E（Usage / Subscription）扩展探针：

- `sync-session-usage`
- `usage-data-sources-after-session-sync`
- `usage-summary-after-session-sync`
- `usage-trends-after-session-sync`
- `request-logs-after-session-sync`
- `request-detail-after-session-sync`
- `request-detail-not-found`
- `model-pricing-list`
- `model-pricing-upsert`
- `model-pricing-delete`
- `subscription-quota-claude-not-found`
- `subscription-quota-gemini-parse-error`
- `balance-unknown-provider`
- `coding-plan-unknown-provider`
- `usage-script-invalid-app`

这批扩展探针的当前状态：

- Config SQL 下载已能返回浏览器附件流。
- Config SQL export -> import 的空数据 roundtrip 已修复，并已有 Rust 回归测试兜底。
- Prompt 上传导入已收口；`PromptService::upsert_prompt()` 不再对新导入/新建的 disabled prompt 误写真实 live prompt 文件，`import-prompt-upload` 与 `prompts-claude-after-upload` 已能在 `pnpm smoke:web-server` 中连续通过。
- 六类 provider “导入当前配置”已收口到 runtime smoke：脚本会预置隔离 HOME 下的 Claude/Codex/Gemini/OpenCode/OpenClaw/Hermes live 配置，再验证导入命令与导入后 providers 列表回查全部通过。
- Providers 写回与切换已开始进入 runtime smoke：当前已用 `update-claude-current-provider-writes-live`、`switch-claude-provider-writes-live` 和 `update-opencode-live-managed-provider-writes-live` 三条探针，分别验证 switch-mode 的当前更新、switch-mode 的真实切换，以及 additive-mode 的代表性 live 写回语义。
- MCP 已收口到 unified + legacy 双轨 runtime smoke：当前不只验证 `/api/mcp/get-mcp-servers`，还验证 import 后 unified merge/normalize、legacy Claude read/config 投影、按 app toggle/set enabled、unified upsert/delete、legacy Claude upsert/delete，以及 config route upsert/delete 对 live 配置文件的真实写回。
- 这轮 MCP smoke 也已经把两个后端兼容问题固定下来：`validate_mcp_command` 必须读取 `POST` JSON body，而 `get_mcp_config` / `McpConfigResponse` 必须返回前端期望的 `configPath` camelCase 字段。
- Sessions 已收口到真实文件级 runtime smoke：脚本会生成三条隔离 Codex session JSONL，验证列表、消息读取、单删、批删，以及 `launch-session-terminal -> WEB_DESKTOP_ONLY` 的桌面专属降级语义。
- Workspace / Daily Memory 已收口到真实文件级 runtime smoke：脚本会预置 `~/.openclaw/workspace/AGENTS.md` 与 `~/.openclaw/workspace/memory/2026-03-04.md`，验证 Workspace 文件 read/write、Daily Memory list/read/write/search/delete，以及 `open_workspace_directory -> WEB_DESKTOP_ONLY` 的桌面专属降级语义。
- OpenClaw 已收口到 defaults/runtime smoke：脚本会预置 `env.vars`、`env.shellEnv`、不受支持的 `tools.profile` 和遗留 `agents.defaults.timeout`，然后验证 env/tools/agents defaults 的读取、写回 live `openclaw.json`、`timeout -> timeoutSeconds` 迁移，以及 `scan_openclaw_config_health` 对 `invalid_tools_profile` / `legacy_agents_timeout` 的检测与清除。
- Hermes memory 已收口到真实文件与 YAML 级 runtime smoke：脚本会预置 `~/.hermes/memories/{MEMORY,USER}.md` 与 `config.yaml` 的 `memory:` 段，验证 `get_hermes_memory`、`set_hermes_memory`、`get_hermes_memory_limits`、`set_hermes_memory_enabled` 和 live 文件 / YAML 写回。
- Workspace / OpenClaw / Hermes 这一组现在也已收口到真实 rendered-page smoke：`tests/integration/WorkspaceOpenClawHermes.web-server.test.tsx` 会在隔离 `CC_SWITCH_TEST_HOME` 下真实渲染 `WorkspaceFilesPanel`、`DailyMemoryPanel`、`EnvPanel`、`ToolsPanel`、`AgentsDefaultsPanel` 与 `HermesMemoryPanel`，验证 Workspace / Daily Memory 的路径提示与编辑保存回显、OpenClaw Env/Tools 保存后写回 live `openclaw.json` 并 reload 回显、OpenClaw agents.defaults 的 legacy timeout 迁移与 reload，以及 Hermes memory 的远程提示、保存与启停；`tests/components/DailyMemoryPanel.test.tsx` 另外兜底 Daily Memory 搜索结果删除后的列表刷新、当前搜索刷新与删除失败 detail，`tests/components/HermesMemoryPanel.test.tsx` 另外兜底 user tab 的独立保存/启停参数。
- DeepLink Web paste flow 已收口到 runtime smoke：脚本现在直接验证 `parse_deeplink`、`merge_deeplink_config` 和 `import_from_deeplink_unified`，并在导入后回查 OpenClaw provider 列表与 live `openclaw.json`，确认不依赖 OS 协议注册也能完成 provider 导入。
- Proxy / Failover 已开始收口到 DB + runtime-semantics smoke：当前不只验证 `/api/proxy/get-proxy-status`，还验证 `get_proxy_takeover_status`、Claude app proxy config 的读取与写回、failover queue 的 empty-queue guard / available providers / add / remove、`set_auto_failover_enabled` 的持久化，以及 `get_circuit_breaker_stats -> WEB_NOT_SUPPORTED` 的 Web 占位语义。
- 这轮 Proxy / Failover smoke 同时固定了一个真实兼容问题：Rust 侧 `ProxyTakeoverStatus` 先前缺少 `hermes` 字段，已补齐以匹配前端类型与 Web handler 返回。
- Usage / Subscription 已收口到 DB + runtime-semantics smoke：当前不只验证 `/api/usage/get-usage-summary` 与 `/api/usage/get-usage-data-sources`，还验证隔离 Codex archived session log 的 `sync-session-usage` 导入、sync 后的 `usage-summary` / `usage-trends` / `request logs` / `request detail`、`request detail -> null` 的空态、`model_pricing` 的 list/upsert/delete，以及 `get_subscription_quota`、`get_balance`、`get_coding_plan_quota`、`testUsageScript` 的确定性 fallback / error 语义。
- 这轮 Usage / Subscription smoke 同时固定了一个真实兼容问题：`src-tauri/src/services/usage_stats.rs` 的 `get_request_detail()` 先前在 join `providers` 后使用了未限定的 `created_at`，现已改成显式 `l.created_at` 等限定列，避免 Web 请求详情 500。

这条扩展 runtime smoke 已经收口；当前风险已不再是 Sessions / MCP / Workspace / OpenClaw / Hermes / DeepLink / Proxy / Failover / Usage / Subscription 的基础 CRUD、写回链路或显式 Web 占位语义，而是发布前手工逐页复核、新增入口防回退，以及少量外网 happy-path 是否需要本地 mock。若需要继续做人工逐页审计，可继续按临时数据目录直接启动 Web server：

```bash
CC_SWITCH_DATA_DIR=/tmp/cc-switch-web-audit \
PORT=3011 \
cargo run --no-default-features --features web-server --example server
```

验证以下分组：

- Providers / Universal Providers
- Prompts
- Sessions
- Skills
- MCP
- Settings / Backup / Import Export
- WebDAV
- Proxy / Failover
- Usage / Subscription
- Workspace / OpenClaw / Hermes
- DeepLink paste import

### UI Acceptance

- 顶部所有入口点击后不出现 404。
- 设置页所有 tab 打开后不出现空 handler 错误。
- Providers 非空列表场景下仍可直接触发“导入当前配置”。
- Provider 卡片操作要么成功，要么显示明确 Web 不支持提示。
- Providers 自定义端点管理当前已经具备“读取已保存端点、测速并自动选最快端点、编辑态按 diff 保存新增/删除、创建态拒绝非法/重复 URL”的真实页面级证据。
- Failover 队列页需要完成“开关切换、队列新增/删除、空状态提示、桌面模式下代理未启动时禁用；Web 模式下允许编辑队列/阈值但必须明确显示 config-only 与 runtime unavailable 提示”的页面闭环。
- Usage dashboard 当前已经具备 empty-state、session import、logs/detail、pricing、app/date/refresh 过滤器和 logs/providers/models 多面板联动的真实页面级证据；图表容器 warning 与详情弹窗可访问性提示也已清理。剩余需要继续收口的主要是人工矩阵与少量长尾空态/失败态补验。
- Skills、Prompts、Sessions、MCP 面板的主要 CRUD 操作可完成。
- SQL/ZIP/Prompt 文件上传可用，SQL 导出可下载。
- 桌面专属功能不会以未捕获异常暴露给用户。

### Manual Regression Matrix

每个页面至少记录以下结果：

- Page path / panel name。
- 触发的前端 API command。
- 命中的 Web route。
- handler 类型：real、upload/download replacement、explicit unsupported、parity fallback。
- 结果：pass、blocked、desktop-only、needs implementation。
- 失败时的错误码和用户提示文本。

当前已记录结果：

| Page / Panel                                                | Frontend command                                                                                                                                    | Web route                                                                                                                                                                                                                                                 | Handler 类型                              | 结果 | 证据                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| ----------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------- | ---- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Settings / Advanced / Config Import`                       | `import_config_from_file`                                                                                                                           | `POST /api/config/import-config-upload`                                                                                                                                                                                                                   | upload/download replacement               | pass | `pnpm smoke:web-server` 的 `import-config-upload` 已证明真实 upload handler 可用；`tests/components/ImportExportSection.test.tsx` 与 `tests/hooks/useImportExport.test.tsx` 已覆盖基础 UI/hook 语义；`tests/integration/ImportExportSection.web-server.test.tsx` 进一步真实验证了渲染页面上的文件选择、multipart upload、provider snapshot 回滚、backupId 展示，以及 live `~/.claude/settings.json` 随导入恢复                                                   |
| `Settings / Advanced / Config Export`                       | `export_config_to_file`                                                                                                                             | `GET /api/config/export-config-download`                                                                                                                                                                                                                  | upload/download replacement               | pass | `pnpm smoke:web-server` 的 `export-config-download`，以及 `tests/integration/ImportExportSection.web-server.test.tsx` 已真实验证渲染后的导出按钮、浏览器 download、时间戳文件名与 SQL blob 内容                                                                                                                                                                                                                                                                  |
| `Settings / Advanced / Browser SQL Picker`                  | `open_file_dialog`                                                                                                                                  | `N/A (browser pickWebFile replacement)`                                                                                                                                                                                                                    | browser replacement                       | pass | `tests/lib/web-file-replacements.test.ts` 已覆盖 `settingsApi.openFileDialog()` 在 Web 下调用 `pickWebFile(\".sql,text/sql,application/sql\")` 且不触发 desktop `invoke`；`tests/integration/ImportExportSection.web-server.test.tsx` 已在真实页面层验证该选择结果继续进入 multipart import upload                                                                                                                                                              |
| `Settings / Advanced / Browser SQL Download Name`           | `save_file_dialog`                                                                                                                                  | `N/A (browser download filename replacement)`                                                                                                                                                                                                              | browser replacement                       | pass | `tests/lib/web-file-replacements.test.ts` 已覆盖 `settingsApi.saveFileDialog()` / `exportConfigToFile()` 在 Web 下直接走浏览器 download filename 与 `downloadBlob`，且不触发 desktop `invoke`；`tests/integration/ImportExportSection.web-server.test.tsx` 已在真实页面层验证导出按钮产生非空 SQL blob 与时间戳文件名                                                                                                                                             |
| `Settings / Advanced / Backup Manager`                      | `create_db_backup` + `list_db_backups` + `rename_db_backup` + `restore_db_backup` + `delete_db_backup`                                              | `POST /api/backups/create-db-backup` + `GET /api/backups/list-db-backups` + `POST /api/backups/rename-db-backup` + `POST /api/backups/restore-db-backup` + `DELETE /api/backups/delete-db-backup`                                                         | real                                      | pass | `pnpm smoke:web-server` 的 `backup-create` + `backups-after-create` + `backup-rename` + `backups-after-rename` + `backup-restore` + `backups-after-restore` + `backup-delete` + `backups-after-delete`，`tests/components/BackupListSection.test.tsx` 覆盖页面级按钮、确认对话框和 toast 反馈，`tests/integration/BackupListSection.web-server.test.tsx` 进一步验证真实页面在临时 `web-server` 下可完成 create -> rename -> restore -> delete                    |
| `Settings / General / Restart Required`                     | `save_settings` + `set_app_config_dir_override` + `restart_app`                                                                                      | `PUT /api/settings/save-settings` + `PUT /api/config/set-app-config-dir-override` + `POST /api/system/restart_app`                                                                                                                                       | real                                      | pass | `tests/integration/SettingsDialog.test.tsx` 已验证修改 app config dir 后触发 restart prompt、`restartLater` 关闭对话框，以及在 production 分支下 `settingsApi.restart()` 失败时展示 `settings.restartFailed` + extracted detail；`tests/components/SettingsDialog.test.tsx` 继续兜底 dev-mode 快捷提示与结构化失败文案 |
| `Settings / Advanced / Model Test Config`                   | `get_stream_check_config` + `save_stream_check_config`                                                                                              | `GET /api/config/get-stream-check-config` + `PUT /api/config/save-stream-check-config`                                                                                                                                                                    | real                                      | pass | `pnpm smoke:web-server` 的 `stream-check-config` + `stream-check-config-save` + `stream-check-config-after-save`，`tests/components/ModelTestConfigPanel.test.tsx` 已覆盖 load/save 与空数字输入回退默认值，`tests/integration/AdvancedConfigPanels.web-server.test.tsx` 进一步在真实 `web-server` 上验证了 load -> save -> reload 的页面闭环 |
| `Settings / Advanced / Log Config`                          | `get_log_config` + `set_log_config`                                                                                                                 | `GET /api/config/get-log-config` + `PUT /api/config/set-log-config`                                                                                                                                                                                       | real                                      | pass | `pnpm smoke:web-server` 的 `log-config` + `log-config-set` + `log-config-after-save`，`tests/components/LogConfigPanel.test.tsx` 已覆盖 load、enabled toggle、level 保存以及失败回滚，`tests/integration/AdvancedConfigPanels.web-server.test.tsx` 进一步在真实 `web-server` 上验证了 enabled toggle、level 选择与 reload 后回显 |
| `Settings / About / Rendered Web Mode`                      | `getCurrentVersion` + `check_for_updates` + `get_update_info` + `get_tool_versions` + `open_external`                                              | `GET /api/health` + `POST /api/system/check_for_updates` + `GET /api/system/get_update_info` + `POST /api/system/get_tool_versions` + browser `window.open`（Web replacement for `open_external`）                                                        | real + browser replacement                | pass | `tests/lib/updater.test.ts` 已验证版本徽标在 Web 下从 `/api/health` 读取，`tests/lib/updater-adapter.test.ts` 已验证 `get_update_info` 适配与失败回退，`tests/components/AboutSection.test.tsx` 已覆盖 up-to-date toast、release notes、Web 下载页打开与远程 Web 语义，`tests/integration/AboutSection.web-server.test.tsx` 进一步在真实 `web-server` 上验证当前 release notes 跳转、发现新版本后根据服务端返回的 `html_url` 打开下载页，以及 `get_tool_versions` 四张工具卡片的本地版本/最新版本徽标与 refresh 行为 |
| `Prompts / PromptPanel`                                     | `import_prompt_from_file` + `get_current_prompt_file_content`                                                                                       | `POST /api/prompts/import-prompt-upload` + `GET /api/prompts/get-current-prompt-file-content?app=claude`                                                                                                                                                  | upload/download replacement + real        | pass | `pnpm smoke:web-server` 的 `import-prompt-upload` + `prompts-claude-after-upload` 已证明真实 upload handler 可用；`tests/components/PromptPanel.test.tsx` 与 `tests/hooks/usePromptActions.test.tsx` 已覆盖页面入口、取消文件选择 no-op、失败 detail 与 current-file 缺失/异常语义；`tests/integration/PromptPanel.web-server.test.tsx` 进一步真实验证了渲染页面上的导入按钮、导入后列表刷新、enable/disable 对 live `~/.claude/CLAUDE.md` 的写入/清空、delete 后空状态恢复，以及 live `CLAUDE.md` 读取异常时的 `prompts.currentFileLoadFailed` toast detail |
| `Providers / Claude`                                        | `import_default_config`                                                                                                                             | `POST /api/config/import-default-config`                                                                                                                                                                                                                  | real                                      | pass | `pnpm smoke:web-server` 的 `import-default-claude` + `providers-claude-after-import`                                                                                                                                                                                                                                                                                                                                                                             |
| `Providers / Codex`                                         | `import_default_config`                                                                                                                             | `POST /api/config/import-default-config`                                                                                                                                                                                                                  | real                                      | pass | `pnpm smoke:web-server` 的 `import-default-codex` + `providers-codex-after-import`                                                                                                                                                                                                                                                                                                                                                                               |
| `Providers / Gemini`                                        | `import_default_config`                                                                                                                             | `POST /api/config/import-default-config`                                                                                                                                                                                                                  | real                                      | pass | `pnpm smoke:web-server` 的 `import-default-gemini` + `providers-gemini-after-import`                                                                                                                                                                                                                                                                                                                                                                             |
| `Providers / Endpoint Speed Test`                           | `test_api_endpoints` + `get_custom_endpoints` + `add_custom_endpoint` + `remove_custom_endpoint`                                                   | `POST /api/system/test_api_endpoints` + `GET /api/system/get_custom_endpoints` + `POST /api/system/add_custom_endpoint` + `DELETE /api/system/remove_custom_endpoint`                                                                                     | real                                      | pass | `tests/components/EndpointSpeedTest.test.tsx` 已覆盖超时参数、创建态 URL 校验、自动择优与编辑态 diff；`tests/integration/EndpointSpeedTest.web-server.test.tsx` 已进一步在真实 `web-server` 上验证最快健康端点自动选中、服务端失败态可见，以及编辑态自定义端点 add/remove diff 经真实 Web API 持久化 |
| `Providers / Claude Current Writeback`                      | `update_provider`                                                                                                                                   | `PUT /api/providers/update-provider`                                                                                                                                                                                                                      | real                                      | pass | `pnpm smoke:web-server` 的 `update-claude-current-provider-writes-live`，并直接校验 `~/.claude/settings.json` 被更新                                                                                                                                                                                                                                                                                                                                             |
| `Providers / Claude Switch`                                 | `add_provider` + `switch_provider`                                                                                                                  | `POST /api/providers/add-provider` + `POST /api/providers/switch-provider`                                                                                                                                                                                | real                                      | pass | `pnpm smoke:web-server` 的 `switch-claude-provider-writes-live`，并直接校验 current provider 与 `~/.claude/settings.json` 一起更新                                                                                                                                                                                                                                                                                                                               |
| `Providers / OpenCode`                                      | `import_opencode_providers_from_live`                                                                                                               | `POST /api/providers/import-opencode-providers-from-live`                                                                                                                                                                                                 | real                                      | pass | `pnpm smoke:web-server` 的 `import-opencode-from-live` + `providers-opencode-after-import`                                                                                                                                                                                                                                                                                                                                                                       |
| `Providers / OpenCode Live Writeback`                       | `update_provider`                                                                                                                                   | `PUT /api/providers/update-provider`                                                                                                                                                                                                                      | real                                      | pass | `pnpm smoke:web-server` 的 `update-opencode-live-managed-provider-writes-live`，并直接校验 `~/.config/opencode/opencode.json` 被更新                                                                                                                                                                                                                                                                                                                             |
| `Providers / Rendered Claude Import + Edit + Switch`        | `import_default_config` + `update_provider` + `add_provider` + `switch_provider`                                                                   | `POST /api/config/import-default-config` + `PUT /api/providers/update-provider` + `POST /api/providers/add-provider` + `POST /api/providers/switch-provider`                                                                                             | real                                      | pass | `tests/integration/ProviderList.web-server.test.tsx` 已在真实 `web-server` 上验证空态导入、编辑后卡片标题更新、`~/.claude/settings.json.ui.displayName` 与 `env` 同步写回，以及新增第二个 provider 后的切换写回；当前仍保留一条实现细节：初始 live import 显示的卡片标题仍为 `default`，但这已不阻塞页面闭环 |
| `Providers / Rendered OpenCode Import + Edit`               | `import_opencode_providers_from_live` + `update_provider`                                                                                           | `POST /api/providers/import-opencode-providers-from-live` + `PUT /api/providers/update-provider`                                                                                                                                                         | real                                      | pass | `tests/integration/ProviderList.web-server.test.tsx` 已在真实 `web-server` 上验证空态导入、编辑后卡片标题更新，以及 `provider[page-opencode].name` 与 `options.baseURL` 同步写回 live `opencode.json` |
| `Providers / Rendered OpenClaw Import`                      | `import_openclaw_providers_from_live`                                                                                                               | `POST /api/openclaw/import-openclaw-providers-from-live`                                                                                                                                                                                                  | real                                      | pass | `tests/integration/ProviderList.web-server.test.tsx` 已在真实 `web-server` 上验证空态导入、基于 live `openclaw.json` 渲染卡片，以及 `Live Config` 状态在页面可见；对应 provider 也会落库到 `providersApi.getAll(\"openclaw\")` |
| `Providers / Rendered Hermes Import`                        | `import_hermes_providers_from_live`                                                                                                                 | `POST /api/hermes/import-hermes-providers-from-live`                                                                                                                                                                                                      | real                                      | pass | `tests/integration/ProviderList.web-server.test.tsx` 已在真实 `web-server` 上验证空态导入、基于 live `config.yaml` 渲染卡片、`Live Config` 状态可见，以及当前 provider 卡片高亮；对应 provider 也会落库到 `providersApi.getAll(\"hermes\")` |
| `Universal / Rendered Panel`                                | `get_universal_providers` + `upsert_universal_provider` + `sync_universal_provider` + `delete_universal_provider`                                 | `GET /api/providers/get-universal-providers` + `POST /api/providers/upsert-universal-provider` + `POST /api/providers/sync-universal-provider` + `DELETE /api/providers/delete-universal-provider`                                                       | real                                      | pass | `tests/integration/UniversalProviderPanel.web-server.test.tsx` 已在真实 `web-server` 上验证 create 后自动 sync、edit(save only) 不自动改动子 provider、manual sync 将变更写回 Claude/Codex/Gemini、以及 delete 后联动清理生成的子 provider |
| `Providers / OpenClaw`                                      | `import_openclaw_providers_from_live`                                                                                                               | `POST /api/openclaw/import-openclaw-providers-from-live`                                                                                                                                                                                                  | real                                      | pass | `pnpm smoke:web-server` 的 `import-openclaw-from-live` + `providers-openclaw-after-import`；`tests/integration/ProviderList.web-server.test.tsx` 也已把空态导入、live `openclaw.json` 渲染卡片与 `Live Config` 状态补到真实页面层 |
| `Providers / Hermes`                                        | `import_hermes_providers_from_live`                                                                                                                 | `POST /api/hermes/import-hermes-providers-from-live`                                                                                                                                                                                                      | real                                      | pass | `pnpm smoke:web-server` 的 `import-hermes-from-live` + `providers-hermes-after-import`；`tests/integration/ProviderList.web-server.test.tsx` 也已把空态导入、live `config.yaml` 渲染卡片、`Live Config` 状态与当前 provider 高亮补到真实页面层 |
| `Sessions / List`                                           | `list_sessions`                                                                                                                                     | `GET /api/sessions/list-sessions`                                                                                                                                                                                                                         | real                                      | pass | `pnpm smoke:web-server` 的 `sessions-list`                                                                                                                                                                                                                                                                                                                                                                                                                       |
| `Sessions / Messages`                                       | `get_session_messages`                                                                                                                              | `GET /api/sessions/get-session-messages`                                                                                                                                                                                                                  | real                                      | pass | `pnpm smoke:web-server` 的 `session-messages`                                                                                                                                                                                                                                                                                                                                                                                                                    |
| `Sessions / Single Delete`                                  | `delete_session`                                                                                                                                    | `DELETE /api/sessions/delete-session`                                                                                                                                                                                                                     | real                                      | pass | `pnpm smoke:web-server` 的 `delete-session`                                                                                                                                                                                                                                                                                                                                                                                                                      |
| `Sessions / Batch Delete`                                   | `delete_sessions`                                                                                                                                   | `DELETE /api/sessions/delete-sessions`                                                                                                                                                                                                                    | real                                      | pass | `pnpm smoke:web-server` 的 `delete-sessions-batch`                                                                                                                                                                                                                                                                                                                                                                                                               |
| `Sessions / Resume Terminal`                                | `launch_session_terminal`                                                                                                                           | `POST /api/sessions/launch-session-terminal`                                                                                                                                                                                                              | desktop-only                              | pass | `pnpm smoke:web-server` 的 `session-launch-terminal-desktop-only`，返回 `WEB_DESKTOP_ONLY`                                                                                                                                                                                                                                                                                                                                                                       |
| `Sessions / Page Flow`                                      | `list_sessions` + `delete_sessions` + Web resume copy                                                                                               | `GET /api/sessions/list-sessions` + `DELETE /api/sessions/delete-sessions` + `N/A (browser clipboard replacement)`                                                                                                                                        | real + browser replacement                | pass | `tests/integration/SessionManagerPage.web-server.test.tsx` 已验证排序后的列表渲染、详情切换、Web 模式下复制 resume 命令而不是开终端、批量删除，以及底层 Codex session JSONL 文件被真实清理                                                                                                                                                                                                                                                                       |
| `Providers / Open Terminal Action`                          | `open_provider_terminal`                                                                                                                            | `POST /api/providers/open-provider-terminal`                                                                                                                                                                                                              | explicit unsupported + desktop-only route | pass | `tests/components/ProviderList.test.tsx` 已覆盖 Web 下不再向 ProviderCard 传递终端入口，`tests/lib/adapter.test.ts` 已覆盖 adapter 直接抛 `WebNotSupportedError`，`pnpm smoke:web-server` 的 `desktop-only-open-provider-terminal` 已覆盖直连 API 返回 `WEB_DESKTOP_ONLY`                                                                                                                                                                                        |
| `Workspace / AGENTS.md`                                     | `read_workspace_file` + `write_workspace_file`                                                                                                      | `GET /api/workspace/read-workspace-file` + `PUT /api/workspace/write-workspace-file`                                                                                                                                                                      | real                                      | pass | `pnpm smoke:web-server` 的 `workspace-read-file` + `workspace-write-file`，并直接校验 `~/.openclaw/workspace/AGENTS.md` 被写回                                                                                                                                                                                                                                                                                                                                   |
| `Workspace / Open Directory`                                | `open_workspace_directory`                                                                                                                          | `POST /api/workspace/open-workspace-directory`                                                                                                                                                                                                            | desktop-only                              | pass | `pnpm smoke:web-server` 的 `workspace-open-directory-desktop-only`，返回 `WEB_DESKTOP_ONLY`；UI 级降级由 `tests/components/WorkspacePanels.test.tsx` 覆盖                                                                                                                                                                                                                                                                                                        |
| `Settings / Advanced / Directory Browse`                    | `pick_directory`                                                                                                                                    | `POST /api/system/pick_directory`                                                                                                                                                                                                                         | explicit unsupported + desktop-only route | pass | `tests/components/DirectorySettings.test.tsx` 已覆盖 Web 下目录浏览按钮禁用，`tests/lib/adapter.test.ts` 已覆盖 adapter 直接抛 `WebNotSupportedError`，`pnpm smoke:web-server` 的 `desktop-only-pick-directory` 已覆盖直连 API 返回 `WEB_DESKTOP_ONLY`；`tests/integration/DirectorySettings.web-server.test.tsx` 又补上真实页面级证据，验证 browse disabled、手动路径输入、保存后 restart prompt 与 reload 回显 |
| `Skills / ZIP Install`                                      | `install_skills_from_zip`                                                                                                                           | `POST /api/skills/install-skills-upload`                                                                                                                                                                                                                  | upload/download replacement               | pass | `tests/lib/web-file-replacements.test.ts` 已覆盖 `skillsApi.openZipFileDialog/installFromZip` 的 Web 分支，`tests/components/UnifiedSkillsPanel.test.tsx` 已覆盖 `openInstallFromZip` 页面入口，`tests/integration/UnifiedSkillsPanel.web-server.test.tsx` 进一步真实验证了渲染页面上的文件选择、multipart upload、installed skill 列表可见，以及 `dataDir/skills/page-smoke-zip-skill/SKILL.md` 与 live `~/.claude/skills/page-smoke-zip-skill/SKILL.md` 的写回 |
| `Skills / Browser ZIP Picker`                               | `open_zip_file_dialog`                                                                                                                              | `N/A (browser pickWebFile replacement)`                                                                                                                                                                                                                    | browser replacement                       | pass | `tests/lib/web-file-replacements.test.ts` 已覆盖 `skillsApi.openZipFileDialog()` 在 Web 下调用 `pickWebFile(\".zip,.skill,application/zip\")` 且不触发 desktop `invoke`；`tests/components/UnifiedSkillsPanel.test.tsx` 与 `tests/integration/UnifiedSkillsPanel.web-server.test.tsx` 已覆盖该入口从页面按钮进入 ZIP upload handler                                                                                                                                 |
| `Skills / Discovery Repo Install + Linkage`                 | `get_skill_repos` + `add_skill_repo` + `discover_available_skills` + `install_skill_unified` + `uninstall_skill_unified`                            | `GET /api/skills/get-skill-repos` + `POST /api/skills/add-skill-repo` + `POST /api/skills/discover-available-skills` + `POST /api/skills/install-skill-unified` + `POST /api/skills/uninstall-skill-unified`                                              | real                                      | pass | `tests/integration/SkillsPage.web-server.test.tsx` 已在真实 `web-server` 上验证 `openRepoManager -> add repo -> discover -> install`，并确认安装后 `UnifiedSkillsPanel` 已安装行可见、卸载后 `SkillsPage` 卡片回退到 install；2026-05-05 暴露的 `searchSource` 回归也已补上自动/手动 source 切换区分，repo 重新出现后会恢复 repo 结果，而显式选中的 `skills.sh` 仍保持不变 |
| `Skills / Discovery Empty Repo Fallback`                    | `get_skill_repos` + `discover_available_skills`                                                                                                     | `GET /api/skills/get-skill-repos` + `POST /api/skills/discover-available-skills`                                                                                                                                                                          | real + web UI state                       | pass | `tests/integration/SkillsPage.web-server.test.tsx` 已在真实 `web-server` 初始空数据目录下验证 `get_skill_repos` 返回空数组时，`SkillsPage` 自动 fallback 到 `skills.sh` idle placeholder 且不会发起无意义 skills.sh 搜索；用户手动切回 `repos` 后显示 `skills.empty` / `skills.emptyDescription` 和 `skills.addRepo` 入口 |
| `Skills / Discovery skills.sh Search + Retry`               | `search_skills_sh`                                                                                                                                  | `POST /api/skills/search-skills-sh`                                                                                                                                                                                                                       | real                                      | pass | `tests/integration/SkillsPage.web-server.test.tsx` 已在真实 `web-server` 上验证 skills.sh 搜索分页、安装结果、空结果 `skills.skillssh.noResults`、trim 后查询、同一标准化 query 的重复搜索会重新请求，以及本地 skills.sh 假服务返回 503 时页面展示 `skills.skillssh.error`、可见错误 detail，并通过错误卡片内 `common.refresh` 对同一 query 重试后恢复搜索结果 |
| `Skills / Update Check + Update`                            | `check_skill_updates` + `update_skill`                                                                                                              | `GET /api/skills/check-skill-updates` + `PUT /api/skills/update-skill`                                                                                                                                                                                    | real                                      | pass | `tests/integration/SkillsPage.web-server.test.tsx` 已通过本地 repo archive mirror 真实验证 `check updates -> update` 页面闭环，确认 `skills.updateAvailable` badge 出现后可执行更新，并直接校验 `InstalledSkill.description`、SSOT `dataDir/skills/repo-smoke-skill/SKILL.md` 与 live `~/.claude/skills/repo-smoke-skill/SKILL.md` 从 v1 切到 v2 |
| `Skills / Unified Panel`                                    | `scan_unmanaged_skills` + `import_skills_from_apps` + `toggle_skill_app` + `uninstall_skill_unified` + `get_skill_backups` + `restore_skill_backup` | `GET /api/skills/scan-unmanaged-skills` + `POST /api/skills/import-skills-from-apps` + `POST /api/skills/toggle-skill-app` + `POST /api/skills/uninstall-skill-unified` + `GET /api/backups/get-skill-backups` + `POST /api/backups/restore-skill-backup` | real                                      | pass | `tests/integration/UnifiedSkillsPanel.web-server.test.tsx` 已验证空态导入、import from apps、Codex app toggle、卸载备份、恢复，以及隔离 `dataDir/skills` SSOT 与 live `~/.claude/skills` / `~/.codex/skills` 的真实文件副作用                                                                                                                                                                                                                                    |
| `Daily Memory / List + Read`                                | `list_daily_memory_files` + `read_daily_memory_file`                                                                                                | `POST /api/system/list_daily_memory_files` + `POST /api/system/read_daily_memory_file`                                                                                                                                                                    | real                                      | pass | `pnpm smoke:web-server` 的 `daily-memory-list` + `daily-memory-read`                                                                                                                                                                                                                                                                                                                                                                                             |
| `Daily Memory / Save + Search + Delete`                     | `write_daily_memory_file` + `search_daily_memory_files` + `delete_daily_memory_file`                                                                | `POST /api/system/write_daily_memory_file` + `POST /api/system/search_daily_memory_files` + `POST /api/system/delete_daily_memory_file`                                                                                                                   | real                                      | pass | `pnpm smoke:web-server` 的 `daily-memory-write` + `daily-memory-search` + `daily-memory-delete`，并直接校验 `~/.openclaw/workspace/memory/2026-03-04.md` 的写回与删除；`tests/components/DailyMemoryPanel.test.tsx` 已覆盖搜索结果删除后 list reload / active search refresh 与 delete failure detail                                                                                                                                                                                                                               |
| `OpenClaw / Env`                                            | `get_openclaw_env` + `set_openclaw_env`                                                                                                             | `GET /api/env/get-openclaw-env` + `PUT /api/env/set-openclaw-env`                                                                                                                                                                                         | real                                      | pass | `pnpm smoke:web-server` 的 `openclaw-get-env` + `openclaw-set-env`，并直接校验 live `openclaw.json`；`tests/integration/WorkspaceOpenClawHermes.web-server.test.tsx` 已在真实页面层验证 Env JSON 编辑保存、live `openclaw.json` 写回与 reload 回显                                                                                                                                                                                                                  |
| `OpenClaw / Tools + Health`                                 | `get_openclaw_tools` + `set_openclaw_tools` + `scan_openclaw_config_health`                                                                         | `GET /api/openclaw/get-openclaw-tools` + `PUT /api/openclaw/set-openclaw-tools` + `GET /api/config/scan-openclaw-config-health`                                                                                                                           | real                                      | pass | `pnpm smoke:web-server` 的 `openclaw-get-tools` + `openclaw-scan-health-initial` + `openclaw-set-tools`；已验证 `invalid_tools_profile` 检测与 live 写回；`tests/integration/WorkspaceOpenClawHermes.web-server.test.tsx` 已在真实页面层验证 Tools allow/deny 列表编辑保存、live `openclaw.json` 写回与 reload 回显                                                                                                                                                 |
| `OpenClaw / Agents Defaults`                                | `get_openclaw_agents_defaults` + `set_openclaw_agents_defaults`                                                                                     | `GET /api/openclaw/get-openclaw-agents-defaults` + `PUT /api/openclaw/set-openclaw-agents-defaults`                                                                                                                                                       | real                                      | pass | `pnpm smoke:web-server` 的 `openclaw-get-agents-defaults` + `openclaw-set-agents-defaults`；已验证 `timeout -> timeoutSeconds` 迁移、warning 清除与 live `openclaw.json` 写回                                                                                                                                                                                                                                                                                    |
| `Proxy / Takeover Status`                                   | `get_proxy_takeover_status`                                                                                                                         | `GET /api/proxy/get-proxy-takeover-status`                                                                                                                                                                                                                | real                                      | pass | `pnpm smoke:web-server` 的 `proxy-takeover-status`；这一轮已顺带修正 Rust `ProxyTakeoverStatus` 缺少 `hermes` 字段的问题                                                                                                                                                                                                                                                                                                                                         |
| `Proxy / Claude App Config`                                 | `get_proxy_config_for_app` + `update_proxy_config_for_app`                                                                                          | `GET /api/config/get-proxy-config-for-app?appType=claude` + `PUT /api/config/update-proxy-config-for-app`                                                                                                                                                 | real                                      | pass | `pnpm smoke:web-server` 的 `proxy-config-claude-initial` + `proxy-update-config-for-claude`，已验证重试/超时/熔断参数持久化                                                                                                                                                                                                                                                                                                                                      |
| `Failover / Empty Queue Guard`                              | `set_auto_failover_enabled`                                                                                                                         | `PUT /api/failover/set-auto-failover-enabled`                                                                                                                                                                                                             | real                                      | pass | `pnpm smoke:web-server` 的 `failover-enable-codex-without-queue-blocked`，当队列为空时返回 `BAD_REQUEST`                                                                                                                                                                                                                                                                                                                                                         |
| `Failover / Claude Queue CRUD`                              | `get_failover_queue` + `get_available_providers_for_failover` + `add_to_failover_queue` + `remove_from_failover_queue`                              | `GET /api/failover/get-failover-queue` + `GET /api/failover/get-available-providers-for-failover` + `POST /api/failover/add-to-failover-queue` + `DELETE /api/failover/remove-from-failover-queue`                                                        | real                                      | pass | `pnpm smoke:web-server` 的 `failover-queue-claude-initial` + `failover-available-providers-claude` + `failover-add-claude-provider` + `failover-disable-and-remove-claude`                                                                                                                                                                                                                                                                                       |
| `Settings / ProxyTabContent Rendered Web Mode`             | `get_proxy_takeover_status` + `get_proxy_config_for_app` + `get_failover_queue` + `get_available_providers_for_failover` + `add_to_failover_queue` + `set_auto_failover_enabled` | `GET /api/proxy/get-proxy-takeover-status` + `GET /api/config/get-proxy-config-for-app?appType=claude` + `GET /api/failover/get-failover-queue` + `GET /api/failover/get-available-providers-for-failover` + `POST /api/failover/add-to-failover-queue` + `PUT /api/failover/set-auto-failover-enabled` | real + web UI degradation                 | pass | `tests/components/ProxyTabContent.test.tsx` 已证明 Web 下 runtime-control disabled、failover config-only 与 runtime-stats placeholder 文案；`tests/integration/ProxyTabContent.web-server.test.tsx` 已把这一页接到真实 `web-server`，并实际跑通 failover provider `Select`、queue add/remove、auto switch toggle、app config save 与运行时降级提示，说明这块剩余问题已从交互稳定性收口到已通过的页面级证据 |
| `Failover / Auto Switch + Runtime Stats Placeholder`        | `set_auto_failover_enabled` + `get_circuit_breaker_stats`                                                                                           | `PUT /api/failover/set-auto-failover-enabled` + `POST /api/system/get_circuit_breaker_stats`                                                                                                                                                              | real + explicit unsupported               | pass | `pnpm smoke:web-server` 的 `failover-enable-claude` + `failover-runtime-stats-web-not-supported`，已验证 toggle 持久化与 `WEB_NOT_SUPPORTED` 占位语义                                                                                                                                                                                                                                                                                                            |
| `Usage / Session Sync + Data Sources`                       | `sync_session_usage` + `get_usage_data_sources`                                                                                                     | `POST /api/sessions/sync-session-usage` + `GET /api/usage/get-usage-data-sources`                                                                                                                                                                         | real                                      | pass | `pnpm smoke:web-server` 的 `sync-session-usage` + `usage-data-sources-after-session-sync`；已验证隔离 Codex archived session usage 导入和 `codex_session` 数据源统计                                                                                                                                                                                                                                                                                             |
| `Usage / Summary + Trends`                                  | `get_usage_summary` + `get_usage_trends`                                                                                                            | `GET /api/usage/get-usage-summary?appType=codex` + `GET /api/usage/get-usage-trends?...`                                                                                                                                                                  | real                                      | pass | `pnpm smoke:web-server` 的 `usage-summary-after-session-sync` + `usage-trends-after-session-sync`；已验证 sync 后 tokens/cost 汇总与时间窗趋势                                                                                                                                                                                                                                                                                                                   |
| `Usage / Logs + Detail`                                     | `get_request_logs` + `get_request_detail`                                                                                                           | `POST /api/system/get_request_logs` + `POST /api/system/get_request_detail`                                                                                                                                                                               | real                                      | pass | `pnpm smoke:web-server` 的 `request-logs-after-session-sync` + `request-detail-after-session-sync` + `request-detail-not-found`；这一轮已顺带修正 `get_request_detail()` 未限定 `created_at` 导致的 Web 500                                                                                                                                                                                                                                                      |
| `Settings / Usage / Rendered Dashboard`                     | `sync_session_usage` + `get_usage_data_sources` + `get_request_logs` + `get_request_detail` + `get_usage_summary` + `get_usage_trends` + `get_provider_stats` + `get_model_stats` | `POST /api/sessions/sync-session-usage` + `GET /api/usage/get-usage-data-sources` + `POST /api/system/get_request_logs` + `POST /api/system/get_request_detail` + `GET /api/usage/get-usage-summary` + `GET /api/usage/get-usage-trends` + `GET /api/providers/get-provider-stats` + `POST /api/system/get_model_stats` | real | pass | `tests/integration/UsageDashboard.web-server.test.tsx` 已在真实临时 `web-server` 上验证 empty-state 保留 `Import Sessions`、Codex archived session usage 导入、`codex_session` data source 可见、request logs / detail 回显，以及 app filter / refresh interval / date range 与 logs/providers/models 多面板联动；`tests/components/UsageDashboard.test.tsx` 与 `tests/components/DataSourceBar.test.tsx` 同时兜底当前 tab 保持、空来源态、up-to-date 与 sync failure detail |
| `Usage / Model Pricing`                                     | `get_model_pricing` + `update_model_pricing` + `delete_model_pricing`                                                                               | `POST /api/system/get_model_pricing` + `POST /api/system/update_model_pricing` + `POST /api/system/delete_model_pricing`                                                                                                                                  | real                                      | pass | `pnpm smoke:web-server` 的 `model-pricing-list` + `model-pricing-upsert` + `model-pricing-delete`                                                                                                                                                                                                                                                                                                                                                                |
| `Subscription / Quota Fallback`                             | `get_subscription_quota`                                                                                                                            | `GET /api/subscription/get-subscription-quota?tool=claude` + `GET /api/subscription/get-subscription-quota?tool=gemini`                                                                                                                                 | real                                      | pass | `pnpm smoke:web-server` 的 `subscription-quota-claude-not-found` + `subscription-quota-gemini-parse-error`；已验证 no-credential / parse-error 两条确定性语义 |
| `Subscription / Balance + Coding Plan Deterministic States` | `get_balance` + `get_coding_plan_quota` + `testUsageScript`                                                                                         | `GET /api/usage/get-balance` + `GET /api/usage/get-coding-plan-quota` + `POST /api/usage/testusagescript`                                                                                                                                                 | real                                      | pass | `pnpm smoke:web-server` 的 `balance-unknown-provider` + `coding-plan-unknown-provider` + `usage-script-invalid-app`；已验证无外网条件下的结构化错误语义                                                                                                                                                                                                                                                                                                          |
| `MCP / Unified Import`                                      | `import_mcp_from_apps`                                                                                                                              | `POST /api/mcp/import-mcp-from-apps`                                                                                                                                                                                                                      | real                                      | pass | `pnpm smoke:web-server` 的 `import-mcp-from-apps` + `mcp-servers-after-import`；`src-tauri/tests/mcp_commands.rs` 已覆盖所有来源失败时返回结构化 detail，以及单来源失败但其它来源成功时保留成功导入 count                                                                                                                                                                                                                                                                                                                                                                                   |
| `MCP / Unified Panel`                                       | `import_mcp_from_apps` + `toggle_mcp_app` + `delete_mcp_server`                                                                                     | `POST /api/mcp/import-mcp-from-apps` + `POST /api/mcp/toggle-mcp-app` + `DELETE /api/mcp/delete-mcp-server`                                                                                                                                               | real                                      | pass | `tests/integration/UnifiedMcpPanel.web-server.test.tsx` 已验证空态导入、列表渲染、Codex app toggle、删除确认，以及 live `~/.codex/config.toml` / `~/.claude.json` 写回与清理                                                                                                                                                                                                                                                                                     |
| `MCP / Validate Command`                                    | `validate_mcp_command`                                                                                                                              | `POST /api/mcp/validate-mcp-command`                                                                                                                                                                                                                      | real                                      | pass | `pnpm smoke:web-server` 的 `validate-mcp-command`；已回归 `POST` JSON body 解析                                                                                                                                                                                                                                                                                                                                                                                  |
| `MCP / Legacy Claude Read`                                  | `read_claude_mcp_config` + `get_mcp_config`                                                                                                         | `GET /api/config/read-claude-mcp-config` + `GET /api/config/get-mcp-config?app=claude`                                                                                                                                                                    | real                                      | pass | `pnpm smoke:web-server` 的 `read-claude-mcp-config` + `get-mcp-config-claude`；已回归 `configPath` camelCase 投影                                                                                                                                                                                                                                                                                                                                                |
| `MCP / App Toggle`                                          | `toggle_mcp_app` + `set_mcp_enabled`                                                                                                                | `POST /api/mcp/toggle-mcp-app` + `PUT /api/mcp/set-mcp-enabled`                                                                                                                                                                                           | real                                      | pass | `pnpm smoke:web-server` 的 `toggle-mcp-app-disable-codex-live` + `set-mcp-enabled-restore-codex-live`                                                                                                                                                                                                                                                                                                                                                            |
| `MCP / Unified Upsert/Delete`                               | `upsert_mcp_server` + `delete_mcp_server`                                                                                                           | `POST /api/mcp/upsert-mcp-server` + `DELETE /api/mcp/delete-mcp-server`                                                                                                                                                                                   | real                                      | pass | `pnpm smoke:web-server` 的 `upsert-mcp-server-writes-live` + `delete-mcp-server-removes-live`                                                                                                                                                                                                                                                                                                                                                                    |
| `MCP / Legacy Claude Upsert/Delete`                         | `upsert_claude_mcp_server` + `delete_claude_mcp_server`                                                                                             | `POST /api/mcp/upsert-claude-mcp-server` + `DELETE /api/mcp/delete-claude-mcp-server`                                                                                                                                                                     | real                                      | pass | `pnpm smoke:web-server` 的 `upsert-claude-mcp-server-live` + `delete-claude-mcp-server-live`                                                                                                                                                                                                                                                                                                                                                                     |
| `MCP / Legacy Config Route`                                 | `upsert_mcp_server_in_config` + `delete_mcp_server_in_config`                                                                                       | `POST /api/config/upsert-mcp-server-in-config` + `DELETE /api/config/delete-mcp-server-in-config`                                                                                                                                                         | real                                      | pass | `pnpm smoke:web-server` 的 `upsert-mcp-server-in-config-live` + `delete-mcp-server-in-config-live`                                                                                                                                                                                                                                                                                                                                                               |
| `Hermes / Memory Panel`                                     | `get_hermes_memory` + `get_hermes_memory_limits` + `set_hermes_memory` + `set_hermes_memory_enabled` + `open_hermes_web_ui`                         | `GET /api/hermes/get-hermes-memory` + `GET /api/hermes/get-hermes-memory-limits` + `PUT /api/hermes/set-hermes-memory` + `PUT /api/hermes/set-hermes-memory-enabled` + `POST /api/hermes/open-hermes-web-ui`                                              | real + web UI degradation                 | pass | `pnpm smoke:web-server` 的 `hermes-memory-limits` + `hermes-memory-read` + `hermes-memory-write` + `hermes-memory-disable`，`tests/components/HermesMemoryPanel.test.tsx` + `tests/hooks/useHermes.test.tsx` 覆盖 Web 提示与禁用按钮语义，`tests/integration/WorkspaceOpenClawHermes.web-server.test.tsx` 进一步覆盖真实页面保存 / 启停 / reload / remote-hint 链路                                                                                                                                                                                      |
| `DeepLink / Paste Import`                                   | `parse_deeplink` + `merge_deeplink_config` + `import_from_deeplink_unified`                                                                         | `POST /api/deeplink/parse-deeplink` + `POST /api/config/merge-deeplink-config` + `POST /api/deeplink/import-from-deeplink-unified`                                                                                                                        | real                                      | pass | `pnpm smoke:web-server` 的 `deeplink-parse-provider` + `deeplink-merge-provider-config` + `deeplink-import-provider-unified` + `providers-openclaw-after-deeplink-import`，`tests/components/DeepLinkImportDialog.test.tsx` 覆盖 Web paste flow，`tests/integration/DeepLinkImportDialog.web-server.test.tsx` 进一步验证真实页面在临时 `web-server` 下可完成 parse/import 并写入 OpenClaw providers                                                              |
| `Agents / Placeholder`                                      | `N/A`                                                                                                                                               | `N/A`                                                                                                                                                                                                                                                     | explicit placeholder                      | pass | `tests/components/AgentsPanel.test.tsx`，已覆盖多语言占位文案与“无操作按钮”语义                                                                                                                                                                                                                                                                                                                                                                                  |
| `System / Config Folders`                                   | `open_app_config_folder` + `open_config_folder`                                                                                                     | `POST /api/config/open-app-config-folder` + `POST /api/config/open-config-folder`                                                                                                                                                                         | explicit unsupported + desktop-only route | pass | `tests/lib/adapter.test.ts` 已覆盖两条命令在 Web adapter 中直接抛 `WebNotSupportedError`，`pnpm smoke:web-server` 的 `desktop-only-open-app-config-folder` + `desktop-only-open-config-folder` 已覆盖直连 API 返回 `WEB_DESKTOP_ONLY`                                                                                                                                                                                                                            |

## Assumptions

- 本阶段按“先功能后安全”推进。
- 不先实现完整登录、真实 CSRF、session 和 rate limit。
- 即使安全延后，server 默认仍只监听 `127.0.0.1`。
- 远程 Web 使用以浏览器上传/下载为主，不依赖服务端 GUI 文件选择器。
- 优先复用已有 Tauri commands/services，避免重复实现业务逻辑。

## Known Risks

- 目前 `auth`、`csrf`、`rate_limit` middleware 是 permissive stub，远程暴露前必须补齐安全。
- 文件上传/下载会改变部分前端交互，需要同时修改 UI 和 API adapter。
- 一次性扫平所有可见页面范围较大，建议按页面分批提交，避免长分支难以验证。
- 有些桌面命令在 Web 中只能降级为提示，不能实现完全一致体验。
