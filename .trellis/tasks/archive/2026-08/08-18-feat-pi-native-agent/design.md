# Design — feat-pi-native-agent（pi native coding agent + session usage）

## 架构与边界

本子任务在 Web-first fork 上移植 Product upstream 两个提交：
- `84e75ad2` feat(pi): add native coding agent support (#6064) — 156 文件
- `40d747c0` feat(pi): add session usage statistics (#6463) — 17 文件（依赖前者）

### Pi 运行模型（根本事实）

Pi 是**本地配置文件管理的 additive provider**（类比 OpenCode/Hermes），不是进程式 agent、不是 proxy provider。

- CC Switch 读写 `~/.pi/agent/{models.json, settings.json, AGENTS.md, SYSTEM.md, APPEND_SYSTEM.md, prompts/*.md, skills/}` + 会话 JSONL。
- 原子写入 + revision 比较防跨进程覆盖；`pi_config::MAX_PI_FILE_BYTES = 1 MiB`。
- additive 模式：启用 = 在 `models.json.providers` 增加节点；不写 `defaultProvider`/`defaultModel`（Pi 自管）。
- 契约明确不做：`/login`、`auth.json`、路由/网关/代理/故障转移、内置供应商/模型目录复制、完整 compat/modelOverrides、相对会话目录全局猜测。

### Web-first fork 兼容性

- fork Web 模式已支持同类本地文件读写（session_manager 全系 `fs::read`）。
- pi 新 Tauri 命令（`get_pi_current_state`/`update_pi_provider_usage_script`/`get_pi_session_discovery` + 6 个 prompt 命令）仅本地文件 IO，无进程 spawn。
- `terminal/mod.rs` 改动仅 1 行：`shell_escape` 由 `fn` 改 `pub(crate)`（pi session_manager 复用 shell_escape，不改 launch 行为）。
- proxy 层：`providers/mod.rs` 仅加 `AppType::Pi => return None`（pi 不参与 proxy provider 解析）；forwarder/handlers 无 pi 改动。
- → **Pi 对 Web 完全兼容，无 desktop-only 阻塞**。无新 desktop-only 命令需 `web_desktop_only` 501 stub。

### 移植方法

selective port（逐文件移植 + Web 适配），不直接 cherry-pick 单个 156 文件大提交。原因：上游 84e75ad2 建立在仍含 `AppType::ClaudeDesktop` 的基线，diff 有 56 处 ClaudeDesktop 引用横跨 20 文件，fork 已移除 ClaudeDesktop → 整体 cherry-pick 会产生大量 ClaudeDesktop 冲突 hunk。沿用父任务 S1–S8 的 selective port 方法论：取上游最终态，逐文件落到 fork，丢弃 ClaudeDesktop/zh-TW hunk。

## 数据流与契约

### Provider 管理（84e75ad2）

```
前端 PiProviderForm → lib/api/pi.ts → POST /api/providers/* (app="pi")
  → commands/pi.rs → services/provider/pi.rs → pi_config/mod.rs
  → 原子读写 ~/.pi/agent/models.json + DB save (app="pi")
  → revision 比较 + MODELS_FILE_LOCK 串行化
```

- `services/provider/mod.rs` dispatch 加 `AppType::Pi` arm → `pi::{list,add,update,delete,remove,enable}`。
- `app_config.rs` `AppType` enum 加 `Pi`，`as_str()="pi"`，`is_additive_mode()` 含 Pi，`all()` 含 Pi，`FromStr` 接受 "pi"。
- provider credentials 解析：`AppType::Pi` 走 native baseUrl/apiKey（同 OpenClaw/Hermes），`pi_config::provider_base_url`。

### Prompt 管理（84e75ad2）

```
前端 PiPromptPanel → lib/api/prompts.ts (app="pi" dispatch) → commands/prompt.rs
  → services/pi_prompt_files.rs → 原子读写 ~/.pi/agent/{AGENTS.md, SYSTEM.md, prompts/*.md}
```

- pi prompt 命令在 `commands/prompt.rs`（非 commands/pi.rs）：`get_pi_prompt_file`/`replace_pi_prompt_file`/`delete_pi_prompt_file`/`list_pi_prompt_templates`/`upsert_pi_prompt_template`/`delete_pi_prompt_template`。
- `services/prompt.rs` dispatch 加 `AppType::Pi` → `pi_prompt_files` service。
- `prompt_files.rs`：`prompt_file_path(AppType::Pi)` → `pi_config::get_pi_agent_dir()/AGENTS.md`。
- `PromptPanel.tsx`：`appId === "pi"` → 渲染 `PiPromptPanel`（含 templates/system files 标签）。

### Session usage（40d747c0）

```
session_usage_pi.rs → 读 ~/.pi/agent/sessions/*.jsonl → 解析 token/cost → DB insert
  → sync_all_unlocked 加 "Pi" merge_sync_step
  → v16→v17 迁移：session_usage_dedup_ledger（去重账本，通用基础设施）
  → UsageDashboard APP_FILTER_OPTIONS + types/usage.ts AppType 加 "pi"
```

- `services/session_usage.rs::sync_all_unlocked` 加 `merge_sync_step(&mut result, "Pi", session_usage_pi::sync_pi_usage(db))`。
- `services/mod.rs` 加 `pub mod session_usage_pi`。
- DB `SCHEMA_VERSION` 16→17；`migrate_v16_to_v17` 创建 dedup ledger 表。
- `session_manager/providers/pi.rs` scan_sessions/load_messages/session_roots（本地文件枚举，无进程）。
- `session_manager/mod.rs` scan_sessions 加 `pi::scan_sessions` handle。
- `types/usage.ts` AppType 加 `"pi"`，KNOWN_APP_TYPES 加 `"pi"`。
- `UsageDashboard.tsx`/`UsageHero.tsx` 加 pi filter + theme。

### model_fetch（84e75ad2）

- `services/model_fetch.rs` 加 `request_headers: Option<&BTreeMap<String,String>>` 参数 + `api_format` header 构建（pi 支持自定义 request headers）。
- `MAX_REQUEST_HEADERS = 64`，secret scrub 扩展覆盖 request_headers values。
- 通用增强，无 ClaudeDesktop 依赖，无新命令。

### i18n

- 新增独立 `"pi"` 顶层命名空间（empty/provider/form/current/prompts 子树，en/ja/zh 各 ~150 键）。
- `provider` 命名空间加散键：`needsRouting`/`noRoutingSupport`/`removeFromConfigFailed`/`piDefaultProviderWarning`。
- `settings` 命名空间加：`piConfigDir`/`piConfigDirDescription`/`browsePlaceholderPi`。
- `apps`/`appSwitcher` 加 `"pi": "Pi"`。
- `usage.appFilter` 加 `"pi": "Pi"`（40d747c0）。
- `oneClickInstallHint` 文案加 "Pi"。
- zh-TW hunk 全量丢弃（fork 仅 en/ja/zh）。
- fork 无 `"pi"` 顶层键，无碰撞。

### docs

- 移植 `docs/pi-native-contract-zh.md`（60 行，pi_config 行为的可审计契约 SSOT）。
- 排除 `pi-frontend-uiux-guidelines-zh.md`/`pi-thinking-level-map-requirements-zh.md`/`pi-live-provider-sync-requirements-zh.md`（UX/需求设计文档，fork docs/ 不收此类）。

## Carry-forward 碰撞处理

### ClaudeDesktop hunk 丢弃（20 文件，56 处）
- `app_config.rs`：上游 diff 同时含 `ClaudeDesktop` arm 和 `Pi` arm；fork 无 ClaudeDesktop → 只取 Pi arm，丢弃 ClaudeDesktop arm。
- `App.tsx`/`appConfig.tsx`/`AppSwitcher.tsx`/`ProviderList.tsx`/`ProviderCard.tsx`/`AddProviderDialog.tsx`/`EditProviderDialog.tsx`/`ProviderForm.tsx`/`ProviderPresetSelector.tsx`/`AppVisibilitySettings.tsx`：同理，取 pi 集成 hunk，丢弃 claude-desktop 引用。
- `services/{skill,mcp,provider/live,stream_check,prompt}.rs`/`database/dao/mcp.rs`/`prompt_files.rs`/`settings.rs`/`proxy/providers/mod.rs`：取 `AppType::Pi` arm，丢弃 `AppType::ClaudeDesktop` arm。

### zh-TW 丢弃
- 两提交均改 `zh-TW.json` → 全量 no-op（fork 无 zh-TW，locale gate 仅 en/ja/zh）。

### PromptPanel 非 pi 路径重构（Q3 裁定）
- 上游 PromptPanel.tsx 重构了非 pi 路径（删 ManagementListSearch + filteredPromptEntries，改用 `<PromptLibrary>`）。
- **裁定**：只取 pi dispatch hunk（`appId === "pi"` → PiPromptPanel），**不移植非 pi 路径重构**。fork 现有 claude/codex 等 prompt 列表 UI 保持原样，PromptLibrary 仅被 PiPromptPanel 内部使用。理由：非 pi 路径重构是独立 UI refactor，不属于 pi sync 范围；移植会改动现有 7 个 app 的 prompt 列表行为，扩大回归面。

## 批次依赖与顺序

```
P1 Rust 后端核心 ──┐
  (pi_config, services, commands, session_manager, AppType dispatch, web handlers)
P2 前端 provider UI ──┤  依赖 P1 的 Rust API
  (AppId union, appConfig, PiProviderForm, presets/catalog/thinking, lib/api/pi, query/pi, web-commands.ts FE)
P3 前端 prompts/skills/sessions UI ──┤  依赖 P1+P2
  (PiPromptPanel, PromptLibrary, PiNativePromptResources, ProviderStatusBadge, RequestHeadersEditor, StructuredOptionsEditor, skill 集成)
P4 40d747c0 session usage + docs + i18n 收口 + 全量门禁 ── 最后
  (session_usage_pi.rs, v17 迁移, UsageDashboard/Hero, usage types, pi-native-contract-zh.md, en/ja/zh pi keys)
```

P1 先行建立 Rust pi provider/prompt/session 基础；P2/P3 前端消费；P4 session usage 依赖 P1 的 session_manager provider + 收口 i18n/docs。

## Web API parity（新命令注册）

pi 新 Tauri 命令需在 `src/lib/api/web-commands.ts` 注册 + Axum handler + 过 `check:web-routes`：
- `get_pi_current_state` → `GET /api/pi/get-pi-current-state`
- `update_pi_provider_usage_script` → `POST /api/pi/update-pi-provider-usage-script`
- `get_pi_session_discovery` → `GET /api/pi/get-pi-session-discovery`
- `get_pi_prompt_file` → `GET /api/pi/get-pi-prompt-file`
- `replace_pi_prompt_file` → `POST /api/pi/replace-pi-prompt-file`
- `delete_pi_prompt_file` → `DELETE /api/pi/delete-pi-prompt-file`
- `list_pi_prompt_templates` → `GET /api/pi/list-pi-prompt-templates`
- `upsert_pi_prompt_template` → `POST /api/pi/upsert-pi-prompt-template`
- `delete_pi_prompt_template` → `DELETE /api/pi/delete-pi-prompt-template`

（具体 method/path 在 P1/P2 实现时按 fork 既有命名约定确定，过 `check:web-routes` 0 missing/mismatch/fallback）

## 兼容性与回滚

- 每批独立 commit → 单批失败可 `git reset` 回滚。
- DB 迁移 v16→v17 前 fork 既有备份流程；失败可从备份恢复。
- pi_config 本地文件 IO 保留 fork 既有 `atomic_write` + 2s deadline + heap/stack 上限等价口径。
- 子任务独立分支/独立 PR，回滚不影响父主体。

## 重要权衡

- **逐文件移植 vs 整体 cherry-pick**：逐文件移植避免 56 处 ClaudeDesktop 冲突 + zh-TW 冲突，但需手工对齐每个 AppType dispatch arm。已确认逐文件（沿用父任务方法论）。
- **PromptLibrary 非 pi 路径**：只移植 pi 需要的 PromptLibrary 组件本身 + PiPromptPanel 消费，不重构现有非 pi prompt 列表 UI。避免扩大回归面。
- **docs 范围**：只移植行为契约文档，不收 UX/需求设计文档。保持 fork docs/ 惯例（ADR/audit/guides/release-notes + 行为契约）。
- **usage AppType 扩展**：fork usage AppType 仅 5 个（无 openclaw/hermes/pi）。40d747c0 加 pi（有 session importer）。不连带加 openclaw/hermes（它们无 proxy handler 也无 session importer，usage 体系本就缺席）。
