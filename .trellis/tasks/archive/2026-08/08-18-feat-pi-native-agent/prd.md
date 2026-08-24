# PRD — pi native coding agent + session usage

> 父任务：`08-18-sync-upstream-v3.20.0`。本子任务独立规划/实现/归档。

## 范围

- `84e75ad2` feat(pi): add native coding agent support (#6064) — 156 文件（34 纯新增 / 117 修改 / 4 docs），+18,765 行
- `40d747c0` feat(pi): add session usage statistics (#6463) — 17 文件，+1,705 行（依赖 pi provider + session_manager）

## 已确认事实（代码库调研）

### Pi 运行模型
- Pi 是**本地配置文件管理的 additive provider**（类比 OpenCode/Hermes），**不是**进程式 agent、**不是** proxy provider。
- CC Switch 读写 `~/.pi/agent/{models.json, settings.json, AGENTS.md, SYSTEM.md, APPEND_SYSTEM.md, prompts/*.md, skills/}` + 会话 JSONL；原子写入 + revision 比较防跨进程覆盖。
- 契约（`docs/pi-native-contract-zh.md`）明确不做：`/login`、`auth.json` OAuth/API Key、路由/网关/代理/故障转移、内置供应商/模型目录复制、完整 compat/modelOverrides、相对会话目录全局猜测。
- additive 模式：启用 = 在 `models.json.providers` 增加节点；不写 `defaultProvider`/`defaultModel`（Pi 自管）。
- **Web-first fork 兼容性**：fork Web 模式已支持同类本地文件读写（session_manager 全系 `fs::read`）；pi 新 Tauri 命令（`get_pi_current_state`/`update_pi_provider_usage_script`/`get_pi_session_discovery`）仅本地文件 IO，无进程 spawn；`terminal/mod.rs` 改动仅 1 行（`shell_escape` 由 `fn` 改 `pub(crate)`，可见性，pi session_manager 复用）。→ **Pi 对 Web 完全兼容，无 desktop-only 阻塞**。

### DB 迁移
- `40d747c0` v16→v17（session_usage_dedup_ledger，会话用量持久去重账本）从 fork 当前 `SCHEMA_VERSION=16` **严格连续**，不跳号。
- v17 是 pi session usage 所需的通用去重基础设施（非 pi 专属表）。

### 冲突面（carry-forward 碰撞）
- **ClaudeDesktop 碰撞**：84e75ad2 建立在上游仍含 `AppType::ClaudeDesktop` 的基线；diff 中 **56 处 ClaudeDesktop/claude-desktop 引用横跨 20 个文件**（app_config.rs、App.tsx、appConfig.tsx、skill.rs、mcp.rs、provider/live.rs、ProviderForm.tsx 等）。fork 已移除 ClaudeDesktop → 这些 hunk 必须丢弃（同 S5 ClaudeDesktopProviderForm skip 模式）。
- **zh-TW**：两提交均改 `zh-TW.json`（158 行×2）→ 丢弃（fork 仅 en/ja/zh）。
- **结构**：34 纯新增文件（pi 核心，最低冲突）+ 117 修改文件（AppType dispatch 加 Pi arm + ClaudeDesktop arm 丢弃）。

### 新增文件清单（纯新增，pi 核心）
- Rust：`commands/pi.rs`、`pi_config/mod.rs`、`services/{pi_prompt_files,pi_state,provider/pi,session_usage_pi}.rs`、`session_manager/providers/pi.rs`
- 前端：`config/{piModelCatalog,piProviderPresets,piThinkingProfiles}.ts`、`lib/{api/pi,piPromptSlug,piPromptTemplate,query/pi}.ts`、`components/prompts/{PiNativePromptResources,PiPromptPanel,PromptLibrary}.tsx`、`components/providers/forms/{PiProviderForm,RequestHeadersEditor,StructuredOptionsEditor,helpers/requestHeaders}.tsx`、`components/providers/ProviderStatusBadge.tsx`
- 测试：13 个新测试文件
- docs：`pi-native-contract-zh.md`(60)、`pi-frontend-uiux-guidelines-zh.md`(292)、`pi-thinking-level-map-requirements-zh.md`(118)、`pi-live-provider-sync-requirements-zh.md`(126)

### Prompt 管理 UI
- `PromptPanel.tsx` 加 `appId === "pi"` → 渲染 `PiPromptPanel`（pi 专属，含 templates/system files 标签）。
- `PromptLibrary.tsx` 是通用列表组件（PromptListItem 包装 + 搜索），被 PiPromptPanel 复用；fork 现有 PromptPanel 可选择性采纳。
- `PromptFormPanel.tsx`：pi 用 `AGENTS.md` 文件名 + 不 trim 内容。

## 前置依赖
- 父任务 S2 安全基线已落地（无强阻塞；pi 为纯新增 provider，与主体冲突面最小）。
- `40d747c0` 依赖 `84e75ad2` 的 pi session_manager provider 落地。

## 约束（carry-forward）
- 双运行时：新命令注册 `src/lib/api/web-commands.ts`，过 `check:web-routes`；web-only feature-gate。
- 安全边界、无认证部署、updater 禁用不退化。
- ClaudeDesktop hunk 全量丢弃（fork 不支持）；zh-TW hunk 丢弃。
- `.pi/`、`.pi-subagents/` 不得修改或提交。
- pi_config 本地文件 IO 保留 fork 既有 atomic_write + 2s deadline + heap/stack 上限等价口径。

## 验收标准
- [x] pi provider 完成 Web API / browser UI / headless runtime 适配；9 个新命令注册 web-commands.ts 并过 check:web-routes（0 missing/mismatch/fallback）。
- [x] session usage statistics（40d747c0）在双运行时下正确计数；v16→v17 迁移连续不跳号。
- [x] ClaudeDesktop hunk 零回潮（grep `ClaudeDesktop`/`claude-desktop` 在 src-tauri/src/、src/ 为空，除既有 usage logger 兼容残留）；zh-TW hunk 零回潮。
- [x] 全量门禁通过：test:unit 全绿（非 flake 项）、test:integration（4 PRD flakes 外全绿；P4 全量另有 1 个 PromptPanel Gemini 时序 flake，隔离 3/3 通过）、Rust parity（web_api::/dual_runtime_parity::/web_proxy_lifecycle:: 全绿）、web-routes、locales（en/ja/zh parity）、build:web exit 0、smoke:web-server exit 0；与父主体无回归。
- [x] docs 处置：移植 `pi-native-contract-zh.md`；排除 3 个 UX/需求设计文档。
- [x] PromptPanel 非 pi 路径不被重构（fork 现有 claude/codex 等 prompt 列表 UI 保持原样，PromptLibrary 仅 PiPromptPanel 内部使用）。
- [x] 安全上限不退化：2s JS deadline、16 MiB heap、256 KiB stack、128 MiB body cap、32 MiB catalog cap 保留；pi_config `MAX_PI_FILE_BYTES=1MiB`。

## 裁定记录（brainstorm 2026-08-23，用户授权采纳推荐方案）
- **Q1 docs 范围**：仅移植 `pi-native-contract-zh.md`（行为契约 SSOT）；排除 `pi-frontend-uiux-guidelines-zh.md`/`pi-thinking-level-map-requirements-zh.md`/`pi-live-provider-sync-requirements-zh.md`（UX/需求设计，fork docs/ 不收）。
- **Q2 执行分批**：逐文件 selective port（非整体 cherry-pick），分 P1–P4 四批（Rust 后端 → 前端 provider UI → 前端 prompts/skills/sessions UI → session usage+docs+i18n 收口）。详见 design.md/implement.md。
- **Q3 PromptPanel 非 pi 路径**：只取 pi dispatch hunk，不移植非 pi 路径 PromptLibrary 重构（避免扩大回归面）。
- **Q4 model_fetch**：通用增强（request_headers + api_format），随 P1 落地。
- **Q5 proxy 层**：仅 `providers/mod.rs` 加 `Pi => return None`，无其他 proxy 改动。
- **Q6 i18n**：独立 `pi` 顶层命名空间 + 散键，无碰撞；zh-TW 丢弃。
- **usage AppType**：fork usage AppType 仅 5 个，40d747c0 加 pi（有 session importer）；不连带加 openclaw/hermes。
